use gpui::{
    AnyElement, AnyView, App, AvailableSpace, Context, Entity, IntoElement, ParentElement as _,
    Pixels, Render, SharedString, StyleRefinement, Styled, Window, div, size,
};

use super::{RenderOptions, SettingGroup};

/// The reactive boundary of one visible keyed group.
///
/// GPUI's cached view needs an explicit layout size, while the page's list
/// needs the group's intrinsic height, so the view measures itself and keeps
/// that height until something it depends on changes.
pub(super) struct GroupView {
    group: SettingGroup,
    inputs: GroupInputs,
    height: Option<Pixels>,
}

/// Everything outside the group's own revision key that changes what it draws.
#[derive(Clone, PartialEq)]
pub(super) struct GroupInputs {
    pub(super) query: SharedString,
    pub(super) options: RenderOptions,
    pub(super) width: Pixels,
    pub(super) rem: Pixels,
}

impl GroupView {
    pub(super) fn new(group: SettingGroup, inputs: GroupInputs) -> Self {
        Self {
            group,
            inputs,
            height: None,
        }
    }

    pub(super) fn update(
        &mut self,
        group: SettingGroup,
        inputs: GroupInputs,
        cx: &mut Context<Self>,
    ) {
        if self.group.key != group.key || self.inputs != inputs {
            self.height = None;
            cx.notify();
        }
        self.group = group;
        self.inputs = inputs;
    }

    pub(super) fn element(view: Entity<Self>, window: &mut Window, cx: &mut App) -> AnyElement {
        let height = match view.read(cx).height {
            Some(height) => height,
            None => {
                let width = view.read(cx).inputs.width;
                let measured = view.clone().into_any_element().layout_as_root(
                    size(AvailableSpace::Definite(width), AvailableSpace::MaxContent),
                    window,
                    cx,
                );
                view.update(cx, |view, _| view.height = Some(measured.height));
                measured.height
            }
        };
        AnyView::from(view)
            .cached(StyleRefinement::default().w_full().h(height))
            .into_any_element()
    }
}

impl Render for GroupView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let GroupInputs { query, options, .. } = &self.inputs;
        // Keep horizontal padding inside the list's clip bounds so focus rings
        // can extend beyond the group's edges.
        div()
            .w_full()
            .px_4()
            .child(self.group.clone().py_4().render(query, options, window, cx))
            .on_children_prepainted(move |bounds, window, cx| {
                let Some(bounds) = bounds.first() else {
                    return;
                };
                let height = bounds.size.height;
                // An entity-backed field can grow without the app changing the
                // key; keep the list's next measurement in step with it.
                _ = view.update(cx, |view, cx| {
                    if view.height != Some(height) {
                        view.height = Some(height);
                        cx.defer_in(window, |_, _, cx| cx.notify());
                    }
                });
            })
    }
}
