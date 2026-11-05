use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, SharedString, Styled as _, Window, div,
};

use crate::{
    setting::{SettingSection, settings::render_no_results},
    v_flex,
};

/// [`SettingSection`]s narrowed by one caller-owned search query.
///
/// Matching is case-insensitive over each section's title and description
/// and each row's label, description, info, value and keywords. A section
/// whose title matches keeps all its rows; otherwise only matching rows
/// remain. Order is preserved, so a page stays spatially familiar while it
/// narrows. The caller owns the query, usually from its own search `Input`,
/// and the page chrome around the list.
#[derive(IntoElement)]
pub struct SettingList {
    id: ElementId,
    query: SharedString,
    sections: Vec<SettingSection>,
    empty: Option<AnyElement>,
}

impl SettingList {
    /// Create an empty list with a stable `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            query: SharedString::default(),
            sections: Vec::new(),
            empty: None,
        }
    }

    /// Set the search query, default is empty, which shows everything.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Add a section to the list.
    pub fn section(mut self, section: SettingSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Add multiple sections to the list.
    pub fn sections(mut self, sections: impl IntoIterator<Item = SettingSection>) -> Self {
        self.sections.extend(sections);
        self
    }

    /// Set what to show when no section remains.
    ///
    /// Defaults to the settings no-results state while searching, and to
    /// nothing otherwise.
    pub fn empty(mut self, empty: impl IntoElement) -> Self {
        self.empty = Some(empty.into_any_element());
        self
    }
}

impl RenderOnce for SettingList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let query = self.query.trim();
        let sections: Vec<_> = if query.is_empty() {
            self.sections
        } else {
            self.sections
                .into_iter()
                .filter_map(|section| section.filtered(query, cx))
                .collect()
        };

        if sections.is_empty() {
            let empty = self
                .empty
                .or_else(|| (!query.is_empty()).then(|| render_no_results().into_any_element()));
            return div()
                .id(self.id)
                .w_full()
                .children(empty)
                .into_any_element();
        }

        v_flex()
            .id(self.id)
            .w_full()
            .gap_6()
            .children(sections)
            .into_any_element()
    }
}
