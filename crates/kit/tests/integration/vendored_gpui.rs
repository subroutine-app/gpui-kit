//! Behavior the vendored GPUI backend adds (see `vendor/README.md`): eased notched-wheel
//! scrolling, animated programmatic scrolling, and held element drags. Time is the test
//! executor's clock, so every frame below samples a known point of a transition.
use crate::common;
use gpui_kit::test::{TestSupportExt, TestWindowExt};
use gpui_kit::{
    AppContext, Context, InputEvent, IntoElement, ListAlignment, ListState, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollDelta, ScrollHandle,
    ScrollStrategy, TestAppContext, UniformListScrollHandle, Window, WindowHandle, base::Root, div,
    list, point, prelude::*, px, size, uniform_list,
};
use std::{cell::Cell, rc::Rc, time::Duration};

const ROW: Pixels = px(40.);
/// Past both the 80 ms wheel transition and the 200 ms animated-scroll transition.
const SETTLED: Duration = Duration::from_millis(250);

fn row(ix: usize) -> impl IntoElement {
    div().h(ROW).flex_none().child(format!("Row {ix}"))
}

/// A 100 px wide column of each scroll container, 200 px tall: 20 rows in the div (range
/// 600 px), 100 in the uniform list, and 50 in the list (range 1800 px once measured).
struct Containers {
    scroll: ScrollHandle,
    uniform: UniformListScrollHandle,
    list: ListState,
}

impl Containers {
    fn new(list: ListState) -> Self {
        Self {
            scroll: ScrollHandle::new(),
            uniform: UniformListScrollHandle::new(),
            list,
        }
    }
}

impl Render for Containers {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .child(
                div()
                    .id("rows")
                    .test_support()
                    .w(px(100.))
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children((0..20).map(row)),
            )
            .child(
                uniform_list("uniform", 100, |range, _, _| range.map(row).collect())
                    .track_scroll(&self.uniform)
                    .w(px(100.))
                    .h_full(),
            )
            .child(
                div().id("list").test_support().w(px(100.)).h_full().child(
                    list(self.list.clone(), |ix, _, _| row(ix).into_any_element()).size_full(),
                ),
            )
    }
}

/// A list whose scroll range covers every row, as it only spans rows of known height.
fn measured_list() -> ListState {
    ListState::new(50, ListAlignment::Top, px(0.)).measure_all()
}

fn open(
    cx: &mut TestAppContext,
    list: ListState,
) -> (WindowHandle<Root>, gpui_kit::Entity<Containers>) {
    let (handle, view) = common::open_window(cx, Some(size(px(300.), px(200.))), |_, cx| {
        cx.new(|_| Containers::new(list))
    });
    frame(cx, handle, Duration::ZERO);
    (handle, view)
}

fn frame(cx: &mut TestAppContext, handle: WindowHandle<Root>, after: Duration) {
    cx.executor().advance_clock(after);
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();
}

fn wheel(cx: &mut TestAppContext, handle: WindowHandle<Root>, target: &str, delta: ScrollDelta) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.scroll(target.to_string(), delta, cx)
    })
    .unwrap();
}

fn assert_close(actual: Pixels, expected: Pixels) {
    assert!(
        (actual - expected).abs() < px(0.01),
        "expected {expected:?}, got {actual:?}"
    );
}

fn list_offset(view: &gpui_kit::Entity<Containers>, cx: &mut TestAppContext) -> Pixels {
    view.read_with(cx, |view, _| view.list.scroll_px_offset_for_scrollbar().y)
}

#[gpui_kit::test]
fn notched_wheel_eases_accumulates_and_stays_in_range(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, measured_list());
    let scroll = view.read_with(cx, |view, _| view.scroll.clone());
    let line = cx
        .update_window(handle.into(), |_, window, _| window.line_height())
        .unwrap();

    wheel(cx, handle, "rows", ScrollDelta::Lines(point(0., -3.)));
    assert_eq!(
        scroll.offset().y,
        px(0.),
        "a notch eases instead of jumping"
    );
    frame(cx, handle, Duration::from_millis(40));
    let partway = scroll.offset().y;
    assert!(partway < px(0.) && partway > -line * 3., "{partway:?}");

    // A notch during the motion extends its target without dropping any distance.
    wheel(cx, handle, "rows", ScrollDelta::Lines(point(0., -3.)));
    frame(cx, handle, SETTLED);
    assert_close(scroll.offset().y, -line * 6.);

    // The target is clamped to the range: no overshoot on the way, none at rest.
    wheel(cx, handle, "rows", ScrollDelta::Lines(point(0., -1000.)));
    frame(cx, handle, Duration::from_millis(20));
    assert!(scroll.offset().y > px(-600.));
    frame(cx, handle, SETTLED);
    assert_close(scroll.offset().y, px(-600.));

    // Trackpad deltas stay immediate.
    wheel(
        cx,
        handle,
        "rows",
        ScrollDelta::Pixels(point(px(0.), px(50.))),
    );
    assert_close(scroll.offset().y, px(-550.));

    // `list` eases and clamps its notches the same way.
    wheel(cx, handle, "list", ScrollDelta::Lines(point(0., -1000.)));
    assert_close(list_offset(&view, cx), px(0.));
    frame(cx, handle, Duration::from_millis(20));
    let partway = list_offset(&view, cx);
    assert!(partway < px(0.) && partway > px(-1800.), "{partway:?}");
    frame(cx, handle, SETTLED);
    assert_close(list_offset(&view, cx), px(-1800.));
}

#[gpui_kit::test]
fn animated_scrolls_ease_to_their_targets(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, measured_list());
    let (scroll, uniform) =
        view.read_with(cx, |view, _| (view.scroll.clone(), view.uniform.clone()));
    let uniform_offset = || uniform.0.borrow().base_handle.offset().y;

    view.read_with(cx, |view, _| {
        view.scroll.set_offset_animated(point(px(0.), px(-300.)));
        view.uniform
            .scroll_to_item_animated(40, ScrollStrategy::Top);
        view.list.scroll_to_reveal_item_animated(30);
    });
    frame(cx, handle, Duration::ZERO);
    frame(cx, handle, Duration::from_millis(100));
    for (offset, target) in [
        (scroll.offset().y, px(-300.)),
        (uniform_offset(), px(-1600.)),
        (list_offset(&view, cx), px(-1040.)),
    ] {
        assert!(
            offset < px(0.) && offset > target,
            "{offset:?} toward {target:?}"
        );
    }

    frame(cx, handle, SETTLED);
    assert_close(scroll.offset().y, px(-300.));
    assert_close(uniform_offset(), px(-1600.));
    // Item 30 ends at 1240 px, so revealing it scrolls its bottom to the viewport's.
    assert_close(list_offset(&view, cx), px(-1040.));

    // An animated target is clamped to the range measured by the last layout.
    scroll.set_offset_animated(point(px(0.), px(-5000.)));
    frame(cx, handle, Duration::ZERO);
    frame(cx, handle, SETTLED);
    assert_close(scroll.offset().y, px(-600.));
}

#[gpui_kit::test]
fn reduced_motion_scrolls_without_easing(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (handle, view) = open(cx, measured_list());
    let scroll = view.read_with(cx, |view, _| view.scroll.clone());
    let line = cx
        .update_window(handle.into(), |_, window, _| window.line_height())
        .unwrap();

    wheel(cx, handle, "rows", ScrollDelta::Lines(point(0., -3.)));
    assert_close(scroll.offset().y, -line * 3.);

    scroll.set_offset_animated(point(px(0.), px(-300.)));
    frame(cx, handle, Duration::ZERO);
    assert_close(scroll.offset().y, px(-300.));
}

struct Grab;
impl Render for Grab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct HeldDrag {
    clicks: Rc<Cell<usize>>,
    grabbed_at: Rc<Cell<Option<Point<Pixels>>>>,
}

impl Render for HeldDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let grabbed_at = self.grabbed_at.clone();
        div().size_full().child(
            div()
                .id("held")
                .test_support()
                .size(px(40.))
                .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
                .on_drag(Grab, move |_, offset, _, cx| {
                    grabbed_at.set(Some(offset));
                    cx.new(|_| Grab)
                })
                .drag_hold(Duration::from_millis(200)),
        )
    }
}

#[gpui_kit::test]
fn drag_hold_defers_the_drag_and_keeps_the_press_point(cx: &mut TestAppContext) {
    let clicks = Rc::new(Cell::new(0));
    let grabbed_at = Rc::new(Cell::new(None));
    let (handle, _) = common::open_window(cx, Some(size(px(100.), px(100.))), {
        let (clicks, grabbed_at) = (clicks.clone(), grabbed_at.clone());
        move |_, cx| cx.new(|_| HeldDrag { clicks, grabbed_at })
    });
    let origin = cx
        .update_window(handle.into(), |_, window, cx| {
            window.render_frame(cx);
            window.find("held").bounds().origin
        })
        .unwrap();
    let at = |x: f32, y: f32| origin + point(px(x), px(y));
    let pointer = |cx: &mut TestAppContext, event: gpui_kit::PlatformInput| {
        cx.update_window(handle.into(), |_, window, cx| {
            window.dispatch_event(event, cx);
            window.render_frame(cx);
        })
        .unwrap();
    };
    let down = |position| {
        MouseDownEvent {
            position,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input()
    };
    let moved = |position| {
        MouseMoveEvent {
            position,
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
        }
        .to_platform_input()
    };
    let up = |position| {
        MouseUpEvent {
            position,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
        }
        .to_platform_input()
    };

    // Movement past the drag threshold during the hold is not a drag; the press still clicks.
    pointer(cx, down(at(10., 10.)));
    pointer(cx, moved(at(20., 10.)));
    assert!(
        !cx.update(|cx| cx.has_active_drag()),
        "a flick is not a drag"
    );
    pointer(cx, up(at(20., 10.)));
    assert_eq!(clicks.get(), 1, "a flick still resolves as a click");

    // After the hold, the first movement drags, grabbing the element where it was pressed.
    pointer(cx, down(at(10., 10.)));
    pointer(cx, moved(at(14., 10.)));
    assert!(!cx.update(|cx| cx.has_active_drag()));
    cx.executor().advance_clock(Duration::from_millis(200));
    pointer(cx, moved(at(30., 30.)));
    assert!(cx.update(|cx| cx.has_active_drag()));
    assert_eq!(grabbed_at.get(), Some(point(px(10.), px(10.))));
}
