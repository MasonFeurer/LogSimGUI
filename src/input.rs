use crate::app::AppItem;
use eframe::egui::{Context, Key, Pos2, Vec2};

#[cfg(target_os = "macos")]
pub const CMD_KEY: &str = "⌘";
#[cfg(not(target_os = "macos"))]
pub const CMD_KEY: &str = "Ctrl";

#[derive(Default)]
pub struct FrameInput {
    pub pressed_del: bool,
    pub pressed_esc: bool,
    pub pressed_space: bool,
    pub pressed_enter: bool,
    pub pressed_down: bool,
    pub pressed_up: bool,
    pub pressed_l: bool,
    pub pressed_s: bool,
    pub shift: bool,
    pub cmd: bool,
    pub press_pos: Pos2,

    // pointer
    pub prev_pointer_pos: Pos2,
    pub pointer_pos: Pos2,
    pub pressed_prim: bool,
    pub pressed_sec: bool,
    pub clicked_prim: bool,
    pub clicked_sec: bool,

    // drag
    pub drag_item: Option<(Vec2, AppItem)>,
}
impl FrameInput {
    pub fn update(&mut self, ctx: &Context, hovered: AppItem) {
        let input = ctx.input();
        self.prev_pointer_pos = self.pointer_pos;
        self.pointer_pos = input
            .pointer
            .interact_pos()
            .unwrap_or(self.prev_pointer_pos);
        self.pressed_del = input.key_pressed(Key::Backspace);
        self.pressed_esc = input.key_pressed(Key::Escape);
        self.pressed_space = input.key_pressed(Key::Space);
        self.pressed_enter = input.key_pressed(Key::Enter);
        self.pressed_down = input.key_pressed(Key::ArrowDown);
        self.pressed_up = input.key_pressed(Key::ArrowUp);
        self.pressed_l = input.key_pressed(Key::L);
        self.pressed_s = input.key_pressed(Key::S);
        self.shift = input.modifiers.shift;
        self.cmd = input.modifiers.command;
        self.pressed_prim = input.pointer.primary_clicked();
        self.pressed_sec = input.pointer.secondary_clicked();

        if self.pressed_prim {
            self.drag_item = Some((Vec2::ZERO, hovered));
            self.press_pos = self.pointer_pos;
        }
        let pointer_delta = self.pointer_pos - self.prev_pointer_pos;
        if let Some((delta, _)) = &mut self.drag_item {
            *delta = pointer_delta;
        }
        self.clicked_prim = input.pointer.primary_released() && self.press_pos == self.pointer_pos;
        self.clicked_sec = input.pointer.secondary_released() && self.press_pos == self.pointer_pos;

        if input.pointer.any_released() {
            self.drag_item = None;
        }
    }

    #[inline(always)]
    pub fn drag_delta(&self) -> Option<(Vec2, AppItem)> {
        self.drag_item
    }
}
