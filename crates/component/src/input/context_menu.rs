//! What draws a text field's right-click menu.

use gpui::{App, Global, Pixels, Point, Window};

use crate::native_menu::NativeMenu;

/// What draws a text field's right-click menu.
///
/// Inputs, textareas and code editors use [`Self::Native`] unless the
/// application chooses one style for every text field with
/// [`Self::set_app_default`], the ones inside other components too.
/// `context_menu_style` on [`super::Input`], [`super::Textarea`] or
/// [`super::Editor`] sets the style of one field, which wins over the
/// application's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContextMenuStyle {
    /// A [`PopupMenu`](crate::menu::PopupMenu) drawn by GPUI in the theme's
    /// colors, on every platform. It stays inside the window.
    Drawn,
    /// The operating system's menu on macOS and Windows, which can reach past
    /// the window's edge. Other platforms have none, and show the drawn menu.
    #[default]
    Native,
}

/// The style [`ContextMenuStyle::set_app_default`] chose.
struct AppContextMenuStyle(ContextMenuStyle);

impl Global for AppContextMenuStyle {}

impl ContextMenuStyle {
    /// Use `style` for the right-click menu of every text field that sets no
    /// style of its own with `context_menu_style`, and of selectable text.
    ///
    /// This reaches the fields that other components build for themselves,
    /// such as the search field of [`crate::setting::Settings`] or of a
    /// searchable [`crate::select::Select`], which an application can't style
    /// one by one. A menu reads the style as it opens, so calling this again
    /// changes the next menu of every field.
    ///
    /// ```ignore
    /// gpui_kit::init(cx);
    /// ContextMenuStyle::set_app_default(ContextMenuStyle::Drawn, cx);
    /// ```
    pub fn set_app_default(style: Self, cx: &mut App) {
        cx.set_global(AppContextMenuStyle(style));
    }

    /// The style [`Self::set_app_default`] chose, or `None` when the
    /// application chose none.
    pub fn app_default(cx: &App) -> Option<Self> {
        cx.try_global::<AppContextMenuStyle>().map(|style| style.0)
    }

    /// The style a menu opens in: the field's `own` when it set one, else the
    /// application's default, else [`Self::Native`].
    pub(crate) fn resolve(own: Option<Self>, cx: &App) -> Self {
        own.or_else(|| Self::app_default(cx)).unwrap_or_default()
    }

    /// Whether the operating system draws a menu of this style on `os`, a
    /// [`std::env::consts::OS`] value.
    pub(crate) fn uses_os_menu(self, os: &str) -> bool {
        self == Self::Native && matches!(os, "macos" | "windows")
    }

    /// Show `menu` at `position` in this style.
    pub(crate) fn show(
        self,
        menu: NativeMenu,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        #[cfg(test)]
        cx.default_global::<ShownMenuStyles>().0.push(self);
        if self.uses_os_menu(std::env::consts::OS) {
            menu.show(position, window, cx);
        } else {
            menu.show_drawn(position, window, cx);
        }
    }

    /// The styles menus opened in so far, oldest first, for tests.
    #[cfg(test)]
    pub(crate) fn shown(cx: &App) -> Vec<Self> {
        cx.try_global::<ShownMenuStyles>()
            .map(|shown| shown.0.clone())
            .unwrap_or_default()
    }
}

/// The styles menus opened in, for tests.
#[cfg(test)]
#[derive(Default)]
struct ShownMenuStyles(Vec<ContextMenuStyle>);

#[cfg(test)]
impl Global for ShownMenuStyles {}

#[cfg(test)]
mod tests {
    use gpui::{
        AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, Modifiers,
        MouseButton, ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext,
        Window, div, px,
    };

    use super::ContextMenuStyle;
    use crate::input::{Editor, EditorState, Input, InputState, Textarea, TextareaState};

    /// One field of each kind, and an input that sets its own style.
    struct Fields {
        input: Entity<InputState>,
        native_input: Entity<InputState>,
        textarea: Entity<TextareaState>,
        editor: Entity<EditorState>,
    }

    impl Render for Fields {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let field =
                |selector: &'static str| div().w_full().debug_selector(move || selector.into());
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(field("input").child(Input::new(&self.input)))
                .child(field("native-input").child(
                    Input::new(&self.native_input).context_menu_style(ContextMenuStyle::Native),
                ))
                .child(field("textarea").child(Textarea::new(&self.textarea).h(px(60.))))
                .child(field("editor").child(Editor::new(&self.editor).h(px(60.))))
        }
    }

    fn open(cx: &mut TestAppContext) -> &mut VisualTestContext {
        cx.update(crate::init);
        let (_, cx) = cx.add_window_view(|window, cx| Fields {
            input: cx.new(|cx| InputState::new(window, cx).default_value("one")),
            native_input: cx.new(|cx| InputState::new(window, cx).default_value("two")),
            textarea: cx.new(|cx| TextareaState::new(window, cx).default_value("three")),
            editor: cx.new(|cx| EditorState::new(window, cx).default_value("four")),
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx
    }

    /// Right-click the field `selector` names, and return the style its menu
    /// opened in.
    fn menu_style(selector: &'static str, cx: &mut VisualTestContext) -> ContextMenuStyle {
        let position = cx.debug_bounds(selector).expect(selector).center();
        cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
        cx.simulate_mouse_up(position, MouseButton::Right, Modifiers::default());
        cx.run_until_parked();
        let shown = cx.update(|_, cx| ContextMenuStyle::shown(cx));
        *shown.last().expect("the menu opened")
    }

    /// Without an application default every field keeps the native menu. An
    /// application default reaches every field without a style of its own,
    /// as its next menu opens, and a field's own style still wins.
    #[gpui::test]
    fn fields_follow_the_app_default(cx: &mut TestAppContext) {
        use ContextMenuStyle::{Drawn, Native};
        let cx = open(cx);
        let fields = ["input", "native-input", "textarea", "editor"];
        let styles = |cx: &mut VisualTestContext| fields.map(|field| menu_style(field, cx));

        assert_eq!(cx.update(|_, cx| ContextMenuStyle::app_default(cx)), None);
        assert_eq!(styles(cx), [Native; 4]);

        cx.update(|_, cx| ContextMenuStyle::set_app_default(Drawn, cx));
        assert_eq!(
            cx.update(|_, cx| ContextMenuStyle::app_default(cx)),
            Some(Drawn)
        );
        assert_eq!(styles(cx), [Drawn, Native, Drawn, Drawn]);

        cx.update(|_, cx| ContextMenuStyle::set_app_default(Native, cx));
        assert_eq!(styles(cx), [Native; 4]);
    }

    /// The drawn style is drawn by GPUI everywhere. The native one is the
    /// operating system's where there is one.
    #[test]
    fn only_the_native_style_uses_the_os_menu() {
        for os in ["macos", "windows"] {
            assert!(ContextMenuStyle::Native.uses_os_menu(os));
            assert!(!ContextMenuStyle::Drawn.uses_os_menu(os));
        }
        assert!(!ContextMenuStyle::Native.uses_os_menu("linux"));
    }
}
