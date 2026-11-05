//! A textarea's send keys, pressed through the platform's key bindings.
use gpui_kit::{
    AppContext, Context, ElementId, Entity, Subscription, TestAppContext, Window, WindowHandle,
    component::input::{InputEvent, SubmitKey, Textarea, TextareaState},
    div,
    prelude::*,
    px, size,
    test::TestWindowExt,
};

use crate::common;

struct Composer {
    text: Entity<TextareaState>,
    sent: Vec<String>,
    _subscription: Subscription,
}

impl Render for Composer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child(Textarea::new(&self.text).w_full())
    }
}

fn composer(
    cx: &mut TestAppContext,
    keys: &'static [SubmitKey],
) -> (
    WindowHandle<gpui_kit::base::Root>,
    Entity<Composer>,
    Entity<TextareaState>,
) {
    let (window, view) = common::open_window(cx, Some(size(px(480.), px(320.))), |window, cx| {
        cx.new(|cx| {
            let text =
                cx.new(|cx| TextareaState::new(window, cx).submit_keys(keys.iter().copied()));
            // A composer sends only on the chord it was told submits.
            let subscription = cx.subscribe(&text, |this: &mut Composer, text, event, cx| {
                if let InputEvent::PressEnter { secondary, shift } = event
                    && text.read(cx).is_submit_chord(*secondary, *shift)
                {
                    this.sent.push(text.read(cx).value().to_string());
                }
            });
            Composer {
                text,
                sent: Vec::new(),
                _subscription: subscription,
            }
        })
    });
    let text = cx.update(|cx| view.read(cx).text.clone());
    (window, view, text)
}

fn target(text: &Entity<TextareaState>) -> ElementId {
    ("input", text.entity_id()).into()
}

#[gpui_kit::test]
fn each_send_key_submits_and_the_other_chords_break_lines(cx: &mut TestAppContext) {
    use SubmitKey::{Enter, SecondaryEnter, ShiftEnter};

    // Once: each init binds the keys again, and a submit that propagates
    // would reach the second binding.
    cx.update(gpui_kit::init);

    let chords = [
        ("enter", Enter),
        ("shift-enter", ShiftEnter),
        ("secondary-enter", SecondaryEnter),
    ];
    let sets: [&'static [SubmitKey]; 5] = [
        &[],
        &[Enter],
        &[ShiftEnter, SecondaryEnter],
        &[SecondaryEnter],
        &[Enter, ShiftEnter, SecondaryEnter],
    ];
    for keys in sets {
        let (handle, owner, text) = composer(cx, keys);
        for (keystroke, chord) in chords {
            cx.update_window(handle.into(), |_, window, cx| {
                window.click(target(&text), cx);
                text.update(cx, |state, cx| {
                    state.set_value("hi", window, cx);
                    state.set_selected_range(2..2, cx);
                });
                window.press(keystroke, cx);
                let expected = if keys.contains(&chord) { "hi" } else { "hi\n" };
                assert_eq!(text.read(cx).value(), expected, "{keys:?} {keystroke}");
            })
            .unwrap();
            cx.run_until_parked();
        }
        let sent = cx.update(|cx| owner.read(cx).sent.clone());
        assert_eq!(sent.len(), keys.len(), "{keys:?} {sent:?}");
    }
}
