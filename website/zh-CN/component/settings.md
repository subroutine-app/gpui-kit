---
title: Settings
description: 用于构建设置页、设置分组和设置项的界面组件。
---

# Settings

> Since: v0.5.0

Settings 组件用于构建应用设置界面，支持页面分组、按标题/描述/自定义关键词搜索过滤以及多种字段类型，适合实现类似 macOS 或 iOS 设置页的结构。

## 导入

```rust
use gpui_kit::component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField};
// 由应用自行管理布局时使用的构件：
use gpui_kit::component::setting::{SettingList, SettingRow, SettingSection};
```

## 用法

### 构建设置界面

可组合以下组件来组织设置页：

- [Settings]：顶层设置容器，持有多个设置页面。
- [SettingPage]：一组相关设置的页面。
- [SettingGroup]：设置分组，渲染为一个 [SettingSection]。
- [SettingItem]：单个设置项，包含标题、描述和字段，渲染为一个 [SettingRow]。
- [SettingField]：具体字段，如 Input、Dropdown、Switch 等。

整体层级如下：

```
Settings
  SettingPage
    SettingGroup          -> SettingSection：标题和描述位于表面上方
      SettingItem         -> SettingRow，与相邻项之间以细线分隔
        Title（info 图标）
        Description（可选，位于标题下方）
        SettingField      -> 行尾控件
```

当应用自行管理页面布局和导航时，也可以单独使用 [SettingRow]、[SettingSection]
和 [SettingList]，见[构件](#构件)。

### 基础示例

```rust
use gpui_kit::component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField};

Settings::new("my-settings")
    .pages(vec![
        SettingPage::new("General")
            .group(
                SettingGroup::new()
                    .title("Basic Options")
                    .item(
                        SettingItem::new(
                            "Enable Feature",
                            SettingField::switch(
                                |cx: &App| true,
                                |val: bool, cx: &mut App| {
                                    println!("Feature enabled: {}", val);
                                },
                            )
                        )
                    )
            )
    ])
```

### 多页面

:::info
如果你希望某个页面默认展开，可在 [SettingPage] 上使用 `default_open(true)`。
:::

```rust
Settings::new("app-settings")
    .pages(vec![
        SettingPage::new("General")
            .default_open(true)
            .group(SettingGroup::new().title("Appearance").items(vec![...])),
        SettingPage::new("Software Update")
            .group(SettingGroup::new().title("Updates").items(vec![...])),
        SettingPage::new("About")
            .group(SettingGroup::new().items(vec![...])),
    ])
```

### 搜索时的选择行为

搜索时，如果当前页面仍包含匹配的设置项，就保留当前页面；否则选择第一个匹配页面。
当前选中的分组仍匹配时保留该分组，否则退回页面级选择。
清空搜索会保留当前页面，不会恢复搜索前的选择。
没有匹配结果时不显示页面内容，并暂存选择，供结果恢复时使用。

### 窄宽度布局

当 Settings 的宽度小于 480 px 时，搜索框和页面选择器会取代侧边栏，显示在当前页面上方，所有设置项也改用纵向布局。
选择器只列出与搜索匹配的页面，选中某一项即打开对应页面。按 Tab 键可从搜索框移到选择器，再进入页面内容。
宽度恢复后，侧边栏重新出现，并保持当前选中的页面。

### 分组样式

分组样式决定设置项所在的表面。设置项之间始终以细线分隔，分组的标题和描述位于表面上方：

- `GroupBoxVariant::Normal`（默认）：没有表面，设置行直接位于页面上。
- `GroupBoxVariant::Outline`：带边框的卡片。
- `GroupBoxVariant::Fill`：使用 `group_box` 颜色填充、带边框的卡片。

```rust
use gpui_kit::component::group_box::GroupBoxVariant;

Settings::new("my-settings")
    .with_group_variant(GroupBoxVariant::Outline)
    .pages(vec![...])

Settings::new("my-settings")
    .with_group_variant(GroupBoxVariant::Fill)
    .pages(vec![...])
```

单个分组可以覆盖全局分组样式，例如让某一页的内容直接呈现，而其他页面
保留全局的卡片外观：

```rust
SettingGroup::new()
    .variant(GroupBoxVariant::Normal)
    .items(vec![...])
```

## Setting Page

### 基础页面

```rust
SettingPage::new("General")
    .group(SettingGroup::new().title("Options").items(vec![...]))
```

### 多分组

```rust
SettingPage::new("General")
    .groups(vec![
        SettingGroup::new().title("Appearance").items(vec![...]),
        SettingGroup::new().title("Font").items(vec![...]),
        SettingGroup::new().title("Other").items(vec![...]),
    ])
```

### 图标

```rust
SettingPage::new("General")
    .icon(IconName::Settings)
    .groups(vec![...])
```

### 标题后缀

使用 `title_suffix` 在页头标题后渲染自定义元素，例如一个点击后打开帮助文档的 info 图标按钮：

```rust
SettingPage::new("General")
    .title_suffix(|_, _| {
        Button::new("help")
            .icon(IconName::Info)
            .ghost()
            .xsmall()
            .on_click(|_, _, cx| cx.open_url("https://example.com/help"))
    })
    .groups(vec![...])
```

### 默认展开

```rust
SettingPage::new("General")
    .default_open(true)
    .groups(vec![...])
```

### 支持重置

```rust
SettingPage::new("General")
    .resettable(true)
    .groups(vec![...])
```

## Setting Group

### 基础分组

```rust
SettingGroup::new()
    .title("Appearance")
    .items(vec![
        SettingItem::new(...),
        SettingItem::new(...),
    ])
```

### 单项分组

```rust
SettingGroup::new()
    .title("Font")
    .item(SettingItem::new(...))
```

### 无标题分组

```rust
SettingGroup::new()
    .items(vec![...])
```

### 分组表面外的底部说明

用 `footer` 在分组的背景或边框下方渲染辅助内容。它与分组标题左对齐，并像描述文字一样以小号 muted 文本渲染，直接传入纯文本即可；闭包接收当前窗口和应用上下文，可用于更复杂的内容。它随分组一起滚动和过滤，不会成为独立的可搜索设置项，也不会新增侧栏入口；分组仍需至少一个设置项才会显示。

```rust
SettingGroup::new()
    .item(SettingItem::new(
        "Update source",
        SettingField::render(|_, _, _| "GitHub Releases"),
    ))
    .footer(|_, _| "Changes apply to this device only.")
```

### 缓存独立分组

为分组传入 `SettingGroup::key(u64)` 后，在 key 改变之前会复用该分组的布局与绘制结果。
key 应由所有会影响分组内容、样式或回调的应用数据得出，例如修订计数或哈希值。
`Input` 等基于 Entity 的控件在其状态通知时仍会自行重绘；搜索、可用宽度、尺寸、分组样式和主题的变化也会自动刷新受影响的分组。

```rust
SettingGroup::new()
    .key(preferences_revision)
    .title("Editing")
    .items(editing_items)
```

key 表示修订版本，而不是身份标识：分组移动或被替换时，需要同时更改相应位置的 key。
未设置 key 的分组仍像以前一样随页面一起渲染。编辑过程中请保持页面和分组顺序稳定，以便控件保留自身状态。

## Setting Item

### 基础设置项

```rust
SettingItem::new("Title", SettingField::switch(...))
    .description("Description text")
```

描述以较小的 muted 文本显示在标题下方。当一行的宽度不足以并排放下标题和字段时，字段会换到标题下方。

### Info

用 `info` 提供大多数人不必每次阅读的补充说明。它会在标题旁渲染一个 info 图标，并在 tooltip 中显示说明文字；该图标可以通过键盘聚焦，说明文字即其无障碍标签。info 文字也参与搜索。用户做出选择所必需的信息应放在描述中。

```rust
SettingItem::new("Telemetry", SettingField::switch(...))
    .description("Send anonymous usage counts.")
    .info("File contents and names are never included.")
```

### 使用 render closure 的自定义项

```rust
SettingItem::render(|options, _, _| {
    h_flex()
        .w_full()
        .justify_between()
        .child("Custom content")
        .child(
            Button::new("action")
                .label("Action")
                .with_size(options.size)
        )
        .into_any_element()
})
```

### 纵向布局

设置项默认是横向布局；可通过 `layout(Axis::Vertical)` 切换为纵向布局：

```rust
SettingItem::new(
    "CLI Path",
    SettingField::input(...)
)
.layout(Axis::Vertical)
.description("This item uses vertical layout.")
```

### Markdown 描述

```rust
use gpui_kit::component::text::markdown;

SettingItem::new(
    "Documentation",
    SettingField::element(...)
)
.description(markdown("Rust doc for the `gpui-component` crate."))
```

### 禁用状态

通过 `disabled(true)` 可以将设置项置为不可交互状态：整行会变暗，内置字段
（Switch、Checkbox、Input、Dropdown、NumberInput）也会被自动禁用。

```rust
SettingItem::new(
    "Dark Mode",
    SettingField::switch(...)
)
.description("Switch between light and dark themes.")
.disabled(true)
```

对于 [SettingItem::render] 自定义项，整行同样会自动变暗，但其中可交互的控件
需要渲染闭包通过 `options.disabled` 自行处理：

```rust
SettingItem::render(|options, _, _| {
    h_flex()
        .child("Custom content")
        .child(
            Button::new("action")
                .label("Action")
                .with_size(options.size)
                .disabled(options.disabled)
        )
        .into_any_element()
})
.disabled(true)
```

### 搜索关键词

通过 `keywords` 可以为设置项附加额外的搜索关键词。这些关键词仅用于搜索匹配，
不会被渲染出来。例如，标题为 "Enable Two-factor auth" 的设置项可以通过 "MFA"
搜索到：

```rust
SettingItem::new(
    "Enable Two-factor auth",
    SettingField::switch(...)
)
.keywords(["MFA", "2FA"])
```

这对于没有标题和描述、但仍希望能被搜索到的 [SettingItem::render] 自定义项同样
有用：

```rust
SettingItem::render(|options, _, _| {
    h_flex().child("Custom content").into_any_element()
})
.keywords(["Advanced", "Network"])
```

## 构件

[Settings] 管理一个完整的页面：侧栏、搜索和滚动列表。当应用自行管理页面布局、导航和搜索框时，可以用它的组成部分构建同样的外观。这些构件不持有状态：所有值和控件都由应用持有，并在每次渲染时传入。

- [SettingRow]：标签及其下方的描述、可选的 info 图标，以及行尾的控件或值。
- [SettingSection]：带标题的一组设置行，位于同一表面上并以细线分隔，可带底部说明。
- [SettingList]：按应用持有的搜索词过滤的一组分区。

```rust
use gpui_kit::component::{
    group_box::GroupBoxVariants as _,
    setting::{SettingList, SettingRow, SettingSection},
    select::Select,
    switch::Switch,
};

let general = SettingSection::new("general")
    .title("General")
    .description("How the application starts and behaves.")
    .fill()
    .row(
        SettingRow::new("launch-at-login", "Open at login")
            .description("Start in the background when you sign in.")
            .switch(
                Switch::new("launch-at-login-switch")
                    .checked(launch_at_login)
                    .on_click(|checked, _, cx| { /* 保存 `*checked` */ }),
            ),
    )
    .row(
        SettingRow::new("density", "Density")
            .info("Compact fits more rows on screen.")
            .keywords(["comfortable", "compact"])
            .control(Select::new(&density_state).w(rems(10.))),
    )
    .row(SettingRow::new("version", "Version").value("0.9.0"));

SettingList::new("settings-list")
    .query(search_input.read(cx).value())
    .sections([general]);
```

### Setting Row

- `control(element)`：在行尾放置任意元素，例如 `Select`、`Button`、`Input` 或自定义组合控件。
- `switch(switch)`：放置一个 `Switch`，并以该行的标签为其命名。
- `value(text)`：当该行没有控件时，在行尾以 muted 文本显示值，例如状态。
- `layout(Axis::Vertical)`：始终把全宽控件放在标签下方。
- `label_width(width)`：设置标签列的最小宽度（默认 `7.5rem`）；当一行窄于该宽度加上控件宽度时，控件换到标签下方。分区的 `label_width` 作用于未自行设置宽度的设置行。

禁用的设置行会变暗。通过 `switch` 传入的 Switch 会随之禁用。设置行不会干预通过 `control` 传入的控件，因此请同时传入已禁用、且不注册操作处理函数的控件：

```rust
SettingRow::new("sync-now", "Sync now")
    .disabled(!sync_enabled)
    .control(
        Button::new("sync-now-button")
            .label("Sync Now")
            .disabled(!sync_enabled)
            .when(sync_enabled, |this| this.on_click(|_, _, cx| { /* 同步 */ })),
    )
```

### Setting Section

分区的表面遵循其 [GroupBox] 样式，可通过 `normal()`、`outline()`、`fill()` 或 `with_variant(...)` 设置，默认为 `Normal`。通过 `child` 添加的子元素是位于设置行之间的全宽块，按调用顺序排列，并使用相同的内边距和分隔线：

```rust
SettingSection::new("data")
    .title("Data")
    .outline()
    .row(SettingRow::new("cache", "Offline data").control(remove_button))
    .child(confirmation_panel)
    .footer("Synced data stays in your account.")
```

### Setting List

[SettingList] 以不区分大小写的方式，将搜索词与分区的标题和描述，以及设置行的标签、描述、info、值和关键词进行匹配。标题或描述匹配的分区保留其全部内容；否则只保留匹配的设置行，块会被移除，因为其内容无法被搜索。顺序保持不变。可用 `empty(element)` 替换默认的无结果状态。

## Setting Fields

[SettingField] 枚举提供了多种常见字段类型。

### Switch

```rust
SettingItem::new(
    "Dark Mode",
    SettingField::switch(
        |cx: &App| cx.theme().mode.is_dark(),
        |val: bool, cx: &mut App| {
            // Handle value change
        },
    )
    .default_value(false)
)
```

### Checkbox

```rust
SettingItem::new(
    "Auto Switch Theme",
    SettingField::checkbox(
        |cx: &App| AppSettings::global(cx).auto_switch_theme,
        |val: bool, cx: &mut App| {
            AppSettings::global_mut(cx).auto_switch_theme = val;
        },
    )
    .default_value(false)
)
```

### Input

```rust
SettingItem::new(
    "CLI Path",
    SettingField::input(
        |cx: &App| AppSettings::global(cx).cli_path.clone(),
        |val: SharedString, cx: &mut App| {
            AppSettings::global_mut(cx).cli_path = val;
        },
    )
    .default_value("/usr/local/bin/bash".into())
)
.layout(Axis::Vertical)
.description("Path to the CLI executable.")
```

### Dropdown

选项文字超出整行宽度时会以省略号截断，菜单中仍完整显示。

```rust
SettingItem::new(
    "Font Family",
    SettingField::dropdown(
        vec![
            ("Arial".into(), "Arial".into()),
            ("Helvetica".into(), "Helvetica".into()),
            ("Times New Roman".into(), "Times New Roman".into()),
        ],
        |cx: &App| AppSettings::global(cx).font_family.clone(),
        |val: SharedString, cx: &mut App| {
            AppSettings::global_mut(cx).font_family = val;
        },
    )
    .default_value("Arial".into())
)
```

### NumberInput

```rust
use gpui_kit::component::setting::NumberFieldOptions;

SettingItem::new(
    "Font Size",
    SettingField::number_input(
        NumberFieldOptions {
            min: 8.0,
            max: 72.0,
            ..Default::default()
        },
        |cx: &App| AppSettings::global(cx).font_size,
        |val: f64, cx: &mut App| {
            AppSettings::global_mut(cx).font_size = val;
        },
    )
    .default_value(14.0)
)
```

### 使用 render closure 创建自定义字段

```rust
SettingItem::new(
    "GitHub Repository",
    SettingField::render(|options, _window, _cx| {
        Button::new("open-url")
            .outline()
            .label("Repository...")
            .with_size(options.size)
            .on_click(|_, _window, cx| {
                cx.open_url("https://github.com/example/repo");
            })
    })
)
```

### 自定义字段元素

如果某个字段逻辑较复杂并且需要复用，可以实现 [SettingFieldElement] trait：

```rust
use gpui_kit::component::setting::{SettingFieldElement, RenderOptions};

struct OpenURLSettingField {
    label: SharedString,
    url: SharedString,
}

impl SettingFieldElement for OpenURLSettingField {
    type Element = Button;

    fn render_field(&self, options: &RenderOptions, _: &mut Window, _: &mut App) -> Self::Element {
        let url = self.url.clone();
        Button::new("open-url")
            .outline()
            .label(self.label.clone())
            .with_size(options.size)
            .on_click(move |_, _window, cx| {
                cx.open_url(url.as_str());
            })
    }
}
```

然后在设置项中这样使用：

```rust
SettingItem::new(
    "GitHub Repository",
    SettingField::element(OpenURLSettingField {
        label: "Repository...".into(),
        url: "https://github.com/longbridge/gpui-kit".into(),
    })
)
```

## API 参考

- [Settings]
- [SettingPage]
- [SettingGroup]
- [SettingItem]
- [SettingField]
- [SettingSection]
- [SettingRow]
- [SettingList]
- [NumberFieldOptions]

### 尺寸

实现了 [Sizable] trait：

- `xsmall()`：超小尺寸
- `small()`：小尺寸
- `medium()`：中尺寸，默认值
- `large()`：大尺寸
- `with_size(Size)`：指定具体尺寸

## 示例

### 完整设置页示例

```rust
use gpui_kit::{App, SharedString};
use gpui_kit::component::{
    Settings, SettingPage, SettingGroup, SettingItem, SettingField,
    setting::NumberFieldOptions,
    group_box::GroupBoxVariant,
    Size,
};

Settings::new("app-settings")
    .with_size(Size::Medium)
    .with_group_variant(GroupBoxVariant::Outline)
    .pages(vec![
        SettingPage::new("General")
            .resettable(true)
            .default_open(true)
            .groups(vec![
                SettingGroup::new()
                    .title("Appearance")
                    .items(vec![
                        SettingItem::new(
                            "Dark Mode",
                            SettingField::switch(
                                |cx: &App| cx.theme().mode.is_dark(),
                                |val: bool, cx: &mut App| {
                                    // Handle theme change
                                },
                            )
                        )
                        .description("Switch between light and dark themes."),
                    ]),
                SettingGroup::new()
                    .title("Font")
                    .items(vec![
                        SettingItem::new(
                            "Font Family",
                            SettingField::dropdown(
                                vec![
                                    ("Arial".into(), "Arial".into()),
                                    ("Helvetica".into(), "Helvetica".into()),
                                ],
                                |cx: &App| "Arial".into(),
                                |val: SharedString, cx: &mut App| {
                                    // Handle font change
                                },
                            )
                        ),
                        SettingItem::new(
                            "Font Size",
                            SettingField::number_input(
                                NumberFieldOptions {
                                    min: 8.0,
                                    max: 72.0,
                                    ..Default::default()
                                },
                                |cx: &App| 14.0,
                                |val: f64, cx: &mut App| {
                                    // Handle size change
                                },
                            )
                        ),
                    ]),
            ]),
        SettingPage::new("Software Update")
            .resettable(true)
            .group(
                SettingGroup::new()
                    .title("Updates")
                    .items(vec![
                        SettingItem::new(
                            "Auto Update",
                            SettingField::switch(
                                |cx: &App| true,
                                |val: bool, cx: &mut App| {
                                    // Handle auto update
                                },
                            )
                        )
                        .description("Automatically download and install updates."),
                    ])
            ),
    ])
```

[Settings]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.Settings.html
[SettingPage]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingPage.html
[SettingGroup]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingGroup.html
[SettingItem]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingItem.html
[SettingField]: https://docs.rs/gpui-component/latest/gpui_component/setting/enum.SettingField.html
[SettingFieldElement]: https://docs.rs/gpui-component/latest/gpui_component/setting/trait.SettingFieldElement.html
[NumberFieldOptions]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.NumberFieldOptions.html
[SettingSection]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingSection.html
[SettingRow]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingRow.html
[SettingList]: https://docs.rs/gpui-component/latest/gpui_component/setting/struct.SettingList.html
[GroupBox]: ./group-box.md
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
