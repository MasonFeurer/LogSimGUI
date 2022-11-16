pub mod chip;

use crate::settings::Settings;
use crate::*;
use eframe::egui::Color32;
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombGate {
    pub input: BitField,
    pub output: BitField,
    pub table: TruthTable,
}
impl CombGate {
    pub fn new(table: TruthTable) -> Self {
        let output = *table.map.get(0).unwrap_or(&BitField::empty(0));
        Self {
            input: BitField {
                len: table.num_inputs,
                data: 0,
            },
            output,
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
    pub links: Vec<Vec<LinkTarget<u64>>>,
    pub name: String,
    pub color: Color32,

    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
}
impl Device {
    pub fn new(
        pos: Pos2,
        data: DeviceData,
        name: String,
        color: Color32,
        input_names: Vec<String>,
        output_names: Vec<String>,
    ) -> Self {
        Self {
            pos,
            data,
            links: vec![Vec::new(); output_names.len()],
            name,
            color,

            input_names,
            output_names,
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

// :IO
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

// :SCENE DECL
#[derive(Debug)]
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

    pub fn load_layout(&mut self, layout: SceneLayout) {
        self.write_queue = WriteQueue::new();
        self.inputs = layout.inputs;
        self.outputs = layout.outputs;
        self.input_groups = layout.input_groups;
        self.output_groups = layout.output_groups;
        self.devices.clear();
        for (id, device) in layout.devices {
            self.devices.insert(id, device.to_device());
        }
    }

    pub fn layout(&self) -> SceneLayout {
        let mut devices = HashMap::with_capacity(self.devices.len());
        for (id, device) in &self.devices {
            let data = match &device.data {
                DeviceData::CombGate(e) => DeviceDataLayout::CombGate(e.clone()),
                DeviceData::Chip(e) => DeviceDataLayout::Chip {
                    input: e.input,
                    output: e.output,
                    input_links: e.input_links.clone(),
                    devices: e.devices.clone(),
                },
            };
            devices.insert(
                *id,
                DeviceLayout {
                    pos: device.pos.into(),
                    data,
                    links: device.links.clone(),
                    name: device.name.clone(),
                    color: device.color.to_array(),

                    input_names: device.input_names.clone(),
                    output_names: device.output_names.clone(),
                },
            );
        }
        SceneLayout {
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            input_groups: self.input_groups.clone(),
            output_groups: self.output_groups.clone(),
            devices,
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
}
// :SCENE DEVICES
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
// :SCENE INPUTS
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
}
// :SCENE OUTPUTS
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
}
// :SCENE LINKS
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
    pub fn get_link_target(&self, target: &LinkTarget<u64>) -> Option<bool> {
        match target {
            LinkTarget::DeviceInput(device, input) => {
                let device = self.devices.get(device)?;
                Some(device.data.input().get(*input))
            }
            LinkTarget::Output(output) => Some(self.outputs.get(output)?.state),
        }
    }
    #[inline(always)]
    pub fn get_link_start(&self, start: &LinkStart<u64>) -> Option<bool> {
        match start {
            LinkStart::DeviceOutput(device, output) => {
                let device = self.devices.get(device)?;
                Some(device.data.output().get(*output))
            }
            LinkStart::Input(input) => Some(self.inputs.get(input)?.state),
        }
    }
}

// :GROUP
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

// :LAYOUT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceDataLayout {
    CombGate(CombGate),
    Chip {
        input: BitField,
        output: BitField,
        input_links: Vec<Vec<DeviceInput<usize>>>,
        devices: Vec<chip::Device>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLayout {
    pub pos: [f32; 2],
    pub data: DeviceDataLayout,
    pub links: Vec<Vec<LinkTarget<u64>>>,
    pub name: String,
    pub color: [u8; 4],

    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
}
impl DeviceLayout {
    pub fn to_device(&self) -> Device {
        let data = match &self.data {
            DeviceDataLayout::CombGate(e) => DeviceData::CombGate(e.clone()),
            DeviceDataLayout::Chip {
                input,
                output,
                input_links,
                devices,
            } => DeviceData::Chip(Chip {
                write_queue: WriteQueue::new(),
                input: input.clone(),
                output: output.clone(),
                input_links: input_links.clone(),
                devices: devices.clone(),
            }),
        };
        let [r, g, b, a] = self.color;
        Device {
            pos: self.pos.into(),
            data,
            links: self.links.clone(),
            name: self.name.clone(),
            color: Color32::from_rgba_premultiplied(r, g, b, a),
            input_names: self.input_names.clone(),
            output_names: self.output_names.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneLayout {
    pub inputs: HashMap<u64, Input>,
    pub input_groups: HashMap<u64, Group>,
    pub output_groups: HashMap<u64, Group>,
    pub outputs: HashMap<u64, Output>,
    pub devices: HashMap<u64, DeviceLayout>,
}
