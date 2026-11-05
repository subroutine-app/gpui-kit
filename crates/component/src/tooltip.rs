use crate::root::WindowState;
use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{
    Action, AnyElement, AnyView, App, AppContext, AvailableSpace, Bounds, ContentMask, Context,
    Element, ElementId, Entity, GlobalElementId, InspectorElementId, InteractiveElement as _,
    IntoElement, LayoutId, MouseButton, ParentElement, Pixels, Refineable as _, Render,
    SharedString, StatefulInteractiveElement, Style, StyleRefinement, Styled, Window, div, point,
    prelude::FluentBuilder, px, transparent_black,
};
pub use gpui_base::TooltipDefaults;
use gpui_base::{
    Interpolate as _, Sequence, Tooltip as BaseTooltip, TooltipOverlay as BaseTooltipOverlay,
    TooltipRequest as BaseTooltipRequest, TooltipTransition as BaseTooltipTransition,
    TooltipTrigger, Transition,
};

use crate::{
    ActiveTheme, Placement, StyledExt,
    animation::{EffectTransition, ease_out_cubic},
    kbd::Kbd,
    text::Text,
};

pub(crate) fn init(_cx: &mut App) {
    // No app-level init needed — TooltipOverlay is per-window via Root.
}

// ── Tooltip view (unchanged API) ────────────────────────────────────────────

enum TooltipContext {
    Text(Text),
    Element(Box<dyn Fn(&mut Window, &mut App) -> AnyElement>),
}

/// A Tooltip element that can display text or custom content,
/// with optional key binding information.
pub struct Tooltip {
    style: StyleRefinement,
    content: TooltipContext,
    key_binding: Option<Kbd>,
    action: Option<(Box<dyn Action>, Option<SharedString>)>,
    /// Shown by the tooltip overlay, which draws the box itself.
    in_overlay: bool,
}

impl Tooltip {
    /// Create a Tooltip with a text content.
    pub fn new(text: impl Into<Text>) -> Self {
        Self {
            style: StyleRefinement::default(),
            content: TooltipContext::Text(text.into()),
            key_binding: None,
            action: None,
            in_overlay: false,
        }
    }

    /// Create a Tooltip with a custom element.
    pub fn element<E, F>(builder: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        Self {
            style: StyleRefinement::default(),
            key_binding: None,
            action: None,
            in_overlay: false,
            content: TooltipContext::Element(Box::new(move |window, cx| {
                builder(window, cx).into_any_element()
            })),
        }
    }

    /// Set Action to display key binding information for the tooltip if it exists.
    pub fn action(mut self, action: &dyn Action, context: Option<&str>) -> Self {
        self.action = Some((action.boxed_clone(), context.map(SharedString::new)));
        self
    }

    /// Set KeyBinding information for the tooltip.
    pub fn key_binding(mut self, key_binding: Option<Kbd>) -> Self {
        self.key_binding = key_binding;
        self
    }

    /// Build the tooltip and return it as an `AnyView`.
    pub fn build(self, _: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| self).into()
    }

    /// The style of the tooltip's box, for the overlay to draw it.
    fn box_style(&self, cx: &App) -> Style {
        let mut element = tooltip_box(div(), cx).refine_style(&self.style);
        let mut style = Style::default();
        style.refine(element.style());
        style
    }
}

/// The tooltip's box: its padding, background, border, shadow and corners.
fn tooltip_box<E: Styled>(element: E, cx: &App) -> E {
    element
        .py_0p5()
        .px_2()
        .bg(cx.theme().tokens.popover)
        .border_1()
        .border_color(cx.theme().border)
        .shadow_md()
        .rounded(cx.theme().radius)
}

impl FluentBuilder for Tooltip {}
impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key_binding = if let Some(key_binding) = &self.key_binding {
            Some(key_binding.clone())
        } else {
            if let Some((action, context)) = &self.action {
                Kbd::binding_for_action(
                    action.as_ref(),
                    context.as_ref().map(|s| s.as_ref()),
                    window,
                )
            } else {
                None
            }
        };

        let popup = tooltip_box(BaseTooltip::new("tooltip-popup"), cx)
            .h_flex()
            .font_family(cx.theme().font_family.clone())
            .when(!self.in_overlay, |this| this.m_3())
            .text_color(cx.theme().popover_foreground)
            .justify_between()
            .text_sm()
            .gap_3()
            .refine_style(&self.style)
            .when(self.in_overlay, |this| {
                this.bg(transparent_black())
                    .border_color(transparent_black())
                    .shadow(Vec::new())
            })
            .map(|this| {
                this.child(div().map(|this| match self.content {
                    TooltipContext::Text(ref text) => this.child(text.clone()),
                    TooltipContext::Element(ref builder) => this.child(builder(window, cx)),
                }))
            })
            .when_some(key_binding, |this, kbd| {
                this.child(
                    div()
                        .text_xs()
                        .flex_shrink_0()
                        .text_color(cx.theme().muted_foreground)
                        .child(kbd),
                )
            });
        if self.in_overlay {
            popup.into_any_element()
        } else {
            // Wrap in a child, to ensure the left margin is applied to the tooltip
            div().child(popup).into_any_element()
        }
    }
}

// ── Managed tooltip system ──────────────────────────────────────────────────

/// Duration of the slide-down enter animation.
const ENTER_DURATION: Duration = Duration::from_millis(150);
/// How long a switch along a row moves the box. Shorter than the theme's fast
/// duration, because the new text can be read only once the box has
/// uncovered it.
const SWITCH_DURATION: Duration = Duration::from_millis(80);

pub(crate) fn render_tooltip(
    content_view: AnyView,
    transition: BaseTooltipTransition,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    // The selector lets tests see whether a tooltip was painted.
    div()
        .debug_selector(|| "tooltip".into())
        .map(|this| match content_view.downcast::<Tooltip>() {
            Ok(tooltip) => this
                .p_3()
                .child(TooltipBox::new(tooltip, transition, window, cx)),
            Err(view) => this.child(view),
        })
        .map(|element| match transition {
            BaseTooltipTransition::Switch { .. } => element.into_any_element(),
            BaseTooltipTransition::Enter { epoch } => EffectTransition::new(ENTER_DURATION)
                .ease(ease_out_cubic)
                .slide_y(px(4.), px(0.))
                .fade(0.0, 1.0)
                .apply(
                    element,
                    ElementId::NamedInteger("tooltip-enter".into(), epoch as u64),
                )
                .into_any_element(),
        })
}

/// What a switch hands over from the tooltip drawn before it.
#[derive(Default)]
struct Handover {
    epoch: usize,
    /// The view drawn last frame and its box as laid out.
    drawn: Option<(AnyView, Bounds<Pixels>)>,
    /// The box painted last frame, which a switch moves on from.
    painted: Option<Bounds<Pixels>>,
    /// The tooltip being replaced and the box it was laid out in.
    outgoing: Option<(AnyView, Bounds<Pixels>)>,
    /// The box the switch moves from.
    from: Option<Bounds<Pixels>>,
}

/// A tooltip of the kit's own, whose box the overlay draws.
///
/// On a switch along a row, the box moves and resizes from the last tooltip's
/// to this one's, on whole device pixels. The texts never move. The old one
/// stays for the switch's first frame, until the new one is laid out, and then
/// the new one shows whole at its place, clipped by the box, which uncovers it
/// as it moves.
struct TooltipBox {
    tooltip: Entity<Tooltip>,
    content: AnyElement,
    outgoing: Option<(AnyElement, Bounds<Pixels>)>,
    from: Option<Bounds<Pixels>>,
    progress: f32,
    handover: Entity<Handover>,
    painted: Bounds<Pixels>,
}

impl TooltipBox {
    fn new(
        tooltip: Entity<Tooltip>,
        transition: BaseTooltipTransition,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        tooltip.update(cx, |tooltip, _| tooltip.in_overlay = true);
        let (epoch, same_row) = match transition {
            BaseTooltipTransition::Enter { epoch } => (epoch, false),
            BaseTooltipTransition::Switch {
                epoch,
                previous,
                current,
            } => (
                epoch,
                (current.origin.y - previous.origin.y).abs() < px(10.),
            ),
        };
        let handover = window.use_keyed_state("tooltip-handover", cx, |_, _| Handover::default());
        handover.update(cx, |handover, _| {
            if handover.epoch != epoch {
                handover.epoch = epoch;
                handover.outgoing = handover.drawn.take().filter(|_| same_row);
                handover.from = handover.painted.filter(|_| same_row);
            }
        });
        let (outgoing, from, to) = {
            let handover = handover.read(cx);
            let to = handover.drawn.as_ref().map(|(_, bounds)| *bounds);
            (handover.outgoing.clone(), handover.from, to)
        };
        let progress = match from {
            Some(_) => Sequence::new(
                ElementId::NamedInteger("tooltip-switch".into(), epoch as u64),
                0.0,
            )
            // The box eases with `ease_out_cubic` as it is painted.
            .with_step(1.0, Transition::new(SWITCH_DURATION).ease(|t| t))
            .sample(window, cx)
            .into_value(),
            None => 1.0,
        };
        // The old text stays for the switch's first frame, and the new one
        // shows from the second on, once it has been laid out.
        let laid_out = progress >= 1.0 || to.is_some();
        Self {
            content: div()
                .opacity(if laid_out { 1.0 } else { 0.0 })
                .child(tooltip.clone())
                .into_any_element(),
            outgoing: outgoing
                .filter(|_| !laid_out)
                .map(|(view, bounds)| (div().child(view).into_any_element(), bounds)),
            from: from.filter(|_| progress < 1.0),
            tooltip,
            progress,
            handover,
            painted: Bounds::default(),
        }
    }
}

impl IntoElement for TooltipBox {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TooltipBox {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.content.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.content.prepaint(window, cx);
        let shown = self.from.map_or(bounds, |from| {
            from.interpolate(&bounds, ease_out_cubic(self.progress))
        });
        // Whole device pixels keep the box's edges crisp while it moves.
        let scale = window.scale_factor();
        let snap = |value: Pixels| px((f32::from(value) * scale).round() / scale);
        self.painted = Bounds::from_corners(
            point(snap(shown.left()), snap(shown.top())),
            point(snap(shown.right()), snap(shown.bottom())),
        );
        if let Some((outgoing, laid_out)) = &mut self.outgoing {
            window.with_element_namespace("outgoing", |window| {
                outgoing.prepaint_as_root(
                    laid_out.origin,
                    laid_out.size.map(AvailableSpace::Definite),
                    window,
                    cx,
                );
            });
        }
        let (view, painted) = (AnyView::from(self.tooltip.clone()), self.painted);
        self.handover.update(cx, |handover, _| {
            handover.drawn = Some((view, bounds));
            handover.painted = Some(painted);
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let style = self.tooltip.read(cx).box_style(cx);
        style.paint(self.painted, window, cx, |_, _| {});
        let mask = ContentMask {
            bounds: self.painted,
        };
        window.with_content_mask(Some(mask), |window| {
            if let Some((outgoing, _)) = &mut self.outgoing {
                window.with_element_namespace("outgoing", |window| outgoing.paint(window, cx));
            }
            self.content.paint(window, cx);
        });
    }
}

// ── Shared tooltip state for components ─────────────────────────────────────

/// Shared tooltip state that components (Button, Switch, Checkbox, Radio, etc.)
/// can embed to get `.tooltip()` support with minimal boilerplate.
#[derive(Default)]
pub(crate) struct ComponentTooltip {
    pub text: Option<(
        SharedString,
        Option<(Rc<Box<dyn Action>>, Option<SharedString>)>,
    )>,
    pub builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyView>>,
    pub show_delay: Option<Duration>,
}

impl ComponentTooltip {
    /// Apply this tooltip to a `Stateful<Div>` (or any `ManagedTooltipExt` element).
    pub fn apply<E: ManagedTooltipExt>(self, el: E) -> E {
        let show_delay = self.show_delay;
        if let Some(builder) = self.builder {
            el.managed_tooltip_with(None, show_delay, move |window, cx| builder(window, cx))
        } else if let Some((text, action)) = self.text {
            el.managed_tooltip_with(None, show_delay, move |window, cx| {
                Tooltip::new(text.clone())
                    .when_some(action.clone(), |this, (action, context)| {
                        this.action(
                            action.boxed_clone().as_ref(),
                            context.as_ref().map(|c| c.as_ref()),
                        )
                    })
                    .build(window, cx)
            })
        } else {
            el
        }
    }
}

// ── Managed tooltips ────────────────────────────────────────────────────────

/// Kit-managed tooltips for any stateful interactive element, including a `div` with an id.
///
/// These tooltips share the window's timing, placement and lifecycle rules: they leave with
/// their trigger, switch along a row of triggers, hide while their trigger's menu or popover
/// is open and are disabled on mobile platforms. GPUI's `InteractiveElement::tooltip` uses a
/// separate tooltip system.
pub trait ManagedTooltipExt: StatefulInteractiveElement + crate::ElementExt + Sized {
    /// Show a tooltip on hover, with placement chosen to fit the window.
    fn managed_tooltip(
        self,
        build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        self.managed_tooltip_with(None, None, build_tooltip)
    }

    /// Show a tooltip on hover, preferring `placement` when it fits the window.
    fn managed_tooltip_at(
        self,
        placement: Placement,
        build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        self.managed_tooltip_with(Some(placement), None, build_tooltip)
    }

    /// Show a tooltip on hover. A `None` placement is chosen to fit the
    /// window, and a `None` show delay is [`TooltipDefaults::show_delay`].
    fn managed_tooltip_with(
        self,
        preferred_placement: Option<Placement>,
        show_delay: Option<Duration>,
        build_tooltip: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    ) -> Self {
        let build_tooltip = Rc::new(build_tooltip);
        let trigger = TooltipTrigger::new();
        // Where the trigger was drawn last, for the press that dismisses its
        // tooltip until the pointer leaves those bounds.
        let bounds = Rc::new(Cell::new(Bounds::default()));

        self.on_prepaint({
            let trigger = trigger.clone();
            let bounds = bounds.clone();
            move |drawn, window, _| {
                trigger.prepaint(drawn, window);
                bounds.set(drawn);
            }
        })
        .on_hover({
            let bounds = bounds.clone();
            move |hovered, window, cx| {
                // A press keeps the tooltip dismissed until the pointer leaves
                // the trigger. This also drops the hover-enter that a click
                // removing the trigger dispatches to the stale frame.
                if *hovered && WindowState::is_tooltip_trigger_pressed(bounds.get(), window, cx) {
                    return;
                }
                let Some(overlay) = WindowState::tooltip_overlay(window, cx) else {
                    return;
                };
                overlay.update(cx, |overlay: &mut BaseTooltipOverlay, cx| {
                    if !*hovered {
                        overlay.request_hide_for(&trigger, window, cx);
                        return;
                    }
                    let build = build_tooltip.clone();
                    let request = BaseTooltipRequest::for_trigger(&trigger, move |window, cx| {
                        build(window, cx)
                    });
                    let request = match preferred_placement {
                        Some(placement) => request.with_placement(placement),
                        None => request,
                    };
                    let request = match show_delay {
                        Some(delay) => request.with_show_delay(delay),
                        None => request,
                    };
                    overlay.request_show(request, window, cx);
                });
            }
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            WindowState::press_tooltip_trigger(bounds.get(), window, cx);
            if let Some(overlay) = WindowState::tooltip_overlay(window, cx) {
                overlay.update(cx, |overlay, cx| {
                    overlay.hide(cx);
                });
            }
        })
    }
}

impl<E: StatefulInteractiveElement + crate::ElementExt> ManagedTooltipExt for E {}
#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc, time::Duration};

    use gpui::{
        AppContext as _, Context, Entity, InputEvent as _, InteractiveElement as _, IntoElement,
        MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement as _, Render,
        StatefulInteractiveElement as _, Styled as _, TestAppContext, VisualTestContext, Window,
        div, point, prelude::FluentBuilder as _, px,
    };

    use super::ManagedTooltipExt as _;
    use crate::Root;

    // Distinct scene markers let assertions observe retained tooltip output
    // without requiring its renderer to run on every frame.
    const TOOLTIP_COLORS: [u32; 2] = [0x7a135e, 0x136b7a];

    fn assert_tooltip_painted(cx: &mut VisualTestContext, index: usize, shown: bool) {
        draw(cx);
        let painted = cx.update(|window, _| {
            let marker: gpui::Background = gpui::rgb(TOOLTIP_COLORS[index]).into();
            window
                .painted_quads()
                .iter()
                .any(|quad| quad.background == marker)
        });
        assert_eq!(
            painted, shown,
            "tooltip {index} visibility in the rendered scene"
        );
    }

    /// A trigger that removes itself when clicked, like a button that
    /// replaces the view it sits in.
    struct Probe {
        shown: bool,
        removes_on_click: bool,
        builds: Rc<Cell<usize>>,
    }

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let builds = self.builds.clone();
            div().size_full().when(self.shown, |this| {
                this.child(
                    div()
                        .id("trigger")
                        .absolute()
                        .left(px(10.))
                        .top(px(10.))
                        .w(px(100.))
                        .h(px(30.))
                        .managed_tooltip(move |_, cx| {
                            builds.set(builds.get() + 1);
                            cx.new(|_| super::Tooltip::new("tip").bg(gpui::rgb(TOOLTIP_COLORS[0])))
                                .into()
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.removes_on_click {
                                this.shown = false;
                            }
                            cx.notify();
                        })),
                )
            })
        }
    }

    fn open(
        cx: &mut TestAppContext,
        removes_on_click: bool,
        builds: Rc<Cell<usize>>,
    ) -> (VisualTestContext, Entity<Probe>) {
        cx.update(|cx| {
            crate::init(cx);
            // These tests cover tooltip lifetime, not intermediate fade frames.
            cx.set_reduce_motion(true);
        });
        let probe = cx.new(|_| Probe {
            shown: true,
            removes_on_click,
            builds,
        });
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| Root::new(probe.clone(), window, cx))
            })
            .unwrap()
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        draw(&mut cx);
        (cx, probe)
    }

    /// Presses at `at` and releases a pixel away, as most real clicks do.
    fn click_with_drift(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>) {
        let drift = point(at.x + px(1.), at.y);
        cx.simulate_mouse_down(at, MouseButton::Left, Default::default());
        cx.simulate_mouse_move(drift, Some(MouseButton::Left), Default::default());
        cx.simulate_mouse_up(drift, MouseButton::Left, Default::default());
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        })
    }

    /// Lets any show delay or grace period run out, then draws.
    fn wait(cx: &mut VisualTestContext) {
        cx.executor().advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
        draw(cx);
    }

    struct AdjacentTriggers;

    impl Render for AdjacentTriggers {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().children((0..2).map(|index| {
                div()
                    .id(("trigger", index))
                    .absolute()
                    .left(px(10. + index as f32 * 100.))
                    .top(px(10.))
                    .w(px(100.))
                    .h(px(30.))
                    .managed_tooltip(move |_, cx| {
                        cx.new(|_| super::Tooltip::new("tip").bg(gpui::rgb(TOOLTIP_COLORS[index])))
                            .into()
                    })
            }))
        }
    }

    fn open_adjacent_triggers(
        cx: &mut TestAppContext,
    ) -> (VisualTestContext, Entity<AdjacentTriggers>) {
        cx.update(|cx| {
            crate::init(cx);
            // These tests cover tooltip lifetime, not intermediate fade frames.
            cx.set_reduce_motion(true);
        });
        let probe = cx.new(|_| AdjacentTriggers);
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| Root::new(probe.clone(), window, cx))
            })
            .unwrap()
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        draw(&mut cx);
        (cx, probe)
    }

    fn assert_tooltip_shown(cx: &mut VisualTestContext, index: usize) {
        assert_tooltip_painted(cx, index, true);
        assert_tooltip_painted(cx, 1 - index, false);
    }

    #[gpui::test]
    fn tooltips_switch_in_both_directions(cx: &mut TestAppContext) {
        let (mut cx, probe) = open_adjacent_triggers(cx);
        for (step, index) in [0, 1, 0].into_iter().enumerate() {
            let at = point(px(50. + index as f32 * 100.), px(25.));
            cx.simulate_mouse_move(at, None, Default::default());
            if step > 0 {
                // The open tooltip stays until the pointer rests on the next
                // trigger for the switch delay.
                draw(&mut cx);
                assert_tooltip_shown(&mut cx, 1 - index);
                cx.executor()
                    .advance_clock(super::TooltipDefaults::new().switch_delay());
                cx.run_until_parked();
                assert_tooltip_shown(&mut cx, index);
            }
            wait(&mut cx);
            assert_tooltip_shown(&mut cx, index);
            // Rebuilding the triggers must preserve ownership for hover-leave.
            cx.update(|_, cx| probe.update(cx, |_, cx| cx.notify()));
            draw(&mut cx);
        }
        cx.simulate_mouse_move(point(px(300.), px(300.)), None, Default::default());
        wait(&mut cx);
        assert_tooltip_painted(&mut cx, 0, false);
        assert_tooltip_painted(&mut cx, 1, false);
    }

    #[gpui::test]
    fn leaving_a_trigger_does_not_cancel_the_next_tooltip_delay(cx: &mut TestAppContext) {
        for (source, destination) in [(0, 1), (1, 0)] {
            let (mut cx, _) = open_adjacent_triggers(cx);
            cx.simulate_mouse_move(
                point(px(50. + source as f32 * 100.), px(25.)),
                None,
                Default::default(),
            );
            cx.executor().advance_clock(Duration::from_millis(100));
            cx.run_until_parked();
            cx.simulate_mouse_move(
                point(px(50. + destination as f32 * 100.), px(25.)),
                None,
                Default::default(),
            );
            wait(&mut cx);
            assert_tooltip_shown(&mut cx, destination);
        }
    }

    /// A press dismisses the tooltip, and it stays dismissed while the
    /// pointer stays on the trigger; leaving and coming back shows it again.
    #[gpui::test]
    fn a_press_dismisses_the_tooltip_until_the_pointer_leaves(cx: &mut TestAppContext) {
        let builds = Rc::new(Cell::new(0));
        let (mut cx, _) = open(cx, false, builds.clone());
        let at = point(px(50.), px(25.));
        cx.simulate_mouse_move(at, None, Default::default());
        wait(&mut cx);
        assert!(builds.get() > 0, "the hovered trigger shows its tooltip");

        // Hover reads false while the button is held, so the move after the
        // release is a hover-enter that must not show the tooltip.
        click_with_drift(&mut cx, at);
        cx.simulate_mouse_move(point(px(52.), px(25.)), None, Default::default());
        wait(&mut cx);
        assert_tooltip_painted(&mut cx, 0, false);

        cx.simulate_mouse_move(point(px(300.), px(300.)), None, Default::default());
        wait(&mut cx);
        cx.simulate_mouse_move(at, None, Default::default());
        wait(&mut cx);
        assert_tooltip_painted(&mut cx, 0, true);
    }

    /// A pointer move that lands between the release and the next frame is
    /// dispatched to the old frame, where the trigger is still hovered, so it
    /// used to request the tooltip again. The trigger is gone from the next
    /// frame, nothing ever sent the hover-leave, and the tooltip appeared
    /// after the show delay and stayed.
    #[gpui::test]
    fn clicking_a_trigger_away_does_not_leave_its_tooltip(cx: &mut TestAppContext) {
        let builds = Rc::new(Cell::new(0));
        let (mut cx, _) = open(cx, true, builds.clone());

        let at = point(px(50.), px(25.));
        cx.simulate_mouse_move(at, None, Default::default());
        draw(&mut cx);
        wait(&mut cx);
        draw(&mut cx);
        assert!(builds.get() > 0, "the hovered trigger shows its tooltip");

        // Press, drift a pixel while held, release, and drift again before
        // the next frame is drawn.
        cx.simulate_mouse_down(at, MouseButton::Left, Default::default());
        let drift = point(px(51.), px(25.));
        cx.simulate_mouse_move(drift, Some(MouseButton::Left), Default::default());
        // One update, so no frame is drawn between the two: the test
        // platform otherwise draws a dirty window after every event.
        cx.update(|window, cx| {
            window.dispatch_event(
                MouseUpEvent {
                    button: MouseButton::Left,
                    position: drift,
                    modifiers: Default::default(),
                    click_count: 1,
                }
                .to_platform_input(),
                cx,
            );
            window.dispatch_event(
                MouseMoveEvent {
                    position: point(px(52.), px(25.)),
                    pressed_button: None,
                    modifiers: Default::default(),
                }
                .to_platform_input(),
                cx,
            );
        });
        draw(&mut cx);
        wait(&mut cx);

        let before = builds.get();
        draw(&mut cx);
        assert_eq!(
            builds.get(),
            before,
            "no tooltip may be shown for a trigger the click removed"
        );
    }

    /// A trigger that a click removed never reports a hover-leave, so the
    /// press must end when the pointer moves away, or a trigger later shown
    /// at the same bounds would lose its tooltip.
    #[gpui::test]
    fn a_press_on_a_removed_trigger_ends_when_the_pointer_moves_away(cx: &mut TestAppContext) {
        let builds = Rc::new(Cell::new(0));
        let (mut cx, probe) = open(cx, true, builds.clone());
        let at = point(px(50.), px(25.));
        cx.simulate_mouse_move(at, None, Default::default());
        wait(&mut cx);

        cx.simulate_click(at, Default::default());
        wait(&mut cx);
        cx.simulate_mouse_move(point(px(300.), px(300.)), None, Default::default());
        wait(&mut cx);

        cx.update(|_, cx| {
            probe.update(cx, |probe, cx| {
                probe.shown = true;
                cx.notify();
            })
        });
        wait(&mut cx);
        let before = builds.get();
        cx.simulate_mouse_move(at, None, Default::default());
        wait(&mut cx);
        draw(&mut cx);
        assert!(
            builds.get() > before,
            "the trigger shown again has its tooltip"
        );
    }

    struct Buttons {
        shown: [bool; 2],
        delay: Duration,
    }

    impl Render for Buttons {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().children((0..2).filter_map(|index| {
                self.shown[index].then(|| {
                    crate::button::Button::new(("button", index))
                        .absolute()
                        .left(px(10. + index as f32 * 100.))
                        .top(px(10.))
                        .w(px(80.))
                        .h(px(30.))
                        .label(if index == 0 { "A" } else { "B" })
                        .tooltip(if index == 0 { "First" } else { "Second" })
                        .tooltip_show_delay(self.delay)
                })
            }))
        }
    }

    struct CachedButtons(Entity<Buttons>);

    impl Render for CachedButtons {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                self.0
                    .clone()
                    .cached(gpui::StyleRefinement::default().size_full()),
            )
        }
    }

    fn open_buttons(
        cx: &mut TestAppContext,
        shown: [bool; 2],
        delay: Duration,
    ) -> (VisualTestContext, Entity<Buttons>, Rc<Cell<usize>>) {
        cx.update(|cx| {
            crate::init(cx);
            // These tests cover tooltip lifetime, not intermediate fade frames.
            cx.set_reduce_motion(true);
        });
        let buttons = cx.new(|_| Buttons { shown, delay });
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let content = cx.new(|_| CachedButtons(buttons.clone()));
                cx.new(|cx| Root::new(content, window, cx))
            })
            .unwrap()
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        draw(&mut cx);
        let builds = Rc::new(Cell::new(0));
        cx.update(|window, cx| {
            let builds = builds.clone();
            // Observe the real Button's overlay without changing Base's public
            // API or adding test-only markup to the production Tooltip.
            super::WindowState::tooltip_overlay(window, cx)
                .unwrap()
                .update(cx, |overlay, _| {
                    *overlay =
                        std::mem::take(overlay).render_with(move |view, transition, window, cx| {
                            builds.set(builds.get() + 1);
                            div()
                                .bg(gpui::rgb(TOOLTIP_COLORS[0]))
                                .child(super::render_tooltip(view, transition, window, cx))
                                .into_any_element()
                        });
                });
        });
        (cx, buttons, builds)
    }

    #[gpui::test]
    fn removing_button_hides_visible_tooltip(cx: &mut TestAppContext) {
        let (mut cx, buttons, _) = open_buttons(cx, [true, false], Duration::ZERO);
        cx.simulate_mouse_move(point(px(50.), px(25.)), None, Default::default());
        wait(&mut cx);
        assert_tooltip_painted(&mut cx, 0, true);
        // A redraw reusing the cached button subtree preserves the owner.
        draw(&mut cx);
        assert_tooltip_painted(&mut cx, 0, true);
        cx.update(|_, cx| {
            buttons.update(cx, |buttons, cx| {
                buttons.shown[0] = false;
                cx.notify();
            })
        });
        draw(&mut cx);
        cx.run_until_parked();
        assert_tooltip_painted(&mut cx, 0, false);
    }

    #[gpui::test]
    fn removing_button_cancels_pending_tooltip(cx: &mut TestAppContext) {
        let (mut cx, buttons, builds) = open_buttons(cx, [true, false], Duration::from_millis(500));
        cx.simulate_mouse_move(point(px(50.), px(25.)), None, Default::default());
        draw(&mut cx);
        // Finish the hover frame and drain its deferred cleanup before removal,
        // without advancing the pending tooltip's show delay.
        cx.run_until_parked();
        assert_eq!(builds.get(), 0, "tooltip is still pending");
        cx.update(|_, cx| {
            buttons.update(cx, |buttons, cx| {
                buttons.shown[0] = false;
                cx.notify();
            })
        });
        draw(&mut cx);
        cx.run_until_parked();
        wait(&mut cx);
        assert_eq!(
            builds.get(),
            0,
            "a removed trigger must never build its tooltip"
        );
        assert_tooltip_painted(&mut cx, 0, false);
    }

    #[gpui::test]
    fn pending_button_shows_after_deferred_work_drains(cx: &mut TestAppContext) {
        let (mut cx, _, builds) = open_buttons(cx, [true, false], Duration::from_millis(500));
        cx.simulate_mouse_move(point(px(50.), px(25.)), None, Default::default());
        draw(&mut cx);
        cx.run_until_parked();
        assert_eq!(builds.get(), 0, "tooltip is still pending");
        wait(&mut cx);
        assert!(
            builds.get() > 0,
            "a surviving trigger builds its delayed tooltip"
        );
        assert_tooltip_painted(&mut cx, 0, true);
    }

    #[gpui::test]
    fn removing_old_button_preserves_new_tooltip(cx: &mut TestAppContext) {
        let (mut cx, buttons, _) = open_buttons(cx, [true, true], Duration::ZERO);
        cx.simulate_mouse_move(point(px(50.), px(25.)), None, Default::default());
        wait(&mut cx);
        cx.simulate_mouse_move(point(px(150.), px(25.)), None, Default::default());
        wait(&mut cx);
        assert_tooltip_painted(&mut cx, 0, true);
        cx.update(|_, cx| {
            buttons.update(cx, |buttons, cx| {
                buttons.shown[0] = false;
                cx.notify();
            })
        });
        draw(&mut cx);
        cx.run_until_parked();
        assert_tooltip_painted(&mut cx, 0, true);
    }
}
