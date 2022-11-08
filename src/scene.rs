pub mod chip;

use crate::graphics::View;
use crate::preset::PinPreset;
use crate::settings::Settings;
use crate::*;
use hashbrown::HashMap;

pub use chip::Chip;

// :WRITE
#[derive(Debug, Clone)]
pub struct Write<T> {
    pub target: LinkTarget<T>,
    pub state: bool,
    pub delay: u8,
}

#[derive(Debug, Clone)]
pub struct WriteQueue<T>(pub Vec<Write<T>>);
impl<T: Clone + PartialEq> WriteQueue<T> {
    #[inline(always)]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, target: LinkTarget<T>, state: bool) {
        // if there is already a queued write to the same target, this write must eventually execute after it,
        // by making sure it's delay is greater than the already queued event.
        // let mut min_delay = 0;
        for write in &mut self.0 {
            if write.target == target {
                write.delay += fastrand::u8(0..3);
                write.state = state;
                return;
            }
            // min_delay = std::cmp::max(min_delay, write.delay);
        }
        // let delay = min_delay + fastrand::u8(1..6);
        self.0.push(Write {
            target,
            state,
            // delay,
            delay: fastrand::u8(0..3),
        })
    }

    pub fn update(&mut self, ready: &mut Vec<Write<T>>) {
        let mut keep = Vec::with_capacity(self.0.len());
        for write in &self.0 {
            if write.delay == 0 {
                ready.push(write.clone());
            } else {
                keep.push(Write {
                    delay: write.delay - 1,
                    target: write.target.clone(),
                    state: write.state,
                });
            }
        }
        self.0 = keep;
    }
}

#[derive(Debug, Clone)]
pub struct SetOutput {
    pub output: usize,
    pub state: bool,
}

// :COMBGATE
#[derive(Clone, Debug)]
pub struct CombGate {
    pub input: BitField,
    pub output: BitField,
    pub table: TruthTable,
}
impl CombGate {
    pub fn new(table: TruthTable) -> Self {
        Self {
            input: BitField {
                len: table.num_inputs,
                data: 0,
            },
            output: table.map[0],
            table,
        }
    }

    pub fn set_input(&mut self, input: usize, state: bool, set_outputs: &mut Vec<SetOutput>) {
        self.input.set(input, state);
        let result = self.table.get(self.input.data as usize);

        if result == self.output {
            return;
        }

        for i in 0..self.output.len() {
            if self.output.get(i) == result.get(i) {
                continue;
            }
            set_outputs.push(SetOutput {
                output: i,
                state: result.get(i),
            });
        }
        self.output = result;
    }
}

// :DDATA
#[derive(Debug, Clone)]
pub enum DeviceData {
    CombGate(CombGate),
    Chip(Chip),
}
impl DeviceData {
    pub fn from_preset(preset: &preset::PresetData) -> Self {
        match preset {
            preset::PresetData::CombGate(e) => Self::CombGate(CombGate::new(e.table.clone())),
            preset::PresetData::Chip(e) => Self::Chip(Chip::from_preset(e)),
        }
    }

    pub fn set_input(&mut self, input: usize, state: bool, set_outputs: &mut Vec<SetOutput>) {
        match self {
            Self::CombGate(e) => e.set_input(input, state, set_outputs),
            Self::Chip(e) => e.set_input(input, state, set_outputs),
        }
    }

    #[inline(always)]
    pub fn input(&self) -> BitField {
        match self {
            Self::CombGate(e) => e.input,
            Self::Chip(e) => e.input,
        }
    }
    #[inline(always)]
    pub fn output(&self) -> BitField {
        match self {
            Self::CombGate(e) => e.output,
            Self::Chip(e) => e.output,
        }
    }
}

// :DEVICE
#[derive(Debug, Clone)]
pub struct Device {
    pub pos: Pos2,
    pub data: DeviceData,
    pub links: Vec<Vec<LinkTarget<IntId>>>,
    pub name: String,
    pub color: Color,

    pub input_presets: Vec<PinPreset>,
    pub output_presets: Vec<PinPreset>,
}
impl Device {
    pub fn new(
        pos: Pos2,
        data: DeviceData,
        name: String,
        color: Color,
        input_presets: Vec<PinPreset>,
        output_presets: Vec<PinPreset>,
    ) -> Self {
        Self {
            pos,
            data,
            links: vec![Vec::new(); output_presets.len()],
            name,
            color,

            input_presets,
            output_presets,
        }
    }

    #[inline(always)]
    pub fn pos(&self, view: &View) -> Pos2 {
        view.map_pos(self.pos)
    }

    #[inline(always)]
    pub fn size(&self, settings: &Settings, view: &View) -> Vec2 {
        view.map_vec(settings.device_size(
            self.input_presets.len(),
            self.output_presets.len(),
            &self.name,
        ))
    }
    #[inline(always)]
    pub fn rect(&self, settings: &Settings, view: &View) -> Rect {
        Rect::from_min_size(self.pos(view), self.size(settings, view))
    }

    #[inline(always)]
    pub fn input_pins(&self, settings: &Settings, view: &View) -> Vec<Pin> {
        Pin::spread_presets(
            &self.input_presets,
            self.pos(view),
            self.size(settings, view).y,
            true,
            false,
        )
    }
    #[inline(always)]
    pub fn output_pins(&self, settings: &Settings, view: &View) -> Vec<Pin> {
        let size = self.size(settings, view);
        Pin::spread_presets(
            &self.output_presets,
            self.pos(view) + Vec2::new(size.x, 0.0),
            size.y,
            false,
            false,
        )
    }
}

// :IO
#[derive(Clone, Debug)]
pub struct Input {
    pub name: String,
    pub y_pos: f32,
    pub state: bool,
    pub links: Vec<DeviceInput<IntId>>,
}
impl Input {
    pub fn new(y_pos: f32) -> Self {
        Self {
            name: String::new(),
            y_pos,
            state: false,
            links: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Output {
    pub name: String,
    pub y_pos: f32,
    pub state: bool,
}
impl Output {
    pub fn new(y_pos: f32) -> Self {
        Self {
            name: String::new(),
            y_pos,
            state: false,
        }
    }
}

// :SCENE DECL
#[derive(Debug)]
pub struct Scene {
    pub rect: Rect,
    pub write_queue: WriteQueue<IntId>,

    pub inputs: HashMap<IntId, Input>,
    pub outputs: HashMap<IntId, Output>,
    pub devices: HashMap<IntId, Device>,
}
impl Scene {
    pub fn new() -> Self {
        Self {
            rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            write_queue: WriteQueue::new(),

            inputs: HashMap::new(),
            outputs: HashMap::new(),
            devices: HashMap::new(),
        }
    }

    pub fn update(&mut self) {
        let mut ready_writes = Vec::new();
        self.write_queue.update(&mut ready_writes);
        for write in ready_writes {
            match write.target {
                LinkTarget::DeviceInput(device, input) => {
                    let Some(device) = self.devices.get_mut(&device) else { return };
                    let mut set_outputs = Vec::new();

                    device.data.set_input(input, write.state, &mut set_outputs);

                    for SetOutput { output, state } in set_outputs {
                        for target in &device.links[output] {
                            self.write_queue.push(target.clone(), state);
                        }
                    }
                }
                LinkTarget::Output(output) => {
                    let Some(output) = self.outputs.get_mut(&output) else { return };
                    output.state = write.state;
                }
            }
        }

        // executed the writes for the scene, but contained chips may have queued writes
        for (_, device) in &mut self.devices {
            let DeviceData::Chip(chip) = &mut device.data else { continue };

            let mut set_outputs = Vec::new();
            chip.update(&mut set_outputs);

            for SetOutput { output, state } in set_outputs {
                for target in &device.links[output] {
                    self.write_queue.push(target.clone(), state);
                }
            }
        }
    }
}
// :SCENE DEVICES
impl Scene {
    pub fn add_device(&mut self, id: IntId, device: Device) {
        self.devices.insert(id, device);
    }

    pub fn drag_device(&mut self, id: IntId, drag: Vec2) {
        self.devices.get_mut(&id).unwrap().pos += drag;
    }

    pub fn del_device(&mut self, id: IntId) {
        let device = self.devices.get(&id).unwrap();
        for output_idx in 0..device.data.output().len() {
            if device.data.output().get(output_idx) == false {
                continue;
            }
            for link in &device.links[output_idx] {
                self.write_queue.push(link.clone(), false);
            }
        }
        self.devices.remove(&id).unwrap();
    }

    pub fn set_device_input(&mut self, id: IntId, input: usize, state: bool) {
        let Some(device) = self.devices.get_mut(&id) else { return };
        let mut set_outputs = Vec::new();

        device.data.set_input(input, state, &mut set_outputs);

        for SetOutput { output, state } in set_outputs {
            for target in &device.links[output] {
                self.write_queue.push(target.clone(), state);
            }
        }
    }

    #[inline(always)]
    pub fn get_device_input(&self, device: IntId, input: usize) -> Option<bool> {
        Some(self.devices.get(&device)?.data.input().get(input))
    }
    #[inline(always)]
    pub fn get_device_output(&self, device: IntId, output: usize) -> Option<bool> {
        Some(self.devices.get(&device)?.data.output().get(output))
    }
}
// :SCENE INPUTS
impl Scene {
    pub fn set_input(&mut self, input: IntId, state: bool) {
        let Some(input) = self.inputs.get_mut(&input) else { return };
        input.state = state;
        for target in input.links.clone() {
            self.write_queue.push(target.wrap(), state);
        }
    }

    pub fn add_input(&mut self, id: IntId, input: Input) {
        self.inputs.insert(id, input);
    }

    pub fn drag_input(&mut self, id: IntId, drag: Vec2) {
        self.inputs.get_mut(&id).unwrap().y_pos += drag.y;
    }

    pub fn del_input(&mut self, id: IntId) {
        self.inputs.remove(&id).unwrap();
    }

    pub fn get_input(&self, id: IntId) -> Option<&Input> {
        self.inputs.get(&id)
    }
}
// :SCENE OUTPUTS
impl Scene {
    #[inline(always)]
    pub fn add_output(&mut self, id: IntId, output: Output) {
        self.outputs.insert(id, output);
    }

    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    pub fn get_output(&self, id: IntId) -> Option<&Output> {
        self.outputs.get(&id)
    }

    pub fn drag_output(&mut self, id: IntId, drag: Vec2) {
        self.outputs.get_mut(&id).unwrap().y_pos += drag.y;
    }

    pub fn del_output(&mut self, id: IntId) {
        self.outputs.remove(&id).unwrap();
    }
}
// :SCENE ADD ITEMS
impl Scene {
    pub fn add_link(&mut self, link: NewLink<IntId>) {
        match link {
            NewLink::InputToDeviceInput(input, target) => {
                let input = self.inputs.get_mut(&input).unwrap();
                input.links.push(target.clone());

                self.write_queue.push(target.wrap(), input.state);
            }
            NewLink::DeviceOutputTo(device, output, target) => {
                let device = self.devices.get_mut(&device).unwrap();
                device.links[output].push(target.clone());
                let state = device.data.output().get(output);

                self.write_queue.push(target, state);
            }
        }
    }
}
// :SCENE DEFS
impl Scene {
    #[inline(always)]
    pub fn input_pin<'a>(&self, input: &'a Input, settings: &Settings, view: &View) -> Pin<'a> {
        Pin {
            origin: Pos2::new(
                self.rect.min.x + settings.scene_pin_col_w,
                view.map_y(input.y_pos),
            ),
            left: false,
            large: true,
            name: input.name.as_str(),
        }
    }
    #[inline(always)]
    pub fn output_pin<'a>(&self, output: &'a Output, settings: &Settings, view: &View) -> Pin<'a> {
        Pin {
            origin: Pos2::new(
                self.rect.max.x - settings.scene_pin_col_w,
                view.map_y(output.y_pos),
            ),
            left: true,
            large: true,
            name: output.name.as_str(),
        }
    }

    #[inline(always)]
    pub fn get_link_target_pin(
        &self,
        target: &LinkTarget<IntId>,
        settings: &Settings,
        view: &View,
    ) -> Option<Pin> {
        match target {
            LinkTarget::DeviceInput(device, input) => {
                let device = self.devices.get(device)?;
                Some(device.input_pins(settings, view)[*input].clone())
            }
            LinkTarget::Output(output) => {
                Some(self.output_pin(self.outputs.get(output)?, settings, view))
            }
        }
    }
    #[inline(always)]
    pub fn get_link_start_pin(
        &self,
        start: &LinkStart<IntId>,
        settings: &Settings,
        view: &View,
    ) -> Option<Pin> {
        match start {
            LinkStart::DeviceOutput(device, output) => {
                let device = self.devices.get(device)?;
                Some(device.output_pins(settings, view)[*output].clone())
            }
            LinkStart::Input(input) => {
                Some(self.input_pin(self.inputs.get(input)?, settings, view))
            }
        }
    }

    #[inline(always)]
    pub fn get_link_target_state(&self, target: &LinkTarget<IntId>) -> Option<bool> {
        match target {
            LinkTarget::DeviceInput(device, input) => {
                let device = self.devices.get(device)?;
                Some(device.data.input().get(*input))
            }
            LinkTarget::Output(output) => Some(self.outputs.get(output)?.state),
        }
    }
    #[inline(always)]
    pub fn get_link_start_state(&self, start: &LinkStart<IntId>) -> Option<bool> {
        match start {
            LinkStart::DeviceOutput(device, output) => {
                let device = self.devices.get(device)?;
                Some(device.data.output().get(*output))
            }
            LinkStart::Input(input) => Some(self.inputs.get(input)?.state),
        }
    }

    #[inline(always)]
    pub fn get_link_target(
        &self,
        target: &LinkTarget<IntId>,
        settings: &Settings,
        view: &View,
    ) -> Option<(Pin, bool)> {
        match target {
            LinkTarget::DeviceInput(device, input) => {
                let device = self.devices.get(device)?;
                Some((
                    device.input_pins(settings, view)[*input].clone(),
                    device.data.input().get(*input),
                ))
            }
            LinkTarget::Output(output) => {
                let output = self.outputs.get(output)?;
                Some((self.output_pin(output, settings, view), output.state))
            }
        }
    }
    #[inline(always)]
    pub fn get_link_start(
        &self,
        start: &LinkStart<IntId>,
        settings: &Settings,
        view: &View,
    ) -> Option<(Pin, bool)> {
        match start {
            LinkStart::DeviceOutput(device, output) => {
                let device = self.devices.get(device)?;
                Some((
                    device.output_pins(settings, view)[*output].clone(),
                    device.data.output().get(*output),
                ))
            }
            LinkStart::Input(input) => {
                let input = self.inputs.get(input)?;
                Some((self.input_pin(input, settings, view), input.state))
            }
        }
    }
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SceneLayout {
//     pub inputs: HashMap<IntId, InputLayout>,
//     pub outputs: HashMap<IntId, OutputLayout>,
//     pub devices: HashMap<IntId, DeviceLayout>,
// }
