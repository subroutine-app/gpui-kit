//! The right-click menu of selectable text.

use std::rc::Rc;

use gpui::{App, Global, Pixels, Point, SharedString, Window};
use gpui_base::{CopyTextSelection, SelectAllText, TextSelection, TextSelectionMenuRequest};
use rust_i18n::t;

use crate::{input::ContextMenuStyle, native_menu::NativeMenu};

/// What the context menu of selectable text opened on: the window's
/// selection and what the text under the pointer can do.
///
/// A right-click on a selectable [`super::TextView`] or
/// [`gpui_base::SelectableText`] opens the menu, and so do Shift-F10 and the
/// Menu key in a focused text view.
pub struct TextSelectionMenuTarget {
    request: TextSelectionMenuRequest,
}

impl TextSelectionMenuTarget {
    /// Where the menu opens, in window coordinates.
    pub fn position(&self) -> Point<Pixels> {
        self.request.position()
    }

    /// The selected text, as Copy and Cmd-C or Ctrl-C copy it, across every
    /// view the selection spans and with a blank line between paragraphs. It
    /// is empty when nothing is selected.
    pub fn selected_text(&self) -> &SharedString {
        self.request.selected_text()
    }

    /// Whether any text is selected.
    pub fn has_selection(&self) -> bool {
        self.request.has_selection()
    }

    /// Whether the text the menu opened on can select all of itself, as a
    /// text view can.
    pub fn supports_select_all(&self) -> bool {
        self.request.supports_select_all()
    }

    /// Add Copy, disabled when nothing is selected, then Select All when the
    /// text the menu opened on can select all of itself.
    pub fn standard_items(&self, menu: NativeMenu) -> NativeMenu {
        let menu = menu.menu_with_disabled(
            t!("Input.Copy"),
            !self.has_selection(),
            Box::new(CopyTextSelection),
        );
        if self.supports_select_all() {
            menu.separator()
                .menu(t!("Input.Select All"), Box::new(SelectAllText))
        } else {
            menu
        }
    }
}

type MenuBuilder =
    Rc<dyn Fn(NativeMenu, &TextSelectionMenuTarget, &mut Window, &mut App) -> NativeMenu>;

/// The builder [`TextSelectionMenu::set_builder`] set.
struct TextSelectionMenuBuilder(MenuBuilder);

impl Global for TextSelectionMenuBuilder {}

/// The context menu of selectable text, for the whole application.
///
/// It shows Copy and Select All (see
/// [`TextSelectionMenuTarget::standard_items`]) in the application's
/// [`ContextMenuStyle`], set with [`ContextMenuStyle::set_app_default`].
pub struct TextSelectionMenu;

impl TextSelectionMenu {
    /// Build the menu with `builder`, which receives an empty menu and
    /// returns the one to show, for example the standard items with the
    /// application's own after them:
    ///
    /// ```ignore
    /// TextSelectionMenu::set_builder(
    ///     |menu, target, _, _| {
    ///         target
    ///             .standard_items(menu)
    ///             .separator()
    ///             .menu_with_disabled("Search the Web", !target.has_selection(), Box::new(Search))
    ///     },
    ///     cx,
    /// );
    /// ```
    ///
    /// An empty menu shows nothing. Setting a builder again replaces it.
    pub fn set_builder(
        builder: impl Fn(NativeMenu, &TextSelectionMenuTarget, &mut Window, &mut App) -> NativeMenu
        + 'static,
        cx: &mut App,
    ) {
        cx.set_global(TextSelectionMenuBuilder(Rc::new(builder)));
    }
}

pub(crate) fn init(cx: &mut App) {
    TextSelection::on_context_menu(show, cx);
}

fn show(request: &TextSelectionMenuRequest, window: &mut Window, cx: &mut App) {
    let target = TextSelectionMenuTarget {
        request: request.clone(),
    };
    let menu = match cx
        .try_global::<TextSelectionMenuBuilder>()
        .map(|builder| builder.0.clone())
    {
        Some(build) => build(NativeMenu::new(), &target, window, cx),
        None => target.standard_items(NativeMenu::new()),
    };
    ContextMenuStyle::resolve(None, cx).show(menu, target.position(), window, cx);
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, Modifiers,
        MouseButton, ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext,
        Window, div, point, px,
    };
    use gpui_base::{CopyTextSelection, SelectAllText, SelectableText, TextSelection};

    use super::{TextSelectionMenu, TextSelectionMenuTarget};
    use crate::{
        Root,
        input::{ContextMenuStyle, Input, InputState},
        menu::PopupMenu,
        text::{TextView, TextViewState},
    };

    /// What each menu that opened knew: the selected text and whether it
    /// offered Select All.
    type Opened = Rc<RefCell<Vec<(String, bool)>>>;

    struct Page {
        input: Entity<InputState>,
        first: Entity<TextViewState>,
        second: Entity<TextViewState>,
    }

    impl Render for Page {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let area = |selector: &'static str| div().debug_selector(move || selector.into());
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(area("input").h(px(30.)).child(Input::new(&self.input)))
                .child(
                    area("first")
                        .h(px(90.))
                        .child(TextView::new(&self.first).selectable(true)),
                )
                .child(
                    area("second")
                        .h(px(40.))
                        .child(TextView::new(&self.second).selectable(true)),
                )
                .child(
                    area("plain")
                        .h(px(20.))
                        .child(SelectableText::new("plain", "Plain words")),
                )
        }
    }

    fn open(cx: &mut TestAppContext) -> (Entity<Page>, Opened, &mut VisualTestContext) {
        cx.update(crate::init);
        let opened: Opened = Rc::default();
        cx.update(|cx| {
            let opened = opened.clone();
            TextSelectionMenu::set_builder(
                move |menu, target: &TextSelectionMenuTarget, _, _| {
                    opened.borrow_mut().push((
                        target.selected_text().to_string(),
                        target.supports_select_all(),
                    ));
                    target.standard_items(menu)
                },
                cx,
            );
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| Page {
                input: cx.new(|cx| InputState::new(window, cx).default_value("typed")),
                first: cx.new(|cx| TextViewState::markdown("One fish\n\nTwo fish", cx)),
                second: cx.new(|cx| TextViewState::markdown("Red fish", cx)),
            });
            page = Some(view.clone());
            Root::new(view, window, cx)
        });
        draw(cx);
        (page.unwrap(), opened, cx)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    fn bounds(selector: &'static str, cx: &mut VisualTestContext) -> gpui::Bounds<gpui::Pixels> {
        cx.debug_bounds(selector).expect(selector)
    }

    fn right_click(position: gpui::Point<gpui::Pixels>, cx: &mut VisualTestContext) {
        cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(position, MouseButton::Right, Modifiers::default());
        draw(cx);
    }

    fn drag(
        from: gpui::Point<gpui::Pixels>,
        to: gpui::Point<gpui::Pixels>,
        cx: &mut VisualTestContext,
    ) {
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        draw(cx);
        cx.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::default());
        draw(cx);
        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
        draw(cx);
    }

    fn selected_text(cx: &mut VisualTestContext) -> String {
        cx.update(|window, cx| TextSelection::selected_text(window, cx).trim().to_string())
    }

    /// Where the items of the drawn menu showing are.
    fn drawn_menu(cx: &mut VisualTestContext) -> Option<gpui::Bounds<gpui::Pixels>> {
        cx.update(|window, cx| {
            let overlay = crate::root::WindowState::native_menu_overlay(window, cx)?;
            let menu: Entity<PopupMenu> = overlay.read(cx).active_menu()?;
            Some(menu.read(cx).item_bounds())
        })
    }

    /// The drawn menu opens in the app's style, and pressing its Copy
    /// copies the selection without clearing it first.
    #[gpui::test]
    fn the_drawn_menu_copies_the_selection(cx: &mut TestAppContext) {
        let (_, _, cx) = open(cx);
        cx.update(|_, cx| ContextMenuStyle::set_app_default(ContextMenuStyle::Drawn, cx));
        let first = bounds("first", cx);
        drag(
            first.origin + point(px(1.), px(4.)),
            first.origin + point(px(40.), px(10.)),
            cx,
        );
        let selection = selected_text(cx);
        assert!(!selection.is_empty());

        right_click(first.origin + point(px(10.), px(8.)), cx);
        assert_eq!(
            cx.update(|_, cx| ContextMenuStyle::shown(cx)),
            [ContextMenuStyle::Drawn]
        );
        let items = drawn_menu(cx).expect("the drawn menu opened");
        let copy = point(items.center().x, items.top() + px(17.));
        cx.simulate_mouse_move(copy, None, Modifiers::default());
        cx.simulate_click(copy, Modifiers::default());
        draw(cx);

        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some(selection.clone())
        );
        assert_eq!(
            selected_text(cx),
            selection,
            "pressing the menu keeps the selection"
        );
        assert!(drawn_menu(cx).is_none(), "the menu closed");
    }

    /// A right-click on a selection across two text views keeps it and opens
    /// the menu. Its Copy copies what Ctrl-C copies, the paragraphs apart,
    /// though a text field has focus.
    #[gpui::test]
    fn the_menu_copies_the_selection(cx: &mut TestAppContext) {
        let (page, opened, cx) = open(cx);
        let first = bounds("first", cx);
        let second = bounds("second", cx);
        drag(
            first.origin + point(px(1.), px(4.)),
            second.origin + point(px(30.), px(10.)),
            cx,
        );
        let selection = selected_text(cx);
        assert!(
            selection.starts_with("One fish\n\nTwo fish\n\nRe"),
            "{selection:?}"
        );
        let input = cx.update(|_, cx| page.read(cx).input.clone());
        cx.update(|window, cx| input.update(cx, |input, cx| input.focus(window, cx)));

        right_click(first.origin + point(px(20.), px(8.)), cx);
        assert_eq!(
            selected_text(cx),
            selection,
            "the right-click keeps the selection"
        );
        assert_eq!(
            opened.borrow().last().cloned(),
            Some((selection.clone(), true))
        );

        cx.dispatch_action(CopyTextSelection);
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some(selection)
        );
    }

    /// Select All selects the whole text view the menu opened on, and
    /// nothing else.
    #[gpui::test]
    fn select_all_selects_the_text_view(cx: &mut TestAppContext) {
        let (_, opened, cx) = open(cx);
        let first = bounds("first", cx);
        drag(
            first.origin + point(px(1.), px(4.)),
            first.origin + point(px(30.), px(10.)),
            cx,
        );
        right_click(bounds("second", cx).origin + point(px(10.), px(8.)), cx);
        assert!(opened.borrow().last().unwrap().1);

        cx.dispatch_action(SelectAllText);
        draw(cx);
        assert_eq!(selected_text(cx), "Red fish");
    }

    /// A text field keeps its own menu. Plain selectable text has no Select
    /// All, and nothing is selected for Copy.
    #[gpui::test]
    fn which_text_gets_the_menu(cx: &mut TestAppContext) {
        let (_, opened, cx) = open(cx);
        right_click(bounds("input", cx).center(), cx);
        assert!(opened.borrow().is_empty());

        right_click(bounds("plain", cx).origin + point(px(10.), px(8.)), cx);
        assert_eq!(opened.borrow_mut().pop(), Some((String::new(), false)));
    }

    /// Selectable text inside an element with a `ContextMenuExt` menu of its
    /// own opens that menu alone.
    #[gpui::test]
    fn an_element_with_its_own_menu_keeps_the_right_click(cx: &mut TestAppContext) {
        use std::cell::Cell;

        use crate::menu::ContextMenuExt as _;

        struct OwnMenu {
            built: Rc<Cell<usize>>,
        }

        impl Render for OwnMenu {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let built = self.built.clone();
                div().size_full().child(
                    div()
                        .id("own-menu")
                        .debug_selector(|| "own-menu".into())
                        .h(px(20.))
                        .child(SelectableText::new("own", "Words with a menu"))
                        .context_menu(move |menu, _, _| {
                            built.set(built.get() + 1);
                            menu.label("Its own")
                        }),
                )
            }
        }

        let (_, opened, _) = open(cx);
        let built = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| OwnMenu {
                built: built.clone(),
            });
            Root::new(view, window, cx)
        });
        draw(cx);

        let position = bounds("own-menu", cx).origin + point(px(10.), px(8.));
        cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
        assert!(
            cx.update(|_, cx| gpui_base::GlobalState::is_text_selection_suppressed(cx)),
            "the element takes the press from selectable text"
        );
        cx.simulate_mouse_up(position, MouseButton::Right, Modifiers::default());
        draw(cx);
        assert_eq!(built.get(), 1, "the element's own menu opened");
        assert!(opened.borrow().is_empty());
        assert!(cx.update(|_, cx| ContextMenuStyle::shown(cx)).is_empty());
    }

    /// A right-click on a link the application handles goes to its handler
    /// alone. On a link without a handler it opens the menu.
    #[gpui::test]
    fn links_with_a_handler_keep_the_right_click(cx: &mut TestAppContext) {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        struct Links {
            plain: Entity<TextViewState>,
            handled: Entity<TextViewState>,
            clicks: Arc<AtomicUsize>,
        }

        impl Render for Links {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let clicks = self.clicks.clone();
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .debug_selector(|| "plain".into())
                            .h(px(30.))
                            .child(TextView::new(&self.plain).selectable(true)),
                    )
                    .child(div().debug_selector(|| "handled".into()).h(px(30.)).child(
                        TextView::new(&self.handled).selectable(true).on_link_click(
                            move |_, _, _, _| {
                                clicks.fetch_add(1, Ordering::SeqCst);
                            },
                        ),
                    ))
            }
        }

        let (_, opened, _) = open(cx);
        let clicks = Arc::new(AtomicUsize::new(0));
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| Links {
                plain: cx.new(|cx| TextViewState::markdown("[example](https://example.com)", cx)),
                handled: cx.new(|cx| TextViewState::markdown("[example](https://example.com)", cx)),
                clicks: clicks.clone(),
            });
            Root::new(view, window, cx)
        });
        draw(cx);

        right_click(bounds("handled", cx).origin + point(px(10.), px(10.)), cx);
        assert_eq!(clicks.load(Ordering::SeqCst), 1);
        assert!(opened.borrow().is_empty());

        right_click(bounds("plain", cx).origin + point(px(10.), px(10.)), cx);
        assert_eq!(opened.borrow().len(), 1);
        assert_eq!(cx.opened_url(), None);
    }

    /// Shift-F10 and the Menu key open the menu of a focused text view.
    #[gpui::test]
    fn the_keyboard_opens_the_menu(cx: &mut TestAppContext) {
        let (_, opened, cx) = open(cx);
        let second = bounds("second", cx);
        cx.simulate_mouse_move(
            second.origin + point(px(10.), px(8.)),
            None,
            Modifiers::default(),
        );
        cx.simulate_click(second.origin + point(px(10.), px(8.)), Modifiers::default());
        draw(cx);
        for key in ["shift-f10", "menu"] {
            cx.simulate_keystrokes(key);
            draw(cx);
            assert_eq!(
                opened.borrow_mut().pop(),
                Some((String::new(), true)),
                "{key}"
            );
        }
    }
}
