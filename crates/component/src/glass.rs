//! A surface that shows what is behind it, out of focus.
//!
//! [`GlassStyled::glass`] turns a styled element into glass: the content
//! behind it is blurred, the theme's color for the surface's role is laid
//! over the blur, and an edge and an elevation finish it. The blur is clipped
//! to the element's own corner radii, so `.rounded(..)` shapes the blur, the
//! fill and the edge together.
//!
//! ```ignore
//! use gpui_component::glass::{Glass, GlassStyled as _, GlassSurface};
//!
//! div()
//!     .rounded(cx.theme().radius_lg)
//!     .p_3()
//!     .glass(Glass::frosted().surface(GlassSurface::Overlay), cx)
//!     .child("Recent")
//! ```
//!
//! The material is a backdrop blur under a translucent fill, nothing more.
//! There is no refraction, lensing, dispersion or specular lighting:
//! [`GlassPreset::Liquid`] suggests a lit edge with a faint highlight along
//! its top, painted over a lighter blur. The blur leaves out the surface's own
//! elevation shadow, which GPUI draws under it as well as around it.
//!
//! # Where there is no backdrop
//!
//! Under the `gpui-fast` backend, which has no backdrop filter, and when
//! [`GlassTokens::reduce_transparency`] is set, a glass surface is its opaque
//! tinted fill with the same edge and elevation.
//!
//! # What glass owns
//!
//! Glass sets the element's background, text color, box shadows and backdrop
//! filter. The caller keeps its radius, border, padding and layout. A
//! `.shadow(..)` or `.bg(..)` after `.glass(..)` replaces part of the
//! material.

use gpui::{App, BoxShadow, Hsla, Pixels, Rgba, Styled, px};

use crate::{
    ActiveTheme as _, GlassTokens, Theme, ThemeStyled as _,
    styled::{popover_ring, popover_shadow, raised_shadow},
};

/// Whether this GPUI backend can blur what is behind an element.
const BACKDROP_FILTER: bool = cfg!(not(feature = "gpui-fast"));

/// WCAG AA contrast for body text.
const BODY_TEXT_CONTRAST: f32 = 4.5;

/// Width of the ring a focused glass surface draws inside its edge.
const FOCUS_EDGE_WIDTH: Pixels = px(2.);

/// How much of the edge color Liquid's hairline keeps. Its fill is thin, so a
/// full-strength edge reads as an outline drawn around the glass.
const LIQUID_EDGE_SHARE: f32 = 0.5;

/// What a glass surface is made of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GlassPreset {
    /// A strong blur under a mostly opaque fill. Reads like a solid surface
    /// that lets color and movement through.
    #[default]
    Frosted,
    /// A light blur under a thin fill, with a faint highlight along the top
    /// edge standing in for a lit edge. Shows more of the backdrop than
    /// Frosted.
    Liquid,
}

impl GlassPreset {
    fn blur(self, tokens: &GlassTokens) -> Pixels {
        match self {
            Self::Frosted => tokens.frosted_blur,
            Self::Liquid => tokens.liquid_blur,
        }
    }

    fn opacity(self, tokens: &GlassTokens) -> f32 {
        match self {
            Self::Frosted => tokens.frosted_opacity,
            Self::Liquid => tokens.liquid_opacity,
        }
    }
}

/// The role of a glass surface, which decides the theme color laid over the
/// blur, the foreground on it, and its default elevation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GlassSurface {
    /// A floating plane — menu, popover, dialog, floating panel. `popover`
    /// colors; overlay elevation.
    #[default]
    Overlay,
    /// A region of the window — sidebar, inspector. `sidebar` colors; raised
    /// elevation.
    Panel,
    /// The window's own background, such as a sticky header over scrolling
    /// content. `background` colors; flat, and without an edge, so the caller
    /// draws whichever borders it needs.
    Canvas,
    /// A control floating over content — a toolbar pill or button.
    /// `secondary` colors; raised elevation.
    Control,
}

impl GlassSurface {
    fn colors(self, theme: &Theme) -> (Hsla, Hsla) {
        match self {
            Self::Overlay => (theme.popover, theme.popover_foreground),
            Self::Panel => (theme.sidebar, theme.sidebar_foreground),
            Self::Canvas => (theme.background, theme.foreground),
            Self::Control => (theme.secondary, theme.secondary_foreground),
        }
    }

    fn elevation(self) -> GlassElevation {
        match self {
            Self::Overlay => GlassElevation::Overlay,
            Self::Panel | Self::Control => GlassElevation::Raised,
            Self::Canvas => GlassElevation::Flat,
        }
    }

    fn has_edge(self) -> bool {
        self != Self::Canvas
    }
}

/// How far a glass surface is lifted off what it floats over. Follows
/// [`Theme::shadow`]: a theme without shadows draws every elevation flat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GlassElevation {
    /// No shadow.
    Flat,
    /// A tight shadow for a control or panel raised out of its container.
    Raised,
    /// The popover shadow, for a surface floating above the page.
    Overlay,
}

impl GlassElevation {
    fn shadows(self) -> Vec<BoxShadow> {
        match self {
            Self::Flat => Vec::new(),
            Self::Raised => raised_shadow(),
            // Without its ring: the edge is drawn inside the glass, where it
            // does not tint the backdrop the blur samples.
            Self::Overlay => popover_shadow(Hsla::transparent_black(), 1.)
                .into_iter()
                .filter(|shadow| shadow.color.a > 0.)
                .collect(),
        }
    }
}

/// A glass material, applied with [`GlassStyled::glass`].
///
/// Built from a preset and refined fluently; every value it does not set comes
/// from the active theme's [`GlassTokens`] and colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glass {
    preset: GlassPreset,
    surface: GlassSurface,
    tint: Option<Hsla>,
    elevation: Option<GlassElevation>,
    blur: Option<Pixels>,
    focused: bool,
    protect_text_contrast: bool,
}

impl Default for Glass {
    fn default() -> Self {
        Self::new(GlassPreset::default())
    }
}

impl Glass {
    pub fn new(preset: GlassPreset) -> Self {
        Self {
            preset,
            surface: GlassSurface::default(),
            tint: None,
            elevation: None,
            blur: None,
            focused: false,
            protect_text_contrast: true,
        }
    }

    /// Glass made with [`GlassPreset::Frosted`].
    pub fn frosted() -> Self {
        Self::new(GlassPreset::Frosted)
    }

    /// Glass made with [`GlassPreset::Liquid`].
    pub fn liquid() -> Self {
        Self::new(GlassPreset::Liquid)
    }

    pub fn preset(mut self, preset: GlassPreset) -> Self {
        self.preset = preset;
        self
    }

    /// The surface role, default [`GlassSurface::Overlay`].
    pub fn surface(mut self, surface: GlassSurface) -> Self {
        self.surface = surface;
        self
    }

    /// Lay this color over the blur instead of the surface's theme color.
    ///
    /// The preset still decides how much of it covers the backdrop; the
    /// color's own alpha scales that coverage. The foreground stays the
    /// surface's, so a tint far from the surface color should be checked for
    /// contrast or paired with
    /// [`protect_text_contrast`](Self::protect_text_contrast).
    pub fn tint(mut self, tint: impl Into<Hsla>) -> Self {
        self.tint = Some(tint.into());
        self
    }

    /// Override the elevation the surface role implies.
    pub fn elevation(mut self, elevation: GlassElevation) -> Self {
        self.elevation = Some(elevation);
        self
    }

    /// Override the preset's backdrop blur radius.
    pub fn blur(mut self, radius: impl Into<Pixels>) -> Self {
        self.blur = Some(radius.into());
        self
    }

    /// Draw the keyboard focus ring inside the edge, in the theme's `ring`
    /// color.
    ///
    /// The ring is inside so that it survives clipping ancestors and does not
    /// tint the backdrop; it replaces the edge while focused.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Raise the fill's coverage where needed so the surface's foreground
    /// keeps a 4.5:1 contrast over any backdrop, default `true`.
    ///
    /// The worst case is a uniform black or white region behind the glass,
    /// which a blur does not soften. Turn this off when the caller owns
    /// legibility, for instance glass carrying only icons or large type; the
    /// preset's coverage is then used as is.
    pub fn protect_text_contrast(mut self, protect: bool) -> Self {
        self.protect_text_contrast = protect;
        self
    }

    fn look(&self, theme: &Theme, edge: Hsla) -> GlassLook {
        let tokens = &theme.glass;
        let (surface, foreground) = self.surface.colors(theme);
        let tint = self.tint.unwrap_or(surface);
        let opaque_tint = tint.alpha(1.);
        let translucent = BACKDROP_FILTER && !tokens.reduce_transparency;

        let coverage = if translucent {
            let preset = (self.preset.opacity(tokens) * tint.a).clamp(0., 1.);
            if self.protect_text_contrast {
                preset.max(legible_coverage(foreground, opaque_tint))
            } else {
                preset
            }
        } else {
            1.
        };
        let fill = opaque_tint.alpha(coverage);
        let liquid = translucent && self.preset == GlassPreset::Liquid;

        let mut shadows = if theme.shadow {
            self.elevation
                .unwrap_or_else(|| self.surface.elevation())
                .shadows()
        } else {
            Vec::new()
        };
        if self.focused {
            shadows.push(inner_ring(theme.ring, FOCUS_EDGE_WIDTH));
        } else if self.surface.has_edge() {
            if liquid {
                shadows.push(top_highlight(tokens.rim_opacity));
                shadows.push(inner_ring(edge.opacity(LIQUID_EDGE_SHARE), px(1.)));
            } else {
                shadows.push(inner_ring(edge, px(1.)));
            }
        }

        GlassLook {
            fill,
            foreground,
            shadows,
            blur: translucent.then(|| self.blur.unwrap_or_else(|| self.preset.blur(tokens))),
        }
    }
}

/// A glass material resolved against a theme.
struct GlassLook {
    fill: Hsla,
    foreground: Hsla,
    shadows: Vec<BoxShadow>,
    blur: Option<Pixels>,
}

/// Glass surfaces for any styled element.
pub trait GlassStyled: Styled + Sized {
    /// Make this element a glass surface. See the [module docs](self) for what
    /// it sets and what stays with the caller.
    fn glass(self, glass: Glass, cx: &App) -> Self;

    /// The popover surface in the material the theme chooses:
    /// [`ThemeStyled::popover_style`](crate::ThemeStyled::popover_style), or
    /// glass with the theme radius when [`GlassTokens::popover`] names a
    /// preset.
    ///
    /// A drop-in for `popover_style` on popup surfaces. An enter animation
    /// that replaces the shadow each frame drops the glass edge until it
    /// settles.
    fn popover_glass_style(self, cx: &App) -> Self;
}

impl<T: Styled + Sized> GlassStyled for T {
    fn glass(self, glass: Glass, cx: &App) -> Self {
        let look = glass.look(cx.theme(), popover_ring(cx));
        backdrop(
            self.bg(look.fill)
                .text_color(look.foreground)
                .shadow(look.shadows),
            look.blur,
        )
    }

    fn popover_glass_style(self, cx: &App) -> Self {
        let theme = cx.theme();
        match theme.glass.popover {
            Some(preset) => self.rounded(theme.radius).glass(Glass::new(preset), cx),
            None => self.popover_style(cx),
        }
    }
}

#[cfg(not(feature = "gpui-fast"))]
fn backdrop<T: Styled>(element: T, blur: Option<Pixels>) -> T {
    match blur {
        // Replaces rather than appends, so glass applied twice blurs once.
        Some(radius) => element.backdrop_filter(vec![gpui::Filter::Blur(radius)]),
        None => element,
    }
}

#[cfg(feature = "gpui-fast")]
fn backdrop<T: Styled>(element: T, _: Option<Pixels>) -> T {
    element
}

/// Glass painted inside `element` blurs the page as it was before `element`
/// painted, so a scrim `element` draws under the glass stays out of the blur.
#[cfg(not(feature = "gpui-fast"))]
pub(crate) fn backdrop_snapshot<T: Styled>(element: T) -> T {
    element.backdrop_snapshot()
}

#[cfg(feature = "gpui-fast")]
pub(crate) fn backdrop_snapshot<T: Styled>(element: T) -> T {
    element
}

/// A ring of `width` just inside the element's edge. An inset shadow is
/// clipped to the element, so it paints over the fill and never under it.
fn inner_ring(color: Hsla, width: Pixels) -> BoxShadow {
    BoxShadow::new(px(0.), px(0.), color)
        .spread_radius(width)
        .inset()
}

/// Liquid's lit edge: a one-pixel line of white along the top, inside the
/// hairline.
fn top_highlight(opacity: f32) -> BoxShadow {
    BoxShadow::new(px(0.), px(1.), gpui::white().alpha(opacity)).inset()
}

/// The smallest share of an opaque `tint` over a black or white backdrop that
/// keeps `foreground` at body-text contrast, or 1 when even the bare tint
/// falls short.
fn legible_coverage(foreground: Hsla, tint: Hsla) -> f32 {
    let foreground = relative_luminance(foreground.to_rgb());
    let tint = tint.to_rgb();
    let black = Rgba {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 1.,
    };
    let white = Rgba {
        r: 1.,
        g: 1.,
        b: 1.,
        a: 1.,
    };
    coverage_against(foreground, tint, black).max(coverage_against(foreground, tint, white))
}

/// Bisects the coverage of `tint` over `backdrop`. Over black or white every
/// channel moves the same way, so luminance is monotonic in coverage and the
/// legible coverages form one interval ending at the bare tint.
fn coverage_against(foreground: f32, tint: Rgba, backdrop: Rgba) -> f32 {
    let legible = |coverage: f32| {
        let fill = relative_luminance(mix(tint, backdrop, coverage));
        contrast(foreground, fill) >= BODY_TEXT_CONTRAST
    };
    if legible(0.) {
        return 0.;
    }
    if !legible(1.) {
        return 1.;
    }
    let (mut illegible, mut legible_at) = (0., 1.);
    for _ in 0..12 {
        let mid = (illegible + legible_at) / 2.;
        if legible(mid) {
            legible_at = mid;
        } else {
            illegible = mid;
        }
    }
    legible_at
}

/// `color` at `coverage` over an opaque `backdrop`, blended per sRGB channel
/// as the renderer does.
fn mix(color: Rgba, backdrop: Rgba, coverage: f32) -> Rgba {
    let channel = |c: f32, b: f32| c * coverage + b * (1. - coverage);
    Rgba {
        r: channel(color.r, backdrop.r),
        g: channel(color.g, backdrop.g),
        b: channel(color.b, backdrop.b),
        a: 1.,
    }
}

/// WCAG relative luminance.
fn relative_luminance(color: Rgba) -> f32 {
    let linear = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

/// WCAG contrast ratio between two relative luminances.
fn contrast(a: f32, b: f32) -> f32 {
    let (light, dark) = if a > b { (a, b) } else { (b, a) };
    (light + 0.05) / (dark + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ThemeColor, ThemeMode};

    fn themes() -> [Theme; 2] {
        let light = Theme::from(ThemeColor::light().as_ref());
        let mut dark = Theme::from(ThemeColor::dark().as_ref());
        dark.mode = ThemeMode::Dark;
        [light, dark]
    }

    #[test]
    fn protection_keeps_body_text_legible_over_black_and_white() {
        for theme in themes() {
            let edge = theme.foreground.alpha(0.1);
            let protected = Glass::liquid().look(&theme, edge);
            let unprotected = Glass::liquid()
                .protect_text_contrast(false)
                .look(&theme, edge);

            let coverage = protected.fill.a;
            assert!(coverage >= theme.glass.liquid_opacity);
            assert_eq!(unprotected.fill.a, theme.glass.liquid_opacity);

            let foreground = relative_luminance(theme.popover_foreground.to_rgb());
            for backdrop in [0., 1.] {
                let backdrop = Rgba {
                    r: backdrop,
                    g: backdrop,
                    b: backdrop,
                    a: 1.,
                };
                let fill = mix(theme.popover.to_rgb(), backdrop, coverage);
                assert!(contrast(foreground, relative_luminance(fill)) >= BODY_TEXT_CONTRAST);
            }
        }
    }

    #[test]
    fn reduced_transparency_is_an_opaque_unblurred_fill() {
        for mut theme in themes() {
            theme.glass.reduce_transparency = true;
            let look = Glass::liquid()
                .tint(theme.primary.alpha(0.3))
                .look(&theme, theme.foreground.alpha(0.1));
            assert_eq!(look.blur, None);
            assert_eq!(look.fill, theme.primary.alpha(1.));
        }
    }
}
