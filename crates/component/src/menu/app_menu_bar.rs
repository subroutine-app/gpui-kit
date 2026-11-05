use std::collections::HashMap;

use crate::{
    InteractiveElementExt as _, Placement, Selectable, Sizable,
    actions::{Cancel, Confirm, SelectDown, SelectLeft, SelectRight},
    button::{Button, ButtonVariants},
    global_state::GlobalState,
    h_flex,
    menu::PopupMenu,
    root::WindowState,
    scroll::ScrollableMask,
};
use gpui::{
    App, AppContext as _, ClickEvent, Context, DismissEvent, DispatchPhase, Entity, FocusHandle,
    Focusable, Global, HighlightStyle, InteractiveElement as _, IntoElement, KeyBinding,
    KeyDownEvent, Keystroke, MouseButton, MouseDownEvent, MouseUpEvent, OwnedMenu, ParentElement,
    Render, Role, ScrollEvent, ScrollHandle, SharedString, StatefulInteractiveElement, Styled,
    StyledText, Subscription, Task, UnderlineStyle, WeakEntity, Window, WindowId, actions,
    anchored, canvas, deferred, div, prelude::FluentBuilder, px,
};
use gpui_base::TestSupportExt as _;

const CONTEXT: &str = "AppMenuBar";

/// How soon after a window gains focus an Alt pressed counts as held from
/// before, as after Alt+Tab or a tiling compositor's Alt+key.
const ALT_HELD_FROM_BEFORE: std::time::Duration = std::time::Duration::from_millis(250);

actions!(
    app_menu_bar,
    [
        /// Move keyboard focus to the window's menu bar, on its first menu,
        /// or back to where it was. Bound to F10 on Windows and Linux.
        FocusAppMenuBar,
        /// A tap of Alt alone. It does what [`FocusAppMenuBar`] does unless
        /// the pointer was pressed or scrolled while Alt was down, or the
        /// window gained or lost focus meanwhile. Bound to `alt` on Windows
        /// and Linux.
        AltTap
    ]
);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("space", Confirm { secondary: false }, Some(CONTEXT)),
    ]);
    // macOS keeps its menus in the system menu bar.
    if !cfg!(target_os = "macos") {
        bind_window_keys(cx);
    }
}

/// Bind F10 and a lone Alt, and route them and Alt with a menu's letter to
/// the menu bar of the window they were pressed in.
///
/// The menu bar is rarely on the focused element's dispatch path, so these
/// listen app-wide. They run after the focused element's own bindings and key
/// listeners, so a binding of F10, Alt or Alt with a letter keeps working.
fn bind_window_keys(cx: &mut App) {
    if cx.has_global::<WindowMenuBars>() {
        return;
    }
    cx.set_global(WindowMenuBars::default());
    cx.bind_keys([
        KeyBinding::new("f10", FocusAppMenuBar, None),
        KeyBinding::new("alt", AltTap, None),
    ]);

    cx.on_action(|_: &FocusAppMenuBar, cx| {
        after_key(cx, |window, cx| {
            if let Some(bar) = WindowMenuBars::reachable(window, cx) {
                bar.update(cx, |bar, cx| bar.toggle_focus(window, cx));
            }
        });
    });
    cx.on_action(|_: &AltTap, cx| {
        after_key(cx, |window, cx| {
            if let Some(bar) = WindowMenuBars::reachable(window, cx) {
                bar.update(cx, |bar, cx| {
                    if std::mem::take(&mut bar.alt.tap_armed) {
                        bar.toggle_focus(window, cx);
                    }
                });
            }
        });
        // The tap is also Alt let go, which the window's modifier listeners
        // must still hear.
        cx.propagate();
    });
    cx.observe_keystrokes(|event, window, cx| {
        if event.action.is_some() {
            return;
        }
        if let Some(key) = alt_letter(&event.keystroke)
            && let Some(bar) = WindowMenuBars::reachable(window, cx)
        {
            bar.update(cx, |bar, cx| bar.open_menu_with_key(key, window, cx));
        }
    })
    .detach();
}

/// Run `f` in the window a key went to once the key has been handled, for an
/// app-wide action listener, which hears the key without its window.
fn after_key(cx: &mut App, f: impl FnOnce(&mut Window, &mut App) + 'static) {
    let Some(window) = cx.active_window() else {
        return;
    };
    cx.defer(move |cx| {
        _ = window.update(cx, |_, window, cx| f(window, cx));
    });
}

/// The letter of a key pressed with Alt alone.
fn alt_letter(keystroke: &Keystroke) -> Option<char> {
    let modifiers = keystroke.modifiers;
    let alt_only =
        modifiers.alt && !modifiers.control && !modifiers.platform && !modifiers.function;
    let mut chars = keystroke.key.chars();
    match (chars.next(), chars.next()) {
        (Some(key), None) if alt_only => Some(key),
        _ => None,
    }
}

/// The menu bar each window renders. Also marks the app-wide key listeners
/// as installed.
#[derive(Default)]
struct WindowMenuBars(HashMap<WindowId, WeakEntity<AppMenuBar>>);

impl Global for WindowMenuBars {}

impl WindowMenuBars {
    fn register(bar: &Entity<AppMenuBar>, window: &Window, cx: &mut App) {
        let Some(bars) = cx.try_global::<Self>() else {
            return;
        };
        let id = window.window_handle().window_id();
        if bars.0.get(&id).and_then(WeakEntity::upgrade).as_ref() == Some(bar) {
            return;
        }
        let bars = &mut cx.global_mut::<Self>().0;
        bars.retain(|_, bar| bar.upgrade().is_some());
        bars.insert(id, bar.downgrade());
    }

    /// The window's menu bar, unless a dialog or sheet is open over the
    /// window: the bar's keys must not reach past a modal layer.
    fn reachable(window: &Window, cx: &App) -> Option<Entity<AppMenuBar>> {
        if has_modal_layer(window, cx) {
            return None;
        }
        cx.try_global::<Self>()?
            .0
            .get(&window.window_handle().window_id())?
            .upgrade()
    }
}

fn has_modal_layer(window: &Window, cx: &App) -> bool {
    WindowState::try_read(window, cx)
        .is_some_and(|state| !state.active_dialogs.is_empty() || state.active_sheet.is_some())
}

/// A menu's access key: the letter Alt opens it with, underlined in its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mnemonic {
    /// The byte offset of the letter in the name.
    offset: usize,
    /// The letter, lowercase.
    key: char,
}

/// Each name's access key: its first letter or digit not taken by an earlier
/// name.
fn mnemonics<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<Option<Mnemonic>> {
    let mut taken = Vec::new();
    names
        .into_iter()
        .map(|name| {
            let found = name
                .char_indices()
                .filter(|(_, ch)| ch.is_alphanumeric())
                .map(|(offset, ch)| Mnemonic {
                    offset,
                    key: ch.to_lowercase().next().unwrap_or(ch),
                })
                .find(|mnemonic| !taken.contains(&mnemonic.key));
            taken.extend(found.map(|mnemonic| mnemonic.key));
            found
        })
        .collect()
}

/// What the window's Alt key is doing, as the menu bar sees it.
#[derive(Default)]
struct AltState {
    /// Alt alone is down in the active window, so the access keys show.
    held: bool,
    /// Letting Alt go now is a tap: it went down alone, and since then no
    /// pointer was pressed, released or scrolled and the window kept focus.
    tap_armed: bool,
    /// The window gained focus moments ago, so an Alt going down now was
    /// held from before.
    just_activated: bool,
    /// Whether any modifier was down at the last change.
    any_modifier: bool,
    _settle: Option<Task<()>>,
}

/// The application menu bar, for Windows and Linux.
///
/// Besides the pointer, the keyboard reaches it the way Windows users expect:
/// F10 or a tap of Alt focuses the first menu's title, Left and Right move
/// between titles, Down, Enter or Space open the focused menu, and Escape
/// goes back. Alt with a menu's underlined letter opens that menu directly,
/// and the letters are underlined while Alt is held or the bar has focus.
/// None of these reach past an open dialog or sheet.
pub struct AppMenuBar {
    menus: Vec<Entity<AppMenu>>,
    selected_index: Option<usize>,
    /// How the open menu enters. Only opening from a closed bar slides a menu
    /// in; moving to a neighbour while one is open swaps it in place, as
    /// repeated re-entry on every move would read as flicker.
    menu_entrance: Option<Placement>,
    action_context: Option<FocusHandle>,
    scroll_handle: ScrollHandle,
    focus_handle: FocusHandle,
    /// The menu whose title has keyboard focus while no menu is open.
    highlighted_index: Option<usize>,
    mnemonics: Vec<Option<Mnemonic>>,
    alt: AltState,
    /// Subscribed on the first render, when the bar has a window.
    _window_subscriptions: Vec<Subscription>,
}

impl AppMenuBar {
    /// Create a new app menu bar.
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let mut this = Self::empty(cx);
            this.reload(cx);
            this
        })
    }

    fn empty(cx: &mut Context<Self>) -> Self {
        Self {
            menus: Vec::new(),
            selected_index: None,
            menu_entrance: None,
            action_context: None,
            scroll_handle: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            highlighted_index: None,
            mnemonics: Vec::new(),
            alt: AltState::default(),
            _window_subscriptions: Vec::new(),
        }
    }

    /// Reload the menus from the app.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let menu_bar = cx.entity();
        let menus: Vec<OwnedMenu> = GlobalState::global(cx)
            .app_menus()
            .iter()
            .cloned()
            .collect();
        self.mnemonics = mnemonics(menus.iter().map(|menu| menu.name.as_ref()));
        self.menus = menus
            .iter()
            .enumerate()
            .map(|(ix, menu)| AppMenu::new(ix, menu, menu_bar.clone(), cx))
            .collect();
        self.selected_index = None;
        self.highlighted_index = None;
        self.action_context = None;
        cx.notify();
    }

    /// Whether the bar has the keyboard: a menu's title is focused or a menu
    /// is open.
    pub fn is_active(&self) -> bool {
        self.selected_index.is_some() || self.highlighted_index.is_some()
    }

    /// The menu whose title has keyboard focus while no menu is open.
    pub fn highlighted_index(&self) -> Option<usize> {
        self.highlighted_index
    }

    /// The menu that is open.
    pub fn open_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Focus the first menu's title from the keyboard, remembering what had
    /// focus, or give focus back if the bar already has the keyboard. What
    /// F10 and a tap of Alt do.
    pub fn toggle_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_active() {
            self.leave(window, cx);
        } else if !self.menus.is_empty() {
            self.action_context = window.focused(cx);
            self.focus_handle.focus(window, cx);
            self.highlighted_index = Some(0);
            cx.notify();
        }
    }

    /// Open the menu whose access key is `key`, as Alt with that letter does.
    /// Returns whether one opened.
    pub fn open_menu_with_key(
        &mut self,
        key: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = key.to_lowercase().next().unwrap_or(key);
        let Some(ix) = self
            .mnemonics
            .iter()
            .position(|mnemonic| mnemonic.is_some_and(|mnemonic| mnemonic.key == key))
        else {
            return false;
        };
        self.highlighted_index = None;
        self.set_selected_index(Some(ix), window, cx);
        true
    }

    fn subscribe_to_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let this = cx.weak_entity();
        self._window_subscriptions = vec![
            cx.on_focus_out(&self.focus_handle.clone(), window, |this, _, window, cx| {
                this.on_focus_out(window, cx)
            }),
            cx.observe_window_activation(window, Self::on_window_activation),
            window.observe_modifiers_changed(move |window, cx| {
                _ = this.update(cx, |this, cx| this.on_modifiers_changed(window, cx));
            }),
        ];
    }

    fn on_modifiers_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = window.modifiers();
        let alt_only = modifiers.alt && modifiers.number_of_modifiers() == 1;
        if alt_only && !self.alt.any_modifier {
            self.alt.tap_armed = !self.alt.just_activated;
        } else if modifiers.number_of_modifiers() > 1 {
            self.alt.tap_armed = false;
        }
        self.alt.any_modifier = modifiers.number_of_modifiers() > 0;
        let held = alt_only && window.is_window_active() && !has_modal_layer(window, cx);
        self.set_alt_held(held, cx);
    }

    fn on_window_activation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = window.is_window_active();
        // Alt pressed before the window lost focus (Alt+Tab away) is no tap
        // when it is let go after the window has it back.
        self.alt.tap_armed = false;
        self.alt.just_activated = active;
        self.alt._settle = active.then(|| {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(ALT_HELD_FROM_BEFORE).await;
                _ = this.update(cx, |this, _| this.alt.just_activated = false);
            })
        });
        if !active {
            self.set_alt_held(false, cx);
        }
    }

    /// A pointer pressed, released or scrolled with Alt down (Alt+click adds
    /// a caret, Alt+drag selects a column) makes Alt's release no tap.
    fn on_pointer_with(&mut self, modifiers: gpui::Modifiers) {
        if modifiers.alt {
            self.alt.tap_armed = false;
        }
    }

    fn set_alt_held(&mut self, held: bool, cx: &mut Context<Self>) {
        if self.alt.held != held {
            self.alt.held = held;
            cx.notify();
        }
    }

    /// Focus moved: if it left the bar and its open menu (another control was
    /// clicked), keyboard mode ends where focus now is.
    fn on_focus_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_active() || self.focus_handle.contains_focused(window, cx) {
            return;
        }
        let in_open_menu = self
            .selected_index
            .and_then(|ix| self.menus.get(ix))
            .and_then(|menu| menu.read(cx).popup_menu.clone())
            .is_some_and(|popup| popup.read(cx).focus_handle(cx).contains_focused(window, cx));
        if in_open_menu {
            return;
        }
        self.highlighted_index = None;
        self.selected_index = None;
        self.action_context = None;
        cx.notify();
    }

    /// Give focus back to what had it and leave keyboard mode.
    fn leave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.highlighted_index = None;
        self.set_selected_index(None, window, cx);
    }

    fn on_confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        self.open_highlighted(window, cx);
    }

    fn on_move_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        self.open_highlighted(window, cx);
    }

    fn open_highlighted(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.highlighted_index.take() {
            self.set_selected_index(Some(ix), window, cx);
        }
    }

    /// While a title has focus, a menu's access key alone opens it.
    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if self.highlighted_index.is_none() || keystroke.modifiers.modified() {
            return;
        }
        let mut chars = keystroke.key.chars();
        if let (Some(key), None) = (chars.next(), chars.next())
            && self.open_menu_with_key(key, window, cx)
        {
            cx.stop_propagation();
        }
    }

    fn on_move_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.step(-1, window, cx);
    }

    fn on_move_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        self.step(1, window, cx);
    }

    /// Move the focused title or the open menu by `delta`, wrapping around.
    fn step(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.menus.len() as isize;
        let next = |ix: usize| (ix as isize + delta).rem_euclid(count.max(1)) as usize;
        if let Some(ix) = self.highlighted_index {
            self.highlighted_index = Some(next(ix));
            cx.notify();
        } else if let Some(ix) = self.selected_index {
            self.set_selected_index(Some(next(ix)), window, cx);
        }
    }

    fn on_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.leave(window, cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Keep what had focus before the bar took the keyboard, not the bar.
        if self.selected_index.is_none() && ix.is_some() && self.action_context.is_none() {
            self.action_context = window.focused(cx);
        } else if ix.is_none() {
            match self.action_context.take() {
                Some(action_context) => action_context.focus(window, cx),
                // Nothing had focus before the bar took it, so nothing has
                // it after.
                None if self.focus_handle.contains_focused(window, cx) => window.blur(cx),
                None => {}
            }
        }

        if ix != self.selected_index {
            self.menu_entrance = self.selected_index.is_none().then_some(Placement::Bottom);
        }
        self.selected_index = ix;
        cx.notify();
    }

    #[inline]
    fn has_activated_menu(&self) -> bool {
        self.selected_index.is_some()
    }

    /// Hears the pointer anywhere in the window, before any element can stop
    /// it, to tell an Alt tap from Alt used with the pointer.
    fn pointer_listeners(this: WeakEntity<Self>) -> impl IntoElement {
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                fn listen<E: gpui::MouseEvent>(
                    window: &mut Window,
                    this: WeakEntity<AppMenuBar>,
                    modifiers: fn(&E) -> gpui::Modifiers,
                ) {
                    window.on_mouse_event(move |event: &E, phase, _, cx| {
                        if phase == DispatchPhase::Capture {
                            _ = this.update(cx, |this, _| this.on_pointer_with(modifiers(event)));
                        }
                    });
                }
                listen::<MouseDownEvent>(window, this.clone(), |event| event.modifiers);
                listen::<MouseUpEvent>(window, this.clone(), |event| event.modifiers);
                listen::<ScrollEvent>(window, this, |event| event.modifiers);
            },
        )
        .absolute()
        .size_0()
    }
}

impl Render for AppMenuBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        WindowMenuBars::register(&cx.entity(), window, cx);
        if self._window_subscriptions.is_empty() {
            self.subscribe_to_window(window, cx);
        }

        div()
            .relative()
            .size_full()
            .child(Self::pointer_listeners(cx.weak_entity()))
            .child(
                h_flex()
                    .id("app-menu-bar")
                    .overflow_x_scroll()
                    .lock_scroll_axis()
                    .track_scroll(&self.scroll_handle)
                    .test_support()
                    .track_focus(&self.focus_handle)
                    .role(Role::MenuBar)
                    .key_context(CONTEXT)
                    .on_action(cx.listener(Self::on_move_left))
                    .on_action(cx.listener(Self::on_move_right))
                    .on_action(cx.listener(Self::on_move_down))
                    .on_action(cx.listener(Self::on_confirm))
                    .on_action(cx.listener(Self::on_cancel))
                    .on_key_down(cx.listener(Self::on_key_down))
                    .size_full()
                    .gap_x_1()
                    .children(self.menus.clone()),
            )
            .child(ScrollableMask::vertical_wheel(&self.scroll_handle))
    }
}

/// A menu in the menu bar.
pub(super) struct AppMenu {
    menu_bar: Entity<AppMenuBar>,
    ix: usize,
    name: SharedString,
    menu: OwnedMenu,
    popup_menu: Option<Entity<PopupMenu>>,
    /// Registered on the menu's title and never focused, so the popup can look
    /// its shortcuts up through the menu bar's key contexts on the frame it
    /// opens, when nothing has focus or focus is outside those contexts. GPUI
    /// resolves a handle in the previously rendered frame, where the popup's
    /// own element is not yet.
    trigger_focus: FocusHandle,

    _subscription: Option<Subscription>,
}

impl AppMenu {
    pub(super) fn new(
        ix: usize,
        menu: &OwnedMenu,
        menu_bar: Entity<AppMenuBar>,
        cx: &mut App,
    ) -> Entity<Self> {
        let name = menu.name.clone();
        cx.new(|cx| Self {
            ix,
            menu_bar,
            name,
            menu: menu.clone(),
            popup_menu: None,
            trigger_focus: cx.focus_handle(),
            _subscription: None,
        })
    }

    /// Whether this menu's popup is the one the menu bar currently has open.
    fn is_open(&self, cx: &App) -> bool {
        self.menu_bar.read(cx).selected_index == Some(self.ix)
    }

    fn build_popup_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let action_context = self.menu_bar.read(cx).action_context.clone();
        let entrance = self.menu_bar.read(cx).menu_entrance;
        let popup_menu = match self.popup_menu.as_ref() {
            None => {
                let items = self.menu.items.clone();
                let popup_menu = PopupMenu::build(window, cx, |menu, window, cx| {
                    menu.with_menu_items(items, window, cx)
                });
                let trigger_focus = self.trigger_focus.clone();
                popup_menu.update(cx, |menu, cx| {
                    menu.set_action_context(action_context.clone(), cx);
                    menu.set_trigger_focus(Some(trigger_focus), cx);
                });
                self._subscription =
                    Some(cx.subscribe_in(&popup_menu, window, Self::handle_dismiss));
                self.popup_menu = Some(popup_menu.clone());

                popup_menu
            }
            Some(menu) => {
                menu.update(cx, |menu, cx| {
                    menu.set_action_context(action_context.clone(), cx);
                });
                menu.clone()
            }
        };

        popup_menu.update(cx, |menu, _| menu.entrance = entrance);
        let focus_handle = popup_menu.read(cx).focus_handle(cx);
        if !focus_handle.contains_focused(window, cx) {
            focus_handle.focus(window, cx);
        }

        popup_menu
    }

    fn handle_dismiss(
        &mut self,
        _: &Entity<PopupMenu>,
        _: &DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._subscription.take();
        self.popup_menu.take();
        self.menu_bar.update(cx, |state, cx| {
            state.on_cancel(&Cancel, window, cx);
        });
    }

    fn handle_trigger_click(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, ClickEvent::Mouse(_)) {
            return;
        }

        self.toggle(window, cx);
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let is_open = self.is_open(cx);
        _ = self.menu_bar.update(cx, |state, cx| {
            let new_ix = if is_open { None } else { Some(self.ix) };
            state.highlighted_index = None;
            state.set_selected_index(new_ix, window, cx);
        });
    }

    fn handle_hover(&mut self, hovered: &bool, window: &mut Window, cx: &mut Context<Self>) {
        if !*hovered {
            return;
        }

        let has_activated_menu = self.menu_bar.read(cx).has_activated_menu();
        if !has_activated_menu {
            return;
        }

        _ = self.menu_bar.update(cx, |state, cx| {
            state.set_selected_index(Some(self.ix), window, cx);
        });
    }

    /// The name with its access key underlined.
    fn underlined_name(&self, mnemonic: Mnemonic) -> StyledText {
        let end = mnemonic.offset
            + self.name[mnemonic.offset..]
                .chars()
                .next()
                .map_or(1, char::len_utf8);
        StyledText::new(self.name.clone()).with_highlights([(
            mnemonic.offset..end,
            HighlightStyle {
                underline: Some(UnderlineStyle {
                    thickness: px(1.),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )])
    }
}

impl Render for AppMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = self.is_open(cx);
        let bar = self.menu_bar.read(cx);
        let is_highlighted = bar.highlighted_index == Some(self.ix);
        let mnemonic = bar
            .mnemonics
            .get(self.ix)
            .copied()
            .flatten()
            .filter(|_| bar.alt.held || bar.is_active());
        let trigger_focus = self.trigger_focus.clone();

        div()
            .id(self.ix)
            .relative()
            // Registers the handle on this path without making the title
            // focusable.
            .child(
                canvas(
                    move |_, window, cx| window.set_focus_handle(&trigger_focus, cx),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_0(),
            )
            .child(
                Button::new("menu")
                    .small()
                    .py_0p5()
                    .compact()
                    .ghost()
                    .map(|this| match mnemonic {
                        Some(mnemonic) => this
                            .accessibility_label(self.name.clone())
                            .child(self.underlined_name(mnemonic)),
                        None => this.label(self.name.clone()),
                    })
                    .selected(is_highlighted)
                    .open(is_open)
                    .on_mouse_down(
                        MouseButton::Left,
                        window.listener_for(&cx.entity(), move |this, _, window, cx| {
                            // Stop propagation to avoid dragging the window.
                            window.prevent_default();
                            cx.stop_propagation();
                            this.toggle(window, cx);
                        }),
                    )
                    .on_click(cx.listener(Self::handle_trigger_click)),
            )
            .on_hover(cx.listener(Self::handle_hover))
            .when(is_open, |this| {
                this.child(deferred(
                    anchored()
                        .anchor(gpui::Anchor::TopLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(
                            div()
                                .size_full()
                                .occlude()
                                .top_1()
                                .child(self.build_popup_menu(window, cx)),
                        ),
                ))
            })
    }
}

#[cfg(test)]
#[path = "app_menu_bar_tests.rs"]
mod tests;
