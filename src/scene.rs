pub mod chip;

use crate::graphics::DEVICE_NAME_CHAR_W;
use crate::*;
use std::collections::HashMap;

pub use chip::Chip;

#[derive(Debug, Clone)]
pub struct SetOutput {
    pub output: usize,
    pub state: bool,
}

#[derive(Debug, Clone)]
pub struct Write {
    pub target: LinkTarget<IntId>,
    pub state: bool,
}

#[derive(Debug, Clone)]
// TODO rename to `DeviceState`
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

    pub fn num_inputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_inputs(),
            Self::Chip(e) => e.num_inputs(),
        }
    }
    pub fn num_outputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_outputs(),
            Self::Chip(e) => e.num_outputs(),
        }
    }
    pub fn get_input(&self, input: usize) -> Option<bool> {
        match self {
            Self::CombGate(e) => Some(e.get_input(input)),
            Self::Chip(e) => e.get_input(input),
        }
    }
    pub fn get_output(&self, output: usize) -> Option<bool> {
        match self {
            Self::CombGate(e) => Some(e.get_output(output)),
            Self::Chip(e) => e.get_output(output),
        }
    }
}

pub fn device_input_defs(rect: Rect, num_io: usize, out: &mut Vec<IoDef>) {
    let sp = rect.height() / (num_io + 1) as f32;

    let mut y = sp + rect.min.y;
    for _ in 0..num_io {
        out.push(IoDef {
            pos: Pos2::new(rect.min.x, y),
            size: IoSize::Small,
            dir: IoDir::Left,
        });
        y += sp;
    }
}
pub fn device_output_defs(rect: Rect, num_io: usize, out: &mut Vec<IoDef>) {
    let sp = rect.height() / (num_io + 1) as f32;

    let mut y = sp + rect.min.y;
    for _ in 0..num_io {
        out.push(IoDef {
            pos: Pos2::new(rect.max.x, y),
            size: IoSize::Small,
            dir: IoDir::Right,
        });
        y += sp;
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    pub pos: Pos2,
    pub size: Vec2,
    // TODO rename to `state`
    pub data: DeviceData,
    pub links: Vec<Vec<LinkTarget<IntId>>>,
    pub vis: DeviceVisuals,

    pub input_presets: Vec<preset::Io>,
    pub output_presets: Vec<preset::Io>,
    pub input_defs: Vec<IoDef>,
    pub output_defs: Vec<IoDef>,
}
impl Device {
    pub fn new(
        vis: DeviceVisuals,
        data: DeviceData,
        pos: Pos2,
        input_presets: Vec<preset::Io>,
        output_presets: Vec<preset::Io>,
    ) -> Self {
        let (num_inputs, num_outputs) = (data.num_inputs(), data.num_outputs());

        let height = f32::max(
            graphics::io_presets_height(&input_presets),
            graphics::io_presets_height(&output_presets),
        );
        let width = vis.name.len() as f32 * DEVICE_NAME_CHAR_W;

        let size = Vec2::new(width, height);
        let rect = Rect::from_min_size(pos, size);

        let mut input_defs = Vec::with_capacity(num_inputs);
        device_input_defs(rect, num_inputs, &mut input_defs);

        let mut output_defs = Vec::with_capacity(num_outputs);
        device_output_defs(rect, num_outputs, &mut output_defs);

        Self {
            pos,
            size,
            data,
            links: vec![Vec::new(); num_outputs],
            vis,

            input_defs,
            output_defs,
            input_presets,
            output_presets,
        }
    }

    #[inline(always)]
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.size)
    }

    pub fn drag(&mut self, v: Vec2) {
        let (num_inputs, num_outputs) = (self.input_defs.len(), self.output_defs.len());
        self.pos += v;
        self.input_defs.clear();
        self.output_defs.clear();
        let rect = self.rect();
        device_input_defs(rect, num_inputs, &mut self.input_defs);
        device_output_defs(rect, num_outputs, &mut self.output_defs);
    }
}

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

    #[inline(always)]
    pub fn num_inputs(&self) -> usize {
        self.table.num_inputs as usize
    }
    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.table.num_outputs as usize
    }

    #[inline(always)]
    pub fn get_output(&self, output: usize) -> bool {
        self.output.get(output as u8)
    }
    #[inline(always)]
    pub fn get_input(&self, input: usize) -> bool {
        self.input.get(input as u8)
    }

    pub fn set_input(&mut self, input: usize, state: bool, set_outputs: &mut Vec<SetOutput>) {
        self.input.set(input as u8, state);
        let result = self.table.get(self.input.data as usize);

        if result == self.output {
            return;
        }

        for i in 0..self.num_outputs() {
            if self.output.get(i as u8) == result.get(i as u8) {
                continue;
            }
            set_outputs.push(SetOutput {
                output: i,
                state: result.get(i as u8),
            });
        }
        self.output = result;
    }
}

#[derive(Clone, Debug)]
pub struct Input {
    pub preset: preset::Io,
    pub y_pos: f32,
    pub state: bool,
    pub links: Vec<DeviceInput<IntId>>,
}
impl Input {
    pub fn new(y_pos: f32) -> Self {
        Self {
            preset: preset::Io::new(),
            y_pos,
            state: false,
            links: Vec::new(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Output {
    pub preset: preset::Io,
    pub y_pos: f32,
    pub state: bool,
}
impl Output {
    pub fn new(y_pos: f32) -> Self {
        Self {
            preset: preset::Io::new(),
            y_pos,
            state: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub rect: Rect,
    pub inputs: HashMap<IntId, Input>,
    pub outputs: HashMap<IntId, Output>,
    pub devices: HashMap<IntId, Device>,
    pub writes: Vec<Write>,
}
impl Scene {
    pub fn new() -> Self {
        Self {
            rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            devices: HashMap::new(),
            writes: Vec::new(),
        }
    }

    pub fn exec_write(&mut self, write: Write, new_writes: &mut Vec<Write>) {
        match write.target {
            LinkTarget::DeviceInput(device, input) => {
                let Some(device) = self.devices.get_mut(&device) else { return };
                let mut set_outputs = Vec::new();

                device.data.set_input(input, write.state, &mut set_outputs);

                for SetOutput { output, state } in set_outputs {
                    for target in device.links[output].clone() {
                        new_writes.push(Write { target, state });
                    }
                }
            }
            LinkTarget::Output(output) => {
                let Some(output) = self.outputs.get_mut(&output) else { return };
                output.state = write.state;
            }
        }
    }

    pub fn update(&mut self) {
        let mut writes = Vec::new();
        std::mem::swap(&mut writes, &mut self.writes);
        let mut new_writes = Vec::new();

        for write in writes {
            self.exec_write(write, &mut new_writes);
        }
        self.writes = new_writes;

        // executed the writes for the scene, but contained chips may have queued writes
        for (_, device) in &mut self.devices {
            let DeviceData::Chip(chip) = &mut device.data else { continue };

            let mut set_outputs = Vec::new();
            chip.update(&mut set_outputs);

            for SetOutput { output, state } in set_outputs {
                for target in device.links[output].clone() {
                    self.writes.push(Write { target, state });
                }
            }
        }
    }
}
/// SETTERS
impl Scene {
    pub fn set_input(&mut self, input: IntId, state: bool) {
        let Some(input) = self.inputs.get_mut(&input) else { return };
        input.state = state;
        for target in input.links.clone() {
            self.writes.push(Write {
                target: target.wrap(),
                state,
            });
        }
    }
    pub fn set_device_input(&mut self, device: IntId, input: usize, state: bool) {
        let Some(device) = self.devices.get_mut(&device) else { return };
        let mut set_outputs = Vec::new();

        device.data.set_input(input, state, &mut set_outputs);

        for SetOutput { output, state } in set_outputs {
            for target in device.links[output].clone() {
                self.writes.push(Write { target, state });
            }
        }
    }
}

/// ADD ITEMS
impl Scene {
    pub fn add_input(&mut self, input: Input) -> IntId {
        let id = IntId::new();
        self.inputs.insert(id, input);
        id
    }

    pub fn add_output(&mut self, output: Output) -> IntId {
        let id = IntId::new();
        self.outputs.insert(id, output);
        id
    }

    pub fn add_device(&mut self, device: Device) -> IntId {
        let id = IntId::new();
        self.devices.insert(id, device);
        id
    }

    pub fn add_link(&mut self, link: NewLink<IntId>) {
        match link {
            NewLink::InputToDeviceInput(input, device_input) => {
                let input = self.inputs.get_mut(&input).unwrap();
                input.links.push(device_input.clone());

                self.writes.push(Write {
                    state: input.state,
                    target: device_input.wrap(),
                });
            }
            NewLink::DeviceOutputTo(device, output, target) => {
                let device = self.devices.get_mut(&device).unwrap();
                device.links[output].push(target.clone());
                let state = device.data.get_output(output).unwrap();

                self.writes.push(Write { state, target });
            }
        }
    }
}

/// GETTERS
impl Scene {
    #[inline(always)]
    pub fn get_input_def(&self, input: &Input) -> IoDef {
        IoDef {
            pos: Pos2::new(self.rect.min.x, input.y_pos),
            size: IoSize::Large,
            dir: IoDir::Right,
        }
    }
    #[inline(always)]
    pub fn get_output_def(&self, output: &Output) -> IoDef {
        IoDef {
            pos: Pos2::new(self.rect.max.x, output.y_pos),
            size: IoSize::Large,
            dir: IoDir::Left,
        }
    }

    pub fn get_link_target_def(&self, target: &LinkTarget<IntId>) -> Option<IoDef> {
        Some(self.get_link_target(target)?.0)
    }
    pub fn get_link_start_def(&self, start: &LinkStart<IntId>) -> Option<IoDef> {
        Some(self.get_link_start(start)?.0)
    }

    #[inline(always)]
    pub fn get_link_target(&self, target: &LinkTarget<IntId>) -> Option<(IoDef, bool)> {
        match target {
            LinkTarget::DeviceInput(device, input) => {
                let device = self.devices.get(device)?;
                Some((
                    device.input_defs.get(*input)?.clone(),
                    device.data.get_input(*input)?,
                ))
            }
            LinkTarget::Output(output) => {
                let output = self.outputs.get(output)?;
                Some((self.get_output_def(output), output.state))
            }
        }
    }
    #[inline(always)]
    pub fn get_link_start(&self, start: &LinkStart<IntId>) -> Option<(IoDef, bool)> {
        match start {
            LinkStart::DeviceOutput(device, output) => {
                let device = self.devices.get(device)?;
                Some((
                    device.output_defs.get(*output)?.clone(),
                    device.data.get_output(*output)?,
                ))
            }
            LinkStart::Input(input) => {
                let input = &self.inputs.get(input)?.clone();
                Some((self.get_input_def(input), input.state))
            }
        }
    }

    #[inline(always)]
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }
    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    #[inline(always)]
    pub fn get_input(&self, input: IntId) -> Option<bool> {
        Some(self.inputs.get(&input)?.state)
    }
    #[inline(always)]
    pub fn get_output(&self, output: IntId) -> Option<bool> {
        Some(self.outputs.get(&output)?.state)
    }

    #[inline(always)]
    pub fn get_device_input(&self, device: IntId, input: usize) -> Option<bool> {
        self.devices.get(&device)?.data.get_input(input)
    }
    #[inline(always)]
    pub fn get_device_output(&self, device: IntId, output: usize) -> Option<bool> {
        self.devices.get(&device)?.data.get_output(output)
    }
}
