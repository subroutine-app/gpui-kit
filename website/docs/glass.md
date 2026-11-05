---
title: Glass
description: Frosted and Liquid glass surfaces that blur the content behind them, with theme-derived tints, edges, elevation, and an opaque fallback.
order: -3.05
---

# Glass

A glass surface shows what is behind it, out of focus. GPUI Component builds it from three things the renderer already has: a backdrop blur, a translucent fill in a theme color, and box shadows for the edge and elevation. Use it for surfaces that float over moving or colorful content — a menu, a floating panel, a toolbar pill, a sticky header over a scrolling list — where seeing that the content continues underneath helps orientation.

Glass is a material, not a component. `GlassStyled::glass` is a style refinement that any `Styled` element accepts:

```rust
use gpui_kit::component::glass::{Glass, GlassStyled as _, GlassSurface};

div()
    .rounded(cx.theme().radius_lg)
    .p_3()
    .glass(Glass::frosted().surface(GlassSurface::Overlay), cx)
    .child("Recent")
```

The blur is clipped to the element's own corner radii, so `.rounded(..)` shapes the blur, the fill, and the edge together; there is no separate glass radius. Glass sets the element's background, text color, box shadows, and backdrop filter. The caller keeps the radius, border, padding, and layout. A `.bg(..)` or `.shadow(..)` written after `.glass(..)` replaces that part of the material.

The story gallery's **Glass** page shows every preset and surface over busy content; run `cargo run -- Glass` from the repository root.

## Presets

| Preset | Backdrop | Fill | Edge |
| --- | --- | --- | --- |
| `GlassPreset::Frosted` (default) | `frosted_blur`, 24px | Surface color at `frosted_opacity`, 75% | Hairline |
| `GlassPreset::Liquid` | `liquid_blur`, 8px | Surface color at `liquid_opacity`, 35% | A half-strength hairline and a faint one-pixel white highlight along the top |

Liquid is an approximation. There is no refraction, lensing, color dispersion, or specular lighting: the renderer only blurs. The top highlight suggests a lit edge; they do not bend the backdrop. Choose Frosted for anything that carries reading text, and Liquid for small controls or surfaces where seeing more of the backdrop matters.

`Glass::blur(px(..))` overrides the preset's blur radius for one surface.

## Surfaces

The surface role decides the color laid over the blur, the text color on it, and the default elevation:

| `GlassSurface` | Colors | Elevation | Use |
| --- | --- | --- | --- |
| `Overlay` (default) | `popover`, `popover_foreground` | Overlay | Menus, popovers, dialogs, floating panels |
| `Panel` | `sidebar`, `sidebar_foreground` | Raised | Sidebars and inspectors over content |
| `Canvas` | `background`, `foreground` | Flat, no edge | Sticky headers over scrolling content |
| `Control` | `secondary`, `secondary_foreground` | Raised | Toolbar pills and floating buttons |

`Canvas` draws no edge of its own, so a sticky header adds only the border it needs, such as `.border_b_1().border_color(cx.theme().border)`.

`Glass::tint(color)` lays another color over the blur instead of the surface's. The preset still decides how much of it covers the backdrop, and the color's own alpha scales that coverage. The text color stays the surface's.

## Shape, elevation, and focus

- **Radius:** use the element's radius — `.rounded(cx.theme().radius)` for controls and menus, `.rounded(cx.theme().radius_lg)` for larger surfaces, `.rounded_full_style(cx)` for pills, or an explicit `px(..)` when a composition needs one.
- **Elevation:** `Glass::elevation(GlassElevation::Flat | Raised | Overlay)` overrides the surface's default. `Overlay` is the popover shadow, `Raised` the tight shadow of a raised control. A theme with `shadow` off draws every elevation flat.
- **Focus:** `Glass::focused(true)` draws the focus ring inside the edge in the theme's `ring` color. It stays visible inside clipping ancestors and does not tint the backdrop. Pass the control's real focus state, for instance `handle.is_focused(window)`.

The edge is an inset shadow, painted over the fill and clipped to the element. Drop shadows sit under the element and are part of what the blur samples, which darkens the glass slightly near its lower edge; keep elevation for surfaces that really float.

## Glass under a scrim

A glass dialog or sheet blurs the page beneath the scrim, not the scrim itself, so its fill keeps its contrast in a light theme. The Dialog, AlertDialog, and Sheet hosts record a backdrop snapshot before they paint the scrim, and a backdrop blur painted inside an element reads the nearest such snapshot instead of the live frame. Outside the glass, the scrim dims the page as before.

A custom overlay does the same by calling `.backdrop_snapshot()` on the element that paints the scrim and contains the glass:

```rust
div()
    .absolute()
    .inset_0()
    .bg(cx.theme().overlay)
    .backdrop_snapshot()
    .child(panel.glass(Glass::frosted(), cx))
```

What the element paints with `deferred` paints after it and blurs the live frame, and the scrim must be the element or a descendant painted before the glass. Under the `gpui-fast` backend there is no `backdrop_snapshot`, and glass is opaque there anyway.

## Text contrast

By default glass raises its fill coverage until the surface's foreground keeps a 4.5:1 contrast over a uniform black or white region behind it — the worst case, which a blur does not soften. Frosted usually already covers enough; Liquid is raised in most themes. `Glass::protect_text_contrast(false)` uses the preset's coverage as is, for glass that carries only icons or large type, or whose caller has checked legibility against its real backdrop.

Muted and secondary text is not part of this guarantee. Check it over the content the surface actually floats over.

## Reduce transparency and backends

GPUI does not read the platform's reduce-transparency preference, so the theme has a switch for it:

```rust
Theme::update(cx, |theme| theme.glass.reduce_transparency = true);
```

When it is set, every glass surface is its opaque tinted fill, with the same edge, elevation, and text color, and no blur.

The backdrop blur comes from the GPUI fork this kit is built on ([subroutine-app/gpui](https://github.com/subroutine-app/gpui), see its `docs/upstream.md`). The same opaque fallback is used under the `gpui-fast` backend, which has no backdrop filter, so code using glass compiles and stays legible on both backends.

## Tokens

`cx.theme().glass` holds the material's tuning. The values do not depend on light or dark mode — each surface takes its color from the active theme — and a theme or mode change keeps whatever the application set.

| `GlassTokens` field | Default | Meaning |
| --- | --- | --- |
| `frosted_blur` | `px(24.)` | Frosted backdrop blur radius |
| `liquid_blur` | `px(8.)` | Liquid backdrop blur radius |
| `frosted_opacity` | `0.75` | Share of the surface color over Frosted's blur |
| `liquid_opacity` | `0.35` | Share of the surface color over Liquid's blur |
| `rim_opacity` | `0.14` | Opacity of Liquid's white highlight along the top edge |
| `reduce_transparency` | `false` | Opaque tinted fill everywhere |
| `popover` | `None` | The preset popup surfaces use with `popover_glass_style` |

Edit them through `Theme::update`, which refreshes every window:

```rust
Theme::update(cx, |theme| {
    theme.glass.liquid_opacity = 0.5;
    theme.glass.popover = Some(GlassPreset::Frosted);
});
```

## Popup surfaces

`GlassStyled::popover_glass_style(cx)` is a drop-in for `ThemeStyled::popover_style(cx)`: it gives the element the opaque popover surface, or glass with the theme radius when `theme.glass.popover` names a preset. GPUI Component's own popups still use `popover_style`; adopting the helper there makes them follow the switch.

A popup animates in by replacing its shadow on each frame, which drops the glass edge until the animation settles. Popups drawn with `deferred` blur the whole window beneath them.
