---
title: Glass
description: 模糊下方内容的 Frosted 与 Liquid 玻璃表面，色调、边缘与层级来自主题，并提供不透明回退。
order: -3.05
---

# Glass

玻璃表面会显示位于其后方、经过模糊的内容。GPUI Component 用渲染器已有的三样东西构成它：背景模糊（backdrop blur）、主题颜色的半透明填充，以及表示边缘和层级的阴影。它适合浮在动态或多彩内容之上的表面——菜单、浮动面板、工具栏胶囊按钮、滚动列表上方的吸顶标题——让用户看到下方内容仍在延续，有助于保持方位感。

玻璃是一种材质，而不是组件。`GlassStyled::glass` 是一个样式修饰，任何 `Styled` 元素都可以使用：

```rust
use gpui_kit::component::glass::{Glass, GlassStyled as _, GlassSurface};

div()
    .rounded(cx.theme().radius_lg)
    .p_3()
    .glass(Glass::frosted().surface(GlassSurface::Overlay), cx)
    .child("Recent")
```

模糊会按元素自身的圆角裁剪，因此 `.rounded(..)` 同时决定模糊、填充和边缘的形状，不需要单独的玻璃圆角。玻璃会设置元素的背景、文字颜色、阴影和背景滤镜；圆角、边框、内边距和布局仍由调用方负责。在 `.glass(..)` 之后再调用 `.bg(..)` 或 `.shadow(..)`，会替换材质中对应的部分。

组件展示应用的 **Glass** 页面在杂乱内容上展示了所有预设和表面；在仓库根目录运行 `cargo run -- Glass` 即可打开。

## 预设

| 预设 | 背景 | 填充 | 边缘 |
| --- | --- | --- | --- |
| `GlassPreset::Frosted`（默认） | `frosted_blur`，24px | 表面颜色，覆盖度 `frosted_opacity`，75% | 细线 |
| `GlassPreset::Liquid` | `liquid_blur`，8px | 表面颜色，覆盖度 `liquid_opacity`，35% | 半强度细线，顶部一像素淡白色高光 |

Liquid 只是近似效果。这里没有折射、透镜变形、色散或高光照明：渲染器只能模糊。顶部高光用来暗示受光的边缘，并不会弯折背景。承载正文阅读的表面请用 Frosted；小型控件，或需要透出更多背景的表面可以用 Liquid。

`Glass::blur(px(..))` 可以为单个表面覆盖预设的模糊半径；`Glass::opacity(..)` 可以覆盖填充遮住模糊背景的程度，适用于内容需要比预设更安静背景的表面。

## 表面

表面角色决定覆盖在模糊之上的颜色、其上的文字颜色以及默认层级：

| `GlassSurface` | 颜色 | 层级 | 用途 |
| --- | --- | --- | --- |
| `Overlay`（默认） | `popover`、`popover_foreground` | Overlay | 菜单、Popover、对话框、浮动面板 |
| `Panel` | `sidebar`、`sidebar_foreground` | Raised | 浮在内容之上的侧栏和检查器 |
| `Canvas` | `background`、`foreground` | Flat，无边缘 | 滚动内容上方的吸顶标题 |
| `Control` | `secondary`、`secondary_foreground` | Raised | 工具栏胶囊按钮和浮动按钮 |

`Canvas` 不绘制自己的边缘，吸顶标题只需添加它需要的那条边框，例如 `.border_b_1().border_color(cx.theme().border)`。

`Glass::tint(color)` 用另一种颜色代替表面颜色覆盖在模糊之上。覆盖多少仍由预设决定，颜色自身的 alpha 会按比例缩放覆盖度。文字颜色仍使用表面的前景色。

## 形状、层级与焦点

- **圆角：**使用元素自身的圆角——控件和菜单用 `.rounded(cx.theme().radius)`，较大的表面用 `.rounded(cx.theme().radius_lg)`，胶囊用 `.rounded_full_style(cx)`；组合确有需要时才使用显式的 `px(..)`。
- **层级：**`Glass::elevation(GlassElevation::Flat | Raised | Overlay)` 覆盖表面的默认层级。`Overlay` 是 Popover 的阴影，`Raised` 是凸起控件的紧凑阴影。主题关闭 `shadow` 时，所有层级都按 Flat 绘制。
- **焦点：**`Glass::focused(true)` 以主题的 `ring` 颜色在边缘内侧绘制焦点环。它在裁剪内容的祖先元素中依然可见，也不会给背景染色。请传入控件的真实焦点状态，例如 `handle.is_focused(window)`。

边缘是一个内阴影，绘制在填充之上并按元素裁剪。外阴影既绘制在元素四周，也绘制在元素下方，但模糊不会采样表面自身的阴影：带层级的玻璃表面在绘制阴影之前，GPUI 会先记录一个背景快照，模糊读取的就是这个快照，因此无论亮色还是暗色主题，玻璃都能保持对比度。遮罩上的玻璃则读取遮罩的快照，两者都不会被采样。只有带层级的玻璃才会记录这样的快照；Flat 玻璃模糊的是实时画面。在玻璃之前绘制的其他元素的阴影（例如相邻元素的阴影）仍会被模糊采样，就像透过真实的玻璃看到的一样。

## 遮罩上的玻璃

玻璃材质的对话框或 Sheet 模糊的是遮罩下方的页面，而不是遮罩本身，因此在亮色主题中填充依然保持对比度。Dialog、AlertDialog 和 Sheet 的宿主元素会在绘制遮罩之前记录一个背景快照（backdrop snapshot），元素内部绘制的背景模糊会读取离它最近的快照，而不是当前的实时画面。玻璃之外，遮罩仍像以前一样压暗页面。

自定义覆盖层也可以这样做：在绘制遮罩并包含玻璃的元素上调用 `.backdrop_snapshot()`：

```rust
div()
    .absolute()
    .inset_0()
    .bg(cx.theme().overlay)
    .backdrop_snapshot()
    .child(panel.glass(Glass::frosted(), cx))
```

该元素通过 `deferred` 绘制的内容会在它之后绘制，模糊的是实时画面；遮罩必须是该元素本身，或在玻璃之前绘制的子元素。`gpui-fast` 后端没有 `backdrop_snapshot`，而玻璃在该后端下本来就是不透明的。

## 文字对比度

默认情况下，玻璃会提高填充覆盖度，确保即使背后是一整片纯黑或纯白——模糊也无法缓和的最坏情况——表面前景色仍保持 4.5:1 的对比度。Frosted 通常已经足够；Liquid 在大多数主题中会被提高。`Glass::protect_text_contrast(false)` 直接使用预设的覆盖度，适用于只承载图标或大号文字的玻璃，或调用方已针对实际背景检查过可读性的情况。

弱化文字和次要文字不在这项保证之内。请在表面实际所处的内容之上检查它们。

## 降低透明度与后端

GPUI 不读取平台的“降低透明度”偏好，因此主题为此提供了一个开关：

```rust
Theme::update(cx, |theme| theme.glass.reduce_transparency = true);
```

开启后，所有玻璃表面都绘制为不透明的着色填充，保留相同的边缘、层级和文字颜色，不再模糊。

背景模糊来自本套件所基于的 GPUI 分支（[subroutine-app/gpui](https://github.com/subroutine-app/gpui)，见其 `docs/upstream.md`）。`gpui-fast` 后端没有背景滤镜，同样使用这种不透明回退，因此使用玻璃的代码在两种后端下都能编译并保持可读。

## Token

`cx.theme().glass` 保存材质的调校参数。这些值与亮色、暗色模式无关——每个表面都从当前主题取色——切换主题或模式时，会保留应用设置的值。

| `GlassTokens` 字段 | 默认值 | 含义 |
| --- | --- | --- |
| `frosted_blur` | `px(24.)` | Frosted 的背景模糊半径 |
| `liquid_blur` | `px(8.)` | Liquid 的背景模糊半径 |
| `frosted_opacity` | `0.75` | Frosted 模糊之上表面颜色的覆盖度 |
| `liquid_opacity` | `0.35` | Liquid 模糊之上表面颜色的覆盖度 |
| `rim_opacity` | `0.14` | Liquid 顶部白色高光的不透明度 |
| `reduce_transparency` | `false` | 所有玻璃都使用不透明着色填充 |
| `popover` | `None` | 弹出表面通过 `popover_glass_style` 使用的预设 |

请通过 `Theme::update` 修改，它会刷新所有窗口：

```rust
Theme::update(cx, |theme| {
    theme.glass.liquid_opacity = 0.5;
    theme.glass.popover = Some(GlassPreset::Frosted);
});
```

## 弹出表面

`GlassStyled::popover_glass_style(cx)` 可直接替换 `ThemeStyled::popover_style(cx)`：它为元素提供不透明的 Popover 表面；当 `theme.glass.popover` 指定了预设时，则改为使用主题圆角的玻璃。GPUI Component 自身的弹出组件目前仍使用 `popover_style`；在其中改用这个辅助方法后，它们就会跟随该开关。

弹出组件的入场动画会在每一帧替换阴影，因此动画结束前玻璃边缘不会显示。使用 `deferred` 绘制的弹出层会模糊其下方的整个窗口。
