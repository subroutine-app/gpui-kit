use super::*;

use gpui::{
    Capslock, Modifiers, ModifiersChangedEvent, PlatformInput, TestAppContext, VisualTestContext,
    point,
};

gpui::actions!(app_menu_bar_tests, [SaveDoc]);

#[test]
fn access_keys_take_each_names_first_free_letter() {
    let keys: Vec<_> = mnemonics(["File", "Edit", "Format", "Fx", "Ünicode", "123", "--"])
        .into_iter()
        .map(|mnemonic| mnemonic.map(|m| (m.offset, m.key)))
        .collect();
    assert_eq!(
        keys,
        vec![
            Some((0, 'f')),
            Some((0, 'e')),
            Some((1, 'o')),
            Some((1, 'x')),
            Some((0, 'ü')),
            Some((0, '1')),
            None
        ]
    );
}

#[test]
fn only_alt_with_a_single_key_is_an_access_key() {
    let letter = |keystroke| alt_letter(&Keystroke::parse(keystroke).unwrap());
    assert_eq!(letter("alt-e"), Some('e'));
    assert_eq!(letter("alt-shift-e"), Some('e'));
    assert_eq!(letter("ctrl-alt-e"), None);
    assert_eq!(letter("e"), None);
    assert_eq!(letter("alt-f4"), None);
}

/// A document under a menu bar, inside the key context its shortcuts are
/// bound in, the way an application lays out its window.
struct DocRoot {
    menu_bar: Entity<AppMenuBar>,
    doc: FocusHandle,
}

impl Render for DocRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("Doc")
            .on_action(|_: &SaveDoc, _, _| {})
            .size_full()
            .child(div().h(px(30.)).child(self.menu_bar.clone()))
            .child(div().id("doc").track_focus(&self.doc).size_full())
    }
}

fn set_menus(cx: &mut App) {
    crate::init(cx);
    cx.bind_keys([KeyBinding::new("ctrl-alt-shift-s", SaveDoc, Some("Doc"))]);
    GlobalState::global_mut(cx).set_app_menus(vec![
        gpui::Menu::new("File")
            .items([gpui::MenuItem::action("Save a Copy", SaveDoc)])
            .owned(),
        gpui::Menu::new("Edit")
            .items([gpui::MenuItem::action("Undo", SaveDoc)])
            .owned(),
    ]);
}

/// A menu opens complete: its first frame draws the shortcuts beside the
/// items, whether or not the document, where they are bound, has focus.
#[gpui::test]
fn a_menu_opens_with_its_shortcuts(cx: &mut TestAppContext) {
    cx.update(set_menus);
    for focused in [true, false] {
        let (root, cx) = cx.add_window_view(|window, cx| {
            let doc = cx.focus_handle();
            if focused {
                doc.focus(window, cx);
            }
            DocRoot {
                menu_bar: AppMenuBar::new(cx),
                doc,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_mouse_down(
            point(px(10.), px(10.)),
            MouseButton::Left,
            Default::default(),
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let open = root.read_with(cx, |root, cx| root.menu_bar.read(cx).open_index());
        assert_eq!(open, Some(0), "focused: {focused}");
        assert!(
            cx.debug_bounds("kbd:ctrl-alt-shift-s").is_some(),
            "focused: {focused}, the first frame shows the shortcut"
        );
    }
}

/// A window with File and Edit menus under the window's root, active, with
/// the document focused or nothing focused.
fn open_with_menus(
    focused: bool,
    cx: &mut TestAppContext,
) -> (Entity<AppMenuBar>, FocusHandle, &mut VisualTestContext) {
    cx.update(|cx| {
        set_menus(cx);
        bind_window_keys(cx);
    });
    let menu_bar = cx.update(AppMenuBar::new);
    let mut doc = None;
    let (_, cx) = cx.add_window_view({
        let menu_bar = menu_bar.clone();
        |window, cx| {
            let handle = cx.focus_handle();
            if focused {
                handle.focus(window, cx);
            }
            doc = Some(handle.clone());
            let view = cx.new(|_| DocRoot {
                menu_bar,
                doc: handle,
            });
            crate::Root::new(view, window, cx)
        }
    });
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    settle_after_focus(cx);
    assert_eq!(
        cx.update(|window, cx| window.focused(cx).is_some()),
        focused
    );
    (menu_bar, doc.unwrap(), cx)
}

/// Let the moment after the window gained focus pass, in which an Alt
/// pressed counts as held from before.
fn settle_after_focus(cx: &mut VisualTestContext) {
    cx.background_executor
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn press(cx: &mut VisualTestContext, keystrokes: &str) {
    cx.simulate_keystrokes(keystrokes);
    draw(cx);
}

fn hold(cx: &mut VisualTestContext, modifiers: Modifiers) {
    cx.update(|window, cx| {
        window.dispatch_event(
            PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                modifiers,
                capslock: Capslock { on: false },
            }),
            cx,
        );
    });
}

/// Alt down and up with nothing between: GPUI turns the release into the
/// lone `alt` keystroke.
fn tap_alt(cx: &mut VisualTestContext) {
    hold(cx, Modifiers::alt());
    hold(cx, Modifiers::default());
    draw(cx);
}

fn state(bar: &Entity<AppMenuBar>, cx: &mut VisualTestContext) -> (Option<usize>, Option<usize>) {
    bar.read_with(cx, |bar, _| (bar.highlighted_index(), bar.open_index()))
}

fn is_focused(handle: &FocusHandle, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, _| handle.is_focused(window))
}

fn nothing_focused(cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| window.focused(cx).is_none())
}

/// F10 or a tap of Alt focuses the first menu's title, Left and Right move
/// between titles, Down opens one, Escape goes back to the document, and Alt
/// with a menu's letter opens that menu.
#[gpui::test]
fn the_menu_bar_is_reachable_from_the_keyboard(cx: &mut TestAppContext) {
    let (bar, doc, cx) = open_with_menus(true, cx);
    let bar_focus = bar.read_with(cx, |bar, _| bar.focus_handle.clone());

    press(cx, "f10");
    assert!(is_focused(&bar_focus, cx));
    assert_eq!(state(&bar, cx), (Some(0), None));
    press(cx, "right");
    assert_eq!(state(&bar, cx), (Some(1), None));
    press(cx, "right");
    assert_eq!(state(&bar, cx), (Some(0), None), "Right wraps around");
    press(cx, "left");
    assert_eq!(state(&bar, cx), (Some(1), None), "and so does Left");
    press(cx, "down");
    assert_eq!(state(&bar, cx), (None, Some(1)), "Down opens the menu");
    press(cx, "escape");
    assert_eq!(state(&bar, cx), (None, None));
    assert!(is_focused(&doc, cx), "focus goes back");

    tap_alt(cx);
    assert_eq!(state(&bar, cx), (Some(0), None), "a tap of Alt is F10");
    tap_alt(cx);
    assert_eq!(state(&bar, cx), (None, None));
    assert!(is_focused(&doc, cx));

    press(cx, "alt-e");
    assert_eq!(state(&bar, cx), (None, Some(1)), "Alt+E opens Edit");
    press(cx, "escape");
    assert!(is_focused(&doc, cx));

    // With a title focused, its letter alone opens the menu.
    press(cx, "f10 e");
    assert_eq!(state(&bar, cx), (None, Some(1)));
    press(cx, "escape");
    assert!(is_focused(&doc, cx));
}

/// With nothing focused, as in a window not yet clicked, the keys reach the
/// bar too, and leaving the bar leaves nothing focused again.
#[gpui::test]
fn the_menu_bar_is_reachable_with_nothing_focused(cx: &mut TestAppContext) {
    let (bar, _, cx) = open_with_menus(false, cx);

    press(cx, "f10");
    assert_eq!(state(&bar, cx), (Some(0), None));
    press(cx, "escape");
    assert_eq!(state(&bar, cx), (None, None));
    assert!(nothing_focused(cx), "focus goes back to nothing");

    tap_alt(cx);
    assert_eq!(state(&bar, cx), (Some(0), None));
    tap_alt(cx);
    assert_eq!(state(&bar, cx), (None, None));
    assert!(nothing_focused(cx));

    press(cx, "alt-e");
    assert_eq!(state(&bar, cx), (None, Some(1)));
    press(cx, "escape");
    assert_eq!(state(&bar, cx), (None, None));
    assert!(nothing_focused(cx));
}

/// Alt with the pointer (Alt+click, Alt+wheel) is no tap.
#[gpui::test]
fn alt_with_the_pointer_is_no_tap(cx: &mut TestAppContext) {
    let (bar, doc, cx) = open_with_menus(true, cx);
    let position = point(px(100.), px(100.));

    hold(cx, Modifiers::alt());
    cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::alt());
    cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::alt());
    hold(cx, Modifiers::default());
    draw(cx);
    assert_eq!(state(&bar, cx), (None, None));
    assert!(is_focused(&doc, cx));

    hold(cx, Modifiers::alt());
    cx.simulate_event(gpui::ScrollEvent {
        position,
        modifiers: Modifiers::alt(),
        ..Default::default()
    });
    hold(cx, Modifiers::default());
    draw(cx);
    assert_eq!(state(&bar, cx), (None, None));

    // A drag begun before Alt and let go with it held is no tap either.
    cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
    hold(cx, Modifiers::alt());
    cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::alt());
    hold(cx, Modifiers::default());
    draw(cx);
    assert_eq!(state(&bar, cx), (None, None));

    tap_alt(cx);
    assert_eq!(state(&bar, cx), (Some(0), None), "a plain tap still is one");
}

/// Alt held across a change of window focus (Alt+Tab away and back, or a
/// tiling compositor's Alt+key) is no tap; a tap a moment later is one.
#[gpui::test]
fn alt_across_a_focus_change_is_no_tap(cx: &mut TestAppContext) {
    let (bar, doc, cx) = open_with_menus(true, cx);
    hold(cx, Modifiers::alt());
    cx.deactivate_window();
    cx.update(|window, _| window.activate_window());
    hold(cx, Modifiers::default());
    draw(cx);
    assert_eq!(state(&bar, cx), (None, None));
    assert!(is_focused(&doc, cx));

    // Gaining focus with Alt already down.
    cx.deactivate_window();
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    tap_alt(cx);
    assert_eq!(state(&bar, cx), (None, None));

    settle_after_focus(cx);
    tap_alt(cx);
    assert_eq!(state(&bar, cx), (Some(0), None));
}

/// While a dialog is open, F10, Alt and Alt with a letter don't reach the
/// menu bar behind it.
#[gpui::test]
fn the_menu_bar_keys_stop_at_a_dialog(cx: &mut TestAppContext) {
    use crate::WindowExt as _;

    let (bar, _, cx) = open_with_menus(true, cx);
    cx.update(|window, cx| window.open_dialog(cx, |dialog, _, _| dialog.title("Delete?")));
    draw(cx);
    press(cx, "f10");
    tap_alt(cx);
    press(cx, "alt-f");
    assert_eq!(state(&bar, cx), (None, None));
    hold(cx, Modifiers::alt());
    assert!(!bar.read_with(cx, |bar, _| bar.alt.held));
    hold(cx, Modifiers::default());
}

/// Clicking another control ends keyboard mode there.
#[gpui::test]
fn focus_moving_elsewhere_ends_keyboard_mode(cx: &mut TestAppContext) {
    let (bar, doc, cx) = open_with_menus(false, cx);
    press(cx, "f10");
    assert_eq!(state(&bar, cx), (Some(0), None));
    cx.update(|window, cx| doc.focus(window, cx));
    draw(cx);
    assert_eq!(state(&bar, cx), (None, None));

    tap_alt(cx);
    assert_eq!(state(&bar, cx), (Some(0), None));
    tap_alt(cx);
    assert!(is_focused(&doc, cx), "focus goes back to the document");
}

/// Holding Alt alone underlines the access keys, with or without focus, and
/// only in the active window.
#[gpui::test]
fn alt_held_underlines_the_access_keys(cx: &mut TestAppContext) {
    for focused in [false, true] {
        let mut cx = cx.clone();
        let (bar, _, cx) = open_with_menus(focused, &mut cx);
        let held = |cx: &mut VisualTestContext| bar.read_with(cx, |bar, _| bar.alt.held);

        hold(cx, Modifiers::alt());
        assert!(held(cx), "focused: {focused}");
        hold(cx, Modifiers::default());
        assert!(!held(cx), "focused: {focused}");

        hold(
            cx,
            Modifiers {
                alt: true,
                shift: true,
                ..Default::default()
            },
        );
        assert!(!held(cx), "Alt with Shift is no access key");
        hold(cx, Modifiers::default());

        cx.deactivate_window();
        hold(cx, Modifiers::alt());
        assert!(!held(cx), "an inactive window shows none");
        hold(cx, Modifiers::default());
    }
}

#[gpui::test]
fn preserves_action_context_while_switching_menus(cx: &mut TestAppContext) {
    struct TestRoot {
        menu_bar: Entity<AppMenuBar>,
        first_focus: FocusHandle,
        second_focus: FocusHandle,
    }

    impl Render for TestRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .child(div().id("first").track_focus(&self.first_focus))
                .child(div().id("second").track_focus(&self.second_focus))
                .child(self.menu_bar.clone())
        }
    }

    let (root, cx) = cx.add_window_view(|window, cx| {
        let first_focus = cx.focus_handle();
        let second_focus = cx.focus_handle();
        first_focus.focus(window, cx);

        TestRoot {
            menu_bar: cx.new(AppMenuBar::empty),
            first_focus,
            second_focus,
        }
    });

    let (menu_bar, first_focus, second_focus) = root.read_with(cx, |root, _| {
        (
            root.menu_bar.clone(),
            root.first_focus.clone(),
            root.second_focus.clone(),
        )
    });

    menu_bar.update_in(cx, |menu_bar, window, cx| {
        menu_bar.set_selected_index(Some(0), window, cx);
        assert_eq!(menu_bar.action_context.as_ref(), Some(&first_focus));

        second_focus.focus(window, cx);
        menu_bar.set_selected_index(Some(1), window, cx);
        assert_eq!(menu_bar.action_context.as_ref(), Some(&first_focus));

        menu_bar.set_selected_index(None, window, cx);
        assert!(menu_bar.action_context.is_none());
        assert_eq!(window.focused(cx).as_ref(), Some(&first_focus));

        second_focus.focus(window, cx);
        menu_bar.set_selected_index(Some(0), window, cx);
        assert_eq!(menu_bar.action_context.as_ref(), Some(&second_focus));
    });
}
