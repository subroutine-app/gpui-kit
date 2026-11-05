use gpui::{Pixels, px};

use crate::glass::GlassPreset;

/// Semantic tuning for [`crate::glass`] surfaces.
///
/// The values are independent of light and dark mode: each surface derives its
/// color from the active theme, so the same blur radii and coverages read the
/// same way in both. A theme change leaves them as the application set them.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GlassTokens {
    /// Backdrop blur radius of [`GlassPreset::Frosted`].
    pub frosted_blur: Pixels,
    /// Backdrop blur radius of [`GlassPreset::Liquid`].
    pub liquid_blur: Pixels,
    /// How much of the surface color [`GlassPreset::Frosted`] lays over its
    /// blurred backdrop, from 0 (none) to 1 (opaque).
    pub frosted_opacity: f32,
    /// How much of the surface color [`GlassPreset::Liquid`] lays over its
    /// blurred backdrop, from 0 (none) to 1 (opaque).
    pub liquid_opacity: f32,
    /// Opacity of the white rim and sheen that [`GlassPreset::Liquid`] uses to
    /// suggest a lit edge.
    pub rim_opacity: f32,
    /// Paint every glass surface as its opaque tinted fill, without a
    /// backdrop blur.
    ///
    /// GPUI does not read the platform's reduce-transparency preference, so an
    /// application that follows it sets this switch itself.
    pub reduce_transparency: bool,
    /// The glass preset GPUI Kit popup surfaces are made of, or `None` for the
    /// opaque popover surface. Read by
    /// [`crate::glass::GlassStyled::popover_glass_style`].
    pub popover: Option<GlassPreset>,
}

impl Default for GlassTokens {
    fn default() -> Self {
        Self {
            frosted_blur: px(24.),
            liquid_blur: px(8.),
            frosted_opacity: 0.75,
            liquid_opacity: 0.35,
            rim_opacity: 0.45,
            reduce_transparency: false,
            popover: None,
        }
    }
}
