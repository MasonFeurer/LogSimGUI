pub mod chip;

use crate::*;
use std::collections::HashMap;

pub use chip::Chip;

pub const IO_COL_W: f32 = 40.0;
pub const IO_SIZE: Pos2 = Pos2::new(30.0, 10.0);
pub const DEVICE_IO_SIZE: Pos2 = Pos2::new(15.0, 8.0);

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
pub enum DeviceData {
    CombGate(CombGate),
    Chip(Chip),
}
impl DeviceData {
    pub fn from_preset(id: IntId, preset: &preset::Preset, presets: &preset::Presets) -> Self {
        match preset {
            preset::Preset::CombGate(e) => Self::CombGate(CombGate::new(id, e.table.clone())),
            preset::Preset::Chip(e) => Self::Chip(Chip::from_preset(e, presets)),
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

    pub fn size(&self) -> Pos2 {
        const DEVICE_IO_SP: f32 = DEVICE_IO_SIZE.y + 5.0;

        let num_ios = std::cmp::max(self.num_inputs(), self.num_outputs());
        let height = num_ios as f32 * DEVICE_IO_SP + DEVICE_IO_SP;

        Pos2::new(50.0, height)
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    pub preset: IntId,
    pub pos: Pos2,
    pub data: DeviceData,
    pub links: Vec<Vec<LinkTarget<IntId>>>,
    rel_input_locs: Vec<Pos2>,
    rel_output_locs: Vec<Pos2>,
}
impl Device {
    pub fn new(preset_id: IntId, preset: &preset::Preset, data: DeviceData, pos: Pos2) -> Self {
        let (num_inputs, num_outputs) = (data.num_inputs(), data.num_outputs());
        let size = data.size();

        let rel_input_locs = (0..num_inputs)
            .map(|idx| {
                Pos2::new(
                    -size.x * 0.5,
                    size.y * preset.get_input_loc(idx).unwrap() - size.y * 0.5,
                )
            })
            .collect();
        let rel_output_locs = (0..num_outputs)
            .map(|idx| {
                Pos2::new(
                    size.x * 0.5,
                    size.y * preset.get_output_loc(idx).unwrap() - size.y * 0.5,
                )
            })
            .collect();
        Self {
            preset: preset_id,
            data,
            pos,
            links: vec![Vec::new(); num_outputs],
            rel_input_locs,
            rel_output_locs,
        }
    }

    #[inline(always)]
    pub fn get_input_def(&self, input: usize) -> Option<IoDef> {
        let rel = self.rel_input_locs.get(input)?;
        Some(IoDef {
            y: self.pos.y + rel.y,
            h: DEVICE_IO_SIZE.y,
            base_x: self.pos.x + rel.x,
            tip_x: self.pos.x + rel.x - DEVICE_IO_SIZE.x,
        })
    }
    #[inline(always)]
    pub fn get_output_def(&self, output: usize) -> Option<IoDef> {
        let rel = self.rel_output_locs.get(output)?;
        Some(IoDef {
            y: self.pos.y + rel.y,
            h: DEVICE_IO_SIZE.y,
            base_x: self.pos.x + rel.x,
            tip_x: self.pos.x + rel.x + DEVICE_IO_SIZE.x,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CombGate {
    pub preset: IntId,
    pub input: BitField,
    pub output: BitField,
    pub table: TruthTable,
}
impl CombGate {
    pub fn new(preset: IntId, table: TruthTable) -> Self {
        Self {
            preset,
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
        let result = self.table.get(self.input);

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

#[derive(Debug, Clone)]
pub struct Io {
    pub preset: preset::Io,
    pub state: bool,
}
impl Io {
    pub fn default_at(y_pos: f32) -> Self {
        Self {
            preset: preset::Io::default_at(y_pos),
            state: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub rect: Rect,
    pub inputs: HashMap<IntId, WithLinks<Io, IntId>>,
    pub outputs: HashMap<IntId, Io>,
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
        let Some(WithLinks { item: input, links }) = self.inputs.get_mut(&input) else { return };
        input.state = state;
        for target in links.clone() {
            self.writes.push(Write { target, state });
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
    pub fn add_input(&mut self, input: Io) -> IntId {
        let id = IntId::new();
        self.inputs.insert(id, WithLinks::none(input));
        id
    }

    pub fn add_output(&mut self, output: Io) -> IntId {
        let id = IntId::new();
        self.outputs.insert(id, output);
        id
    }

    pub fn add_device(&mut self, device: Device) -> IntId {
        let id = IntId::new();
        self.devices.insert(id, device);
        id
    }

    pub fn add_link(&mut self, start: LinkStart<IntId>, target: LinkTarget<IntId>) {
        match start {
            LinkStart::Input(input) => {
                let input = self.inputs.get_mut(&input).unwrap();
                input.links.push(target.clone());
                let state = input.item.state;

                self.writes.push(Write { state, target });
            }
            LinkStart::DeviceOutput(device, output) => {
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
    pub fn get_input_def(&self, input: &Io) -> IoDef {
        IoDef {
            y: input.preset.y_pos,
            h: IO_SIZE.y,
            base_x: self.rect.min.x,
            tip_x: self.rect.min.x + IO_SIZE.x,
        }
    }
    pub fn get_output_def(&self, output: &Io) -> IoDef {
        IoDef {
            y: output.preset.y_pos,
            h: IO_SIZE.y,
            base_x: self.rect.max.x,
            tip_x: self.rect.max.x - IO_SIZE.x,
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
                    device.get_input_def(*input)?,
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
                    device.get_output_def(*output)?,
                    device.data.get_output(*output)?,
                ))
            }
            LinkStart::Input(input) => {
                let input = &self.inputs.get(input)?.item;
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
        Some(self.inputs.get(&input)?.item.state)
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
