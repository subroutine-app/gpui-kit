# Vendored GPUI backend

GPUI Kit depends on GPUI through the crates.io `gpui-pre-*` snapshot crates.
Four of them are vendored here to add framework primitives the published
snapshot does not have: an in-scene backdrop blur and content filter, an edge
fade content filter, eased scrolling for notched mouse wheels and programmatic
requests, held element drags, and paint animations. Every other `gpui-pre-*`
crate still comes from crates.io.

| Directory | Package | Upstream |
| --- | --- | --- |
| `gpui-pre/` | `gpui-pre` (library `gpui`) | crates.io `gpui-pre` 0.3.11 |
| `gpui-pre-apple/` | `gpui-pre-apple` (Metal renderer) | crates.io `gpui-pre-apple` 0.3.11 |
| `gpui-pre-wgpu/` | `gpui-pre-wgpu` (wgpu renderer) | crates.io `gpui-pre-wgpu` 0.3.11 |
| `gpui-pre-windows/` | `gpui-pre-windows` (DirectX 11 renderer) | crates.io `gpui-pre-windows` 0.3.11 |

The 0.3.11 snapshot is Zed at commit `f816e1f`. GPUI and the vendored crates
are licensed Apache-2.0; each directory keeps the published `LICENSE-APACHE`.

## How the copies were made

Each directory started as a verbatim copy of the published package from the
Cargo registry, with only `.cargo-ok` and `.cargo_vcs_info.json` dropped. The
published `Cargo.toml`, `Cargo.toml.orig`, and `Cargo.lock` are unchanged, and
the packages keep version `0.3.11` so `[patch.crates-io]` can replace them.

To see exactly what this repository changed, compare against the registry:

```bash
diff -ru ~/.cargo/registry/src/index.crates.io-*/gpui-pre-0.3.11 vendor/gpui-pre
```

(and likewise for `gpui-pre-apple`, `gpui-pre-wgpu`, `gpui-pre-windows`).

## What changed

The filter primitive is ported from the gpui-ce fork, commit `69d467e4d1`
("Filter API + Blur (#40)"), Apache-2.0 like GPUI itself. Later gpui-ce commits
touching the same code only added `dead_code` allowances. The port follows that
commit's behavior, adapted to this newer Zed snapshot:

- `gpui-pre`: `Filter` (`Filter::Blur(Pixels)`) and its device-space form
  `ScaledFilter`; `Style::filter` and `Style::backdrop_filter`;
  `Styled::blur`, `Styled::filter`, `Styled::backdrop_blur`,
  `Styled::backdrop_filter`; `Window::paint_backdrop_filter` and
  `Window::with_filter_layer`; the `BackdropFilter` and `FilterBoundary` scene
  primitives with their `PrimitiveBatch` variants. A backdrop is painted after
  drop shadows and before the element's background, so a translucent `.bg()`
  tints it, and it is clipped to the element's corner radii. Deferred draws
  (popovers, menus, dialogs, tooltips) raise the scene's draw-order floor, so
  an overlay always blurs the whole main scene beneath it.
- `gpui-pre-apple`: Metal blur passes, translated to `objc2-metal`. Filter
  render targets are allocated on the first frame that uses a filter, and
  blur passes are scissored to the filtered region.
- `gpui-pre-wgpu`: wgpu blur passes in `WgpuRendererCore`, used by windows and
  the headless renderer. Targets are allocated lazily and keyed by frame size;
  the blur-params uniform buffer grows to fit the frame instead of wrapping.
- `gpui-pre-windows`: DirectX blur passes. `BlurParams` is bound at constant
  buffer slot `b2`, because this snapshot uses `b1` for the per-frame
  `BatchParams` that instanced draws read. Blur passes use their own clamping
  sampler, since the shared sprite sampler wraps.

All three renderers render a scene without filters exactly as before.

### Eased scrolling, held drags, and edge fades

Ported in behavior from the GPUI Box framework (`crates/gpui`, through commit
`893dc1c9` "Let element drags require a held press"), which is original GPUI
Box work, and adapted to this snapshot; the edge fade is implemented here as a
content filter rather than GPUI Box's per-primitive opacity scope.

- **Notched-wheel easing** (`gpui-pre`: `gestures.rs`, `elements/div.rs`,
  `elements/list.rs`). A `ScrollEvent` whose delta is `ScrollDelta::Lines`
  (a classic notched wheel) no longer jumps: `overflow_scroll` divs (with or
  without a `ScrollHandle`), `uniform_list`, and `list` ease to the new offset
  over 80 ms with an ease-out cubic (`1 - (1 - t)³`), one increment per frame.
  A notch arriving mid-motion extends the running motion's target without
  dropping undelivered distance and restarts the ease. Every target is clamped
  to the container's scroll range when it is pushed, and an axis whose range
  shrinks under the motion is cancelled, so the offset never overshoots.
  `ScrollDelta::Pixels` (trackpads, touch) keeps the previous immediate
  behavior and cancels any motion in flight, as do `set_offset`,
  `scroll_to_item`, `scroll_to_bottom`, the list's programmatic scrolls,
  scrollbar drags, and `ListState::reset`. Under `App::reduce_motion` the wheel
  applies immediately. The shared state is the crate-private
  `ScrollTransition`.
  **Behavior change:** code (and tests) that assert a GPUI scroller moved right
  after a line-delta wheel now has to let a frame after the 80 ms elapse.
- **Animated programmatic scrolling**, on the same transition with a 200 ms
  ease, so wheel notches and requests merge into one motion:
  `ScrollHandle::set_offset_animated(&self, Point<Pixels>)`,
  `ScrollHandle::scroll_to_item_animated(&self, usize)`,
  `UniformListScrollHandle::scroll_to_item_animated(&self, usize, ScrollStrategy)`
  (requires `track_scroll`),
  `ListState::scroll_by_animated(&self, Pixels)`, and
  `ListState::scroll_to_reveal_item_animated(&self, usize)`. Targets are
  clamped to the range from the last layout and judged against where motion in
  flight is heading. Before the first layout, and for a `list` motion that would
  cross rows of unknown height (never measured, no size hint), they fall back to
  the non-animated call; reduced motion completes them in one frame. The
  existing strict/offset `UniformListScrollHandle` variants are not animated.
  The uniform list's animated flag lives in private `ScrollHandle` state, so
  `DeferredScrollToItem` and `UniformListScrollState` keep their public fields.
- **Held element drags** (`elements/div.rs`):
  `Interactivity::drag_hold(&mut self, Duration)` and
  `StatefulInteractiveElement::drag_hold(self, Duration) -> Self`, called after
  `on_drag`. Movement during the hold does not start the drag and the press
  still clicks if released over the element; the first movement past the drag
  threshold after the hold starts the drag with the press point under the
  cursor. Measured on the executor clock; the mouse path only (touch drags
  remain governed by the long-press recognizer).
- **Edge fade content filter** (`style.rs`, `styled.rs`, all three renderers):
  `Filter::FadeEdges(Edges<Pixels>)`, `ScaledFilter::FadeEdges(Edges<ScaledPixels>)`,
  and `Styled::fade_edges(self, impl Into<Edges<Pixels>>) -> Self`, which
  appends to the `filter` chain. Inside each edge with a nonzero band, opacity
  ramps linearly from 0 at the edge to 1 at the band's inner side; bands
  multiply where they meet, like stacked CSS mask gradients. It runs on the
  content filter layer, so it applies to everything the element paints —
  background, children of any kind, border — composited as one group and
  clipped to the element's bounds and corner radii (children overflowing the
  bounds are cut off). Without a blur in the chain the group texture is
  composited 1:1 at full resolution; with one, the mask applies after the blur
  regardless of chain order. In a `backdrop_filter` chain it softens the blurred
  backdrop's edges and has no effect alone. `Edges` gained `Serialize`,
  `Deserialize`, and `JsonSchema` derives so `Filter` keeps its derives.
  The composite uniform (`BlurUniform`/`BlurParams`) grew from 80 to 112 bytes
  (`fade` at byte 80 as a 16-byte-aligned vector, `sample_scale`, padding) in
  Metal, WGSL, and HLSL alike, guarded by size and offset assertions.
  `list` paints none of its own style, so fade a wrapper around it.

Verification for these changes: on macOS, the Metal path was checked through
GPUI's real offscreen renderer (`crates/kit/tests/rendering.rs`, which checks
fade pixels, and a scratch render of `examples/scroll_fade` stepped on the test
clock) and the stitched MSL compiles with `xcrun metal`. The wgpu crate compiles
on macOS and all three WGSL variants validate with naga 30; it was not run.
The DirectX crate compiles for `x86_64-pc-windows-msvc` (with `llvm-rc`
stubbed); the HLSL was not compiled, since no `fxc`/`dxc` is available on
macOS, and it was not run. The vendored crates' own unit tests cannot build
outside Zed's tree, so the behavior tests live in
`crates/kit/tests/integration/vendored_gpui.rs` (wheel easing, accumulation
and clamping, animated scrolls, reduced motion, held drags). Five existing
component and base tests that asserted an immediate line-delta scroll now let
the transition land first.

### Window-wide modifier observer

Implemented here for the app menu bar's keyboard access, ported from Mesa
Hills Research's GPUI Kit fork. Its own GPUI fork (`mhr-gpui`) has an
equivalent observer; this one was written against this snapshot rather than
copied.

- **`Window::observe_modifiers_changed`** (`gpui-pre`: `window.rs`). Registers
  a callback run for every `PlatformInput::ModifiersChanged` the window
  dispatches, after `Window::modifiers` is updated and before any element
  hears the change. Elements hear modifier changes only on the focused
  element's dispatch path, which is the root view alone while nothing has
  focus, so a component outside that path (the menu bar underlining its
  access keys while Alt is held) had no way to follow the keyboard's
  modifiers. Mirrors `observe_button_layout_changed`: a `SingleSubscriberSet`
  on the window, dropped with the returned `Subscription`. Dispatch to
  elements is unchanged.

Its behavior is exercised by the menu bar's tests in
`crates/component/src/menu/app_menu_bar_tests.rs`.

### Recolored text paint

Ported in behavior from Mesa Hills Research's `mhr-gpui` 0.1.9
(`TextLayout::paint_foreground_with_colors`), which GPUI Kit's range highlight
text colors (`RangeHighlight::with_text_color`) need. Written against this
snapshot rather than copied.

- `gpui-pre` (`elements/text.rs`, `text_system/line.rs`):
  `TextLayout::paint_foreground_with_colors(&self, &[(Range<usize>, Hsla)],
  &mut Window, &mut App) -> anyhow::Result<()>` and
  `WrappedLine::paint_with_colors`. They paint like `paint_foreground` and
  `WrappedLine::paint`, with the glyphs in each byte range in its color, a later
  range over an earlier one. The decoration runs are split at the ranges' edges
  for that paint only, so the text keeps its runs, shaped lines and wrapping,
  and moving a highlight does not lay the text out again. Underlines and
  strikethroughs without a color of their own keep the run's original color;
  run backgrounds are untouched. An empty slice paints exactly as before.

The run splitting (`recolor_runs`) has a unit test in `text_system/line.rs`,
which, like the other vendored unit tests, cannot build outside Zed's tree; its
cases were checked against a standalone copy of the function. `gpui-base` tests
check that a highlight's text color reaches each text leaf it covers. Under
`gpui-fast`, which is not patched, `gpui-base` paints the text in its own
colors.

### Paint animations

Ported in behavior from Mesa Hills Research's GPUI fork (`mhr-gpui` 0.1.9 on
crates.io, source at <https://github.com/mesa-hills-research/gpui>),
Apache-2.0, and adapted to this snapshot's scene, which also carries backdrop
filters and filter boundaries. Only `gpui-pre` changes; the renderers draw the
resulting primitives as before.

- **`PaintEffect`** (`scene.rs`): a value type with `opacity`, `rotation` and
  `translation` (logical pixels), with `NONE`, `opacity`, `rotation`,
  `translation` and `with_*` constructors. `Scene::push_effect` and
  `Scene::pop_effect` apply the composed effects to what is inserted between
  them: opacity multiplies every primitive's color (a backdrop filter's
  opacity too; a filter boundary keeps its composite opacity because its
  content is already faded), translation moves bounds, path vertices and
  sprite transformations, and rotation turns monochrome and subpixel sprites
  about the element's centre. Clips stay where they were laid out. What is
  kept for replay is the primitive without the effect, and a primitive an
  effect moves out of its clip is still kept, so a later frame can move it
  back.
- **`AnimationExt::with_paint_animation(id, Animation, impl Fn(f32) ->
  PaintEffect) -> PaintAnimationElement<Self>`** (`elements/animation.rs`):
  lays the element out once and paints it with the effect of each frame.
  The window records each effect over its range of paint operations
  (`PaintEffectRecord` in `Frame::paint_effects`, indexed by `PaintIndex`),
  and `reuse_paint` replays a cached view's scene with those effects
  evaluated at the new frame's time, so a cached view stays cached while its
  effects run. After each draw the window asks for the next display frame
  while an uncapped effect runs, or arms a timer for a capped one
  (`Animation::with_max_fps`), whose steps fall on a grid of the app's synced
  animation clock so capped animations draw in the same frames. No view is
  notified for these frames. Time is the executor clock, captured once per
  draw (`FrameTime`), and under `App::reduce_motion` an effect holds at its
  start (repeating) or end (oneshot) without asking for frames.
  Hit testing, bounds, focus and input-method positions are unaffected.
- **`Window::rendered_scene(&self) -> &Scene`**, test-support only, so tests
  outside the crate can check what the last frame painted.

Verification: `crates/kit/tests/integration/paint_effects.rs` (a cached view
stays cached while its effect runs; the effect survives a re-render; frames
stop when a oneshot ends; reduced motion; a capped rate steps on a timer;
translations add up and opacities multiply without moving the hitbox; an icon
turns), and the scrollbar and spinner tests in `gpui-base` and
`gpui-component`. GPUI Fast has no paint effects, so the scrollbar and the
spinner fall back to re-rendering for each frame there.

## Downstream applications must mirror the patch

Cargo applies `[patch]` only at the root of the workspace being built. An
application that depends on GPUI Kit therefore gets the unpatched crates.io
backend, and the filter API does not exist there, unless its own workspace root
repeats these entries, pointing at this directory:

```toml
[patch.crates-io]
gpui-pre = { path = "../gpui-kit/vendor/gpui-pre" }
gpui-pre-apple = { path = "../gpui-kit/vendor/gpui-pre-apple" }
gpui-pre-wgpu = { path = "../gpui-kit/vendor/gpui-pre-wgpu" }
gpui-pre-windows = { path = "../gpui-kit/vendor/gpui-pre-windows" }
```

Adjust the paths to where the GPUI Kit checkout lives. All four entries are
needed together: the renderers match exhaustively on the patched scene types,
so mixing a patched `gpui-pre` with a published renderer does not compile.

The `gpui-fast` backend is not patched. Code that calls the filter, edge fade,
animated scrolling, `drag_hold`, or paint animation APIs does not compile with
the `gpui-fast` feature, and notched wheels there still jump.

## Updating the snapshot

When the workspace moves to a newer `gpui-pre` release, the vendored crates
must move with it: copy the new published packages here verbatim, reapply the
changes above, and update the versions in this file. A patch whose version no
longer matches the workspace requirement is silently ignored by Cargo apart
from a "patch was not used" warning.
