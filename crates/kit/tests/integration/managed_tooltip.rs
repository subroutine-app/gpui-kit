//! Kit-managed tooltips: they leave with their trigger, switch along a row of
//! buttons, and stay off a trigger whose menu or popover is open.

use crate::common;
use gpui_kit::component::{
    IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem},
    popover::Popover,
    tooltip::{ManagedTooltipExt as _, Tooltip},
};
use gpui_kit::test::{TestSupportExt as _, TestWindowExt as _};
use gpui_kit::{
    App, AppContext as _, Context, Entity, InputEvent as _, Modifiers, MouseButton, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, Render, Size, TestAppContext, VisualTestContext, Window, div,
    point, prelude::*, px, size,
};
use std::time::Duration;

/// Longer than any show delay or grace period.
const LONG: Duration = Duration::from_secs(2);

/// A window showing the view `build` makes, with motion as `reduce_motion`
/// says.
fn open<V: Render>(
    cx: &mut TestAppContext,
    window_size: Size<Pixels>,
    reduce_motion: bool,
    build: impl FnOnce(&mut App) -> Entity<V>,
) -> (Entity<V>, VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(reduce_motion);
    });
    let (window, view) = common::open_window(cx, Some(window_size), |_, cx| build(cx));
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    draw(&mut cx);
    (view, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn wait(cx: &mut VisualTestContext, duration: Duration) {
    cx.executor().advance_clock(duration);
    draw(cx);
}

fn painted(cx: &mut VisualTestContext) -> bool {
    cx.debug_bounds("tooltip").is_some()
}

/// Whether the tooltip painted is the one of the element named `name`.
fn shows_tooltip_of(name: &'static str, cx: &mut VisualTestContext) -> bool {
    let trigger = cx.debug_bounds(name).expect("the trigger is painted");
    cx.debug_bounds("tooltip")
        .is_some_and(|tooltip| (tooltip.center().x - trigger.center().x).abs() < px(1.))
}

/// Moves the pointer onto the middle of the element named `name`.
fn point_at(name: &'static str, cx: &mut VisualTestContext) -> Point<Pixels> {
    let center = cx
        .debug_bounds(name)
        .expect("the trigger is painted")
        .center();
    cx.simulate_mouse_move(center, None, Modifiers::default());
    draw(cx);
    center
}

/// A page with a close button, like a settings page that closes itself.
struct Page {
    open: bool,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = cx.entity();
        div().size_full().p(px(20.)).when(self.open, |this| {
            this.child(
                Button::new("close")
                    .debug_selector(|| "close".into())
                    .label("Close")
                    .tooltip("Close Settings")
                    .on_click(move |_, _, cx| {
                        page.update(cx, |page, cx| {
                            page.open = false;
                            cx.notify();
                        })
                    }),
            )
        })
    }
}

fn page(cx: &mut TestAppContext) -> (Entity<Page>, VisualTestContext) {
    open(cx, size(px(400.), px(300.)), true, |cx| {
        cx.new(|_| Page { open: true })
    })
}

/// Rests the pointer on the close button until its tooltip shows.
fn hover_until_shown(cx: &mut VisualTestContext) -> Point<Pixels> {
    let center = point_at("close", cx);
    assert!(!painted(cx), "a tooltip waits for the show delay");
    wait(cx, LONG);
    assert!(painted(cx), "a resting pointer shows the tooltip");
    center
}

fn close(page: &Entity<Page>, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        page.update(cx, |page, cx| {
            page.open = false;
            cx.notify();
        })
    });
}

fn assert_gone_for_good(cx: &mut VisualTestContext) {
    assert!(
        cx.debug_bounds("close").is_none(),
        "the button left the window"
    );
    assert!(
        !painted(cx),
        "the tooltip disappears with the frame that drops its button"
    );
    wait(cx, LONG);
    assert!(!painted(cx), "and does not come back");
}

/// The click that removes the button, as Windows delivers it: capturing the
/// mouse on press and releasing it on release each produce a mouse move at the
/// same position, and the release's move arrives before the frame that drops
/// the button.
#[gpui_kit::test]
fn clicking_a_button_that_closes_its_page_takes_the_tooltip_with_it(cx: &mut TestAppContext) {
    let (_, mut cx) = page(cx);
    let cx = &mut cx;
    let position = hover_until_shown(cx);

    cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(position, MouseButton::Left, Modifiers::default());
    cx.update(|window, cx| {
        window.dispatch_event(
            MouseUpEvent {
                position,
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            MouseMoveEvent {
                position,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }
            .to_platform_input(),
            cx,
        );
    });
    draw(cx);

    assert_gone_for_good(cx);
}

/// A shortcut can close the page while the pointer rests on the button.
#[gpui_kit::test]
fn a_shown_tooltip_disappears_when_its_button_is_removed(cx: &mut TestAppContext) {
    let (page, mut cx) = page(cx);
    let cx = &mut cx;
    hover_until_shown(cx);

    close(&page, cx);
    draw(cx);

    assert_gone_for_good(cx);
}

/// The show timer can fire after the update that removes the button and
/// before the frame that drops it.
#[gpui_kit::test]
fn a_tooltip_shown_after_its_button_was_removed_is_never_painted(cx: &mut TestAppContext) {
    let (page, mut cx) = page(cx);
    let cx = &mut cx;
    point_at("close", cx);

    cx.update(|_, cx| page.update(cx, |page, _| page.open = false));
    cx.executor().advance_clock(LONG);
    draw(cx);

    assert_gone_for_good(cx);
}

#[gpui_kit::test]
fn a_tooltip_hides_after_the_pointer_leaves(cx: &mut TestAppContext) {
    let (_, mut cx) = page(cx);
    let cx = &mut cx;
    hover_until_shown(cx);

    cx.simulate_mouse_move(point(px(390.), px(290.)), None, Modifiers::default());
    draw(cx);
    assert!(painted(cx), "the tooltip outlasts the pointer briefly");
    wait(cx, LONG);
    assert!(!painted(cx), "then hides");
}

/// A view drawn as a cached view, which GPUI reuses from the last frame while
/// only the window around it renders again.
struct Cached<V: 'static>(Entity<V>);

impl<V: Render> Render for Cached<V> {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            self.0
                .clone()
                .cached(gpui_kit::StyleRefinement::default().size_full()),
        )
    }
}

#[gpui_kit::test]
fn a_tooltip_stays_while_its_button_is_reused_from_the_last_frame(cx: &mut TestAppContext) {
    let (shell, mut cx) = open(cx, size(px(400.), px(300.)), true, |cx| {
        let page = cx.new(|_| Page { open: true });
        cx.new(|_| Cached(page))
    });
    let cx = &mut cx;
    hover_until_shown(cx);

    for _ in 0..3 {
        cx.update(|_, cx| shell.update(cx, |_, cx| cx.notify()));
        draw(cx);
        assert!(painted(cx), "the reused button keeps its tooltip");
    }
    wait(cx, LONG);
    assert!(painted(cx), "for as long as the pointer rests on it");
}

/// Two buttons with tooltips, a pointer's skim apart.
struct Pair;

impl Render for Pair {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p(px(20.))
            .flex()
            .gap(px(80.))
            .child(
                Button::new("a")
                    .debug_selector(|| "a".into())
                    .label("Alpha")
                    .tooltip("Tip A"),
            )
            .child(
                Button::new("b")
                    .debug_selector(|| "b".into())
                    .label("Beta")
                    .tooltip("Tip B"),
            )
    }
}

/// Moving from one button to the next in a reused view while something else
/// redraws the window, such as streaming text: the first tooltip stays through
/// its grace period and until the second replaces it.
#[gpui_kit::test]
fn a_redraw_elsewhere_keeps_a_reused_buttons_tooltip_through_its_grace(cx: &mut TestAppContext) {
    let (window_view, mut cx) = open(cx, size(px(500.), px(200.)), true, |cx| {
        let pair = cx.new(|_| Pair);
        cx.new(|_| Cached(pair))
    });
    let cx = &mut cx;
    let a = cx.debug_bounds("a").expect("a").center();
    let b = cx.debug_bounds("b").expect("b").center();
    let gap = point((a.x + b.x) / 2., a.y + px(60.));

    cx.simulate_mouse_move(a, None, Modifiers::default());
    draw(cx);
    wait(cx, LONG);
    assert!(painted(cx), "A's tooltip shows");

    cx.simulate_mouse_move(gap, None, Modifiers::default());
    draw(cx);
    cx.update(|_, cx| window_view.update(cx, |_, cx| cx.notify()));
    draw(cx);
    assert!(painted(cx), "a redraw elsewhere keeps it through its grace");

    wait(cx, Duration::from_millis(50));
    cx.simulate_mouse_move(b, None, Modifiers::default());
    draw(cx);
    cx.update(|_, cx| window_view.update(cx, |_, cx| cx.notify()));
    draw(cx);
    assert!(painted(cx), "A's tooltip stays until B's replaces it");
    wait(cx, LONG);
    assert!(
        shows_tooltip_of("b", cx),
        "B's shows and stays while the pointer rests on B"
    );
}

/// Icon buttons side by side, as under a message.
struct Actions;

impl Render for Actions {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p(px(20.))
            .pl(px(200.))
            .flex()
            .children(["Edit", "Branch", "Copy"].map(|name| {
                Button::new(name)
                    .debug_selector(move || name.into())
                    .ghost()
                    .xsmall()
                    .icon(IconName::Copy)
                    .tooltip(name)
            }))
    }
}

fn actions(cx: &mut TestAppContext, reduce_motion: bool) -> VisualTestContext {
    open(cx, size(px(500.), px(200.)), reduce_motion, |cx| {
        cx.new(|_| Actions)
    })
    .1
}

/// GPUI reports the pointer entering the next button before it reports the
/// pointer leaving the last one when the pointer moves right.
#[gpui_kit::test]
fn moving_along_adjacent_buttons_shows_each_ones_tooltip(cx: &mut TestAppContext) {
    let mut cx = actions(cx, true);
    let cx = &mut cx;

    for name in ["Edit", "Branch", "Copy", "Branch", "Edit"] {
        point_at(name, cx);
        wait(cx, LONG);
        assert!(
            shows_tooltip_of(name, cx),
            "{name}'s tooltip shows while the pointer rests on it"
        );
    }
}

/// A pointer sweeping across the row leaves the open tooltip as it is.
#[gpui_kit::test]
fn sweeping_across_buttons_keeps_the_open_tooltip(cx: &mut TestAppContext) {
    let mut cx = actions(cx, true);
    let cx = &mut cx;
    point_at("Edit", cx);
    wait(cx, LONG);

    point_at("Branch", cx);
    wait(cx, Duration::from_millis(40));
    point_at("Copy", cx);
    wait(cx, Duration::from_millis(40));
    assert!(shows_tooltip_of("Edit", cx), "the open tooltip stays");
    wait(cx, LONG);
    assert!(
        shows_tooltip_of("Copy", cx),
        "the button the pointer rests on takes over"
    );
}

/// Hiding never waits for the switch's animation.
#[gpui_kit::test]
fn a_click_during_a_switch_hides_the_tooltip_at_once(cx: &mut TestAppContext) {
    let mut cx = actions(cx, false);
    let cx = &mut cx;
    point_at("Edit", cx);
    wait(cx, LONG);
    let branch = point_at("Branch", cx);
    // The switch delay, then partway through the switch.
    wait(cx, Duration::from_millis(100));
    wait(cx, Duration::from_millis(30));
    assert!(painted(cx), "the tooltip is switching");

    cx.simulate_mouse_down(branch, MouseButton::Left, Modifiers::default());
    draw(cx);
    assert!(!painted(cx), "the next frame has no tooltip");
}

/// Buttons with tooltips that open a menu, a popover and a context menu.
struct Triggers;

impl Render for Triggers {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let item =
            |_: &mut Window, _: &mut App| div().debug_selector(|| "item".into()).child("Stop all");
        // Clear of the left edge, which would push a tooltip off center.
        div()
            .size_full()
            .p(px(20.))
            .pl(px(200.))
            .flex()
            .gap(px(80.))
            .child(
                Button::new("menu")
                    .debug_selector(|| "menu".into())
                    .icon(IconName::Ellipsis)
                    .tooltip("More")
                    .dropdown_menu(move |menu, _, _| menu.item(PopupMenuItem::element(item))),
            )
            .child(
                Popover::new("popover")
                    .trigger(
                        Button::new("usage")
                            .debug_selector(|| "usage".into())
                            .label("Usage")
                            .tooltip("Usage"),
                    )
                    .content(|_, _, _| {
                        Button::new("inside")
                            .debug_selector(|| "inside".into())
                            .label("Inside")
                            .tooltip("Inside")
                    }),
            )
            .child(
                Button::new("row")
                    .debug_selector(|| "row".into())
                    .label("Row")
                    .tooltip("Row")
                    .context_menu(move |menu, _, _| menu.item(PopupMenuItem::element(item))),
            )
    }
}

fn triggers(cx: &mut TestAppContext) -> VisualTestContext {
    open(cx, size(px(600.), px(300.)), true, |cx| {
        cx.new(|_| Triggers)
    })
    .1
}

/// Takes the pointer off every trigger and brings it back onto the top-left
/// corner of the one named `name`, clear of a context menu opened at its
/// middle, resting long enough each time for a tooltip.
fn leave_and_return(name: &'static str, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(point(px(590.), px(290.)), None, Modifiers::default());
    draw(cx);
    wait(cx, LONG);
    let corner = cx
        .debug_bounds(name)
        .expect("the trigger is painted")
        .origin;
    cx.simulate_mouse_move(corner + point(px(3.), px(3.)), None, Modifiers::default());
    draw(cx);
    wait(cx, LONG);
}

/// Opens the popup of the trigger named `name` with `open`, moves the pointer
/// away and back, closes it, and does the same again. The popup is open while
/// an element named `surface` is painted.
fn assert_no_tooltip_while_open(
    name: &'static str,
    surface: &'static str,
    open: impl FnOnce(Point<Pixels>, &mut VisualTestContext),
    cx: &mut VisualTestContext,
) {
    let center = point_at(name, cx);
    wait(cx, LONG);
    assert!(
        shows_tooltip_of(name, cx),
        "a closed trigger shows its tooltip"
    );

    open(center, cx);
    draw(cx);
    assert!(cx.debug_bounds(surface).is_some(), "the popup opens");
    assert!(!painted(cx), "opening the popup hides the tooltip");

    leave_and_return(name, cx);
    assert!(cx.debug_bounds(surface).is_some(), "the popup stays open");
    assert!(
        !painted(cx),
        "the pointer coming back to the trigger shows no tooltip over its popup"
    );

    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(
        cx.debug_bounds(surface).is_none(),
        "escape closes the popup"
    );

    leave_and_return(name, cx);
    assert!(
        shows_tooltip_of(name, cx),
        "once closed, the trigger shows its tooltip again"
    );
}

#[gpui_kit::test]
fn a_button_shows_no_tooltip_while_its_dropdown_menu_is_open(cx: &mut TestAppContext) {
    let mut cx = triggers(cx);
    assert_no_tooltip_while_open(
        "menu",
        "item",
        |center, cx| cx.simulate_click(center, Modifiers::default()),
        &mut cx,
    );
}

#[gpui_kit::test]
fn a_button_shows_no_tooltip_while_its_popover_is_open(cx: &mut TestAppContext) {
    let mut cx = triggers(cx);
    assert_no_tooltip_while_open(
        "usage",
        "inside",
        |center, cx| cx.simulate_click(center, Modifiers::default()),
        &mut cx,
    );
}

/// A right click leaves the tooltip up until the menu opens.
#[gpui_kit::test]
fn an_element_shows_no_tooltip_while_its_context_menu_is_open(cx: &mut TestAppContext) {
    let mut cx = triggers(cx);
    assert_no_tooltip_while_open(
        "row",
        "item",
        |center, cx| {
            cx.simulate_mouse_down(center, MouseButton::Right, Modifiers::default());
            cx.simulate_mouse_up(center, MouseButton::Right, Modifiers::default());
            draw(cx);
        },
        &mut cx,
    );
}

#[gpui_kit::test]
fn tooltips_inside_an_open_popover_still_show(cx: &mut TestAppContext) {
    let mut cx = triggers(cx);
    let cx = &mut cx;
    let usage = point_at("usage", cx);
    cx.simulate_click(usage, Modifiers::default());
    draw(cx);

    point_at("inside", cx);
    wait(cx, LONG);
    assert!(shows_tooltip_of("inside", cx));
}

/// A custom element, such as a tree heading, with a managed tooltip and a
/// context menu.
struct Heading;

impl Render for Heading {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            div()
                .id("heading")
                .test_support()
                .w_40()
                .h_10()
                .child("Project")
                .managed_tooltip(|window, cx| Tooltip::new("Project folder").build(window, cx))
                .context_menu(|menu, _, _| menu.item(PopupMenuItem::new("Open folder"))),
        )
    }
}

#[gpui_kit::test]
fn a_custom_heading_uses_managed_tooltips_and_open_menu_suppression(cx: &mut TestAppContext) {
    let (_, mut cx) = open(cx, size(px(400.), px(300.)), true, |cx| cx.new(|_| Heading));
    let cx = &mut cx;
    let move_to = |x, y, cx: &mut VisualTestContext| {
        cx.simulate_mouse_move(point(px(x), px(y)), None, Modifiers::default());
        draw(cx);
        wait(cx, LONG);
    };

    move_to(20., 20., cx);
    assert!(painted(cx));
    cx.update(|window, cx| window.right_click("heading", cx));
    move_to(390., 290., cx);
    move_to(20., 20., cx);
    cx.update(|window, _| assert!(window.try_find("popup-menu").is_some()));
    assert!(!painted(cx));
    cx.update(|window, cx| window.press("escape", cx));
    move_to(390., 290., cx);
    move_to(20., 20., cx);
    assert!(painted(cx));
}
