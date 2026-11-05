use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Icon, IconName, Sizable as _, StyledExt as _, Theme,
    ThemeStyled as _,
    glass::{Glass, GlassStyled as _, GlassSurface},
    h_flex,
    switch::Switch,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Div, Entity, Focusable, Hsla, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, black, div, px, white,
};

use crate::section;

const LINES: [&str; 9] = [
    "Glass blurs whatever is painted behind it and lays a theme color over the blur.",
    "Stripes, text and edges stay recognizable as color and movement, not as detail.",
    "Frosted covers most of the backdrop; Liquid lets more of it through.",
    "Overlay, Panel, Canvas and Control take their color from the active theme.",
    "Reduce transparency paints every glass surface as its opaque tinted fill.",
    "The blur is clipped to the element's own corner radius.",
    "Text contrast protection raises coverage until body text reads at 4.5:1.",
    "Glass blurs whatever is painted behind it and lays a theme color over the blur.",
    "Stripes, text and edges stay recognizable as color and movement, not as detail.",
];

pub struct GlassStory {
    focus_handle: gpui_kit::FocusHandle,
}

impl super::Story for GlassStory {
    fn title() -> &'static str {
        "Glass"
    }

    fn description() -> &'static str {
        "Frosted and Liquid surfaces that blur the content behind them."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl GlassStory {
    pub fn view(_: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        })
    }

    fn render_presets(&self, cx: &App) -> impl IntoElement {
        let reduced = cx.theme().glass.reduce_transparency;
        section("Presets")
            .description(
                "Frosted is a strong blur under a mostly opaque fill. Liquid is a lighter \
                 blur under a thin fill with a white rim; there is no refraction.",
            )
            .sub_title(
                Switch::new("glass-reduce-transparency")
                    .small()
                    .label("Reduce transparency")
                    .checked(reduced)
                    .on_click(|checked, _, cx| {
                        let reduce = *checked;
                        Theme::update(cx, |theme| theme.glass.reduce_transparency = reduce);
                    }),
            )
            .child(
                stage(px(220.), cx).child(
                    h_flex()
                        .absolute()
                        .inset_0()
                        .p_6()
                        .gap_6()
                        .child(card(
                            "Frosted",
                            "Strong blur, 75% fill.",
                            Glass::frosted(),
                            cx,
                        ))
                        .child(card(
                            "Liquid",
                            "Light blur, thin fill, raised for legible text.",
                            Glass::liquid(),
                            cx,
                        ))
                        .child(card(
                            "Liquid, caller-owned legibility",
                            "Text contrast protection off.",
                            Glass::liquid().protect_text_contrast(false),
                            cx,
                        )),
                ),
            )
    }

    fn render_surfaces(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        section("Surfaces")
            .description(
                "Each role takes its color and elevation from the theme: Overlay from \
                 popover, Panel from sidebar, Canvas from background, Control from secondary.",
            )
            .child(
                stage(px(300.), cx)
                    .child(
                        h_flex()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .h_10()
                            .px_4()
                            .items_center()
                            .text_sm()
                            .font_medium()
                            .border_b_1()
                            .border_color(theme.border)
                            .glass(Glass::frosted().surface(GlassSurface::Canvas), cx)
                            .child("Canvas"),
                    )
                    .child(
                        v_flex()
                            .absolute()
                            .top_12()
                            .left_3()
                            .bottom_3()
                            .w_48()
                            .p_2()
                            .gap_1()
                            .text_sm()
                            .rounded(theme.radius_lg)
                            .glass(Glass::frosted().surface(GlassSurface::Panel), cx)
                            .child(panel_row(IconName::Inbox, "Inbox"))
                            .child(panel_row(IconName::Calendar, "Calendar"))
                            .child(panel_row(IconName::Star, "Starred"))
                            .child(panel_row(IconName::Settings, "Settings")),
                    )
                    .child(div().absolute().top_16().left(px(232.)).w_72().child(card(
                        "Overlay",
                        "Menus, popovers and dialogs float on this surface.",
                        Glass::frosted(),
                        cx,
                    )))
                    .child(
                        h_flex()
                            .absolute()
                            .bottom_4()
                            .right_4()
                            .gap_2()
                            .child(pill(
                                "glass-pill-search",
                                IconName::Search,
                                "Search",
                                false,
                                cx,
                            ))
                            .child(pill(
                                "glass-pill-today",
                                IconName::Calendar,
                                "Today",
                                true,
                                cx,
                            ))
                            .child(pill("glass-pill-add", IconName::Plus, "Add", false, cx)),
                    ),
            )
    }

    fn render_tints(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        let ink = if theme.is_dark() { black() } else { white() };
        section("Tint")
            .description(
                "A tint replaces the surface color; the preset still decides how much of \
                 it covers the backdrop.",
            )
            .child(
                stage(px(200.), cx).child(
                    h_flex()
                        .absolute()
                        .inset_0()
                        .p_6()
                        .gap_6()
                        .child(card(
                            "Deepened overlay",
                            "Popover color mixed 65% toward the page's extreme.",
                            Glass::frosted().tint(theme.popover.mix_oklab(ink, 0.35)),
                            cx,
                        ))
                        .child(card(
                            "Accent",
                            "The accent color over a Liquid blur.",
                            Glass::liquid().tint(theme.accent),
                            cx,
                        )),
                ),
            )
    }
}

impl Focusable for GlassStory {
    fn focus_handle(&self, _: &App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GlassStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .child(self.render_presets(cx))
            .child(self.render_surfaces(cx))
            .child(self.render_tints(cx))
    }
}

/// A clipped region filled with busy content for glass to blur.
fn stage(height: gpui_kit::Pixels, cx: &App) -> Div {
    let theme = cx.theme();
    let stripes: [Hsla; 6] = [
        theme.red,
        theme.yellow,
        theme.green,
        theme.cyan,
        theme.blue,
        theme.magenta,
    ];
    div()
        .relative()
        .w_full()
        .h(height)
        .overflow_hidden()
        .rounded(theme.radius_lg)
        .bg(theme.background)
        .child(
            h_flex().absolute().inset_0().children(
                stripes
                    .into_iter()
                    .flat_map(|color| [div().flex_1().h_full().bg(color), div().flex_1().h_full()]),
            ),
        )
        .child(
            v_flex()
                .absolute()
                .inset_0()
                .p_3()
                .gap_1()
                .text_sm()
                .font_medium()
                .text_color(theme.foreground)
                .children(
                    LINES
                        .iter()
                        .map(|line| div().whitespace_nowrap().child(*line)),
                ),
        )
}

fn card(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    glass: Glass,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w_0()
        .gap_1()
        .p_4()
        .rounded(cx.theme().radius_lg)
        .glass(glass, cx)
        .child(div().font_medium().child(title.into()))
        .child(div().text_sm().child(description.into()))
}

fn panel_row(icon: IconName, label: &'static str) -> impl IntoElement {
    h_flex()
        .gap_2()
        .px_2()
        .py_1()
        .child(Icon::new(icon).small())
        .child(label)
}

fn pill(
    id: &'static str,
    icon: IconName,
    label: &'static str,
    focused: bool,
    cx: &App,
) -> impl IntoElement {
    h_flex()
        .id(id)
        .h_8()
        .px_3()
        .gap_2()
        .text_sm()
        .rounded_full_style(cx)
        .glass(
            Glass::liquid()
                .surface(GlassSurface::Control)
                .focused(focused),
            cx,
        )
        .child(Icon::new(icon).small())
        .child(label)
}
