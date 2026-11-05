//! Paint animations, which the vendored GPUI backend adds (see `vendor/README.md`): an element
//! painted with an animated `PaintEffect` changes in each frame while the cached view around it
//! stays cached. Ported from Mesa Hills Research's GPUI fork.
use gpui_kit::{
    Animation, AnimationExt as _, AppContext as _, Context, Entity, IntoElement, Modifiers,
    MouseButton, PaintEffect, StyleRefinement, TestAppContext, TransformationMatrix, Window,
    WindowHandle, canvas, div, point, prelude::*, px, radians, red, size,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

/// A view with a box whose opacity follows a paint animation.
struct Pane {
    animation: Animation,
    renders: Rc<RefCell<usize>>,
}

impl Render for Pane {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        *self.renders.borrow_mut() += 1;
        div()
            .size_full()
            .child(div().size(px(10.)).bg(red()).with_paint_animation(
                "box",
                self.animation.clone(),
                PaintEffect::opacity,
            ))
    }
}

/// A window whose root draws a box of its own above a cached pane with one.
struct PaneRoot {
    pane: Entity<Pane>,
    animation: Animation,
}

impl Render for PaneRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().size(px(10.)).bg(red()).with_paint_animation(
                "root box",
                self.animation.clone(),
                PaintEffect::opacity,
            ))
            .child(
                self.pane
                    .clone()
                    .cached(StyleRefinement::default().w_full().h(px(10.))),
            )
    }
}

struct Fixture {
    window: WindowHandle<PaneRoot>,
    pane: Entity<Pane>,
    pane_renders: Rc<RefCell<usize>>,
}

impl Fixture {
    fn open(cx: &mut TestAppContext, animation: Animation) -> Self {
        let pane_renders = Rc::new(RefCell::new(0));
        let window = cx.open_window(size(px(100.), px(100.)), {
            let renders = pane_renders.clone();
            move |_, cx| PaneRoot {
                pane: cx.new(|_| Pane {
                    animation: animation.clone(),
                    renders,
                }),
                animation,
            }
        });
        cx.run_until_parked();
        let pane = window.update(cx, |root, _, _| root.pane.clone()).unwrap();
        Self {
            window,
            pane,
            pane_renders,
        }
    }

    /// The opacity of each box in the frame drawn last, from the top.
    fn alphas(&self, cx: &mut TestAppContext) -> Vec<f32> {
        self.window
            .update(cx, |_, window, _| {
                let scale_factor = window.scale_factor();
                let mut boxes: Vec<_> = window
                    .rendered_scene()
                    .quads
                    .iter()
                    .filter(|quad| quad.bounds.size.width.0 / scale_factor == 10.)
                    .map(|quad| {
                        (
                            quad.bounds.origin.y.0,
                            quad.background.as_solid().unwrap().a,
                        )
                    })
                    .collect();
                boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
                boxes.into_iter().map(|(_, alpha)| alpha).collect()
            })
            .unwrap()
    }

    /// Advances the clock, delivers the next display frame, and returns how many frame
    /// callbacks were waiting for it.
    fn advance(&self, cx: &mut TestAppContext, by: Duration) -> usize {
        next_frame(&self.window, cx, by)
    }
}

fn next_frame<V: Render>(window: &WindowHandle<V>, cx: &mut TestAppContext, by: Duration) -> usize {
    cx.executor().advance_clock(by);
    let asked = window
        .update(cx, |_, window, cx| window.simulate_next_frame(cx))
        .unwrap();
    cx.run_until_parked();
    asked
}

fn synced() -> Animation {
    Animation::new(Duration::from_secs(1)).repeat_synced()
}

#[gpui_kit::test]
fn a_paint_animation_runs_in_a_view_that_is_reused(cx: &mut TestAppContext) {
    let fixture = Fixture::open(cx, synced());
    let mut seen = Vec::new();
    for _ in 0..8 {
        assert_eq!(fixture.advance(cx, Duration::from_millis(100)), 1);
        let alphas = fixture.alphas(cx);
        assert_eq!(alphas.len(), 2);
        // The box in the cached pane changes as the one the root draws again does.
        assert_eq!(alphas[0], alphas[1]);
        seen.push(alphas[0]);
    }
    assert_eq!(*fixture.pane_renders.borrow(), 1, "the pane is drawn again");
    assert!(seen.windows(2).all(|pair| pair[0] < pair[1]), "{seen:?}");
}

#[gpui_kit::test]
fn a_paint_animation_keeps_its_place_when_its_view_draws_again(cx: &mut TestAppContext) {
    let fixture = Fixture::open(cx, Animation::new(Duration::from_secs(1)).repeat());
    fixture.advance(cx, Duration::from_millis(300));
    let before = fixture.alphas(cx)[1];

    fixture.pane.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    assert_eq!(*fixture.pane_renders.borrow(), 2);
    let after = fixture.alphas(cx)[1];
    assert!(
        before > 0.25 && (after - before).abs() < 0.05,
        "{before} {after}"
    );
}

#[gpui_kit::test]
fn a_paint_animation_stops_asking_for_frames_when_it_ends(cx: &mut TestAppContext) {
    let fixture = Fixture::open(cx, Animation::new(Duration::from_secs(1)));
    assert_eq!(fixture.advance(cx, Duration::from_millis(400)), 1);
    assert!(fixture.alphas(cx)[0] < 0.6);
    assert_eq!(fixture.advance(cx, Duration::from_millis(700)), 1);
    assert_eq!(fixture.alphas(cx), [1., 1.]);
    assert_eq!(fixture.advance(cx, Duration::from_millis(100)), 0);
    assert_eq!(*fixture.pane_renders.borrow(), 1);
}

#[gpui_kit::test]
fn a_paint_animation_holds_still_under_reduced_motion(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let repeating = Fixture::open(cx, synced());
    assert_eq!(repeating.alphas(cx), [0., 0.]);
    assert_eq!(repeating.advance(cx, Duration::from_millis(300)), 0);

    let oneshot = Fixture::open(cx, Animation::new(Duration::from_secs(1)));
    assert_eq!(oneshot.alphas(cx), [1., 1.]);
    assert_eq!(oneshot.advance(cx, Duration::from_millis(300)), 0);
}

#[gpui_kit::test]
fn a_capped_paint_animation_draws_on_a_timer(cx: &mut TestAppContext) {
    let fixture = Fixture::open(cx, synced().with_max_fps(10.));
    assert_eq!(
        fixture
            .window
            .update(cx, |_, window, cx| window.simulate_next_frame(cx))
            .unwrap(),
        0,
        "no display frame is asked for"
    );

    let mut changes = 0;
    let mut last = fixture.alphas(cx)[0];
    for _ in 0..1000 {
        cx.executor().advance_clock(Duration::from_millis(1));
        cx.run_until_parked();
        let alpha = fixture.alphas(cx)[0];
        if alpha != last {
            changes += 1;
            last = alpha;
        }
    }
    assert!((9..=11).contains(&changes), "{changes} steps in a second");
    assert_eq!(*fixture.pane_renders.borrow(), 1);
}

/// A box that rises and fades inside a box that slides across, in a cached view.
struct Slide {
    renders: Rc<RefCell<usize>>,
    presses: Rc<RefCell<usize>>,
}

impl Render for Slide {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        *self.renders.borrow_mut() += 1;
        let presses = self.presses.clone();
        div().size_full().child(
            div()
                .size(px(10.))
                .child(
                    div()
                        .id("box")
                        .size(px(10.))
                        .bg(red())
                        .on_mouse_down(MouseButton::Left, move |_, _, _| {
                            *presses.borrow_mut() += 1;
                        })
                        .with_paint_animation(
                            "rise",
                            Animation::new(Duration::from_secs(1)),
                            |value| {
                                PaintEffect::translation(point(px(0.), px(40. * value)))
                                    .with_opacity(1. - value)
                            },
                        ),
                )
                .with_paint_animation("slide", Animation::new(Duration::from_secs(1)), |value| {
                    PaintEffect::translation(point(px(100. * value), px(0.)))
                }),
        )
    }
}

struct SlideRoot(Entity<Slide>);

impl Render for SlideRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            self.0
                .clone()
                .cached(StyleRefinement::default().w_full().h(px(10.))),
        )
    }
}

#[gpui_kit::test]
fn a_paint_animation_moves_what_it_paints_and_not_its_hitbox(cx: &mut TestAppContext) {
    let renders = Rc::new(RefCell::new(0));
    let presses = Rc::new(RefCell::new(0));
    let (_, cx) = cx.add_window_view({
        let (renders, presses) = (renders.clone(), presses.clone());
        move |_, cx| SlideRoot(cx.new(|_| Slide { renders, presses }))
    });
    cx.run_until_parked();
    let drawn = |cx: &mut gpui_kit::VisualTestContext| {
        cx.update(|window, _| {
            let scale = window.scale_factor();
            let quad = window.rendered_scene().quads[0];
            (
                quad.bounds.origin.x.0 / scale,
                quad.bounds.origin.y.0 / scale,
                quad.background.as_solid().unwrap().a,
            )
        })
    };
    assert_eq!(drawn(cx), (0., 0., 1.));

    // The translations add up and the opacities multiply, and the view isn't drawn again.
    for step in 1..=4 {
        cx.executor().advance_clock(Duration::from_millis(250));
        assert_eq!(cx.update(|window, cx| window.simulate_next_frame(cx)), 1);
        cx.run_until_parked();
        let value = step as f32 * 0.25;
        let (x, y, alpha) = drawn(cx);
        assert!(
            (x - 100. * value).abs() < 0.01
                && (y - 40. * value).abs() < 0.01
                && (alpha - (1. - value)).abs() < 0.01,
            "step {step}: {:?}",
            (x, y, alpha)
        );
    }
    assert_eq!(*renders.borrow(), 1, "the view is drawn again");

    // A press reaches the box where it was laid out, not where it is drawn.
    cx.simulate_mouse_down(
        point(px(105.), px(45.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(*presses.borrow(), 0);
    cx.simulate_mouse_down(point(px(5.), px(5.)), MouseButton::Left, Modifiers::none());
    assert_eq!(*presses.borrow(), 1);
}

struct Icon;

impl Render for Icon {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            canvas(
                |_, _, _| (),
                |bounds, _, window, cx| {
                    let icon = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="4" height="10"/></svg>"#;
                    window
                        .paint_svg(
                            bounds,
                            "icon.svg".into(),
                            Some(icon),
                            TransformationMatrix::unit(),
                            red(),
                            cx,
                        )
                        .unwrap();
                },
            )
            .size(px(10.))
            .with_paint_animation("icon", synced(), |value| {
                PaintEffect::rotation(radians(value * std::f32::consts::TAU))
            }),
        )
    }
}

#[gpui_kit::test]
fn a_paint_animation_turns_an_icon(cx: &mut TestAppContext) {
    let window = cx.open_window(size(px(100.), px(100.)), |_, _| Icon);
    cx.run_until_parked();
    let turned = |cx: &mut TestAppContext| {
        window
            .update(cx, |_, window, _| {
                let sprites = &window.rendered_scene().monochrome_sprites;
                assert_eq!(sprites.len(), 1);
                sprites[0].transformation.rotation_scale
            })
            .unwrap()
    };
    assert_eq!(turned(cx), [[1., 0.], [0., 1.]]);

    // A quarter of a second into a turn of a second is a quarter turn.
    next_frame(&window, cx, Duration::from_millis(250));
    let quarter = turned(cx);
    assert!(
        quarter[0][0].abs() < 0.05 && (quarter[1][0] - 1.).abs() < 0.05,
        "{quarter:?}"
    );
}
