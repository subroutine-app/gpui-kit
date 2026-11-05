use gpui::{
    AnyElement, App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable as _,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Point, Render, Styled as _,
    Subscription, Window, anchored, deferred, div, point, px,
};

use crate::{Placement, menu::PopupMenu, root::WindowState};

/// Shows a [`PopupMenu`] at a window position on request, for menus a view
/// opens from code (a pointer press it handles itself, a toolbar button, a
/// keyboard command) rather than by right-clicking an element wrapped with
/// [`ContextMenuExt`](crate::menu::ContextMenuExt).
///
/// Mount the entity once in the window's view tree; where it sits does not
/// affect the menu's placement. The menu takes focus when it opens and
/// restores the previous focus when it is dismissed, plays the popup entrance,
/// and is drawn above the rest of the window. Opening another menu replaces the
/// one that is showing. The layer emits [`DismissEvent`] when its menu closes.
///
/// ```ignore
/// let layer = cx.new(|_| ContextMenuLayer::new());
/// // later, from any handler with the window:
/// layer.update(cx, |layer, cx| {
///     layer.open(position, window, cx, |menu, _, _| {
///         menu.item(PopupMenuItem::new("Rename").on_click(|_, _, _| {}))
///     })
/// });
/// ```
pub struct ContextMenuLayer {
    menu: Option<OpenMenu>,
    /// On the layer's dispatch path every frame and never focused, so shortcut
    /// hints resolve against the key contexts around the layer on the frame a
    /// menu opens, before the menu itself has been rendered.
    anchor_focus: Option<FocusHandle>,
}

struct OpenMenu {
    menu: Entity<PopupMenu>,
    position: Point<Pixels>,
    _dismissed: Subscription,
}

impl Default for ContextMenuLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextMenuLayer {
    pub fn new() -> Self {
        Self {
            menu: None,
            anchor_focus: None,
        }
    }

    /// Builds a menu with `build` and shows it with its top-left corner at
    /// `position`, in window coordinates, kept inside the window.
    ///
    /// A menu without items is not shown.
    pub fn open(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
        build: impl FnOnce(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu,
    ) {
        let previous_focus = self.focus_to_restore(window, cx);
        let menu = PopupMenu::build(window, cx, build);
        if menu.read(cx).is_empty() {
            return;
        }

        let anchor_focus = self.anchor_focus(cx);
        menu.update(cx, |menu, cx| {
            menu.entrance = Some(Placement::Bottom);
            menu.set_trigger_focus(Some(anchor_focus), cx);
            menu.set_previous_focus(previous_focus, cx);
        });
        // A menu opened from code has no trigger whose tooltips it could
        // suppress, so the one that is up would otherwise cover it.
        if let Some(overlay) = WindowState::tooltip_overlay(window, cx) {
            overlay.update(cx, |overlay, cx| overlay.hide(cx));
        }

        let dismissed = cx.subscribe(&menu, |layer, menu, _: &DismissEvent, cx| {
            if layer.is_showing(&menu) {
                layer.close(cx);
            }
        });
        self.menu = Some(OpenMenu {
            menu: menu.clone(),
            position,
            _dismissed: dismissed,
        });

        // Focus once the event that opened the menu has finished dispatching,
        // so a press that would focus what it landed on cannot take focus back,
        // and still before the menu's first frame, which reports the focused
        // accessibility node.
        let layer = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            let showing = layer
                .read_with(cx, |layer, _| layer.is_showing(&menu))
                .unwrap_or(false);
            if showing {
                menu.focus_handle(cx).focus(window, cx);
            }
        });
        cx.notify();
    }

    /// Closes the menu without activating an item. Focus stays where it is.
    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        if self.menu.is_some() {
            self.close(cx);
        }
    }

    pub fn is_open(&self) -> bool {
        self.menu.is_some()
    }

    /// Whether `position`, in window coordinates, is over the open menu or one
    /// of its open submenus.
    pub fn contains(&self, position: Point<Pixels>, cx: &App) -> bool {
        self.menu
            .as_ref()
            .is_some_and(|open| open.menu.read(cx).contains(position, cx))
    }

    /// Whether focus is in the open menu or one of its submenus.
    pub fn contains_focus(&self, window: &Window, cx: &App) -> bool {
        self.menu
            .as_ref()
            .is_some_and(|open| open.menu.read(cx).focus_handle.contains_focused(window, cx))
    }

    fn is_showing(&self, menu: &Entity<PopupMenu>) -> bool {
        self.menu.as_ref().is_some_and(|open| &open.menu == menu)
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        cx.emit(DismissEvent);
        cx.notify();
    }

    /// The focus a replaced menu would have restored, or the current focus.
    fn focus_to_restore(&self, window: &Window, cx: &App) -> Option<FocusHandle> {
        match &self.menu {
            Some(open) if open.menu.read(cx).focus_handle.contains_focused(window, cx) => {
                open.menu.read(cx).previous_focus_handle.clone()
            }
            _ => window.focused(cx),
        }
    }

    fn anchor_focus(&mut self, cx: &mut App) -> FocusHandle {
        self.anchor_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone()
    }

    fn render_menu(open: &OpenMenu, window: &Window) -> AnyElement {
        let viewport = window.viewport_size();
        deferred(
            anchored().position(point(px(0.), px(0.))).child(
                div()
                    .w(viewport.width)
                    .h(viewport.height)
                    .on_scroll(|_, _, cx| cx.stop_propagation())
                    .child(
                        anchored()
                            .position(open.position)
                            .snap_to_window_with_margin(px(8.))
                            .child(open.menu.clone()),
                    ),
            ),
        )
        .with_priority(gpui_base::POPUP_PRIORITY)
        .into_any_element()
    }
}

impl EventEmitter<DismissEvent> for ContextMenuLayer {}

impl Render for ContextMenuLayer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let anchor_focus = self.anchor_focus(cx);
        div()
            .absolute()
            .size_0()
            .track_focus(&anchor_focus)
            .children(
                self.menu
                    .as_ref()
                    .map(|open| Self::render_menu(open, window)),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{AppContext as _, TestAppContext, VisualTestContext};

    use super::*;
    use crate::menu::PopupMenuItem;

    struct Host {
        layer: Entity<ContextMenuLayer>,
    }

    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(self.layer.clone())
        }
    }

    fn redraw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| crate::test_redraw(window, cx));
    }

    #[gpui::test]
    fn a_menu_opened_from_code_runs_its_item_and_closes(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            cx.set_reduce_motion(true);
        });
        let (host, cx) = cx.add_window_view(|_, cx| Host {
            layer: cx.new(|_| ContextMenuLayer::new()),
        });
        let layer = host.read_with(cx, |host, _| host.layer.clone());
        redraw(cx);

        let clicked = Rc::new(Cell::new(0));
        let position = point(px(40.), px(40.));
        cx.update(|window, cx| {
            let clicked = clicked.clone();
            layer.update(cx, |layer, cx| {
                layer.open(position, window, cx, move |menu, _, _| {
                    menu.item(
                        PopupMenuItem::element(|_, _| {
                            div().debug_selector(|| "item".into()).w(px(80.)).h(px(20.))
                        })
                        .on_click(move |_, _, _| clicked.set(clicked.get() + 1)),
                    )
                })
            });
        });
        cx.run_until_parked();
        redraw(cx);

        let item = cx.debug_bounds("item").expect("the menu is drawn");
        assert!(item.origin.x >= position.x && item.origin.y >= position.y);
        assert!(cx.update(|window, cx| layer.read(cx).contains_focus(window, cx)));
        assert!(layer.read_with(cx, |layer, cx| layer.contains(item.center(), cx)));

        cx.simulate_click(item.center(), Default::default());
        redraw(cx);
        assert_eq!(clicked.get(), 1);
        assert!(!layer.read_with(cx, |layer, _| layer.is_open()));
        assert!(!layer.read_with(cx, |layer, cx| layer.contains(item.center(), cx)));
        assert!(cx.debug_bounds("item").is_none());
    }
}
