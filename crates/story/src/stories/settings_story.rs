use gpui_kit::{
    App, AppContext, Axis, Context, Element, Entity, FocusHandle, Focusable, Global, IntoElement,
    ParentElement as _, Render, SharedString, Styled, Window, div, prelude::FluentBuilder, px,
    rems,
};

use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, Size, Theme, ThemeMode,
    button::{Button, ButtonVariants},
    group_box::{GroupBoxVariant, GroupBoxVariants as _},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    scroll::ScrollableElement as _,
    select::{Select, SelectState},
    setting::{
        NumberFieldOptions, RenderOptions, SettingField, SettingFieldElement, SettingGroup,
        SettingItem, SettingList, SettingPage, SettingRow, SettingSection, Settings,
    },
    switch::Switch,
    tab::TabBar,
    text::markdown,
    v_flex,
};

struct AppSettings {
    auto_switch_theme: bool,
    cli_path: SharedString,
    font_family: SharedString,
    font_size: f64,
    line_height: f64,
    /// Demonstrates a custom element field driving its own state, with reset
    /// support wired up via [`SettingField::on_reset`].
    density: SharedString,
    notifications_enabled: bool,
    auto_update: bool,
    resettable: bool,
    disabled: bool,
    launch_at_login: bool,
    sync_enabled: bool,
    sync_history: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_switch_theme: false,
            cli_path: "/usr/local/bin/bash".into(),
            font_family: "Arial".into(),
            font_size: 14.0,
            line_height: 12.0,
            density: "Comfortable".into(),
            notifications_enabled: true,
            auto_update: true,
            resettable: true,
            disabled: false,
            launch_at_login: true,
            sync_enabled: false,
            sync_history: true,
        }
    }
}

impl Global for AppSettings {}

impl AppSettings {
    fn global(cx: &App) -> &AppSettings {
        cx.global::<AppSettings>()
    }

    pub fn global_mut(cx: &mut App) -> &mut AppSettings {
        cx.global_mut::<AppSettings>()
    }
}

pub struct SettingsStory {
    focus_handle: FocusHandle,
    group_variant: GroupBoxVariant,
    size: Size,
    active_tab_ix: usize,
    search: Entity<InputState>,
    density: Entity<SelectState<Vec<SharedString>>>,
}

struct OpenURLSettingField {
    label: SharedString,
    url: SharedString,
}

impl OpenURLSettingField {
    fn new(label: impl Into<SharedString>, url: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            url: url.into(),
        }
    }
}

impl SettingFieldElement for OpenURLSettingField {
    type Element = Button;
    fn render_field(&self, options: &RenderOptions, _: &mut Window, _: &mut App) -> Self::Element {
        let url = self.url.clone();
        Button::new("open-url")
            .outline()
            .label(self.label.clone())
            .with_size(options.size())
            .on_click(move |_, _window, cx| {
                cx.open_url(url.as_str());
            })
    }
}

impl super::Story for SettingsStory {
    fn title() -> &'static str {
        "Settings"
    }

    fn description() -> &'static str {
        "A collection of settings groups and items for the application."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn paddings() -> gpui_kit::Pixels {
        px(0.)
    }
}

impl SettingsStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.set_global::<AppSettings>(AppSettings::default());

        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search settings"));
        cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();
        let density = cx.new(|cx| {
            SelectState::new(
                vec!["Comfortable".into(), "Compact".into()],
                Some(Default::default()),
                window,
                cx,
            )
        });

        Self {
            focus_handle: cx.focus_handle(),
            group_variant: GroupBoxVariant::Fill,
            size: Size::default(),
            active_tab_ix: 0,
            search,
            density,
        }
    }

    fn setting_pages(&self, _: &mut Window, cx: &mut Context<Self>) -> Vec<SettingPage> {
        let view = cx.entity();
        let default_settings = AppSettings::default();
        let resettable = AppSettings::global(cx).resettable;
        let disabled = AppSettings::global(cx).disabled;

        vec![
            SettingPage::new("General")
                .resettable(resettable)
                .default_open(true)
                .icon(Icon::new(IconName::Settings2))
                .title_suffix(|_, _| {
                    Button::new("help")
                        .icon(IconName::Info)
                        .ghost()
                        .xsmall()
                        .on_click(|_, _, cx| cx.open_url("https://gpui-kit.com/"))
                })
                .groups(vec![
                    SettingGroup::new().title("Appearance").items(vec![
                        SettingItem::new(
                            "Dark Mode",
                            SettingField::switch(
                                |cx: &App| cx.theme().mode.is_dark(),
                                |val: bool, cx: &mut App| {
                                    let mode = if val {
                                        ThemeMode::Dark
                                    } else {
                                        ThemeMode::Light
                                    };
                                    Theme::change(mode, None, cx);
                                },
                            )
                            .default_value(false),
                        )
                        .description("Switch between light and dark themes.")
                        .disabled(disabled),
                        SettingItem::new(
                            "Auto Switch Theme",
                            SettingField::checkbox(
                                |cx: &App| AppSettings::global(cx).auto_switch_theme,
                                |val: bool, cx: &mut App| {
                                    AppSettings::global_mut(cx).auto_switch_theme = val;
                                },
                            )
                            .default_value(default_settings.auto_switch_theme),
                        )
                        .description("Automatically switch theme based on system settings.")
                        .info("The theme follows the system appearance when it changes.")
                        .disabled(disabled),
                        SettingItem::new(
                            "resettable",
                            SettingField::switch(
                                |cx: &App| AppSettings::global(cx).resettable,
                                |checked: bool, cx: &mut App| {
                                    AppSettings::global_mut(cx).resettable = checked
                                },
                            ),
                        )
                        .description("Enable/Disable reset button for settings.")
                        .disabled(disabled),
                        SettingItem::new(
                            "Group Variant",
                            SettingField::dropdown(
                                vec![
                                    (GroupBoxVariant::Normal.as_str().into(), "Normal".into()),
                                    (GroupBoxVariant::Outline.as_str().into(), "Outline".into()),
                                    (GroupBoxVariant::Fill.as_str().into(), "Fill".into()),
                                ],
                                {
                                    let view = view.clone();
                                    move |cx: &App| {
                                        SharedString::from(
                                            view.read(cx).group_variant.as_str().to_string(),
                                        )
                                    }
                                },
                                {
                                    let view = view.clone();
                                    move |val: SharedString, cx: &mut App| {
                                        view.update(cx, |view, cx| {
                                            view.group_variant =
                                                GroupBoxVariant::from_str(val.as_str());
                                            cx.notify();
                                        });
                                    }
                                },
                            )
                            .default_value(GroupBoxVariant::Fill.as_str().to_string()),
                        )
                        .description("Select the variant for setting groups.")
                        .info("Fill draws a card, Outline only its border, Normal no surface.")
                        .disabled(disabled),
                        SettingItem::new(
                            "Group Size",
                            SettingField::dropdown(
                                vec![
                                    (Size::Medium.as_str().into(), "Medium".into()),
                                    (Size::Small.as_str().into(), "Small".into()),
                                    (Size::XSmall.as_str().into(), "XSmall".into()),
                                ],
                                {
                                    let view = view.clone();
                                    move |cx: &App| {
                                        SharedString::from(view.read(cx).size.as_str().to_string())
                                    }
                                },
                                {
                                    let view = view.clone();
                                    move |val: SharedString, cx: &mut App| {
                                        view.update(cx, |view, cx| {
                                            view.size = Size::from_str(val.as_str());
                                            cx.notify();
                                        });
                                    }
                                },
                            )
                            .default_value(Size::default().as_str().to_string()),
                        )
                        .description("Select the size for the setting group.")
                        .disabled(disabled),
                    ]),
                    SettingGroup::new()
                        .title("Font")
                        .footer(|_, _| "Font preferences apply to this story only.")
                        .item(
                            SettingItem::new(
                                "Font Family",
                                SettingField::dropdown(
                                    vec![
                                        ("Arial".into(), "Arial".into()),
                                        ("Helvetica".into(), "Helvetica".into()),
                                        ("Times New Roman".into(), "Times New Roman".into()),
                                        ("Courier New".into(), "Courier New".into()),
                                    ],
                                    |cx: &App| AppSettings::global(cx).font_family.clone(),
                                    |val: SharedString, cx: &mut App| {
                                        AppSettings::global_mut(cx).font_family = val;
                                    },
                                )
                                .default_value(default_settings.font_family),
                            )
                            .description("Select the font family for the story.")
                            .disabled(disabled),
                        )
                        .item(
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
                                .default_value(default_settings.font_size),
                            )
                            .description(
                                "Adjust the font size for better readability between 8 and 72.",
                            )
                            .disabled(disabled),
                        )
                        .item(
                            SettingItem::new(
                                "Line Height",
                                SettingField::number_input(
                                    NumberFieldOptions {
                                        min: 8.0,
                                        max: 32.0,
                                        ..Default::default()
                                    },
                                    |cx: &App| AppSettings::global(cx).line_height,
                                    |val: f64, cx: &mut App| {
                                        AppSettings::global_mut(cx).line_height = val;
                                    },
                                )
                                .default_value(default_settings.line_height),
                            )
                            .description(
                                "Adjust the line height for better readability between 8 and 32.",
                            )
                            .disabled(disabled),
                        ),
                    SettingGroup::new().title("Other").items(vec![
                        SettingItem::new(
                            "Disable Settings",
                            SettingField::switch(
                                |cx: &App| AppSettings::global(cx).disabled,
                                |checked: bool, cx: &mut App| {
                                    AppSettings::global_mut(cx).disabled = checked
                                },
                            )
                            .default_value(false),
                        )
                        .description("Lock the other settings."),
                        SettingItem::new(
                            "Foo",
                            SettingField::switch(
                                |cx: &App| AppSettings::global(cx).disabled,
                                |checked: bool, cx: &mut App| {
                                    AppSettings::global_mut(cx).disabled = checked
                                },
                            )
                            .default_value(false),
                        )
                        .description("Find me by searching for my sibling")
                        .keywords(["Bar"]),
                        SettingItem::render(|options, _, _| {
                            h_flex()
                                .w_full()
                                .justify_between()
                                .flex_wrap()
                                .gap_3()
                                .child("View source, report issues, and follow project updates.")
                                .when(options.is_disabled(), |this| this.opacity(0.5))
                                .child(
                                    Button::new("action")
                                        .icon(IconName::Globe)
                                        .label("Repository...")
                                        .outline()
                                        .with_size(options.size())
                                        .disabled(options.is_disabled())
                                        .on_click(|_, _, cx| {
                                            cx.open_url("https://github.com/longbridge/gpui-kit");
                                        }),
                                )
                                .into_any_element()
                        })
                        .disabled(disabled),
                        SettingItem::new(
                            "Density",
                            SettingField::render(|options, _window, cx| {
                                let current = AppSettings::global(cx).density.clone();
                                h_flex()
                                    .gap_1()
                                    .children(["Comfortable", "Compact"].map(|value| {
                                        Button::new(value)
                                            .label(value)
                                            .with_size(options.size())
                                            .map(|this| {
                                                if current == value {
                                                    this.primary()
                                                } else {
                                                    this.outline()
                                                }
                                            })
                                            .on_click(move |_, _, cx| {
                                                AppSettings::global_mut(cx).density = value.into();
                                            })
                                    }))
                            })
                            // A custom element field manages its own state, so reset
                            // support must be wired up explicitly via `on_reset`.
                            .on_reset(
                                {
                                    let default_density = default_settings.density.clone();
                                    move |cx: &App| {
                                        AppSettings::global(cx).density != default_density
                                    }
                                },
                                {
                                    let default_density = default_settings.density.clone();
                                    move |_window, cx: &mut App| {
                                        AppSettings::global_mut(cx).density =
                                            default_density.clone();
                                    }
                                },
                            ),
                        )
                        .description("A custom element field with reset support via `on_reset`.")
                        .disabled(disabled),
                        SettingItem::new(
                            "CLI Path",
                            SettingField::input(
                                |cx: &App| AppSettings::global(cx).cli_path.clone(),
                                |val: SharedString, cx: &mut App| {
                                    println!("cli-path set value: {}", val);
                                    AppSettings::global_mut(cx).cli_path = val;
                                },
                            )
                            .default_value(default_settings.cli_path),
                        )
                        .layout(Axis::Vertical)
                        .description(
                            "Path to the CLI executable. \n\
                        This item uses Vertical layout. The title,\
                        description, and field are all aligned vertically with width 100%.",
                        )
                        .disabled(disabled),
                    ]),
                ]),
            SettingPage::new("Software Update")
                .resettable(resettable)
                .icon(Icon::new(IconName::Cpu))
                .groups(vec![SettingGroup::new().title("Updates").items(vec![
                    SettingItem::new(
                        "Enable Notifications",
                        SettingField::switch(
                            |cx: &App| AppSettings::global(cx).notifications_enabled,
                            |val: bool, cx: &mut App| {
                                AppSettings::global_mut(cx).notifications_enabled = val;
                            },
                        )
                        .default_value(default_settings.notifications_enabled),
                    )
                    .description("Receive notifications about updates and news.")
                    .disabled(disabled),
                    SettingItem::new(
                        "Auto Update",
                        SettingField::switch(
                            |cx: &App| AppSettings::global(cx).auto_update,
                            |val: bool, cx: &mut App| {
                                AppSettings::global_mut(cx).auto_update = val;
                            },
                        )
                        .default_value(default_settings.auto_update),
                    )
                    .description("Automatically download and install updates.")
                    .disabled(disabled),
                ])]),
            SettingPage::new("About")
                .resettable(resettable)
                .icon(Icon::new(IconName::Info))
                .group(
                    // Nothing in it changes, so it never needs to redraw.
                    SettingGroup::new()
                        .key(0)
                        .item(SettingItem::render(|_options, _, cx| {
                            v_flex()
                                .gap_3()
                                .w_full()
                                .items_center()
                                .justify_center()
                                .child(Icon::new(IconName::GalleryVerticalEnd).size_16())
                                .child("GPUI Component")
                                .child(
                                    Label::new(
                                        "Rust GUI components for building fantastic cross-platform \
                                    desktop application by using GPUI.",
                                    )
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                                )
                                .into_any()
                        })),
                )
                .group(SettingGroup::new().title("Links").items(vec![
                        SettingItem::new(
                            "GitHub Repository",
                            SettingField::element(OpenURLSettingField::new(
                                "Repository...",
                                "https://github.com/longbridge/gpui-kit",
                            )),
                        )
                        .description("Open the GitHub repository in your default browser."),
                        SettingItem::new(
                            "Documentation",
                            SettingField::element(OpenURLSettingField::new(
                                "Rust Docs...",
                                "https://docs.rs/gpui-component"
                            )),
                        )
                        .description(markdown(
                            "Rust doc for the `gpui-component` crate.",
                        )),
                        SettingItem::new(
                            "Website",
                            SettingField::render(|options, _window, _cx| {
                                Button::new("open-url")
                                    .outline()
                                    .label("Website...")
                                    .with_size(options.size())
                                    .on_click(|_, _window, cx| {
                                        cx.open_url("https://gpui-kit.com/");
                                    })
                            }),
                        )
                        .description("Official website and documentation for the GPUI Component."),
                    ])),
        ]
    }
}

impl Focusable for SettingsStory {
    fn focus_handle(&self, _: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_flex().px_4().py_2().child(
                    TabBar::new("settings-story-tabs")
                        .segmented()
                        .selected_index(self.active_tab_ix)
                        .on_click(cx.listener(|this, ix: &usize, _, cx| {
                            this.active_tab_ix = *ix;
                            cx.notify();
                        }))
                        .child("Settings")
                        .child("Standalone"),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .map(|this| {
                        if self.active_tab_ix == 0 {
                            this.child(
                                Settings::new("app-settings")
                                    .with_size(self.size)
                                    .with_group_variant(self.group_variant)
                                    .pages(self.setting_pages(window, cx)),
                            )
                        } else {
                            this.child(self.render_standalone(cx))
                        }
                    }),
            )
    }
}

impl SettingsStory {
    /// The building blocks in an application-owned layout: the application
    /// owns the search field, the page and every value.
    fn render_standalone(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = AppSettings::global(cx);
        let query = self.search.read(cx).value();
        let sync_enabled = settings.sync_enabled;

        let general = SettingSection::new("general")
            .title("General")
            .description("How the application starts and behaves.")
            .fill()
            .row(
                SettingRow::new("launch-at-login", "Open at login")
                    .description("Start in the background when you sign in.")
                    .switch(
                        Switch::new("launch-at-login-switch")
                            .checked(settings.launch_at_login)
                            .on_click(|checked: &bool, _, cx| {
                                AppSettings::global_mut(cx).launch_at_login = *checked;
                            }),
                    ),
            )
            .row(
                SettingRow::new("density", "Density")
                    .info("Compact fits more rows on screen; Comfortable is easier to scan.")
                    .keywords(["comfortable", "compact", "spacing"])
                    .control(Select::new(&self.density).w(rems(10.))),
            )
            .row(
                SettingRow::new("version", "Version")
                    .value("0.9.0")
                    .keywords(["about", "release"]),
            );

        let sync = SettingSection::new("sync")
            .title("Sync")
            .fill()
            .row(
                SettingRow::new("sync-enabled", "Sync settings")
                    .description("Keep preferences the same on every device you sign in to.")
                    .switch(
                        Switch::new("sync-enabled-switch")
                            .checked(sync_enabled)
                            .on_click(|checked: &bool, _, cx| {
                                AppSettings::global_mut(cx).sync_enabled = *checked;
                            }),
                    ),
            )
            .row(
                SettingRow::new("sync-history", "Sync history")
                    .description("Available when sync is on.")
                    .disabled(!sync_enabled)
                    .switch(
                        Switch::new("sync-history-switch")
                            .checked(settings.sync_history)
                            .on_click(|checked: &bool, _, cx| {
                                AppSettings::global_mut(cx).sync_history = *checked;
                            }),
                    ),
            )
            .row(
                SettingRow::new("sync-now", "Last synced")
                    .description("Today at 09:41")
                    .control(
                        Button::new("sync-now-button")
                            .outline()
                            .label("Sync Now")
                            .disabled(!sync_enabled)
                            .when(sync_enabled, |this| this.on_click(|_, _, _| {})),
                    ),
            )
            .footer("Synced data is encrypted on this device before upload.");

        let data = SettingSection::new("data")
            .title("Data")
            .outline()
            .row(
                SettingRow::new("cache", "Offline data")
                    .description("Remove cached copies from this device, not from your account.")
                    .info("Your account and its synced data are not deleted.")
                    .control(Button::new("clear-cache").danger().label("Remove…")),
            )
            .child(
                Label::new("Blocks added as children span the section, with the same padding.")
                    .text_sm()
                    .text_color(cx.theme().muted_foreground),
            );

        gpui_kit::InteractiveElement::id(v_flex(), "standalone-settings")
            .size_full()
            .overflow_y_scrollbar()
            .child(
                v_flex()
                    .w_full()
                    .max_w(rems(42.))
                    .p_6()
                    .gap_6()
                    .child(Input::new(&self.search).prefix(IconName::Search))
                    .child(
                        SettingList::new("standalone-list")
                            .query(query)
                            .sections([general, sync, data]),
                    ),
            )
    }
}
