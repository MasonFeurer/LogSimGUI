use crate::app::AppItem;
use egui::{Context, Event, Key, Modifiers, Pos2, Vec2};
use hashbrown::HashSet;

#[cfg(target_os = "macos")]
pub const CMD_KEY: &str = "⌘";
#[cfg(not(target_os = "macos"))]
pub const CMD_KEY: &str = "Ctrl";

#[cfg(target_os = "macos")]
pub const OPTION_KEY: &str = "⌥";
#[cfg(not(target_os = "macos"))]
pub const OPTION_KEY: &str = "Alt";

#[cfg(target_os = "macos")]
pub const CTRL_KEY: &str = "⌃";
#[cfg(not(target_os = "macos"))]
pub const CTRL_KEY: &str = "Ctrl";

#[cfg(target_os = "macos")]
pub const SHIFT_KEY: &str = "⇧";
#[cfg(not(target_os = "macos"))]
pub const SHIFT_KEY: &str = "Shift";

pub enum Keybind {
    Shift(Key),
    // `command` `⌘` on MacOS, `Ctrl` everywhere else
    Command(Key),
    // `control` `⌃` on MacOS, `Ctrl` everywhere else
    Control(Key),
    // `option` `⌥` on MacOS, `Alt` everywhere else
    Option(Key),
}
impl Keybind {
    pub fn show(&self) -> String {
        match self {
            Self::Shift(key) => format!("{} {:?}", SHIFT_KEY, key),
            Self::Command(key) => format!("{} {:?}", CMD_KEY, key),
            Self::Control(key) => format!("{} {:?}", CTRL_KEY, key),
            Self::Option(key) => format!("{} {:?}", OPTION_KEY, key),
        }
    }

    pub fn matches(&self, key: Key, modifiers: Modifiers) -> bool {
        match self {
            Self::Shift(k) => *k == key && modifiers.shift,
            Self::Command(k) => *k == key && modifiers.command,
            Self::Control(k) => *k == key && modifiers.ctrl,
            Self::Option(k) => *k == key && modifiers.alt,
        }
    }
}

#[derive(Default)]
pub struct FrameInput {
    pub pressed_keys: HashSet<Key>,
    pub modifiers: Modifiers,
    pub press_pos: Pos2,

    // pointer
    pub prev_pointer_pos: Pos2,
    pub pointer_pos: Pos2,
    /// If the primary pointer button was pressed down this frame
    pub pressed_prim: bool,
    /// If the secondary pointer button was pressed down this frame
    pub pressed_sec: bool,
    /// If the primary pointer button was clicked this frame
    pub clicked_prim: bool,
    /// If the secondary pointer button was clicked this frame
    pub clicked_sec: bool,

    pub drag: Option<(Vec2, AppItem)>,
    pub scroll_delta: Vec2,
    /// The app item that was hovered last frame
    pub prev_hovered: AppItem,
    pub hovered_changed: bool,
}
impl FrameInput {
    pub fn update(&mut self, ctx: &Context, hovered: AppItem) {
        let input = ctx.input();

        // key presses
        self.pressed_keys.clear();
        for event in &input.events {
            let Event::Key { key, pressed: true, .. } = event else {
        		continue;
        	};
            self.pressed_keys.insert(*key);
        }
        self.modifiers = input.modifiers;

        // pointer
        self.prev_pointer_pos = self.pointer_pos;
        self.pointer_pos = input
            .pointer
            .interact_pos()
            .unwrap_or(self.prev_pointer_pos);
        self.pressed_prim = input.pointer.primary_clicked();
        self.pressed_sec = input.pointer.secondary_clicked();
        self.scroll_delta = input.scroll_delta;

        if self.pressed_prim {
            self.drag = Some((Vec2::ZERO, hovered));
            self.press_pos = self.pointer_pos;
        }
        let pointer_delta = self.pointer_pos - self.prev_pointer_pos;
        if let Some((delta, _)) = &mut self.drag {
            *delta = pointer_delta;
        }
        self.clicked_prim = input.pointer.primary_released() && self.press_pos == self.pointer_pos;
        self.clicked_sec = input.pointer.secondary_released() && self.press_pos == self.pointer_pos;

        if input.pointer.any_released() {
            self.drag = None;
        }
    }

    #[inline(always)]
    pub fn drag_delta(&self) -> Option<(Vec2, AppItem)> {
        self.drag
    }

    pub fn pressed(&self, key: Key) -> bool {
        self.pressed_keys.contains(&key)
    }
    pub fn keybind_used(&self, keybind: &Keybind) -> bool {
        for key in &self.pressed_keys {
            if keybind.matches(*key, self.modifiers) {
                return true;
            }
        }
        false
    }
}

/// The result of calling `App::update`
#[derive(Default)]
pub struct FrameOutput {
    // the user wants to open the persist directory in a file viewer
    pub reveal_persist_dir: bool,
    /// the user wants to save the while state of the app
    pub save: bool,
    /// the user wants to import presets
    pub import_presets: bool,
    /// the user wants to reload the settings
    pub load_settings: bool,
    /// the user wants to reload the presets
    pub load_presets: bool,
    /// the app item that the pointer was over
    pub hovered: AppItem,
    /// If we've pressed the mouse, make sure that it can no longer be a click when released
    pub void_click: bool,
}
