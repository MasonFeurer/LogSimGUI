use crate::integration::FrameInput;
use crate::preset::Presets;
use egui::*;

#[derive(Default)]
pub struct PresetPlacerResponse {
    pub hovered: bool,
    pub picked: Option<String>,
    pub has_focus: bool,
}

#[derive(Clone)]
pub struct PresetPlacer {
    // A search query into self.presets
    pub field: String,
    // The search results from field
    pub results: Vec<String>,
    // If we are searching a category name (with ":cat")
    pub results_cat: Option<String>,
    pub recent: Vec<String>,
}
impl PresetPlacer {
    pub fn default() -> Self {
        Self {
            field: String::new(),
            results: Vec::new(),
            results_cat: None,
            recent: Vec::new(),
        }
    }

    pub fn push_recent(&mut self, preset: &str) {
        if let Some(idx) = self.recent.iter().position(|e| e.as_str() == preset) {
            self.recent.remove(idx);
        }
        self.recent.insert(0, String::from(preset));
        if self.recent.len() > 10 {
            self.recent.pop();
        }
    }

    pub fn check_recent(&mut self, presets: &Presets) {
        for idx in (0..self.recent.len()).rev() {
            if presets.get_preset(&self.recent[idx]).is_none() {
                self.recent.remove(idx);
            }
        }
    }

    pub fn show(
        &mut self,
        pos: Pos2,
        ui: &mut Ui,
        input: &FrameInput,
        presets: &Presets,
        request_focus: bool,
    ) -> PresetPlacerResponse {
        let mut output = PresetPlacerResponse::default();

        let size = Vec2::new(200.0, 20.0);
        let rect = Rect::from_min_size(pos, size);

        let mut grab_preset = None;
        let mut field_changed = false;
        let mut entered = false;
        let mut field_rs = None;

        let mut ui = ui.child_ui(rect, ui.layout().clone());
        let frame_rs = Frame::menu(ui.style()).show(&mut ui, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.text_edit_width = 100.0;
                ui.style_mut().spacing.item_spacing = Vec2::new(5.0, 0.0);
                ui.style_mut().spacing.button_padding = Vec2::ZERO;

                let rs = ui.add(TextEdit::singleline(&mut self.field).hint_text("Search presets"));
                if request_focus {
                    rs.request_focus();
                    self.field = String::new();
                }
                if rs.has_focus() {
                    output.has_focus = true;
                }
                if rs.lost_focus() && input.pressed(Key::Enter) {
                    entered = true;
                }

                if rs.changed() {
                    field_changed = true;
                }
                let presets = if self.field.is_empty() {
                    self.check_recent(presets);
                    &self.recent
                } else {
                    &self.results
                };
                for result in presets {
                    if ui.button(result).clicked() {
                        grab_preset = Some(result.clone());
                    }
                }
                field_rs = Some(rs);
            })
        });
        let field_rs = field_rs.unwrap();

        output.hovered = frame_rs.response.rect.contains(input.pointer_pos);
        if let Some(preset) = grab_preset {
            output.picked = Some(preset);
        }
        if entered && self.results.len() >= 1 {
            let preset = self.results[0].clone();
            output.picked = Some(preset);
            field_rs.request_focus();
        }
        if field_changed {
            self.results_cat = None;
            // If the search field starts with ':', show results of the cat name given
            if self.field.starts_with(':') {
                if let Some(cat) = presets.search_cats(&self.field[1..]) {
                    self.results = presets.cat_presets(&cat);
                    self.results_cat = Some(cat);
                } else {
                    self.results = Vec::new();
                }
            } else {
                self.results = presets.search_presets(&self.field);
            }
        }
        output
    }
}
