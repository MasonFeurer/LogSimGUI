use crate::app::AppItem;
use egui::{Context, Event, Key, Modifiers, Pos2, TouchPhase, Vec2};
use hashbrown::HashSet;

#[derive(Default)]
pub struct Input {
    pub native: bool,
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
    prev_hovered: AppItem,
    /// The app item that was determined to be hovered this frame
    new_hovered: AppItem,
    pub hovered_changed: bool,
}
impl Input {
    pub fn new(native: bool) -> Self {
        Self {
            native,
            ..Self::default()
        }
    }

    pub fn hovered(&self) -> AppItem {
        self.prev_hovered
    }
    pub fn set_hovered(&mut self, item: AppItem) {
        if item.layer() >= self.new_hovered.layer() {
            self.new_hovered = item;
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        self.hovered_changed = self.prev_hovered != self.new_hovered;
        self.prev_hovered = self.new_hovered;

        let input = ctx.input();
        let mut released_press = input.pointer.any_released();

        // key presses
        self.pressed_keys.clear();
        for event in &input.events {
            match event {
                Event::Key {
                    key, pressed: true, ..
                } => {
                    self.pressed_keys.insert(*key);
                }
                Event::Touch {
                    phase: TouchPhase::End | TouchPhase::Cancel,
                    ..
                } => {
                    released_press = true;
                }
                _ => {}
            }
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
            self.drag = Some((Vec2::ZERO, self.prev_hovered));
            self.press_pos = self.pointer_pos;
        }
        let pointer_delta = self.pointer_pos - self.prev_pointer_pos;
        if let Some((delta, _)) = &mut self.drag {
            *delta = pointer_delta;
        }
        self.clicked_prim = input.pointer.primary_released() && self.press_pos == self.pointer_pos;
        self.clicked_sec = input.pointer.secondary_released() && self.press_pos == self.pointer_pos;

        if released_press {
            self.drag = None;
        }
        self.new_hovered = AppItem::None;
    }

    #[inline(always)]
    pub fn drag_delta(&self) -> Option<(Vec2, AppItem)> {
        self.drag
    }

    pub fn pressed(&self, key: Key) -> bool {
        self.pressed_keys.contains(&key)
    }

    /// Determines if a key was pressed as a command keybind.
    /// The modifiers are:
    /// | platform | native  | web    |
    /// |:--------:|:-------:|:------:|
    /// | Mac      | command | option |
    /// | Windows  | Ctrl    | Alt    |
    /// | Linux    | Ctrl    | Alt    |
    /// |:--------:|:-------:|:------:|
    ///
    pub fn command_used(&self, key: Key) -> bool {
        // On web, I can't use Ctrl/command because those will trigger browser shortcuts.
        let mod_cond = if cfg!(wasm) {
            // .alt is `Alt` on Windows/Linux, but `option` on MacOS
            self.modifiers.alt
        } else {
            // .command is `command` on MacOS, but `Ctrl` on Windows/Linux
            self.modifiers.command
        };
        mod_cond && self.pressed_keys.contains(&key)
    }

    pub fn display_command(key: Key) -> String {
        if cfg!(wasm) {
            // This would display the wrong keybind if viewing the website on MacOS,
            // but I don't know how to check for that
            format!("Alt + {key:?}")
        } else if cfg!(windows) {
            format!("Ctrl + {key:?}")
        } else if cfg!(macos) {
            format!("⌘ + {key:?}")
        } else {
            format!("Ctrl + {key:?}")
        }
    }
}
