//! The smooth caret, driven by keystrokes under the test scheduler's clock,
//! with a frame drawn at each step of the clock.

use std::time::Duration;

use gpui::{
    AppContext as _, Bounds, Context, Entity, IntoElement, ParentElement as _, Pixels, Render,
    Styled as _, TestAppContext, TextAlign, VisualTestContext, Window, div, px, size,
};

use super::*;
use crate::input::{EditorMode, InputMode, Paste, Redo, SelectAll, TextareaMode, Undo};

/// The step between frames.
const FRAME_MS: u64 = 5;

/// How long a keystroke takes to settle by default, in milliseconds.
fn typing_ms() -> u64 {
    SmoothCaretOptions::default().typing().as_millis() as u64
}

struct Harness<M: InputModeKind> {
    state: Entity<InputBaseState<M>>,
    width: Pixels,
}

impl<M: InputModeKind> Render for Harness<M> {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().w(self.width).h(px(200.)).child(self.state.clone()))
    }
}

/// What one frame drew of the caret.
#[derive(Clone)]
struct Sample {
    plan: CaretPlan,
    /// The glide's window x, before it is put on a device pixel, and its
    /// velocity.
    x: f32,
    velocity: f32,
}

/// A focused input in an active window, with the smooth caret on.
struct Glider<M: InputModeKind> {
    cx: VisualTestContext,
    state: Entity<InputBaseState<M>>,
}

/// `text` with its caret marker `ˇ` taken out, and where the marker was.
fn parse(marked: &str) -> (String, usize) {
    let caret = marked.find('ˇ').expect("a caret marker");
    (marked.replacen('ˇ', "", 1), caret)
}

impl<M: InputModeKind> Glider<M> {
    fn with(
        cx: &mut TestAppContext,
        width: Pixels,
        marked: &str,
        make: impl FnOnce(&mut Window, &mut Context<InputBaseState<M>>) -> InputBaseState<M> + 'static,
    ) -> Self {
        cx.update(crate::init);
        let (text, caret) = parse(marked);
        let window = cx.open_window(size(px(600.), px(400.)), move |window, cx| Harness {
            state: cx.new(|cx| make(window, cx).default_value(text)),
            width,
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        let state = window
            .read_with(&cx, |harness, _| harness.state.clone())
            .unwrap();
        cx.update(|window, cx| {
            // The caret is drawn in an active window only.
            window.activate_window();
            state.update(cx, |state, cx| {
                state.focus(window, cx);
                state.set_selected_range(caret..caret, cx);
            });
        });
        let mut glider = Self { cx, state };
        glider.draw();
        glider.frames_requested();
        glider
    }

    fn update<R>(
        &mut self,
        f: impl FnOnce(&mut InputBaseState<M>, &mut Window, &mut Context<InputBaseState<M>>) -> R,
    ) -> R {
        let state = self.state.clone();
        let result = self
            .cx
            .update(|window, cx| state.update(cx, |state, cx| f(state, window, cx)));
        self.draw();
        result
    }

    fn options(&mut self, options: SmoothCaretOptions) {
        self.update(|state, _, cx| state.set_smooth_caret_options(options, cx));
    }

    fn draw(&mut self) {
        self.cx.update(|window, cx| window.draw(cx).clear(cx));
        self.cx.run_until_parked();
    }

    /// Types `text` as the platform delivers it.
    fn type_text(&mut self, text: &str) {
        self.cx.simulate_input(text);
        self.draw();
    }

    /// Presses space-separated keystrokes.
    fn keys(&mut self, keystrokes: &str) {
        self.cx.simulate_keystrokes(keystrokes);
        self.draw();
    }

    fn dispatch(&mut self, action: impl gpui::Action) {
        self.cx.dispatch_action(action);
        self.draw();
    }

    fn value(&self) -> String {
        self.state
            .read_with(&self.cx, |state, _| state.value().to_string())
    }

    /// Moves the clock on and draws a frame.
    fn advance(&mut self, ms: u64) {
        self.cx.executor().advance_clock(Duration::from_millis(ms));
        self.draw();
    }

    /// The last frame's glide, or `None` when it drew the caret where it is.
    fn sample(&self) -> Option<Sample> {
        self.state.read_with(&self.cx, |state, _| {
            state
                .smooth_caret
                .last_frame
                .clone()
                .map(|(plan, x, velocity)| Sample { plan, x, velocity })
        })
    }

    #[track_caller]
    fn gliding(&self) -> Sample {
        self.sample().expect("the caret glides")
    }

    #[track_caller]
    fn assert_still(&self, what: &str) {
        assert!(
            self.sample().is_none(),
            "{what}: the caret is drawn where it is, with nothing hidden or moved"
        );
    }

    /// The window x of the caret where it is, as the last frame laid it out.
    fn caret_x(&self) -> Pixels {
        self.state.read_with(&self.cx, |state, _| {
            state.cursor_layout().expect("a caret").0.origin.x
        })
    }

    /// The caret the last frame painted, if it painted one.
    fn painted_caret(&self) -> Option<Bounds<Pixels>> {
        self.state
            .read_with(&self.cx, |state, _| state.smooth_caret.painted_caret)
    }

    /// The caret is painted where it is, on the device pixel nearest it.
    #[track_caller]
    fn assert_drawn_at_caret(&self) {
        let painted = self.painted_caret().expect("a painted caret");
        assert_close(painted.origin.x, self.caret_x(), "the painted caret");
    }

    /// How many animation frames the frames drawn since the last call asked
    /// for.
    fn frames_requested(&mut self) -> usize {
        self.cx.update(|window, cx| window.simulate_next_frame(cx))
    }

    /// Draws frames until the glide ends, at most `ms` later, and returns
    /// them, the current one first.
    fn run(&mut self, ms: u64) -> Vec<Sample> {
        let mut samples = self.sample().into_iter().collect::<Vec<_>>();
        let mut elapsed = 0;
        while self.sample().is_some() && elapsed < ms {
            self.advance(FRAME_MS);
            elapsed += FRAME_MS;
            samples.extend(self.sample());
        }
        samples
    }
}

impl Glider<TextareaMode> {
    fn new(cx: &mut TestAppContext, marked: &str) -> Self {
        Self::with(cx, px(400.), marked, |window, cx| {
            crate::input::TextareaState::new(window, cx).smooth_caret(true)
        })
    }
}

impl Glider<InputMode> {
    fn line(cx: &mut TestAppContext, width: Pixels, marked: &str, align: TextAlign) -> Self {
        let mut glider = Self::with(cx, width, marked, |window, cx| {
            crate::input::InputState::new(window, cx).smooth_caret(true)
        });
        glider.update(|state, _, cx| state.set_text_align(align, cx));
        glider
    }
}

#[track_caller]
fn assert_close(actual: Pixels, expected: Pixels, what: &str) {
    assert!(
        (actual - expected).abs() <= px(0.5),
        "{what}: {actual:?}, expected {expected:?}"
    );
}

/// The reveal and the text after the caret, as one frame plans them for a
/// caret gliding right over typed text.
#[track_caller]
fn assert_uncovering(sample: &Sample) {
    let plan = &sample.plan;
    // Typed text is drawn up to the caret and no further.
    assert_eq!(plan.head_end, plan.caret_x.min(plan.target_x));
    // The text after the caret starts there, moved left by exactly how far
    // the caret is behind.
    assert_eq!(plan.tail_start, plan.head_end);
    let lag = plan.target_x - px(sample.x).min(plan.target_x);
    assert!(
        (plan.tail_shift + lag).abs() < px(1e-3),
        "moved {:?} with the caret {lag:?} behind",
        plan.tail_shift
    );
    assert!(plan.ghosts.is_empty());
}

/// The deleted text and the text after the caret, as one frame plans them
/// for a caret gliding left over `deleted` pixels of deleted text.
#[track_caller]
fn assert_covering(sample: &Sample, deleted: Pixels) {
    let plan = &sample.plan;
    let end = plan.target_x + deleted;
    let shown = plan
        .ghosts
        .last()
        .expect("deleted text is shown")
        .visible
        .end;
    // Deleted text is drawn from the target up to the caret.
    assert_eq!(plan.ghosts[0].visible.start, plan.target_x);
    assert_eq!(shown, plan.caret_x.max(plan.target_x).min(end));
    // The text after the caret starts there, moved right by exactly how far
    // the caret is from the target.
    assert_eq!(plan.tail_start, shown);
    let ahead = px(sample.x).max(plan.target_x).min(end) - plan.target_x;
    assert!(
        (plan.tail_shift - ahead).abs() < px(1e-3),
        "moved {:?} with the caret {ahead:?} ahead",
        plan.tail_shift
    );
    assert_eq!(plan.head_end, plan.target_x);
}

#[gpui::test]
fn a_typed_character_is_uncovered_by_the_caret(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    let before = glider.caret_x();
    glider.type_text("W");
    assert_eq!(glider.value(), "abWcd");

    // The keystroke's frame: the caret has not moved yet, so nothing of the
    // new character is drawn, and "cd" is drawn where it was.
    let first = glider.gliding();
    let target = first.plan.target_x;
    assert_close(target, glider.caret_x(), "target");
    assert!(target > before + px(1.));
    assert_close(first.plan.caret_x, before, "caret at the keystroke");
    assert_eq!(first.plan.head_end, first.plan.caret_x);
    assert_eq!(first.plan.tail_start, first.plan.caret_x);

    let typing = typing_ms();
    let mut previous = first;
    let mut middle = None;
    for step in 1..=typing / FRAME_MS {
        glider.advance(FRAME_MS);
        let Some(sample) = glider.sample() else {
            assert!(
                step * FRAME_MS > typing * 3 / 4,
                "settled too early, at {} ms",
                step * FRAME_MS
            );
            break;
        };
        assert_uncovering(&sample);
        assert!(sample.x > previous.x, "the caret glides right");
        // Ease out: fastest at the start, slowing as it arrives.
        assert!(sample.velocity < previous.velocity);
        if step * FRAME_MS == typing / 2 {
            middle = Some(sample.clone());
        }
        previous = sample;
    }
    let middle = middle.expect("still gliding halfway");
    assert!(
        middle.plan.caret_x > before + px(0.5) && middle.plan.caret_x < target - px(0.5),
        "halfway the caret is between where it was and where it goes: {:?}",
        middle.plan.caret_x
    );

    // After the settle time everything is drawn as without the smooth caret.
    glider.assert_still(&format!("after {typing} ms"));
    glider.assert_drawn_at_caret();
}

#[gpui::test]
fn typing_and_backspace_settle_in_100_ms_by_default(cx: &mut TestAppContext) {
    assert_eq!(
        SmoothCaretOptions::default().typing(),
        Duration::from_millis(100)
    );
    let mut glider = Glider::new(cx, "abˇcd");
    glider.type_text("W");
    glider.advance(50);
    assert!(glider.sample().is_some(), "typing: gliding at 50 ms");
    glider.advance(50);
    glider.assert_still("100 ms after typing");

    glider.keys("backspace");
    assert_eq!(glider.value(), "abcd");
    glider.advance(50);
    assert!(glider.sample().is_some(), "Backspace: gliding at 50 ms");
    glider.advance(50);
    glider.assert_still("100 ms after Backspace");
}

#[gpui::test]
fn a_zero_typing_duration_types_at_once(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    glider.options(SmoothCaretOptions::default().typing_duration(Duration::ZERO));
    glider.type_text("W");
    glider.assert_still("typing");
    glider.keys("backspace");
    glider.assert_still("Backspace");
    glider.keys("left");
    assert!(glider.sample().is_some(), "the arrow keys still glide");
}

#[gpui::test]
fn rapid_typing_aims_the_glide_anew(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "ˇ");
    glider.type_text("a");
    glider.advance(20);
    let before = glider.gliding();
    glider.type_text("b");
    let after = glider.gliding();
    // The glide keeps its place and speeds toward the further target rather
    // than starting over from rest.
    assert!((after.x - before.x).abs() < 1e-3);
    assert!(after.plan.target_x > before.plan.target_x);
    assert!(after.velocity > 0.);
    assert_uncovering(&after);
}

#[gpui::test]
fn backspace_covers_the_deleted_character(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abcˇde");
    let before = glider.caret_x();
    glider.keys("backspace");
    assert_eq!(glider.value(), "abde");
    let deleted = before - glider.caret_x();
    assert!(deleted > px(1.));

    let first = glider.gliding();
    assert_close(first.plan.caret_x, before, "caret at the keystroke");
    let mut previous: Option<Sample> = None;
    for sample in glider.run(300) {
        // The deleted "c" is drawn from the target up to the caret, and "de"
        // starts there.
        assert_covering(&sample, deleted);
        if let Some(previous) = previous {
            assert!(sample.x < previous.x, "the caret glides left");
        }
        previous = Some(sample);
    }
    glider.assert_still("after the glide");

    // Rapid Backspace keeps covering, glyph after glyph.
    glider.keys("backspace");
    glider.advance(FRAME_MS * 4);
    glider.keys("backspace");
    assert_eq!(glider.value(), "de");
    let sample = glider.gliding();
    assert_eq!(sample.plan.ghosts.len(), 2);
    assert_covering(&sample, deleted * 2.);
}

#[gpui::test]
fn backspace_can_be_instant(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abcˇde");
    glider.options(SmoothCaretOptions::default().backspace(false));
    glider.keys("backspace");
    glider.assert_still("Backspace with its glide off");
    glider.type_text("x");
    assert!(glider.sample().is_some(), "typing still glides");
}

#[gpui::test]
fn forward_delete_is_instant(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    glider.keys("delete");
    assert_eq!(glider.value(), "abd");
    glider.assert_still("Delete");
}

#[gpui::test]
fn the_arrow_keys_glide_the_caret_alone(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    let before = glider.caret_x();
    glider.keys("right");
    let sample = glider.gliding();
    assert!(!sample.plan.splits_row(), "nothing is hidden or moved");
    assert_close(sample.plan.target_x, glider.caret_x(), "target");
    assert!(sample.plan.target_x > before);
    glider.advance(40);
    let sample = glider.gliding();
    assert!(sample.plan.caret_x > before && sample.plan.caret_x < sample.plan.target_x);
    glider.advance(40);
    glider.assert_still("80 ms after Right");

    glider.keys("left");
    assert!(!glider.gliding().plan.splits_row());
    glider.advance(80);
    glider.assert_still("80 ms after Left");

    glider.options(SmoothCaretOptions::default().navigation(false));
    glider.keys("right");
    glider.assert_still("Right with navigation off");
}

#[gpui::test]
fn an_arrow_key_during_typing_aims_the_glide_anew(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    glider.type_text("W");
    glider.advance(20);
    let typing = glider.gliding();
    glider.keys("left");
    let moving = glider.gliding();
    // Hidden text settles at once: only typing uncovers.
    assert!(!moving.plan.splits_row());
    assert!((moving.x - typing.x).abs() < 1e-3);
    assert_close(moving.plan.target_x, glider.caret_x(), "target");
}

#[gpui::test]
fn large_and_vertical_moves_are_instant_and_end_a_glide(cx: &mut TestAppContext) {
    let instant: [(&str, fn(&mut Glider<TextareaMode>)); 9] = [
        ("up", |g| g.keys("up")),
        ("down", |g| g.keys("down")),
        ("home", |g| g.keys("home")),
        ("end", |g| g.keys("end")),
        ("enter", |g| g.keys("enter")),
        ("a selection", |g| g.keys("shift-left")),
        ("select all", |g| g.dispatch(SelectAll)),
        ("undo", |g| g.dispatch(Undo)),
        ("redo", |g| {
            g.dispatch(Undo);
            g.dispatch(Redo);
        }),
    ];
    for (what, act) in instant {
        let mut glider = Glider::new(cx, "one two\nthree fourˇ five\nsix seven");
        glider.type_text("s");
        glider.advance(FRAME_MS);
        assert!(glider.sample().is_some(), "{what}: gliding before");
        act(&mut glider);
        glider.assert_still(what);
        assert!(glider.frames_requested() > 0, "{what}: the glide asked");
        glider.advance(FRAME_MS);
        assert_eq!(glider.frames_requested(), 0, "{what}: no more frames");
    }
}

#[gpui::test]
fn a_paste_is_instant(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    glider
        .cx
        .write_to_clipboard(gpui::ClipboardItem::new_string("x".into()));
    glider.dispatch(Paste);
    assert_eq!(glider.value(), "abxcd");
    glider.assert_still("a one-character paste");
}

#[gpui::test]
fn input_method_composition_is_instant(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler as _;
    let mut glider = Glider::new(cx, "abˇ");
    glider.update(|state, window, cx| {
        state.replace_and_mark_text_in_range(None, "n", None, window, cx)
    });
    glider.assert_still("a composition");
    glider.update(|state, window, cx| state.replace_text_in_range(None, "你", window, cx));
    glider.assert_still("its commit");
}

#[gpui::test]
fn frames_stop_once_the_caret_arrives(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    glider.type_text("W");
    assert!(glider.frames_requested() > 0);
    while glider.sample().is_some() {
        glider.advance(FRAME_MS);
        if glider.sample().is_some() {
            assert!(
                glider.frames_requested() > 0,
                "a gliding frame asks for the next"
            );
        }
    }
    glider.frames_requested();
    glider.advance(FRAME_MS);
    assert_eq!(
        glider.frames_requested(),
        0,
        "a settled caret asks for no frame"
    );
}

#[gpui::test]
fn the_caret_is_solid_while_it_glides(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    glider.type_text("W");
    let plan = glider.gliding().plan;
    let painted = glider
        .painted_caret()
        .expect("the gliding caret is painted");
    assert_eq!(painted.origin.x, plan.caret_x);
    // A blink in the middle of a glide does not hide it.
    glider.advance(60);
    if let Some(sample) = glider.sample() {
        assert_eq!(
            glider.painted_caret().expect("still painted").origin.x,
            sample.plan.caret_x
        );
    }
}

#[gpui::test]
fn reduced_motion_moves_the_caret_at_once(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    glider.cx.update(|_, cx| cx.set_reduce_motion(true));
    glider.type_text("W");
    glider.assert_still("typing");
    for key in ["backspace", "left"] {
        glider.keys(key);
        glider.assert_still(key);
    }
    assert_eq!(glider.frames_requested(), 0);

    glider.cx.update(|_, cx| cx.set_reduce_motion(false));
    glider.type_text("W");
    assert!(glider.sample().is_some());
}

#[gpui::test]
fn the_smooth_caret_is_off_by_default(cx: &mut TestAppContext) {
    let mut glider = Glider::with(cx, px(400.), "abˇcd", |window, cx| {
        crate::input::TextareaState::new(window, cx)
    });
    assert!(
        !glider
            .state
            .read_with(&glider.cx, |state, _| state.has_smooth_caret())
    );
    glider.type_text("W");
    glider.keys("backspace left");
    glider.assert_still("off by default");
}

#[gpui::test]
fn turning_the_smooth_caret_off_ends_a_glide(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    glider.type_text("W");
    assert!(glider.sample().is_some());
    glider.update(|state, _, cx| state.set_smooth_caret(false, cx));
    glider.assert_still("smooth caret turned off");
    glider.assert_drawn_at_caret();
}

#[gpui::test]
fn right_to_left_text_moves_the_caret_at_once(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "שלוםˇ");
    glider.type_text("ש");
    glider.assert_still("Hebrew");
    glider.keys("left");
    glider.assert_still("Hebrew");
}

#[gpui::test]
fn masked_text_moves_the_caret_at_once(cx: &mut TestAppContext) {
    let mut glider = Glider::with(cx, px(400.), "abˇ", |window, cx| {
        crate::input::InputState::new(window, cx)
            .masked(true)
            .smooth_caret(true)
    });
    glider.type_text("c");
    glider.assert_still("a password");
}

#[gpui::test]
fn movements_made_within_one_frame_join(cx: &mut TestAppContext) {
    let mut glider = Glider::new(cx, "abˇcd");
    let before = glider.caret_x();
    // Two characters typed before the next frame are uncovered together.
    glider.cx.simulate_input("xy");
    glider.draw();
    let sample = glider.gliding();
    assert_close(sample.plan.caret_x, before, "from where it was");
    assert_close(sample.plan.target_x, glider.caret_x(), "to both");
    assert_uncovering(&sample);

    // Typing and then a click before the next frame: instant.
    glider.advance(300);
    glider.cx.simulate_input("z");
    glider.update(|state, _, cx| state.set_selected_range(0..0, cx));
    glider.assert_still("typing then a click");
}

/// A single-line input glides its caret and uncovers typed text as a
/// textarea does, and the arrow keys and Backspace glide too.
#[gpui::test]
fn a_single_line_input_glides_too(cx: &mut TestAppContext) {
    let mut glider = Glider::line(cx, px(400.), "abcˇ", TextAlign::Left);
    let before = glider.caret_x();
    glider.type_text("W");
    let first = glider.gliding();
    assert_close(
        first.plan.target_x,
        glider.caret_x(),
        "it glides to the caret",
    );
    assert_close(first.plan.caret_x, before, "from where it was");
    for sample in glider.run(300) {
        assert_uncovering(&sample);
    }
    glider.assert_still("it settles");
    glider.assert_drawn_at_caret();

    glider.keys("left");
    assert!(glider.sample().is_some(), "the arrow keys glide");
    glider.advance(300);
    glider.keys("backspace");
    let sample = glider.gliding();
    assert!(
        !sample.plan.ghosts.is_empty(),
        "the deleted glyph is still drawn"
    );
}

/// Typing at the right edge of a narrow input scrolls the text, and the
/// caret glides in the scrolled text to where it is.
#[gpui::test]
fn a_scrolled_single_line_input_glides_to_the_caret(cx: &mut TestAppContext) {
    let mut glider = Glider::line(
        cx,
        px(120.),
        "a long line of text that does not fitˇ",
        TextAlign::Left,
    );
    for key in ["x", "y", "z"] {
        glider.type_text(key);
        let first = glider.gliding();
        assert_close(
            first.plan.target_x,
            glider.caret_x(),
            "it glides to the caret",
        );
        assert!(glider.caret_x() < px(130.), "the caret stays in view");
        glider.run(300);
        glider.assert_still("it settles");
    }
}

/// In right- and center-aligned text, typing moves the text before the caret
/// too, which no glide explains, so the caret is drawn where it is at once.
/// The arrow keys glide it along the aligned text.
#[gpui::test]
fn aligned_single_line_inputs_glide_to_the_caret(cx: &mut TestAppContext) {
    for align in [TextAlign::Right, TextAlign::Center] {
        let mut glider = Glider::line(cx, px(300.), "abcdefˇ", align);
        glider.type_text("W");
        glider.assert_still(&format!("{align:?}: typing is drawn at once"));
        glider.assert_drawn_at_caret();

        for key in ["left", "left", "right"] {
            let before = glider.caret_x();
            glider.keys(key);
            let target = glider.caret_x();
            let first = glider.gliding();
            assert_close(first.plan.target_x, target, "it glides to the caret");
            assert_close(first.plan.caret_x, before, "from where it was");
            glider.advance(300);
            glider.assert_still(&format!("{align:?} {key}"));
            glider.assert_drawn_at_caret();
        }
    }
}

#[gpui::test]
fn the_code_editor_glides_too(cx: &mut TestAppContext) {
    let mut glider = Glider::<EditorMode>::with(cx, px(400.), "fn mainˇ", |window, cx| {
        crate::input::EditorState::new(window, cx).smooth_caret(true)
    });
    glider.type_text("x");
    assert_uncovering(&glider.gliding());
    glider.run(300);
    glider.assert_still("it settles");
}

/// Whether `state`'s caret glides.
fn glides<M: InputModeKind>(state: &Entity<InputBaseState<M>>, cx: &VisualTestContext) -> bool {
    state.read_with(cx, |state, _| state.has_smooth_caret())
}

/// An input that doesn't choose for itself follows the application's smooth
/// caret as it changes, and one made later starts with it.
#[gpui::test]
fn inputs_follow_the_app_smooth_caret(cx: &mut TestAppContext) {
    let mut glider = Glider::with(cx, px(400.), "abˇcd", |window, cx| {
        crate::input::TextareaState::new(window, cx)
    });
    assert_eq!(glider.cx.update(|_, cx| app_smooth_caret(cx)), None);
    assert!(!glides(&glider.state, &glider.cx));

    glider.cx.update(|_, cx| set_app_smooth_caret(true, cx));
    glider.draw();
    assert_eq!(glider.cx.update(|_, cx| app_smooth_caret(cx)), Some(true));
    assert!(glides(&glider.state, &glider.cx));
    glider.type_text("W");
    assert!(glider.sample().is_some(), "typing glides");
    let later = glider
        .cx
        .update(|window, cx| cx.new(|cx| crate::input::InputState::new(window, cx)));
    assert!(glides(&later, &glider.cx));

    // Turned off, the glide in progress ends.
    glider.cx.update(|_, cx| set_app_smooth_caret(false, cx));
    glider.draw();
    assert!(!glides(&glider.state, &glider.cx));
    assert!(!glides(&later, &glider.cx));
    glider.assert_still("turned off for the app");
}

/// An input's own choice wins over the application's, whichever way either
/// goes.
#[gpui::test]
fn an_input_choice_wins_over_the_app_smooth_caret(cx: &mut TestAppContext) {
    let mut glider = Glider::with(cx, px(400.), "abˇcd", |window, cx| {
        crate::input::TextareaState::new(window, cx).smooth_caret(false)
    });
    glider.cx.update(|_, cx| set_app_smooth_caret(true, cx));
    glider.draw();
    assert!(!glides(&glider.state, &glider.cx));

    let chosen = glider.cx.update(|window, cx| {
        cx.new(|cx| crate::input::InputState::new(window, cx).smooth_caret(true))
    });
    glider.cx.update(|_, cx| set_app_smooth_caret(false, cx));
    glider.draw();
    assert!(glides(&chosen, &glider.cx));
    assert!(!glides(&glider.state, &glider.cx));
}

#[test]
fn a_change_the_steps_do_not_explain_is_instant() {
    let stamp = |revision, caret, text_len| CaretStamp {
        revision,
        caret,
        simple: true,
        text_len,
    };
    let typed = Step {
        motion: CaretMotion::Typing,
        before: stamp(0, 2, 4),
        after: stamp(1, 3, 5),
        edits: vec![(2..2, 1)],
    };
    assert_eq!(
        classify(stamp(0, 2, 4), stamp(1, 3, 5), std::slice::from_ref(&typed)),
        Change::Insert { at: 2, caret: 3 }
    );
    // Something moved the caret after the typing.
    assert_eq!(
        classify(stamp(0, 2, 4), stamp(1, 0, 5), std::slice::from_ref(&typed)),
        Change::Instant
    );
    // Something edited before the typing.
    assert_eq!(
        classify(stamp(0, 2, 3), stamp(1, 3, 5), std::slice::from_ref(&typed)),
        Change::Instant
    );
    // Typing that replaced text, as an input method's commit does.
    let replaced = Step {
        edits: vec![(1..2, 2)],
        ..typed.clone()
    };
    assert_eq!(
        classify(stamp(0, 2, 4), stamp(1, 3, 5), &[replaced]),
        Change::Instant
    );
    // An auto-closed pair: the caret lands between the two.
    let pair = Step {
        after: stamp(1, 3, 6),
        edits: vec![(2..2, 2)],
        ..typed
    };
    assert_eq!(
        classify(stamp(0, 2, 4), stamp(1, 3, 6), &[pair]),
        Change::Insert { at: 2, caret: 3 }
    );
    // A selection is never glided from.
    let selected = CaretStamp {
        simple: false,
        ..stamp(0, 2, 4)
    };
    let moved = Step {
        motion: CaretMotion::Grapheme,
        before: selected,
        after: stamp(0, 4, 4),
        edits: Vec::new(),
    };
    assert_eq!(
        classify(selected, stamp(0, 4, 4), &[moved]),
        Change::Instant
    );
}

#[test]
fn a_glide_aimed_anew_keeps_its_position_and_velocity() {
    let start = Instant::now();
    let glide = Glide::from_rest(start, 0., 10., Duration::from_millis(200), 12.).unwrap();
    let (x0, v0) = glide.at(start);
    assert_eq!(x0, 0.);
    // Ease out: fastest at the start.
    assert!(v0 > glide.at(start + Duration::from_millis(10)).1);

    let later = start + Duration::from_millis(30);
    let (x, v) = glide.at(later);
    let aimed = glide.retarget(later, 20., Duration::from_millis(200), 12.);
    let (x1, v1) = aimed.at(later);
    assert!((x1 - x).abs() < 1e-4 && (v1 - v).abs() < 1e-3);

    // It never passes its target, and it is there after the duration.
    let mut previous = x1;
    for ms in (35..=230).step_by(5) {
        let (x, _) = aimed.at(start + Duration::from_millis(ms));
        assert!(x >= previous - 1e-4 && x <= 20. + 1e-4, "{ms} ms: {x}");
        previous = x;
    }
    assert!(20. - aimed.at(start + Duration::from_millis(230)).0 <= SETTLED);
}

#[test]
fn a_glide_turned_around_comes_back_without_passing_its_target() {
    let start = Instant::now();
    let glide = Glide::from_rest(start, 0., 30., Duration::from_millis(200), 15.).unwrap();
    let later = start + Duration::from_millis(20);
    let (x, v) = glide.at(later);
    assert!(v > 0.);
    // Aimed behind itself while it moves right.
    let back = glide.retarget(later, x - 5., Duration::from_millis(200), 12.);
    let mut furthest = x;
    for ms in (20..=220).step_by(2) {
        let (position, _) = back.at(start + Duration::from_millis(ms));
        furthest = furthest.max(position);
        assert!(
            position >= x - 5. - 1e-3,
            "{ms} ms: passed the target at {position}"
        );
    }
    assert!(furthest > x, "it keeps its velocity and turns around");
    assert!((back.at(start + Duration::from_millis(220)).0 - (x - 5.)).abs() <= SETTLED);
}
