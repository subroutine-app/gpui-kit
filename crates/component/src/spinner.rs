use crate::{Icon, IconName, Sizable, Size};
use gpui::{
    Animation, AnimationExt as _, App, Hsla, IntoElement, ParentElement, RenderOnce, Styled as _,
    Window, div, linear, percentage, prelude::FluentBuilder as _,
};
use instant::Duration;

/// A cycling loading spinner.
///
/// It turns at a constant speed, in a frame for each frame the display shows,
/// and holds still while the system asks for reduced motion.
#[derive(IntoElement)]
pub struct Spinner {
    size: Size,
    icon: Icon,
    duration: Duration,
    easing: Box<dyn Fn(f32) -> f32>,
    color: Option<Hsla>,
    max_fps: Option<f32>,
}

impl Spinner {
    /// Create a new loading spinner.
    pub fn new() -> Self {
        Self {
            size: Size::Medium,
            duration: Duration::from_secs_f64(0.8),
            easing: Box::new(linear),
            icon: Icon::new(IconName::Loader),
            color: None,
            max_fps: None,
        }
    }

    /// Set specified icon for the spinner.
    ///
    /// Default is [`IconName::Loader`].
    ///
    /// Please ensure the icon used is suitable for a loading spinner.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set the icon color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the easing function.
    ///
    /// The default is [`linear`], which turns at a constant speed. Any other
    /// curve makes the spinner speed up and slow down once in each turn.
    pub fn ease(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Box::new(easing);
        self
    }

    /// Set how long one turn takes.
    ///
    /// The default is 0.8 seconds.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Draw at most `max_fps` frames a second, such as
    /// [`LOADING_ANIMATION_MAX_FPS`](crate::LOADING_ANIMATION_MAX_FPS).
    ///
    /// The default is a frame for each frame the display shows, which is the
    /// smoothest. A lower rate draws on a timer that does not line up with the
    /// display, so some frames stay up longer than others.
    pub fn max_fps(mut self, max_fps: f32) -> Self {
        self.max_fps = Some(max_fps);
        self
    }
}

impl Sizable for Spinner {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let animation = Animation::new(self.duration)
            .repeat_synced()
            .with_easing(self.easing);
        let animation = match self.max_fps {
            Some(max_fps) => animation.with_max_fps(max_fps),
            None => animation,
        };
        let icon = self
            .icon
            .with_size(self.size)
            .when_some(self.color, |this, color| this.text_color(color));
        div().child(turn(icon, animation)).into_element()
    }
}

/// Turning as it paints leaves the view around the spinner cached. The shared
/// clock turns spinners together, and keeps the precision of one left up for
/// days.
#[cfg(not(feature = "gpui-fast"))]
fn turn(icon: Icon, animation: Animation) -> impl IntoElement {
    icon.with_paint_animation("circle", animation, |delta| {
        gpui::PaintEffect::rotation(percentage(delta).into())
    })
}

/// GPUI Fast has no paint effects, so the view around the spinner renders for
/// each step.
#[cfg(feature = "gpui-fast")]
fn turn(icon: Icon, animation: Animation) -> impl IntoElement {
    icon.with_animation("circle", animation, |icon, delta| {
        icon.transform(gpui::Transformation::rotate(percentage(delta)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Render, TestAppContext, px, size};
    use std::{cell::Cell, rc::Rc};

    struct SpinnerHost;

    impl Render for SpinnerHost {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            Spinner::new()
        }
    }

    /// A window's root that holds a spinner and counts how often it is drawn.
    struct Counted(Rc<Cell<usize>>, Box<dyn Fn() -> Spinner>);

    impl Render for Counted {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            self.0.set(self.0.get() + 1);
            (self.1)()
        }
    }

    fn counted_window(
        cx: &mut TestAppContext,
        spinner: impl Fn() -> Spinner + 'static,
    ) -> (gpui::WindowHandle<Counted>, Rc<Cell<usize>>) {
        let renders = Rc::new(Cell::new(0));
        let window = cx.open_window(size(px(100.), px(100.)), {
            let renders = renders.clone();
            move |_, _| Counted(renders, Box::new(spinner))
        });
        cx.run_until_parked();
        (window, renders)
    }

    /// A capped spinner redraws at its rate on a timer, not at every frame the
    /// display offers.
    #[gpui::test]
    fn a_capped_spinner_redraws_at_its_rate(cx: &mut TestAppContext) {
        let (window, renders) = counted_window(cx, || Spinner::new().max_fps(30.));
        assert_eq!(
            window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap(),
            0,
            "no redraw is asked of every frame"
        );
        let before = renders.get();
        for _ in 0..30 {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(1000 / 30 + 1));
            cx.run_until_parked();
        }
        let redraws = renders.get() - before;
        assert!(
            (25..=31).contains(&redraws),
            "{redraws} redraws in a second"
        );
    }

    /// A spinner draws in each frame the display shows, unless it is capped.
    #[gpui::test]
    fn a_spinner_redraws_at_every_frame(cx: &mut TestAppContext) {
        let (window, renders) = counted_window(cx, Spinner::new);
        let before = renders.get();
        for _ in 0..60 {
            cx.executor()
                .advance_clock(std::time::Duration::from_micros(16_667));
            assert!(
                window
                    .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                    .unwrap()
                    > 0,
                "the next frame was not asked for"
            );
            cx.run_until_parked();
        }
        assert_eq!(renders.get() - before, 60);
    }

    /// A pane that holds a spinner, and the values its easing was asked for.
    #[cfg(not(feature = "gpui-fast"))]
    struct Pane {
        renders: Rc<Cell<usize>>,
        eased: Rc<std::cell::RefCell<Vec<f32>>>,
        max_fps: Option<f32>,
        duration: Option<Duration>,
    }

    #[cfg(not(feature = "gpui-fast"))]
    impl Render for Pane {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            self.renders.set(self.renders.get() + 1);
            let eased = self.eased.clone();
            Spinner::new()
                .ease(move |delta| {
                    eased.borrow_mut().push(delta);
                    delta
                })
                .when_some(self.duration, Spinner::duration)
                .when_some(self.max_fps, Spinner::max_fps)
        }
    }

    /// A window whose root keeps the pane cached, as the panes of an app's window are.
    #[cfg(not(feature = "gpui-fast"))]
    struct PaneWindow {
        pane: gpui::Entity<Pane>,
    }

    #[cfg(not(feature = "gpui-fast"))]
    impl Render for PaneWindow {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div().size_full().child(
                self.pane
                    .clone()
                    .cached(gpui::StyleRefinement::default().size_full()),
            )
        }
    }

    /// Turns a spinner in a cached pane for a second, with a frame every 1/60 s and the
    /// executor parked after each. Returns how often the pane was drawn and the values
    /// the spinner's easing was asked for.
    #[cfg(not(feature = "gpui-fast"))]
    fn turn_in_a_cached_pane(
        cx: &mut TestAppContext,
        max_fps: Option<f32>,
        duration: Option<Duration>,
    ) -> (usize, Vec<f32>) {
        use gpui::AppContext as _;

        let renders = Rc::new(Cell::new(0));
        let eased = Rc::new(std::cell::RefCell::new(Vec::new()));
        let window = cx.open_window(size(px(100.), px(100.)), {
            let (renders, eased) = (renders.clone(), eased.clone());
            move |_, cx| PaneWindow {
                pane: cx.new(|_| Pane {
                    renders,
                    eased,
                    max_fps,
                    duration,
                }),
            }
        });
        cx.run_until_parked();
        assert_eq!(renders.get(), 1);
        eased.borrow_mut().clear();

        for _ in 0..60 {
            cx.executor()
                .advance_clock(std::time::Duration::from_micros(16_667));
            window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap();
            cx.run_until_parked();
        }
        window
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
        let eased = eased.borrow().clone();
        (renders.get(), eased)
    }

    #[cfg(not(feature = "gpui-fast"))]
    #[gpui::test]
    fn a_spinner_turns_in_a_cached_view(cx: &mut TestAppContext) {
        let (renders, eased) = turn_in_a_cached_pane(cx, None, None);
        assert_eq!(renders, 1, "the pane is drawn again");
        assert!(
            (59..=61).contains(&eased.len()),
            "{} steps in a second",
            eased.len()
        );
        // The turn goes round once in 0.8 s and starts again.
        assert!(eased.iter().any(|delta| *delta > 0.9), "{eased:?}");
        assert!(eased.windows(2).any(|pair| pair[1] < pair[0]), "{eased:?}");
    }

    #[cfg(not(feature = "gpui-fast"))]
    #[gpui::test]
    fn a_spinner_turns_in_the_time_it_was_given(cx: &mut TestAppContext) {
        let (_, eased) = turn_in_a_cached_pane(cx, None, Some(Duration::from_secs(2)));
        let furthest = eased.iter().copied().fold(0., f32::max);
        // A second into a turn of two seconds is halfway round, with a frame of slack.
        assert!((0.47..=0.53).contains(&furthest), "{furthest}");
        assert!(eased.windows(2).all(|pair| pair[1] > pair[0]), "{eased:?}");
    }

    #[cfg(not(feature = "gpui-fast"))]
    #[gpui::test]
    fn a_capped_spinner_turns_in_a_cached_view(cx: &mut TestAppContext) {
        let (renders, eased) = turn_in_a_cached_pane(cx, Some(30.), None);
        assert_eq!(renders, 1, "the pane is drawn again");
        assert!(
            (28..=34).contains(&eased.len()),
            "{} steps in a second",
            eased.len()
        );
    }

    #[gpui::test]
    fn reduced_motion_spinner_is_static_and_requests_no_frame(cx: &mut TestAppContext) {
        cx.update(|cx| cx.set_reduce_motion(true));
        let window = cx.open_window(size(px(100.), px(100.)), |_, _| SpinnerHost);
        cx.run_until_parked();

        assert_eq!(
            window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap(),
            0
        );
    }
}
