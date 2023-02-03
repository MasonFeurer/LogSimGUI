use crate::board::{Board, BoardItem, Device};
use crate::input::Input;
use crate::settings::Settings;
use crate::*;
use egui::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppItem {
    None,
    Board(BoardItem),
    NamePopup,
    Other,
    PresetPlacer,
}
impl Default for AppItem {
    fn default() -> Self {
        Self::None
    }
}
impl AppItem {
    /// If a.layer() > b.layer(), then a is shown above b
    pub fn layer(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Board(BoardItem::Board) => 1,
            Self::Board(_) => 2,
            Self::NamePopup => 3,
            Self::Other => 4,
            Self::PresetPlacer => 5,
        }
    }

    // Overrides `self` with `new` if `new` is above `self`
    pub fn set(&mut self, new: Self) {
        if new.layer() > self.layer() {
            *self = new;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    None,

    RevealConfigDir,
    LoadSettings,
    ReloadLibrary,
    ImportLibrary,

    ToggleLibraryMenu,
    TogglePackMenu,
    ToggleSimMenu,

    OpenSettings,
    CloseSettings,

    PackBoard,
    StepSim,

    HoldPreset(String),
    LoadPreset(String),
    DeletePreset(String),
    Clear,
}
impl Default for AppAction {
    fn default() -> Self {
        Self::None
    }
}
impl AppAction {
    pub fn set(&mut self, new: Self) {
        if self == &Self::None {
            *self = new
        }
    }
}

pub struct CreateLinks {
    pub starts: Vec<LinkStart<u64>>,
    pub color: usize,
    pub anchors: Vec<Pos2>,
}
impl CreateLinks {
    fn new() -> Self {
        Self {
            starts: Vec::new(),
            color: 0,
            anchors: Vec::new(),
        }
    }

    fn push(&mut self, start: LinkStart<u64>) {
        if self.starts.contains(&start) {
            return;
        }
        if self.starts.is_empty() {
            self.color = 0;
            self.anchors.clear();
        }
        self.starts.insert(0, start);
    }
    fn take(&mut self) -> Option<(LinkStart<u64>, usize)> {
        self.starts.pop().map(|start| (start, self.color))
    }
}

pub struct App {
    pub settings: Settings,
    pub library: Library,
    pub board: Board,

    pub input: Input,
    pub int: IntegrationInfo,

    pub settings_open: bool,
    pub library_menu: ui::LibraryMenu,
    pub pack_menu: ui::PackMenu,
    pub sim_menu: ui::SimMenu,

    /// The small window for searching and placing library
    pub preset_placer: ui::ChipPlacer,
    pub name_popup: Option<ui::NamePopup>,

    pub create_links: CreateLinks,
    /// A list of the presets we've picked from the preset placer
    pub held_presets: Vec<String>,
    /// If we've selected multiple devices for bulk actions
    pub selected_devices: Vec<u64>,
    /// If true, we should automatically start/finish placing a link when we hover the pin
    pub auto_link: bool,
}

impl App {
    pub fn new(info: IntegrationInfo, settings: Settings, library: Library, board: Board) -> Self {
        Self {
            settings,
            library,
            board,

            input: Input::new(info.native),
            int: info,

            settings_open: false,
            library_menu: ui::LibraryMenu::default(),
            pack_menu: ui::PackMenu::default(),
            sim_menu: ui::SimMenu::default(),

            preset_placer: ui::ChipPlacer::default(),
            name_popup: None,

            create_links: CreateLinks::new(),
            held_presets: Vec::new(),
            selected_devices: Vec::new(),
            auto_link: false,
        }
    }

    pub fn place_preset(&mut self, name: &str, pos: Pos2) {
        if let Some(preset) = self.library.get_preset(name) {
            let device = Device::from_preset(preset, pos);
            self.board.add_device(rand_id(), device);
            self.preset_placer.push_recent(name);
        }
    }
    pub fn finish_link(&mut self, target: LinkTarget<u64>) -> bool {
        if let Some((start, color)) = self.create_links.take() {
            let anchors = self.create_links.anchors.clone();
            self.board
                .add_link(start, crate::Link::new(target, color, anchors));
            return true;
        }
        false
    }

    pub fn exec_action(&mut self, action: AppAction, out: &mut OutEvent) {
        match action {
            AppAction::None => {}
            AppAction::RevealConfigDir => *out = OutEvent::RevealConfigDir,
            AppAction::LoadSettings => *out = OutEvent::LoadSettings,
            AppAction::ReloadLibrary => *out = OutEvent::LoadLibrary,
            AppAction::ImportLibrary => *out = OutEvent::ImportPresets,

            AppAction::TogglePackMenu => self.pack_menu.open ^= true,
            AppAction::ToggleLibraryMenu => self.library_menu.open ^= true,
            AppAction::ToggleSimMenu => self.sim_menu.open ^= true,

            AppAction::OpenSettings => self.settings_open = true,
            AppAction::CloseSettings => self.settings_open = false,

            AppAction::PackBoard => todo!(),
            AppAction::StepSim => self.board.update(),
            AppAction::HoldPreset(name) => self.held_presets.push(name),
            AppAction::LoadPreset(_name) => todo!(),
            AppAction::DeletePreset(name) => self.library.remove_preset(&name),
            AppAction::Clear => self.board = Board::new(),
        }
    }

    // -----------------------------------------------------------
    // GUI

    pub fn board_input(&mut self, focus_clear: bool) {
        let AppItem::Board(item) = self.input.hovered() else {
    		return;
    	};
        let world_pos = self.sim_menu.view.create_inv_transform() * self.input.pointer_pos;
        let try_link = self.auto_link && self.input.hovered_changed;
        match item {
            BoardItem::Board => {
                if self.input.pressed_prim {
                    self.create_links.anchors.push(world_pos);
                }
            }
            BoardItem::Device(id) => {
                if self.input.pressed(Key::Backspace) {
                    if self.selected_devices.contains(&id) {
                        for id in &self.selected_devices {
                            self.board.remove_device(*id);
                        }
                        self.selected_devices.clear();
                    } else {
                        self.board.remove_device(id);
                    }
                }
                if self.input.pressed_prim && self.input.modifiers.shift {
                    if !self.selected_devices.contains(&id) {
                        self.selected_devices.push(id);
                    }
                }
            }
            BoardItem::InputBulb(id) => {
                if self.input.clicked_prim {
                    let state = self.board.inputs.get(&id).unwrap().io.state;
                    self.board.set_input(id, !state);
                }
                self.name_popup = Some(ui::NamePopup::input(id));
                if self.input.pressed(Key::Backspace) && focus_clear {
                    self.board.remove_input(id);
                }
                if self.input.pressed(Key::ArrowDown) {
                    self.board.stack_input(id, &self.settings);
                }
                if self.input.pressed(Key::ArrowUp) {
                    self.board.unstack_input(id);
                }
            }
            BoardItem::InputPin(id) => {
                if self.input.pressed_prim || try_link {
                    self.create_links.push(LinkStart::Input(id));
                }
            }
            BoardItem::InputLink(input_id, link_idx) => {
                if self.input.pressed(Key::Backspace) {
                    let links = &mut self.board.inputs.get_mut(&input_id).unwrap().links;
                    let target = links[link_idx].target;
                    links.remove(link_idx);
                    self.board.write_queue.push(target, false);
                }
            }
            BoardItem::InputGroup(_) => {}
            BoardItem::OutputBulb(id) => {
                if self.input.pressed(Key::Backspace) {
                    self.board.remove_output(id);
                }
                self.name_popup = Some(ui::NamePopup::output(id));
                if self.input.pressed(Key::ArrowDown) && focus_clear {
                    self.board.stack_output(id, &self.settings);
                }
                if self.input.pressed(Key::ArrowUp) {
                    self.board.unstack_output(id);
                }
            }
            BoardItem::OutputGroup(_) => {}
            BoardItem::OutputPin(id) => {
                if self.input.pressed_prim || try_link {
                    self.finish_link(LinkTarget::Output(id));
                }
            }
            BoardItem::DeviceInput(device, device_input) => {
                let mut created_link = false;
                if self.input.pressed_prim || try_link {
                    created_link = self.finish_link(LinkTarget::DeviceInput(device, device_input));
                }
                if self.input.pressed_prim && !created_link {
                    let state = self.board.get_device_input(device, device_input).unwrap();
                    self.board.set_device_input(device, device_input, !state);
                }
            }
            BoardItem::DeviceOutput(device, output) => {
                if self.input.pressed_prim || try_link {
                    self.create_links
                        .push(LinkStart::DeviceOutput(device, output));
                }
                if self.input.pressed(Key::Backspace) {
                    let device = self.board.devices.get_mut(&device).unwrap();
                    device.links[output].clear();
                }
            }
            BoardItem::DeviceOutputLink(device_id, output_idx, link_idx) => {
                if self.input.pressed(Key::Backspace) {
                    let links =
                        &mut self.board.devices.get_mut(&device_id).unwrap().links[output_idx];
                    let target = links[link_idx].target;
                    links.remove(link_idx);
                    self.board.write_queue.push(target, false);
                }
            }
            BoardItem::InputCol => {
                if self.input.clicked_prim {
                    self.board.add_input(world_pos.y);
                }
            }
            BoardItem::OutputCol => {
                if self.input.clicked_prim {
                    self.board.add_output(world_pos.y);
                }
            }
        };
    }

    pub fn clone_selected_devices(&mut self, pointer_pos: Pos2) {
        let mut selection_min = pos2(f32::INFINITY, f32::INFINITY);
        let mut devices = Vec::with_capacity(self.selected_devices.len());
        for device_id in &self.selected_devices {
            let device = self.board.devices.get(device_id).unwrap();
            selection_min.x = f32::min(selection_min.x, device.pos.x);
            selection_min.y = f32::min(selection_min.y, device.pos.y);
            devices.push(device.clone());
        }
        let offset = self.sim_menu.view.create_inv_transform() * pointer_pos - selection_min;
        let mut ids = Vec::with_capacity(devices.len());
        for mut device in devices {
            device.pos += offset;
            let id = rand_id();
            self.board.add_device(id, device);
            ids.push(id);
        }
        self.selected_devices = ids;
    }

    pub fn update(&mut self, ctx: &Context) -> OutEvent {
        let mut style = (*ctx.style()).clone();
        self.settings.theme.set(&mut style);
        ctx.set_style(style);

        match self.settings_open {
            true => self.show_settings_page(ctx),
            false => self.show_sim_page(ctx),
        }
    }

    pub fn show_settings_page(&mut self, ctx: &Context) -> OutEvent {
        let mut out_event = OutEvent::default();

        TopBottomPanel::top("settings_top").show(ctx, |ui| {
            ui.heading("Settings");
            if ui.button("Done").clicked() {
                self.exec_action(AppAction::CloseSettings, &mut out_event);
            }
            if ui.button("Reset").clicked() {
                self.settings = Settings::default();
            }
        });
        CentralPanel::default().show(ctx, |ui| {
            ui.label("Settings here");
        });
        out_event
    }
    pub fn show_sim_page(&mut self, ctx: &Context) -> OutEvent {
        let mut out_event = OutEvent::default();
        let mut action = AppAction::None;

        self.board_input(ctx.memory().focus().is_none());
        self.input.update(ctx);

        // --- Update sim ---
        if !self.sim_menu.paused {
            for _ in 0..self.sim_menu.speed {
                self.board.update();
            }
        }

        // --- Show UI ---
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let new_action = ui::show_top_panel(ui);
                action.set(new_action);
            });
        });

        if self.library_menu.open {
            SidePanel::left("library_menu").show(ctx, |ui| {
                let mut menu = self.library_menu.clone();
                action.set(ui::show_library_menu(
                    ui,
                    self.settings.debug,
                    &mut menu,
                    self.int.native,
                    &self.library,
                ));
                self.library_menu = menu;
            });
        }
        if self.pack_menu.open {
            SidePanel::left("pack_menu").show(ctx, |ui| {
                let mut menu = self.pack_menu.clone();
                action.set(ui::show_pack_menu(ui, &mut menu, &self.library));
                self.pack_menu = menu;
            });
        }
        if self.settings.debug {
            TopBottomPanel::top("debug_menu").show(ctx, |ui| {
                ui::debug_ui(ui, self);
            });
        }
        if self.sim_menu.open {
            SidePanel::right("sim_menu").show(ctx, |ui| {
                let mut menu = self.sim_menu.clone();
                action.set(ui::show_sim_menu(ui, &mut menu));
                self.sim_menu = menu;
            });
        }

        let mut board_item = None;
        let mut g = graphics::Graphics::new(
            ctx,
            self.sim_menu.view.create_transform(),
            self.input.pointer_pos,
        );

        if let Some(item) = graphics::show_board(
            &mut g,
            &self.settings,
            &self.board,
            &self.library,
            self.settings.debug,
        ) {
            board_item = Some(item);
        }
        graphics::outline_devices(&mut g, &self.settings, &self.selected_devices, &self.board);
        graphics::show_create_links(
            &mut g,
            &self.settings,
            &self.board,
            &self.create_links,
            self.sim_menu.view.create_inv_transform() * self.input.pointer_pos,
        );
        graphics::show_held_presets(
            &mut g,
            &self.settings,
            &self.library,
            self.input.pointer_pos,
            &self.held_presets,
        );

        let shapes = g.finish();

        let board_rs = CentralPanel::default()
            .show(ctx, |ui| {
                let (_, painter) = ui.allocate_painter(ui.available_size(), Sense::drag());
                painter.extend(shapes);

                if painter.clip_rect().contains(self.input.pointer_pos) {
                    self.input.set_hovered(AppItem::Board(BoardItem::Board));
                } else {
                    self.input.set_hovered(AppItem::Other);
                }

                if let Some(popup) = self.name_popup.clone() {
                    if popup.is_dead() {
                        self.name_popup = None;
                    }

                    let t = self.sim_menu.view.create_transform();

                    let rs = popup.show(ui, &self.board, self.settings.board_io_col_w, t);
                    self.name_popup.as_mut().map(|e| e.update());
                    if rs.hovered {
                        self.name_popup.as_mut().map(|e| e.persist());
                        self.input.set_hovered(AppItem::NamePopup);
                    }
                    if rs.edit {
                        println!("edit!");
                    }
                }
            })
            .response;
        if let Some(item) = board_item {
            self.input.set_hovered(AppItem::Board(item));
        }

        // --- Handle key binds ---
        if self.input.command_used(Key::L) {
            self.auto_link = !self.auto_link;
        }
        if self.sim_menu.paused && self.input.command_used(Key::T) {
            self.board.update();
        }
        if self.selected_devices.len() > 0 && self.input.command_used(Key::D) {
            self.clone_selected_devices(self.input.pointer_pos);
        }
        if self.input.pressed(Key::Escape) {
            self.create_links = CreateLinks::new();
        }

        // --- Handle dragging ---
        let inv_t = self.sim_menu.view.create_inv_transform();
        if let Some((delta, item)) = self.input.drag_delta() {
            match item {
                AppItem::Board(BoardItem::Board) => {
                    self.sim_menu.view.drag(delta);
                }
                AppItem::Board(BoardItem::InputBulb(id)) => {
                    self.board.drag_input(id, inv_t * delta);
                }
                AppItem::Board(BoardItem::OutputBulb(id)) => {
                    self.board.drag_output(id, inv_t * delta);
                }
                AppItem::Board(BoardItem::Device(id)) => {
                    if self.selected_devices.contains(&id) {
                        for id in &self.selected_devices {
                            self.board.drag_device(*id, inv_t * delta);
                        }
                    } else {
                        self.board.drag_device(id, inv_t * delta);
                    }
                }
                AppItem::Board(BoardItem::InputCol) => {
                    self.board.rect.min.x += inv_t * delta.x;
                }
                AppItem::Board(BoardItem::OutputCol) => {
                    self.board.rect.max.x += inv_t * delta.x;
                }
                _ => {}
            }
        }

        // --- Handle scrolling ---
        self.sim_menu.view.drag(self.input.scroll_delta);

        // --- Handle zooming ---
        let zoom_delta = ctx.input().zoom_delta();
        if zoom_delta != 1.0 {
            let pos = self.input.pointer_pos - board_rs.rect.min;
            self.sim_menu.view.zoom(zoom_delta, pos.to_pos2());
        }

        // --- Handle placing library ---
        let can_place_preset = matches!(self.input.hovered(), AppItem::Board(_));
        if self.held_presets.len() > 0 && self.input.pressed_prim && can_place_preset {
            let mut held_presets = Vec::new();
            std::mem::swap(&mut held_presets, &mut self.held_presets);

            let t = self.sim_menu.view.create_inv_transform();
            let mut pos = t * (self.input.pointer_pos + vec2(0.0, 30.0));

            for name in held_presets {
                self.place_preset(&name, pos);

                let preset = self.library.get_preset(&name).unwrap();
                let size = graphics::calc_device_size(
                    preset.data.num_inputs(),
                    preset.data.num_outputs(),
                    self.settings.device_min_pin_spacing,
                );
                pos.y += size.y;
            }
        }

        // --- Handle context menu ---
        board_rs.context_menu(|ui| {
            if !can_place_preset {
                ui.close_menu();
                return;
            }

            ui.set_width(100.0);
            let mut place_preset = None;

            for (cat, library) in self.library.cats_sorted() {
                ui.menu_button(cat, |ui| {
                    ui.set_width(100.0);
                    for preset in library {
                        if ui.button(&preset.name).clicked() {
                            place_preset = Some(preset.name.clone());
                            ui.close_menu();
                        }
                    }
                });
            }

            if self.settings.debug {
                if ui.button("debug").clicked() {
                    println!("{:#?}", self.board);
                }
            }
            if let Some(name) = place_preset {
                self.place_preset(
                    &name,
                    self.sim_menu.view.create_inv_transform() * self.input.pointer_pos,
                );
            }
        });
        self.exec_action(action, &mut out_event);
        out_event
    }
}
