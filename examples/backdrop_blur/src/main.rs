use gpui_kit::*;

const TILE_COUNT: usize = 160;

struct Example;

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(rgb(0x101018))
            .child(busy_backdrop())
            .child(frosted_panel())
            .child(blurred_badge())
            .child(deferred(
                anchored()
                    .position(point(px(32.), px(420.)))
                    .child(overlay_card()),
            ))
    }
}

/// Saturated, high-frequency content, so a blur behind a panel is unmistakable.
fn busy_backdrop() -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .flex_wrap()
        .gap_1()
        .p_2()
        .children((0..TILE_COUNT).map(|ix| {
            let hue = ix as f32 / TILE_COUNT as f32;
            div()
                .w(px(52.))
                .h(px(36.))
                .rounded_sm()
                .bg(hsla(hue, 0.85, 0.55, 1.))
                .text_xs()
                .text_color(black())
                .flex()
                .items_center()
                .justify_center()
                .child(format!("{ix:03}"))
        }))
}

/// Backdrop blur clipped to rounded corners, tinted by a translucent fill.
fn frosted_panel() -> impl IntoElement {
    div()
        .absolute()
        .top(px(80.))
        .left(px(220.))
        .w(px(440.))
        .h(px(240.))
        .rounded(px(24.))
        .backdrop_blur(px(24.))
        .bg(hsla(0., 0., 1., 0.18))
        .border_1()
        .border_color(hsla(0., 0., 1., 0.45))
        .p_6()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(white())
        .child(div().text_2xl().child("Frosted glass"))
        .child("backdrop_blur(24px) under a translucent .bg() tint, clipped to 24px corners.")
}

/// Content blur: the element and its children are blurred as one group.
fn blurred_badge() -> impl IntoElement {
    div()
        .absolute()
        .top(px(360.))
        .left(px(520.))
        .px_4()
        .py_2()
        .rounded_full()
        .bg(rgb(0xffffff))
        .text_color(black())
        .blur(px(4.))
        .child("Content blur: .blur(4px)")
}

/// A deferred overlay must blur the main scene beneath it.
fn overlay_card() -> impl IntoElement {
    div()
        .w(px(300.))
        .p_4()
        .rounded(px(14.))
        .backdrop_filter(vec![Filter::Blur(px(12.))])
        .bg(hsla(0.62, 0.3, 0.12, 0.45))
        .border_1()
        .border_color(hsla(0., 0., 1., 0.25))
        .text_color(white())
        .child("Deferred overlay with backdrop_filter([Filter::Blur(12px)])")
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);

        let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(|_| Example),
        )
        .expect("Failed to open window");
        cx.activate(true);
    });
}
