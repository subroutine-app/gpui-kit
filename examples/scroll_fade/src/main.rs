use gpui_kit::{component::button::Button, *};

const DIV_ROWS: usize = 60;
const UNIFORM_ROWS: usize = 1000;
const LIST_ROWS: usize = 200;
const FADE: Pixels = px(40.);

/// One scroll container of each kind, edge-faded over a striped backdrop so the fade is visibly
/// to transparent. A notched mouse wheel eases each of them; the buttons scroll programmatically.
struct Example {
    div_scroll: ScrollHandle,
    uniform_scroll: UniformListScrollHandle,
    list: ListState,
}

impl Example {
    fn new() -> Self {
        Self {
            div_scroll: ScrollHandle::new(),
            uniform_scroll: UniformListScrollHandle::new(),
            // Animated list scrolls need the heights of the rows they cross.
            list: ListState::new(LIST_ROWS, ListAlignment::Top, px(200.)).measure_all(),
        }
    }

    fn div_column(&self, cx: &mut Context<Self>) -> impl IntoElement {
        column(
            "div + ScrollHandle",
            div()
                .flex()
                .gap_2()
                .child(Button::new("div-top").label("Top").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.div_scroll.set_offset_animated(Point::default());
                        cx.notify();
                    },
                )))
                .child(Button::new("div-row").label("Row 30").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.div_scroll.scroll_to_item_animated(30);
                        cx.notify();
                    },
                )))
                .child(
                    Button::new("div-bottom")
                        .label("Bottom")
                        .on_click(cx.listener(|this, _, _, cx| {
                            let bottom = -this.div_scroll.max_offset().y;
                            this.div_scroll.set_offset_animated(point(px(0.), bottom));
                            cx.notify();
                        })),
                ),
            div()
                .id("div-scroll")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.div_scroll)
                .rounded_xl()
                .bg(hsla(0.65, 0.25, 0.12, 0.85))
                .fade_edges(vertical_fade())
                .children((0..DIV_ROWS).map(|ix| row(ix, px(44.)))),
        )
    }

    fn uniform_column(&self, cx: &mut Context<Self>) -> impl IntoElement {
        column(
            "uniform_list",
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("uniform-top")
                        .label("Top")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.uniform_scroll
                                .scroll_to_item_animated(0, ScrollStrategy::Top);
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("uniform-item")
                        .label("Item 500")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.uniform_scroll
                                .scroll_to_item_animated(500, ScrollStrategy::Center);
                            cx.notify();
                        })),
                ),
            uniform_list("uniform-list", UNIFORM_ROWS, |range, _, _| {
                range.map(|ix| row(ix, px(32.))).collect()
            })
            .track_scroll(&self.uniform_scroll)
            .size_full()
            .rounded_xl()
            .bg(hsla(0.45, 0.25, 0.12, 0.85))
            .fade_edges(vertical_fade()),
        )
    }

    fn list_column(&self, cx: &mut Context<Self>) -> impl IntoElement {
        column(
            "list (variable height)",
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("list-page")
                        .label("Page down")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.list.scroll_by_animated(px(320.));
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("list-reveal")
                        .label("Reveal 150")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.list.scroll_to_reveal_item_animated(150);
                            cx.notify();
                        })),
                ),
            // `list` lays out its items but paints none of its own style, so the background and
            // fade belong to a wrapper.
            div()
                .size_full()
                .rounded_xl()
                .bg(hsla(0.05, 0.25, 0.12, 0.85))
                .fade_edges(Edges {
                    top: FADE,
                    bottom: px(96.),
                    ..Default::default()
                })
                .child(
                    list(self.list.clone(), |ix, _, _| {
                        row(ix, px(28. + (ix % 5) as f32 * 14.)).into_any_element()
                    })
                    .size_full(),
                ),
        )
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(rgb(0x101018))
            .text_color(white())
            .child(stripes())
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .p_6()
                    .flex()
                    .gap_6()
                    .child(self.div_column(cx))
                    .child(self.uniform_column(cx))
                    .child(self.list_column(cx)),
            )
    }
}

fn vertical_fade() -> Edges<Pixels> {
    Edges {
        top: FADE,
        bottom: FADE,
        ..Default::default()
    }
}

fn column(title: &'static str, controls: impl IntoElement, body: impl IntoElement) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_3()
        .child(div().text_lg().child(title))
        .child(controls)
        .child(div().flex_1().min_h_0().child(body))
}

fn row(ix: usize, height: Pixels) -> Div {
    let hue = (ix % 24) as f32 / 24.;
    div()
        .w_full()
        .h(height)
        .flex()
        .flex_none()
        .items_center()
        .gap_3()
        .px_4()
        .border_b_1()
        .border_color(hsla(0., 0., 1., 0.08))
        .child(
            div()
                .size(px(14.))
                .rounded_full()
                .bg(hsla(hue, 0.8, 0.6, 1.)),
        )
        .child(format!("Row {ix}"))
}

/// Saturated stripes behind the lists, so a fade to transparent is distinguishable from a fade
/// to a color.
fn stripes() -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .children((0..48).map(|ix| {
            div()
                .flex_1()
                .h_full()
                .bg(hsla(ix as f32 / 48., 0.7, 0.4, 1.))
        }))
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);

        let bounds = Bounds::centered(None, size(px(1000.), px(640.)), cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(|_| Example::new()),
        )
        .expect("Failed to open window");
        cx.activate(true);
    });
}
