# This fork

This is Subroutine's fork of [GPUI Kit](https://github.com/longbridge/gpui-kit).
It follows upstream `main` and adds the changes below.

## GPUI

GPUI comes from [subroutine-app/gpui](https://github.com/subroutine-app/gpui),
a fork of [Mesa Hills Research's GPUI](https://github.com/mesa-hills-research/gpui)
on the `gpui-pre` 0.3.11 snapshot (Zed `f816e1f`), pinned by git revision in
the workspace `Cargo.toml`. Applications depend on `gpui-kit` alone and need no
`[patch]`. Its `docs/upstream.md` lists every change; the ones this kit uses:

- in-scene backdrop blur and content filters (`backdrop_blur`, `blur`, `filter`),
  ported from gpui-ce;
- edge fades (`fade_edges`);
- eased notched-wheel scrolling and animated programmatic scrolling
  (`ScrollHandle::set_offset_animated`, `scroll_to_item_animated`, …);
- held element drags (`drag_hold`);
- paint animations, modifier observers and glyph colors at paint time
  (Mesa Hills Research);
- opt-in native macOS toolbar window chrome (`TitlebarOptions::toolbar_style`).

The optional `gpui-fast` backend is unchanged and lacks these; components fall
back where they use them.

## GPUI Kit

- **Glass** (`component::glass`): a blur-backed material for any styled
  element, with theme glass tokens and opt-in popover glass.
- **Popup motion**: context, dropdown, app menu bar, popover and hover card
  surfaces share one entrance; `menu::ContextMenuLayer` opens a menu at a point.
- **Settings**: a compact row/section hierarchy and the standalone
  `SettingRow`, `SettingSection` and `SettingList` building blocks.
- **Themes**: the `bundled-themes` feature, font overrides, registry layering,
  and Mesa Hills Research's theme families and contrast tooling
  (`script/themes`).
- **From Mesa Hills Research's [gpui_kit](https://github.com/mesa-hills-research/gpui_kit)**:
  smooth caret, quiet inactive inputs, send keys, one focus border on text
  fields, tooltips that switch along a row and leave with their trigger,
  nested corner radii, bordered segmented tabs, roving focus in tab bars and
  sidebars, sidebar and sheet transition durations, nested scrolling, a
  paint-phase scrollbar, an even spinner and capped loading animations, a
  confirming copy button, title bar chrome per platform, menu bar keyboard
  access, key caps, text selection fixes and a context menu for selectable
  text.
