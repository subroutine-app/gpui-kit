---
title: Kbd
description: 以符合平台习惯的方式显示键盘快捷键。
---

# Kbd

Kbd 用于展示键盘快捷键和组合键，并会自动根据平台采用合适的显示格式。macOS 会使用符号，Windows 和 Linux 则使用文本标签，便于文档、菜单和帮助面板保持一致的快捷键表达。

快捷键以键帽形式绘制：一个小圆角按键，上面显示完整的快捷键，例如 `Ctrl+Shift+Z` 或 `⇧⌘Z`，并使用主题的等宽字体。

## 导入

```rust
use gpui_kit::component::kbd::Kbd;
use gpui_kit::Keystroke;
```

## 用法

### 基础快捷键

```rust
let kbd = Kbd::new(Keystroke::parse("cmd-shift-p").unwrap());
let kbd: Kbd = Keystroke::parse("escape").unwrap().into();
```

### 常见快捷键

```rust
Kbd::new(Keystroke::parse("cmd-shift-p").unwrap())
Kbd::new(Keystroke::parse("cmd-t").unwrap())
Kbd::new(Keystroke::parse("cmd--").unwrap())
Kbd::new(Keystroke::parse("cmd-+").unwrap())
Kbd::new(Keystroke::parse("escape").unwrap())
Kbd::new(Keystroke::parse("enter").unwrap())
Kbd::new(Keystroke::parse("backspace").unwrap())
```

### 多修饰键

```rust
Kbd::new(Keystroke::parse("cmd-ctrl-shift-a").unwrap())
Kbd::new(Keystroke::parse("cmd-alt-backspace").unwrap())
Kbd::new(Keystroke::parse("ctrl-alt-shift-a").unwrap())
```

### 方向键与功能键

```rust
Kbd::new(Keystroke::parse("left").unwrap())
Kbd::new(Keystroke::parse("right").unwrap())
Kbd::new(Keystroke::parse("up").unwrap())
Kbd::new(Keystroke::parse("down").unwrap())
Kbd::new(Keystroke::parse("f12").unwrap())
Kbd::new(Keystroke::parse("secondary-f12").unwrap())
Kbd::new(Keystroke::parse("pageup").unwrap())
Kbd::new(Keystroke::parse("pagedown").unwrap())
```

### 关闭默认外观

```rust
Kbd::new(Keystroke::parse("cmd-s").unwrap())
    .appearance(false)
```

### 从 Action 绑定读取

```rust
use gpui_kit::{Action, Window, FocusHandle};

if let Some(kbd) = Kbd::binding_for_action(&MyAction {}, None, window) {
    // 显示该 action 绑定的快捷键
}

if let Some(kbd) = Kbd::binding_for_action(&MyAction {}, Some("Editor"), window) {
    // 显示特定上下文中的快捷键
}

if let Some(kbd) = Kbd::binding_for_action_in(&MyAction {}, &focus_handle, window) {
    // 显示焦点元素上的快捷键
}
```

由多个连续按键组成的绑定（例如 Emacs 的 `ctrl-x ctrl-s`）会为每个按键各显示一个键帽。

## 平台差异

### macOS

- 使用符号：⌃ ⌥ ⇧ ⌘
- 修饰键之间不加分隔符
- 顺序为 Control、Option、Shift、Command
- 特殊键使用 ⌫、⎋、⏎、← → ↑ ↓ 等符号

### Windows / Linux

- 使用文本标签：Ctrl、Alt、Shift、Win
- 修饰键之间使用 `+`
- 顺序为 Ctrl、Alt、Shift、Win
- 特殊键显示为 Backspace、Esc、Enter、Left、Right、Up、Down

### 平台示例

| 输入 | macOS | Windows / Linux |
| --- | --- | --- |
| `cmd-a` | ⌘A | Win+A |
| `ctrl-shift-a` | ⌃⇧A | Ctrl+Shift+A |
| `cmd-alt-backspace` | ⌥⌘⌫ | Win+Alt+Backspace |
| `escape` | ⎋ | Esc |
| `enter` | ⏎ | Enter |
| `left` | ← | Left |

## 示例

### 快捷键帮助面板

```rust
use gpui_kit::{div, h_flex, v_flex};

v_flex()
    .gap_2()
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Open command palette:")
            .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()))
    )
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Save file:")
            .child(Kbd::new(Keystroke::parse("cmd-s").unwrap()))
    )
    .child(
        h_flex()
            .gap_2()
            .items_center()
            .child("Find in files:")
            .child(Kbd::new(Keystroke::parse("cmd-shift-f").unwrap()))
    )
```

### 带快捷键的菜单项

```rust
h_flex()
    .justify_between()
    .items_center()
    .child("New File")
    .child(Kbd::new(Keystroke::parse("cmd-n").unwrap()))
```

### 行内说明

```rust
div()
    .child("Press ")
    .child(Kbd::new(Keystroke::parse("escape").unwrap()))
    .child(" to cancel or ")
    .child(Kbd::new(Keystroke::parse("enter").unwrap()))
    .child(" to confirm.")
```

### 自定义样式

```rust
Kbd::new(Keystroke::parse("cmd-k").unwrap())
    .text_color(cx.theme().accent)
    .border_color(cx.theme().accent)
    .bg(cx.theme().accent.opacity(0.1))
```

### 仅获取文本格式

```rust
let shortcut_text = Kbd::format(&Keystroke::parse("cmd-shift-p").unwrap());
div().child(format!("Shortcut: {}", shortcut_text))
```

## 样式

键帽以主题的 `muted.foreground` 显示文字，底色为 `muted.background`，边缘颜色取自 `muted.foreground` 且底边略粗，因此在菜单的高亮行上也能看清键帽。`outline()` 会改用主题背景色填充键帽。
文字使用主题的 `mono_font_family`，字号比超小字号再小一级（`0.6875rem`），因为同样字号下等宽字体看起来比界面字体更大；可通过 `font_family` 换用其他字体。圆角跟随主题的 radius。

所有样式都可以通过 `Styled` trait 的方法覆盖。
