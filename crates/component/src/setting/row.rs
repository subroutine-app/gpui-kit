use gpui::{
    AbsoluteLength, AnyElement, App, Axis, Div, ElementId, InteractiveElement as _, IntoElement,
    Length, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, rems,
};

use crate::{
    ActiveTheme as _, AxisExt as _, Disableable, Icon, IconName, Sizable, Size,
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    setting::matches_query,
    switch::Switch,
    text::Text,
    v_flex,
};

/// The minimum width of the label column before the control wraps below it.
const DEFAULT_LABEL_WIDTH: f32 = 7.5;

enum RowControl {
    Element(AnyElement),
    Switch(Box<Switch>),
}

/// One setting: a label with its description stacked below, an optional info
/// icon, and a trailing control.
///
/// The row owns no state; the caller owns the value and the control. Use it
/// inside a [`SettingSection`](super::SettingSection), or on its own in an
/// application-owned layout. When the row is too narrow for both, the
/// control wraps below the label.
///
/// ```ignore
/// SettingRow::new("notifications", "Notifications")
///     .description("Show a banner when a build finishes.")
///     .info("Banners follow the system's Focus settings.")
///     .switch(Switch::new("notifications-switch").checked(enabled).on_click(...))
/// ```
#[derive(IntoElement)]
pub struct SettingRow {
    id: ElementId,
    label: SharedString,
    description: Option<Text>,
    info: Option<SharedString>,
    value: Option<SharedString>,
    control: Option<RowControl>,
    keywords: Vec<SharedString>,
    layout: Axis,
    label_width: Option<AbsoluteLength>,
    size: Size,
    disabled: bool,
}

impl SettingRow {
    /// Create a row with a stable `id` and its visible `label`.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            info: None,
            value: None,
            control: None,
            keywords: Vec::new(),
            layout: Axis::Horizontal,
            label_width: None,
            size: Size::Small,
            disabled: false,
        }
    }

    /// Supporting text, shown in muted type below the label.
    ///
    /// Put what someone needs in order to choose here; context most people do
    /// not need every time belongs in [`Self::info`].
    pub fn description(mut self, description: impl Into<Text>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Supplementary help behind an info icon beside the label.
    ///
    /// The icon is a keyboard stop with a tooltip, and the text is its
    /// accessible label. It stays operable when the row is disabled.
    pub fn info(mut self, info: impl Into<SharedString>) -> Self {
        self.info = Some(info.into());
        self
    }

    /// What the setting currently holds, shown as muted trailing text when
    /// the row has no control — a status, or a value someone else decides.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// The trailing control, such as a `Select`, `Button` or `Input`.
    ///
    /// The row does not reach into an arbitrary control: when the row is
    /// [disabled](Disableable::disabled), pass a control that is disabled too,
    /// so it installs no action handler.
    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.control = Some(RowControl::Element(control.into_any_element()));
        self
    }

    /// The trailing switch, named after the row's label and drawn at the
    /// row's [size](Sizable), small by default so a list of settings stays
    /// compact.
    ///
    /// A disabled row disables the switch, so it installs no click handler.
    pub fn switch(mut self, switch: Switch) -> Self {
        self.control = Some(RowControl::Switch(Box::new(switch)));
        self
    }

    /// Extra words used only for search matching in a
    /// [`SettingList`](super::SettingList), such as aliases or option names
    /// inside the control. The label, description, info and value are
    /// searched already.
    pub fn keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// `Axis::Horizontal` (default) places the control after the label,
    /// wrapping it below when space runs out. `Axis::Vertical` always places
    /// a full-width control below the label, for composite editors.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// The minimum width of the label column, default is `7.5rem`.
    ///
    /// The column grows into the space the control leaves; the control moves
    /// below the label once the row is narrower than this plus the control.
    /// A [`SettingSection`](super::SettingSection) can set it for every row.
    pub fn label_width(mut self, width: impl Into<AbsoluteLength>) -> Self {
        self.label_width = Some(width.into());
        self
    }

    pub(super) fn inherit_label_width(mut self, width: Option<AbsoluteLength>) -> Self {
        self.label_width = self.label_width.or(width);
        self
    }

    pub(super) fn is_match(&self, query: &str, cx: &App) -> bool {
        let description = self.description.as_ref().map(|text| text.get_text(cx));
        [
            Some(&self.label),
            description.as_ref(),
            self.info.as_ref(),
            self.value.as_ref(),
        ]
        .into_iter()
        .flatten()
        .chain(&self.keywords)
        .any(|text| matches_query(query, text))
    }

    fn render_names(&self, label_width: Length, stacked: bool, cx: &App) -> Div {
        let selector = format!("setting-title-{}", self.id);
        v_flex()
            .when(cfg!(test), |this| this.debug_selector(|| selector))
            .flex_grow_1()
            .flex_shrink_0()
            .flex_basis(label_width)
            .min_w(label_width)
            .max_w_full()
            .when(stacked, |this| this.flex_none().w_full())
            .gap_0p5()
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_1()
                    .child(Label::new(self.label.clone()).text_sm())
                    .when_some(self.info.clone(), |this, info| {
                        this.child(
                            Button::new("info")
                                .icon(
                                    Icon::new(IconName::Info)
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .ghost()
                                .xsmall()
                                .accessibility_label(info.clone())
                                .tooltip(info),
                        )
                    }),
            )
            .when_some(self.description.clone(), |this, description| {
                this.child(
                    div()
                        .w_full()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(description),
                )
            })
    }
}

impl Sizable for SettingRow {
    /// The size of a switch given through [`SettingRow::switch`]. A control
    /// given through [`SettingRow::control`] keeps its own size.
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Disableable for SettingRow {
    /// Dim the row. A switch given through [`SettingRow::switch`] is disabled
    /// with it; a control given through [`SettingRow::control`] is the
    /// caller's to disable.
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for SettingRow {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let stacked = self.layout.is_vertical();
        let label_width = Length::Definite(
            self.label_width
                .unwrap_or_else(|| rems(DEFAULT_LABEL_WIDTH).into())
                .into(),
        );
        let names = self.render_names(label_width, stacked, cx);
        let trailing = match self.control.take() {
            Some(RowControl::Element(control)) => Some(control),
            Some(RowControl::Switch(switch)) => Some(
                switch
                    .with_size(self.size)
                    .accessibility_label(self.label.clone())
                    .when(self.disabled, |this| this.disabled(true))
                    .into_any_element(),
            ),
            None => self.value.clone().map(|value| {
                Label::new(value)
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .into_any_element()
            }),
        };

        let selector = |part: &str| format!("setting-{part}-{}", self.id);
        let (row_selector, field_selector) = (selector("row"), selector("field"));
        h_flex()
            .id(self.id)
            .when(cfg!(test), |this| this.debug_selector(|| row_selector))
            .w_full()
            .min_w_0()
            .flex_wrap()
            .when(self.description.is_some() || stacked, |this| {
                this.items_start()
            })
            .gap_x_4()
            .gap_y_3()
            .py_3()
            .when(self.disabled, |this| this.opacity(0.5))
            .child(names)
            .when_some(trailing, |this, trailing| {
                this.child(
                    h_flex()
                        .id("field")
                        .when(cfg!(test), |this| this.debug_selector(|| field_selector))
                        .flex_none()
                        .max_w_full()
                        .map(|this| {
                            if stacked {
                                this.w_full().justify_start()
                            } else {
                                this.justify_end()
                            }
                        })
                        .child(trailing),
                )
            })
    }
}
