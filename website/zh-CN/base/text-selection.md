---
title: Text Selection
description: 在多个自绘文本参与者之间实现窗口级文本选择。
order: 3
---

# Text Selection

`gpui-base` 的文本选择基础设施允许一次拖拽跨越多个独立绘制的文本块，同时保留每个参与者自己的布局与绘制逻辑。它适合文档、消息流和其他由多个自定义元素组成、但用户期望像连续文本一样选择的界面。

## 普通可选择文本

`SelectableText` 是现成的参与者，用于一段普通文本：

```rust
use gpui_kit::base::SelectableText;

SelectableText::new("result", "Find matching text")
    .highlights([(0..4, HighlightStyle {
        font_weight: Some(FontWeight::BOLD),
        background_color: Some(match_color),
        ..Default::default()
    })])
```

元素 ID 在多次渲染之间要保持稳定。多段文本组成同一个可选择文档时，使用 `SelectableText::with_handle` 和 `document_order`。

`highlights` 接收有序、互不重叠的 UTF-8 字节范围，每个范围配一个 `HighlightStyle`，例如搜索命中的位置。它们只改变文字的绘制方式，不改变选中和复制的内容；再次调用会替换之前的高亮。

`SelectableText::layout` 返回绘制该文本段所用的 `TextLayout`。包装该文本段的元素可以在 `prepaint` 或 `paint` 中读取它，例如用 `position_for_index` 找到搜索匹配所在的行，无需再次排版文本。

## 开始使用

核心由窗口级 `TextSelectionState`、稳定的 `SelectableTextHandle`、prepaint 阶段注册的几何信息，以及把全局选择投影到各文本 run 的辅助 API 组成。

## 工作方式

每个参与者在 prepaint 时报告文本、屏幕几何和逻辑顺序。窗口状态根据指针锚点与当前点计算跨参与者的选择区间；绘制时，各参与者只查询落在自身范围内的切片并绘制选中背景。状态更新发生在事件回调中，不在 `render` 中写入。

## 安装窗口元素

在窗口内容的稳定外层安装选择宿主，使它能够接收拖拽、释放和复制操作。宿主应覆盖所有需要共同选择的参与者，但不应改变它们的布局或样式。

## 创建稳定句柄

每个参与者的 `SelectableTextHandle` 应保存在 entity 中或由稳定数据键派生。不要在每次渲染时创建新身份，否则拖拽途中重绘会丢失锚点、逻辑顺序或选择投影。

## 在 prepaint 注册几何

文本最终布局完成后注册边界、行与字符位置。注册顺序必须与用户看到和辅助技术读取的逻辑顺序一致。动态插入、删除或移动参与者时，用稳定身份更新对应记录。

## 把选择投影到文本 run

绘制各 run 前查询当前窗口选择，取得与本地文本相交的字节或字符范围，再把范围转换为文本系统需要的高亮几何。注意 UTF-8 边界，不要把字节偏移当作字符索引。

## 完整 Rust 示例

```bash
cargo run -p gpui-base-examples -- text-selection
```

<<< ../../../crates/base/examples/showcase/components/text_selection.rs{rust}

## 查询与控制窗口选择

应用可以读取当前选中文本、主动清除选择，并把复制命令连接到窗口状态。程序化更新后调用 `cx.notify()`，让所有受影响参与者重绘。

双击选中单词，单词边界规则与 `Input` 相同；三击及更多次点击选中以换行分隔的逻辑行。双击或三击后不松开继续拖动，或之后按住 Shift 点击，选区会按整词或整行扩展，并保留最初选中的单词或行。

### 触摸选择

在参与者上长按会选中手指下的单词；抬起手指后，这个选区成为*触摸选区*：两端各带一个拖动 handle，并附带编辑菜单。手势和拖动由 Base 负责；展示层根据 `TextSelectionSnapshot` 之外的 `TouchSelectionSnapshot` 绘制 handle 和菜单，它以窗口坐标提供选区两端的光标行框。

通过 `TextSelection::touch_selection` 读取快照，用 `TextSelection::observe_touch_selection` 在快照变化时重绘；`begin_edge_drag`、`update_edge_drag`、`end_edge_drag` 拖动其中一端，`select_all` 全选被按下的参与者，`close_edit_menu` 在菜单自身的命令执行后关闭菜单。handle 由参与者自己绘制，位于它在绘制顺序中的位置，因此盖住文字的东西也会盖住 handle：在 prepaint 调用 `TextSelectionHandle::prepaint_touch_handles`（插入手指可按的 hitbox），在 paint 末尾、`register` 之后调用 `TextSelectionHandle::paint_touch_handles` 并传入选区颜色；`TextView` 已完成这两步。参与者通过 `TextSelectionRegistration::with_selection_edges` 上报其选区两端的绘制位置。绘制菜单的一方需要在每帧 paint 时调用 `TextSelection::register_touch_ui` 登记菜单 bounds，这样落在菜单上的按压不会清除它所属的选区；GPUI Component 的 `Root` 负责绘制菜单。

### 上下文菜单

在参与者上松开的右键会向应用请求可选择文本的上下文菜单，获得焦点的 `TextView` 中按 Shift-F10 或 Menu 键也会。右键无论落在哪里都不改变选区。指针下接管按压的控件会保留右键：它在右键按下时调用 `GlobalState::suppress_text_selection(cx)`（输入框就是这样做的），或者停止事件传播（设置了 `on_link_click` 的链接就是这样做的）。

```rust
use gpui_kit::base::{CopyTextSelection, SelectAllText, TextSelection};

TextSelection::on_context_menu(
    |request, window, cx| {
        // request.position()、request.selected_text()、request.has_selection()
        // 和 request.supports_select_all() 描述要显示的菜单。
        // 菜单项分派 CopyTextSelection 和 SelectAllText。
    },
    cx,
);

// 从其他地方打开菜单，效果与在 `position` 处右键相同。
TextSelection::show_context_menu(&handle, position, window, cx);
```

整个应用只有一个处理函数，再次设置会替换之前的。`CopyTextSelection` 复制的内容与 Cmd-C 或 Ctrl-C 相同，与焦点在哪里无关；`SelectAllText` 全选打开菜单的那个参与者。两者都由 `Root` 处理。参与者通过 `TextSelectionHandle::select_all_with` 提供回调来支持全选，`TextView` 已经这样做。由 GPUI 绘制的菜单需要在每帧 paint 时调用 `TextSelection::register_menu(bounds, window, cx)`，这样按下菜单项时选区会保留，与按下操作系统菜单一样。GPUI Component 安装了一个显示 `复制` 和 `全选` 的处理函数。

## 高级参与者适配器

自定义文本布局可以实现参与者接口，提供命中测试、范围投影和文本提取。适配器应只桥接已有布局数据，避免在指针移动热路径中重新排版或分配大型缓冲区。

## 用 scope 隔离模态内容

Dialog、Sheet 等模态内容应使用独立 scope，避免一次选择跨越背景与前景。模态关闭后恢复原 scope，不要让已卸载参与者留在窗口注册表中。

## 集成检查清单

- 窗口只安装一个相应 scope 的选择宿主。
- 参与者身份和逻辑顺序跨渲染稳定。
- 只在 prepaint 注册最终几何，并正确处理 UTF-8。
- 拖拽更新有界，不在 render 中修改状态。
- 验证跨块拖拽、反向选择、复制、动态内容、滚动和模态隔离。
