use gpui::{
    AbsoluteLength, AnyElement, App, Div, ElementId, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{
    ActiveTheme as _, StyledExt as _,
    group_box::{GroupBoxVariant, GroupBoxVariants},
    label::Label,
    setting::{SettingRow, matches_query},
    v_flex,
};

pub(super) enum SectionContent {
    Row(Box<SettingRow>),
    Block(AnyElement),
}

/// A titled group of [`SettingRow`]s on one surface, separated by hairlines.
///
/// The title and description sit above the surface and a footer below it.
/// The surface follows its [`GroupBoxVariant`]: `Normal` (default) draws
/// none, `Outline` a bordered card and `Fill` a filled, bordered card. Use it
/// on its own in an application-owned layout, inside a [`SettingList`] for
/// search, or let [`Settings`] build one for each [`SettingGroup`].
///
/// Children added with [`ParentElement`] are full-width blocks among the
/// rows, in call order, with the same padding and separators.
///
/// [`SettingList`]: super::SettingList
/// [`Settings`]: super::Settings
/// [`SettingGroup`]: super::SettingGroup
#[derive(IntoElement)]
pub struct SettingSection {
    id: ElementId,
    style: StyleRefinement,
    variant: GroupBoxVariant,
    title: Option<SharedString>,
    description: Option<SharedString>,
    footer: Option<AnyElement>,
    label_width: Option<AbsoluteLength>,
    content: Vec<SectionContent>,
}

impl SettingSection {
    /// Create an empty section with a stable `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            variant: GroupBoxVariant::default(),
            title: None,
            description: None,
            footer: None,
            label_width: None,
            content: Vec::new(),
        }
    }

    /// Set the title shown above the surface, default is None.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description shown below the title, default is None.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set supporting content below, and outside, the surface, rendered as
    /// small muted text. It is shown whenever the section is.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Set the minimum label column width for rows that do not set their own.
    /// See [`SettingRow::label_width`].
    pub fn label_width(mut self, width: impl Into<AbsoluteLength>) -> Self {
        self.label_width = Some(width.into());
        self
    }

    /// Add a row to the section.
    pub fn row(mut self, row: SettingRow) -> Self {
        self.content.push(SectionContent::Row(Box::new(row)));
        self
    }

    /// Add multiple rows to the section.
    pub fn rows(mut self, rows: impl IntoIterator<Item = SettingRow>) -> Self {
        self.content.extend(
            rows.into_iter()
                .map(|row| SectionContent::Row(Box::new(row))),
        );
        self
    }

    pub(super) fn content(mut self, content: impl IntoIterator<Item = SectionContent>) -> Self {
        self.content.extend(content);
        self
    }

    /// Narrow the section to `query`.
    ///
    /// A section whose title or description matches keeps all its content.
    /// Otherwise only matching rows remain; blocks are opaque to search and
    /// are dropped. `None` when nothing remains.
    pub(super) fn filtered(mut self, query: &str, cx: &App) -> Option<Self> {
        let section_matches = [self.title.as_ref(), self.description.as_ref()]
            .into_iter()
            .flatten()
            .any(|text| matches_query(query, text));
        if section_matches {
            return Some(self);
        }

        self.content.retain(|content| match content {
            SectionContent::Row(row) => row.is_match(query, cx),
            SectionContent::Block(_) => false,
        });
        (!self.content.is_empty()).then_some(self)
    }

    fn render_heading(&self, cx: &App) -> Option<Div> {
        if self.title.is_none() && self.description.is_none() {
            return None;
        }

        Some(
            v_flex()
                .gap_0p5()
                .when_some(self.title.clone(), |this, title| {
                    this.child(
                        Label::new(title)
                            .text_sm()
                            .text_color(cx.theme().foreground),
                    )
                })
                .when_some(self.description.clone(), |this, description| {
                    this.child(
                        Label::new(description)
                            .text_xs()
                            .text_color(cx.theme().muted_foreground),
                    )
                }),
        )
    }
}

impl ParentElement for SettingSection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.content
            .extend(elements.into_iter().map(SectionContent::Block));
    }
}

impl Styled for SettingSection {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl GroupBoxVariants for SettingSection {
    fn with_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl RenderOnce for SettingSection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let heading = self.render_heading(cx);
        let has_surface = self.variant != GroupBoxVariant::Normal;
        let label_width = self.label_width;
        let separator = || div().w_full().h_px().bg(cx.theme().border);

        let mut content = Vec::with_capacity(self.content.len() * 2);
        for item in self.content {
            if !content.is_empty() {
                content.push(separator().into_any_element());
            }
            content.push(match item {
                SectionContent::Row(row) => row.inherit_label_width(label_width).into_any_element(),
                SectionContent::Block(block) => div()
                    .w_full()
                    .min_w_0()
                    .py_3()
                    .child(block)
                    .into_any_element(),
            });
        }

        v_flex()
            .id(self.id)
            .w_full()
            .gap_3()
            .refine_style(&self.style)
            .children(heading)
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        v_flex()
                            .w_full()
                            .text_color(cx.theme().group_box_foreground)
                            .when(has_surface, |this| {
                                this.px_4()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded(cx.theme().radius_lg)
                            })
                            .when(self.variant == GroupBoxVariant::Fill, |this| {
                                this.bg(cx.theme().tokens.group_box)
                            })
                            .children(content),
                    )
                    .when_some(self.footer, |this, footer| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(footer),
                        )
                    }),
            )
    }
}
