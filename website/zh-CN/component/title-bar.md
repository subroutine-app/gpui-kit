---
title: TitleBar
description: 支持窗口控制和自定义内容的自定义标题栏组件。
maturity: [platform-dependent]
---

# TitleBar

TitleBar 用于替换系统默认标题栏，提供可定制的窗口标题区域。它内置平台相关的窗口控制按钮，并支持插入菜单栏、状态信息和自定义操作区。组件会根据 macOS、Windows 和 Linux 自动调整行为和视觉样式。

## 导入

```rust
use gpui_kit::component::TitleBar;
```

## 用法

### 基础标题栏

```rust
TitleBar::new()
    .child(div().child("My Application"))
```

### 带自定义内容的标题栏

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .gap_3()
            .child("App Name")
            .child(Badge::new().count(5))
    )
    .child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(Button::new("settings").icon(IconName::Settings))
            .child(Button::new("profile").icon(IconName::User))
    )
```

### 居中标题与后缀

`center` 会把内容放在标题栏中间，并以整个窗口的宽度居中，就像窗口标题那样，不受窗口按钮和两侧内容宽度的影响。
某一侧空间不足时，居中内容会向另一侧让开；标题栏整体放不下时，它会变窄并被裁剪，因此请为它设置 `min_w_0()`，并为其中的文本设置 `truncate()`。
设置 center 后，子元素位于它之前，`suffix` 位于它之后、窗口按钮之前。它们周围的空白区域仍可拖动窗口。

```rust
TitleBar::new()
    .child(AppMenuBar::new(cx))
    .center(div().min_w_0().truncate().text_sm().child("Untitled.md"))
    .suffix(Button::new("sidebar").icon(IconName::PanelLeft).ghost())
```

### 带菜单栏

```rust
TitleBar::new()
    .child(
        div()
            .flex()
            .items_center()
            .child(AppMenuBar::new(window, cx))
    )
    .child(
        div()
            .flex()
            .items_center()
            .justify_end()
            .gap_2()
            .child(Button::new("github").icon(IconName::GitHub))
            .child(Button::new("notifications").icon(IconName::Bell))
    )
```

在 Windows 和 Linux 上，可以像 Windows 用户习惯的那样用键盘操作菜单栏：按 F10 或单独轻按 Alt 会聚焦第一个菜单标题，左右方向键在标题间移动，下方向键、Enter 或空格打开当前菜单，Esc 则把焦点还给原来的位置。
按住 Alt 再按菜单名中带下划线的字母可直接打开该菜单；单独按住 Alt 或菜单栏获得键盘焦点时，会显示这些下划线。
即使窗口中没有任何焦点，这些按键同样有效；它们在焦点元素自身的快捷键之后处理，并且不会穿过已打开的对话框或 Sheet。
与指针同时使用的 Alt（Alt+点击、Alt+拖动、Alt+滚轮），以及跨越窗口焦点变化而按住的 Alt，都不算轻按。
这些按键对应 `FocusAppMenuBar` 和 `AltTap` 两个 Action，分别绑定到 `f10` 和 `alt`，应用可以重新绑定。

### Linux 自定义关闭行为

```rust
TitleBar::new()
    .on_close_window(|_, window, cx| {
        window.push_notification("Saving before close...", cx);
        window.remove_window();
    })
    .child(div().child("Custom Close Behavior"))
```

### 自定义样式

```rust
TitleBar::new()
    .bg(cx.theme().primary)
    .border_color(cx.theme().primary_border)
    .child(
        div()
            .text_color(cx.theme().primary_foreground)
            .child("Styled Title Bar")
    )
```

### 窗口配置

推荐以 `TitleBar::window_options()` 作为窗口配置的基础，它会配置好标题栏所需的
全部选项，其中包括让标题栏（而不是系统）来处理拖动和双击。

```rust
use gpui_kit::WindowOptions;

WindowOptions {
    window_bounds: Some(window_bounds),
    ..TitleBar::window_options()
}
```

如果自行构造 [`WindowOptions`]，需要同时设置这两个字段：

```rust
use gpui_kit::WindowOptions;

WindowOptions {
    titlebar: Some(TitleBar::title_bar_options()),
    // macOS 上必须设置，否则系统也会处理标题栏双击，
    // 并且会为了判定双击而延迟投递标题栏点击。
    app_owns_titlebar_drag: true,
    ..Default::default()
}
```

## 平台差异

标题栏使用主题的 `title_bar` 颜色绘制；除非主题为它指定渐变，否则为单一纯色。
每个窗口按钮都是名为 Minimize、Maximize、Restore 或 Close 的按钮，便于辅助技术识别。

### macOS

- 使用原生红黄绿窗口按钮
- traffic light 默认位置为 `(9px, 9px)`
- 双击标题栏会调用 `window.titlebar_double_click()`
- 左侧默认预留 `80px`；全屏时红黄绿按钮被隐藏，留白缩小为 `12px`
- 默认表现为透明标题栏

### Windows

- 使用 Windows 11 样式的标题按钮，宽 `46px`，以 Segoe Fluent Icons 字形绘制（Windows 10 上使用 Segoe MDL2 Assets，两者都未安装时使用图标）
- 由 Windows 通过 `WindowControlArea` 处理，因此支持贴靠布局
- 悬停时显示浅色填充，关闭按钮悬停时为系统红色背景配白色字形
- 窗口未激活时字形变淡；窗口不可最小化时禁用最小化按钮，不可调整大小时禁用最大化按钮
- 左侧默认内边距为 `12px`

### Linux

- 在客户端装饰下，按桌面的按钮布局（GNOME 的 `button-layout`）决定窗口按钮所在的一侧和顺序，只绘制合成器支持的按钮，并在布局变化时重绘
- 可通过 `button_layout()` 由应用自行指定按钮布局
- 支持通过 `on_close_window()` 覆盖关闭逻辑
- 支持双击最大化 / 还原
- 支持右键弹出窗口菜单
- 支持在标题栏区域拖动窗口

## API 参考

### TitleBar

| 方法 | 说明 |
| --- | --- |
| `new()` | 创建标题栏 |
| `child(element)` | 向标题栏中添加子元素 |
| `center(element)` | 以窗口宽度居中显示的内容 |
| `suffix(element)` | 位于窗口按钮之前的内容 |
| `button_layout(layout)` | 指定 Linux 窗口按钮的布局 |
| `on_close_window(fn)` | 自定义关闭行为，仅 Linux 有效 |
| `title_bar_options()` | 获取窗口可用的默认标题栏配置 |
| `window_options()` | 获取标题栏配套的默认窗口配置 |

### 窗口配置项

| 属性 | 说明 |
| --- | --- |
| `appears_transparent` | 标题栏透明（默认 true） |
| `traffic_light_position` | macOS 红黄绿按钮位置 |
| `title` | 窗口标题（使用自定义标题栏时可选） |
| `app_owns_titlebar_drag` | 由标题栏自行处理拖动与双击（仅 macOS） |

### 常量

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `TITLE_BAR_HEIGHT` | `34px` | 标准标题栏高度 |

## 说明

- 组件会自动处理平台相关的标题栏行为
- Windows 和 Linux 才会渲染自定义窗口控制按钮
- 拖拽窗口的逻辑已内置在合适区域中
- 自定义样式时应考虑各平台对标题栏的交互习惯
