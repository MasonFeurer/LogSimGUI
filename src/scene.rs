pub mod chip;

use crate::preset;
use crate::{BitField, SimId, TruthTable};
use eframe::egui::Pos2;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ChangedOutput {
    pub output: usize,
    pub state: bool,
}

#[derive(Debug, Clone)]
pub struct Write {
    pub target: WriteTarget,
    pub state: bool,
}
#[derive(Clone, Debug)]
pub enum WriteTarget {
    DeviceInput(SimId, usize),
    SceneOutput(SimId),
}
#[derive(Clone, Debug)]
pub enum LinkStart {
    SceneInput(SimId),
    DeviceOutput(SimId, usize),
}

#[derive(Debug, Clone)]
pub struct Device {
    pub preset: SimId,
    pub pos: Pos2,
    pub data: DeviceData,
    pub links: Vec<Vec<WriteTarget>>,
    pub input_locs: Vec<Pos2>,
    pub output_locs: Vec<Pos2>,
}
impl Device {
    pub fn new(preset: SimId, data: DeviceData, pos: Pos2) -> Self {
        let num_inputs = data.num_inputs();
        let num_outputs = data.num_outputs();
        Self {
            preset,
            data,
            pos,
            links: vec![Vec::new(); num_outputs],
            input_locs: vec![Pos2::new(0.0, 0.0); num_inputs],
            output_locs: vec![Pos2::new(0.0, 0.0); num_outputs],
        }
    }

    #[inline(always)]
    pub fn links_for_output(&self, output: usize) -> Vec<WriteTarget> {
        self.links[output].clone()
    }
}

#[derive(Debug, Clone)]
pub enum DeviceData {
    CombGate(CombGate),
    Chip(chip::Chip),
    Light(bool),
    Switch(bool),
}
impl DeviceData {
    pub fn set_input(
        &mut self,
        input: usize,
        state: bool,
        changed_outputs: &mut Vec<ChangedOutput>,
    ) {
        match self {
            Self::CombGate(e) => e.set_input(input, state, changed_outputs),
            Self::Chip(e) => e.set_input(input, state, changed_outputs),
            Self::Light(e) => {
                assert_eq!(input, 0);
                *e = state;
            }
            Self::Switch(_) => panic!("a switch doent have an input"),
        }
    }

    pub fn get_output(&self, output: usize) -> bool {
        match self {
            Self::CombGate(e) => e.get_output(output),
            Self::Chip(e) => e.get_output(output),
            Self::Light(_) => panic!("a light doesnt have an output"),
            Self::Switch(state) => {
                assert_eq!(output, 0);
                *state
            }
        }
    }
    pub fn get_input(&self, input: usize) -> bool {
        match self {
            Self::CombGate(e) => e.get_input(input),
            Self::Chip(e) => e.get_input(input),
            Self::Light(state) => {
                assert_eq!(input, 0);
                *state
            }
            Self::Switch(_) => panic!("a switch doesnt have an input"),
        }
    }

    pub fn num_inputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_inputs(),
            Self::Chip(e) => e.num_inputs(),
            Self::Light(_) => 1,
            Self::Switch(_) => 0,
        }
    }
    pub fn num_outputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_outputs(),
            Self::Chip(e) => e.num_outputs(),
            Self::Light(_) => 0,
            Self::Switch(_) => 1,
        }
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
            input: BitField(0),
            output: table.map[0],
            table,
        }
    }

    #[inline(always)]
    pub fn num_inputs(&self) -> usize {
        self.table.num_inputs
    }
    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.table.num_outputs
    }

    #[inline(always)]
    pub fn get_output(&self, output: usize) -> bool {
        self.output.get(output)
    }
    #[inline(always)]
    pub fn get_input(&self, input: usize) -> bool {
        self.input.get(input)
    }

    pub fn set_input(
        &mut self,
        input: usize,
        state: bool,
        changed_outputs: &mut Vec<ChangedOutput>,
    ) {
        self.input.set(input, state);
        let result = self.table.get(self.input);

        if result == self.output {
            return;
        }

        for i in 0..self.num_outputs() {
            if self.output.get(i) == result.get(i) {
                continue;
            }
            changed_outputs.push(ChangedOutput {
                output: i,
                state: result.get(i),
            });
        }
        self.output = result;
    }
}

#[derive(Debug, Clone, Default)]
pub struct Input {
    pub label: preset::IoLabel,
    pub state: bool,
    pub links: Vec<WriteTarget>,
}
#[derive(Debug, Default, Clone)]
pub struct Output {
    pub label: preset::IoLabel,
    pub state: bool,
}

#[derive(Debug)]
pub struct Scene {
    pub name: String,
    pub color: [f32; 3],
    pub combinational: bool,
    pub inputs: HashMap<SimId, Input>,
    pub outputs: HashMap<SimId, Output>,
    pub devices: HashMap<SimId, Device>,
    pub writes: Vec<Write>,
}
impl Scene {
    pub fn new() -> Self {
        Self {
            name: format!("NewChip {}", fastrand::u16(10000..)),
            color: [1.0; 3],
            combinational: false,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            devices: HashMap::new(),
            writes: Vec::new(),
        }
    }

    pub fn get_target_loc(
        &self,
        ctx: &crate::graphics::Context,
        target: WriteTarget,
    ) -> Option<Pos2> {
        match target {
            WriteTarget::DeviceInput(device, input) => {
                Some(self.devices.get(&device)?.input_locs[input])
            }
            WriteTarget::SceneOutput(output) => {
                use crate::graphics::{calc_io_unsized_locs, EDITOR_IO_SIZE, EDITOR_IO_SP};

                let output = self.outputs.iter().position(|(key, _)| *key == output)?;
                let output_locs = calc_io_unsized_locs(
                    Pos2::new(
                        ctx.canvas_rect.max.x - EDITOR_IO_SIZE.x,
                        ctx.canvas_rect.min.y,
                    ),
                    self.outputs.len(),
                    EDITOR_IO_SP,
                );
                Some(output_locs[output])
            }
        }
    }

    pub fn get_link_start_loc(
        &self,
        ctx: &crate::graphics::Context,
        link_start: LinkStart,
    ) -> Option<Pos2> {
        match link_start {
            LinkStart::DeviceOutput(device, output) => {
                Some(self.devices.get(&device)?.output_locs[output])
            }
            LinkStart::SceneInput(input) => {
                use crate::graphics::{calc_io_unsized_locs, EDITOR_IO_SIZE, EDITOR_IO_SP};

                let input = self.inputs.iter().position(|(key, _)| *key == input)?;
                let input_locs = calc_io_unsized_locs(
                    Pos2::new(
                        ctx.canvas_rect.min.x + EDITOR_IO_SIZE.x,
                        ctx.canvas_rect.min.y,
                    ),
                    self.inputs.len(),
                    EDITOR_IO_SP,
                );
                Some(input_locs[input])
            }
        }
    }

    pub fn write(&mut self, target: WriteTarget, state: bool) {
        match target {
            WriteTarget::DeviceInput(device, input) => {
                let Some(device) = self.devices.get_mut(&device) else { return };
                let mut changed_outputs = Vec::new();

                device.data.set_input(input, state, &mut changed_outputs);

                for changed_output in changed_outputs {
                    let links = device.links_for_output(changed_output.output);

                    for target in links {
                        self.writes.push(Write {
                            target,
                            state: changed_output.state,
                        });
                    }
                }
            }
            WriteTarget::SceneOutput(output) => {
                let Some(output) = self.outputs.get_mut(&output) else { return };
                output.state = state;
            }
        }
    }

    pub fn update(&mut self) {
        let mut writes = Vec::with_capacity(self.writes.len());
        std::mem::swap(&mut writes, &mut self.writes);

        for write in writes {
            self.write(write.target, write.state);
        }

        for (_, device) in &mut self.devices {
            if let DeviceData::Chip(chip) = &mut device.data {
                let mut changed_outputs = Vec::new();
                chip.update(&mut changed_outputs);

                for changed_output in changed_outputs {
                    for target in device.links_for_output(changed_output.output) {
                        self.writes.push(Write {
                            target,
                            state: changed_output.state,
                        });
                    }
                }
            }
        }
    }

    pub fn set_input(&mut self, input: SimId, state: bool) {
        let Some(input) = self.inputs.get_mut(&input) else { return };
        if input.state == state {
            return;
        }
        input.state = state;
        for target in input.links.clone() {
            self.writes.push(Write { target, state });
        }
    }
    pub fn set_device_input(&mut self, device: SimId, input: usize, state: bool) {
        let Some(device) = self.devices.get_mut(&device) else { return };
        if device.data.get_input(input) == state {
            return;
        }
        let mut changed_outputs = Vec::new();

        device.data.set_input(input, state, &mut changed_outputs);

        for ChangedOutput { output, state } in changed_outputs {
            for target in device.links[output].clone() {
                self.writes.push(Write { target, state });
            }
        }
    }

    #[inline(always)]
    pub fn get_input(&self, input: SimId) -> bool {
        self.inputs.get(&input).unwrap().state
    }
    #[inline(always)]
    pub fn get_output(&self, output: SimId) -> bool {
        self.outputs.get(&output).unwrap().state
    }

    #[inline(always)]
    pub fn get_device_input(&self, device: SimId, input: usize) -> bool {
        self.devices.get(&device).unwrap().data.get_input(input)
    }
    #[inline(always)]
    pub fn get_device_output(&self, device: SimId, output: usize) -> bool {
        self.devices.get(&device).unwrap().data.get_output(output)
    }

    pub fn alloc_preset(&mut self, preset_id: SimId, preset: &preset::Device, pos: Pos2) -> SimId {
        let scene_device = match preset {
            preset::Device::CombGate(e) => DeviceData::CombGate(CombGate::new(e.table.clone())),
            preset::Device::Chip(e) => DeviceData::Chip(chip::Chip::from_preset(e)),
            preset::Device::Light => DeviceData::Light(false),
            preset::Device::Switch => DeviceData::Switch(false),
        };
        self.alloc_device(Device::new(preset_id, scene_device, pos))
    }

    pub fn alloc_input(&mut self, input: Input) -> SimId {
        let id = SimId::new();
        self.inputs.insert(id, input);
        id
    }

    pub fn alloc_output(&mut self, output: Output) -> SimId {
        let id = SimId::new();
        self.outputs.insert(id, output);
        id
    }

    pub fn alloc_device(&mut self, device: Device) -> SimId {
        let id = SimId::new();
        self.devices.insert(id, device);
        id
    }

    pub fn add_link(&mut self, start: LinkStart, link: WriteTarget) {
        match start {
            LinkStart::SceneInput(input) => {
                let input = self.inputs.get_mut(&input).unwrap();
                input.links.push(link.clone());
                let state = input.state;

                self.writes.push(Write {
                    state,
                    target: link,
                });
            }
            LinkStart::DeviceOutput(device, output) => {
                let device = self.devices.get_mut(&device).unwrap();
                device.links[output].push(link.clone());
                let state = device.data.get_output(output);

                self.writes.push(Write {
                    state,
                    target: link,
                });
            }
        }
    }
}
