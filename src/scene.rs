pub mod chip;

use crate::preset::{DevicePreset, PresetData};
use crate::settings::Settings;
use crate::*;
use egui::Color32;
use hashbrown::HashMap;
use tinyrand::RandRange;

pub use chip::Chip;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SceneItem {
    Device(u64),
    DeviceInput(u64, usize),
    DeviceOutput(u64, usize),
    DeviceOutputLink(u64, usize, usize),
    InputPin(u64),
    InputBulb(u64),
    InputLink(u64, usize),
    InputGroup(u64),
    OutputPin(u64),
    OutputBulb(u64),
    OutputGroup(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Write<T> {
    pub target: LinkTarget<T>,
    pub state: bool,
    pub delay: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteQueue<T>(pub Vec<Write<T>>);
impl<T: Clone + PartialEq> WriteQueue<T> {
    #[inline(always)]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, target: LinkTarget<T>, state: bool) {
        // If there is already a queued write to the same target, this write must eventually execute after it,
        // by making sure it's delay is greater than the already queued event.
        let mut rand = crate::RAND.lock().unwrap();
        for write in &mut self.0 {
            if write.target == target {
                write.delay += rand.next_range(0u64..3) as u8;
                write.state = state;
                return;
            }
        }
        self.0.push(Write {
            target,
            state,
            delay: rand.next_range(0u64..3) as u8,
        });
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
            output: table.get(0),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceData {
    CombGate(CombGate),
    Chip(Chip),
}
impl DeviceData {
    pub fn from_preset(preset: &PresetData) -> Self {
        match preset {
            PresetData::CombGate(e) => Self::CombGate(CombGate::new(e.table.clone())),
            PresetData::Chip(e) => Self::Chip(Chip::from_preset(e)),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub pos: Pos2,
    pub data: DeviceData,
    pub links: Vec<Vec<LinkTarget<u64>>>,
    pub name: String,
    pub color: Color32,

    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
}
impl Device {
    pub fn from_preset(preset: &DevicePreset, pos: Pos2) -> Self {
        let [r, g, b, a] = preset.color;
        Self {
            pos,
            data: DeviceData::from_preset(&preset.data),
            links: vec![vec![]; preset.data.num_outputs()],
            name: preset.name.clone(),
            color: Color32::from_rgba_premultiplied(r, g, b, a),
            input_names: preset.data.inputs().to_vec(),
            output_names: preset.data.outputs().to_vec(),
        }
    }

    #[inline(always)]
    pub fn num_inputs(&self) -> usize {
        self.input_names.len()
    }
    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.output_names.len()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
    pub name: String,
    pub y_pos: f32,
    pub state: bool,
    pub links: Vec<DeviceInput<u64>>,
    pub group_member: Option<u64>,
}
impl Input {
    pub fn new(y_pos: f32) -> Self {
        Self {
            name: String::new(),
            y_pos,
            state: false,
            links: Vec::new(),
            group_member: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Output {
    pub name: String,
    pub y_pos: f32,
    pub state: bool,
    pub group_member: Option<u64>,
}
impl Output {
    pub fn new(y_pos: f32) -> Self {
        Self {
            name: String::new(),
            y_pos,
            state: false,
            group_member: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub rect: Rect,
    pub write_queue: WriteQueue<u64>,

    pub inputs: HashMap<u64, Input>,
    pub outputs: HashMap<u64, Output>,
    pub devices: HashMap<u64, Device>,

    pub input_groups: HashMap<u64, Group>,
    pub output_groups: HashMap<u64, Group>,
}
impl Scene {
    pub fn new() -> Self {
        Self {
            rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            write_queue: WriteQueue::new(),

            inputs: HashMap::new(),
            outputs: HashMap::new(),
            devices: HashMap::new(),

            input_groups: HashMap::new(),
            output_groups: HashMap::new(),
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

    pub fn group_value(group: &Group, states: &[bool]) -> String {
        let mut value: i64 = 0;
        let mut bit_value: i64 = 1;
        let mut last_idx = 0;

        if group.lsb_top {
            for idx in 0..group.members.len() - 1 {
                if states[idx] {
                    value += bit_value;
                }
                bit_value *= 2;
            }
            last_idx = group.members.len() - 1;
        } else {
            for idx in (1..group.members.len()).rev() {
                if states[idx] {
                    value += bit_value;
                }
                bit_value *= 2;
            }
        }
        if states[last_idx] {
            if group.signed {
                bit_value *= -1;
            }
            value += bit_value;
        }
        if group.hex {
            format!("{:X}", value)
        } else {
            format!("{}", value)
        }
    }

    pub fn inputs_sorted(&self) -> Vec<u64> {
        let mut keys: Vec<_> = self.inputs.keys().cloned().collect();
        keys.sort_by(|a, b| {
            let a_y = self.inputs.get(a).unwrap().y_pos;
            let b_y = self.inputs.get(b).unwrap().y_pos;
            a_y.partial_cmp(&b_y).unwrap()
        });
        keys
    }
    pub fn outputs_sorted(&self) -> Vec<u64> {
        let mut keys: Vec<_> = self.outputs.keys().cloned().collect();
        keys.sort_by(|a, b| {
            let a_y = self.outputs.get(a).unwrap().y_pos;
            let b_y = self.outputs.get(b).unwrap().y_pos;
            a_y.partial_cmp(&b_y).unwrap()
        });
        keys
    }
}
impl Scene {
    pub fn add_device(&mut self, id: u64, device: Device) {
        self.devices.insert(id, device);
    }

    pub fn drag_device(&mut self, id: u64, drag: Vec2) {
        self.devices.get_mut(&id).unwrap().pos += drag;
    }

    pub fn del_device(&mut self, id: u64) {
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

    pub fn set_device_input(&mut self, id: u64, input: usize, state: bool) {
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
    pub fn get_device_input(&self, device: u64, input: usize) -> Option<bool> {
        Some(self.devices.get(&device)?.data.input().get(input))
    }
    #[inline(always)]
    pub fn get_device_output(&self, device: u64, output: usize) -> Option<bool> {
        Some(self.devices.get(&device)?.data.output().get(output))
    }
}
impl Scene {
    pub fn add_input(&mut self, y: f32) {
        self.inputs.insert(rand_id(), Input::new(y));
    }

    pub fn set_input(&mut self, input: u64, state: bool) {
        let Some(input) = self.inputs.get_mut(&input) else { return };
        input.state = state;
        for target in input.links.clone() {
            self.write_queue.push(target.wrap(), state);
        }
    }

    pub fn drag_input(&mut self, id: u64, drag: Vec2) {
        let input = self.inputs.get_mut(&id).unwrap();
        input.y_pos += drag.y;
        if let Some(group_id) = input.group_member {
            let group = self.input_groups.get_mut(&group_id).unwrap();
            for member_id in group.members.clone() {
                if member_id == id {
                    continue;
                }
                self.inputs.get_mut(&member_id).unwrap().y_pos += drag.y;
            }
        }
    }

    pub fn del_input(&mut self, id: u64) {
        let group_member = self.inputs.get(&id).unwrap().group_member;
        let Some(group_id) = group_member else {
        	self.inputs.remove(&id).unwrap();
        	return;
        };
        let members = self.input_groups.get(&group_id).unwrap().members.clone();
        for member_id in members {
            self.inputs.remove(&member_id);
        }
        self.input_groups.remove(&group_id);
    }

    pub fn stack_input(&mut self, id: u64, settings: &Settings) {
        let input = self.inputs.get(&id).unwrap();
        let state = input.state;
        let name = input.name.clone();
        let y_pos = input.y_pos;

        fn new_name(name: &str, i: usize) -> String {
            if name.trim().is_empty() {
                return String::new();
            }
            format!("{}{}", name, i)
        }

        let sp = settings.scene_pin_col_w;
        if let Some(group_id) = input.group_member {
            let group = self.input_groups.get(&group_id).unwrap();
            let first_input = self.inputs.get(&group.members[0]).unwrap();
            let new_name = new_name(&first_input.name, group.members.len());
            let bottom_y = group.input_bottom_y(self);

            let group = self.input_groups.get_mut(&group_id).unwrap();
            let new_id = rand_id();
            group.members.push(new_id);

            let input = Input {
                y_pos: bottom_y + sp,
                group_member: Some(group_id),
                links: Vec::new(),
                name: new_name,
                state,
            };
            self.inputs.insert(new_id, input);
        } else {
            let group_id = rand_id();
            let new_id = rand_id();
            self.input_groups
                .insert(group_id, Group::new(vec![id, new_id]));
            self.inputs.get_mut(&id).unwrap().group_member = Some(group_id);

            let name = new_name(&name, 1);
            let input = Input {
                y_pos: y_pos + sp,
                group_member: Some(group_id),
                links: Vec::new(),
                name,
                state,
            };
            self.inputs.insert(new_id, input);
        }
    }

    pub fn unstack_input(&mut self, id: u64) {
        let Some(group_id) = self.inputs.get(&id).unwrap().group_member else {
        	return
        };
        let group = self.input_groups.get_mut(&group_id).unwrap();
        let member = group.members.pop().unwrap();

        if group.members.len() == 1 {
            let last_member = group.members[0];
            self.input_groups.remove(&group_id);
            self.inputs.get_mut(&id).unwrap().group_member = None;
            self.inputs.get_mut(&last_member).unwrap().group_member = None;
        }
        self.inputs.remove(&member);
    }
}
impl Scene {
    pub fn add_output(&mut self, y: f32) {
        self.outputs.insert(rand_id(), Output::new(y));
    }

    pub fn drag_output(&mut self, id: u64, drag: Vec2) {
        let output = self.outputs.get_mut(&id).unwrap();
        output.y_pos += drag.y;
        if let Some(group_id) = output.group_member {
            let group = self.output_groups.get_mut(&group_id).unwrap();
            for member_id in group.members.clone() {
                if member_id == id {
                    continue;
                }
                self.outputs.get_mut(&member_id).unwrap().y_pos += drag.y;
            }
        }
    }

    pub fn del_output(&mut self, id: u64) {
        let group_member = self.outputs.get(&id).unwrap().group_member;
        let Some(group_id) = group_member else {
        	self.outputs.remove(&id).unwrap();
        	return;
        };
        let members = self.output_groups.get(&group_id).unwrap().members.clone();
        for member_id in members {
            self.outputs.remove(&member_id);
        }
        self.output_groups.remove(&group_id);
    }

    pub fn stack_output(&mut self, id: u64, settings: &Settings) {
        let output = self.outputs.get(&id).unwrap();
        let state = output.state;
        let name = output.name.clone();
        let y_pos = output.y_pos;

        fn new_name(name: &str, i: usize) -> String {
            if name.trim().is_empty() {
                return String::new();
            }
            format!("{}{}", name, i)
        }

        let sp = settings.scene_pin_col_w;
        if let Some(group_id) = output.group_member {
            let group = self.output_groups.get(&group_id).unwrap();
            let first_output = self.outputs.get(&group.members[0]).unwrap();
            let new_name = new_name(&first_output.name, group.members.len());
            let bottom_y = group.output_bottom_y(self);

            let new_id = rand_id();
            let group = self.output_groups.get_mut(&group_id).unwrap();
            group.members.push(new_id);

            let output = Output {
                y_pos: bottom_y + sp,
                group_member: Some(group_id),
                name: new_name,
                state,
            };
            self.outputs.insert(new_id, output);
        } else {
            let group_id = rand_id();
            let new_id = rand_id();
            self.output_groups
                .insert(group_id, Group::new(vec![id, new_id]));
            self.outputs.get_mut(&id).unwrap().group_member = Some(group_id);

            let name = new_name(&name, 1);
            let output = Output {
                y_pos: y_pos + sp,
                group_member: Some(group_id),
                name,
                state,
            };
            self.outputs.insert(new_id, output);
        }
    }

    pub fn unstack_output(&mut self, id: u64) {
        let Some(group_id) = self.outputs.get(&id).unwrap().group_member else {
        	return
        };
        let group = self.output_groups.get_mut(&group_id).unwrap();
        let member = group.members.pop().unwrap();

        if group.members.len() == 1 {
            let last_member = group.members[0];
            self.output_groups.remove(&group_id);
            self.outputs.get_mut(&id).unwrap().group_member = None;
            self.outputs.get_mut(&last_member).unwrap().group_member = None;
        }
        self.outputs.remove(&member);
    }
}
impl Scene {
    pub fn add_link(&mut self, link: NewLink<u64>) {
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

    #[inline(always)]
    pub fn get_link_target(&self, target: LinkTarget<u64>) -> Option<bool> {
        match target {
            LinkTarget::DeviceInput(device, input) => {
                let device = self.devices.get(&device)?;
                Some(device.data.input().get(input))
            }
            LinkTarget::Output(output) => Some(self.outputs.get(&output)?.state),
        }
    }
    #[inline(always)]
    pub fn get_link_start(&self, start: LinkStart<u64>) -> Option<bool> {
        match start {
            LinkStart::DeviceOutput(device, output) => {
                let device = self.devices.get(&device)?;
                Some(device.data.output().get(output))
            }
            LinkStart::Input(input) => Some(self.inputs.get(&input)?.state),
        }
    }

    pub fn remove_link_to(&mut self, target: LinkTarget<u64>) -> bool {
        for (_, input) in &mut self.inputs {
            for link_idx in 0..input.links.len() {
                if input.links[link_idx].wrap() == target {
                    input.links.remove(link_idx);
                    return true;
                }
            }
        }
        for (_, device) in &mut self.devices {
            for output_links in &mut device.links {
                for link_idx in 0..output_links.len() {
                    if output_links[link_idx] == target {
                        output_links.remove(link_idx);
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub lsb_top: bool,
    pub signed: bool,
    pub hex: bool,
    pub members: Vec<u64>,
}
impl Group {
    pub fn new(members: Vec<u64>) -> Self {
        Self {
            lsb_top: true,
            signed: true,
            hex: false,
            members,
        }
    }

    pub fn input_bottom_y(&self, scene: &Scene) -> f32 {
        scene
            .inputs
            .get(self.members.last().unwrap())
            .unwrap()
            .y_pos
    }
    pub fn output_bottom_y(&self, scene: &Scene) -> f32 {
        scene
            .outputs
            .get(self.members.last().unwrap())
            .unwrap()
            .y_pos
    }
}
