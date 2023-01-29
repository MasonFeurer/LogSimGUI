use crate::presets::{ChipPreset, DevicePreset, PresetData};
use crate::settings::Settings;
use crate::*;
use egui::{Rect, Vec2};
use hashbrown::HashMap;
use tinyrand::{RandRange, Seeded, StdRand};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoardItem {
    Board,
    InputCol,
    OutputCol,
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

pub struct WriteQueue<T> {
    pub writes: Vec<Write<T>>,
    pub buffer: Vec<(LinkTarget<T>, bool)>,
    pub rand: StdRand,
}

use serde::{Deserialize, Deserializer, Serialize, Serializer};
impl<T: Serialize + Clone + PartialEq> Serialize for WriteQueue<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Serialize::serialize(&self.writes, serializer)
    }
}
impl<'de, T: Deserialize<'de> + Clone + PartialEq> Deserialize<'de> for WriteQueue<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let writes: Vec<Write<T>> = Deserialize::deserialize(deserializer)?;
        Ok(Self::new(writes))
    }
}
impl<T: std::fmt::Debug> std::fmt::Debug for WriteQueue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.writes, f)
    }
}
impl<T: Clone> Clone for WriteQueue<T> {
    fn clone(&self) -> Self {
        Self::new(self.writes.clone())
    }
}

impl<T> WriteQueue<T> {
    pub fn new(writes: Vec<Write<T>>) -> Self {
        Self {
            writes,
            buffer: Vec::new(),
            rand: StdRand::seed(rand_id()),
        }
    }
    pub fn empty() -> Self {
        Self::new(vec![])
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.writes.len()
    }

    pub fn clear(&mut self) {
        self.writes.clear();
        self.buffer.clear();
    }
}
impl<T: PartialEq + Clone + Copy> WriteQueue<T> {
    // note: HOT CODE!
    #[inline(always)]
    pub fn push(&mut self, target: LinkTarget<T>, state: bool) {
        self.buffer.push((target, state));
    }

    #[inline(always)] // only one call site
    fn push_raw(&mut self, target: LinkTarget<T>, state: bool) {
        let new_delay = self.rand.next_range(0u64..3) as u8;
        for write in &mut self.writes {
            if write.target == target {
                write.state = state;
                write.delay += new_delay;
                return;
            }
        }
        self.writes.push(Write {
            target,
            state,
            delay: new_delay,
        });
    }

    #[inline(always)]
    pub fn flush(&mut self) {
        for idx in 0..self.buffer.len() {
            let (target, state) = self.buffer[idx];
            self.push_raw(target, state);
        }
        self.buffer.clear();
    }

    // note: HOT CODE!
    #[inline(always)]
    pub fn next(&mut self) -> Option<Write<T>> {
        for idx in 0..self.writes.len() {
            if self.writes[idx].delay == 0 {
                let write = self.writes[idx].clone();
                self.writes.remove(idx);
                return Some(write);
            }
        }
        None
    }

    // Should call after next() returns None, and before flush(),
    // because it expects all writes to have a delay > 0
    #[inline(always)]
    pub fn update(&mut self) {
        for write in &mut self.writes {
            write.delay -= 1;
        }
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
            _ => panic!(),
        }
    }

    pub fn set_input(&mut self, input: usize, state: bool) -> ChangedOutputs {
        match self {
            Self::CombGate(e) => e.set_input(input, state),
            Self::Chip(e) => {
                e.set_input(input, state);
                ChangedOutputs::none()
            }
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
    pub links: Vec<Vec<Link>>,
    pub preset: String,
}
impl Device {
    pub fn from_preset(preset: &DevicePreset, pos: Pos2) -> Self {
        Self {
            pos,
            data: DeviceData::from_preset(&preset.data),
            links: vec![vec![]; preset.data.num_outputs()],
            preset: preset.name.clone(),
        }
    }

    #[inline(always)]
    pub fn num_inputs(&self) -> usize {
        self.data.input().len
    }
    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.data.output().len
    }
}

#[derive(Clone, Copy)]
pub enum IoSel {
    Input,
    Output,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Io {
    pub name: String,
    pub y_pos: f32,
    pub state: bool,
    pub group_member: Option<u64>,
}
impl Io {
    pub fn new(y_pos: f32) -> Self {
        Self {
            name: String::new(),
            y_pos,
            state: false,
            group_member: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
    pub io: Io,
    pub links: Vec<Link>,
}
impl Input {
    pub fn new(io: Io) -> Self {
        Self {
            io,
            links: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Output {
    pub io: Io,
}
impl Output {
    pub fn new(io: Io) -> Self {
        Self { io }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Board {
    /// This is not in screen space, this is in world space
    pub rect: Rect,
    pub write_queue: WriteQueue<u64>,

    pub inputs: HashMap<u64, Input>,
    pub outputs: HashMap<u64, Output>,
    pub devices: HashMap<u64, Device>,

    pub input_groups: HashMap<u64, Group>,
    pub output_groups: HashMap<u64, Group>,
}
impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}
impl Board {
    pub fn new() -> Self {
        Self {
            rect: Rect {
                min: Pos2::new(0.0, 0.0),
                max: Pos2::new(600.0, 400.0),
            },
            write_queue: WriteQueue::empty(),

            inputs: HashMap::new(),
            outputs: HashMap::new(),
            devices: HashMap::new(),

            input_groups: HashMap::new(),
            output_groups: HashMap::new(),
        }
    }

    pub fn item_count(&self) -> usize {
        self.inputs.len() + self.outputs.len() + self.devices.len()
    }

    pub fn update(&mut self) {
        while let Some(write) = self.write_queue.next() {
            match write.target {
                LinkTarget::DeviceInput(device, input) => {
                    let Some(device) = self.devices.get_mut(&device) else { return };

                    let mut changed_outputs = device.data.set_input(input, write.state);
                    while let Some((output, state)) = changed_outputs.next() {
                        for link in &device.links[output] {
                            self.write_queue.push(link.target, state);
                        }
                    }
                }
                LinkTarget::Output(output) => {
                    let Some(output) = self.outputs.get_mut(&output) else { return };
                    output.io.state = write.state;
                }
            }
        }

        // Update the chips on scene
        for (_, device) in &mut self.devices {
            let DeviceData::Chip(chip) = &mut device.data else { continue };

            let mut changed_outputs = chip.update();
            while let Some((output, state)) = changed_outputs.next() {
                for link in &device.links[output] {
                    self.write_queue.push(link.target, state);
                }
            }
        }
        self.write_queue.update();
        self.write_queue.flush();
    }
}
impl Board {
    pub fn add_device(&mut self, id: u64, device: Device) {
        self.devices.insert(id, device);
    }

    pub fn drag_device(&mut self, id: u64, drag: Vec2) {
        self.devices.get_mut(&id).unwrap().pos += drag;
    }

    pub fn remove_device(&mut self, id: u64) {
        let device = self.devices.get(&id).unwrap();
        for output_idx in 0..device.data.output().len {
            if device.data.output().get(output_idx) == false {
                continue;
            }
            for link in &device.links[output_idx] {
                self.write_queue.push(link.target, false);
            }
        }
        self.devices.remove(&id).unwrap();
    }

    pub fn set_device_input(&mut self, id: u64, input: usize, state: bool) {
        let Some(device) = self.devices.get_mut(&id) else { return };

        let mut changed_outputs = device.data.set_input(input, state);
        while let Some((output, state)) = changed_outputs.next() {
            for link in &device.links[output] {
                self.write_queue.push(link.target, state);
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
impl Board {
    pub fn get_io(&self, sel: IoSel, id: u64) -> Option<&Io> {
        match sel {
            IoSel::Input => self.inputs.get(&id).map(|i| &i.io),
            IoSel::Output => self.outputs.get(&id).map(|o| &o.io),
        }
    }
    pub fn mut_io(&mut self, sel: IoSel, id: u64) -> Option<&mut Io> {
        match sel {
            IoSel::Input => self.inputs.get_mut(&id).map(|i| &mut i.io),
            IoSel::Output => self.outputs.get_mut(&id).map(|o| &mut o.io),
        }
    }
    pub fn add_io(&mut self, sel: IoSel, id: u64, io: Io) {
        match sel {
            IoSel::Input => {
                self.inputs.insert(id, Input::new(io));
            }
            IoSel::Output => {
                self.outputs.insert(id, Output::new(io));
            }
        }
    }
    pub fn remove_io_alone(&mut self, sel: IoSel, id: u64) {
        match sel {
            IoSel::Input => {
                self.inputs.remove(&id).unwrap();
            }
            IoSel::Output => {
                self.outputs.remove(&id).unwrap();
            }
        };
    }

    pub fn get_io_group(&self, sel: IoSel, id: u64) -> Option<&Group> {
        match sel {
            IoSel::Input => self.input_groups.get(&id),
            IoSel::Output => self.output_groups.get(&id),
        }
    }
    pub fn mut_io_group(&mut self, sel: IoSel, id: u64) -> Option<&mut Group> {
        match sel {
            IoSel::Input => self.input_groups.get_mut(&id),
            IoSel::Output => self.output_groups.get_mut(&id),
        }
    }
    pub fn insert_io_group(&mut self, sel: IoSel, id: u64, group: Group) {
        match sel {
            IoSel::Input => self.input_groups.insert(id, group),
            IoSel::Output => self.output_groups.insert(id, group),
        };
    }
    pub fn remove_io_group(&mut self, sel: IoSel, id: u64) {
        match sel {
            IoSel::Input => {
                self.input_groups.remove(&id);
            }
            IoSel::Output => {
                self.output_groups.remove(&id);
            }
        };
    }

    pub fn drag_io(&mut self, sel: IoSel, id: u64, drag: Vec2) {
        let io = self.mut_io(sel, id).unwrap();
        if let Some(group_id) = io.group_member {
            let group = self.get_io_group(sel, group_id).unwrap();
            for member_id in group.members.clone() {
                self.mut_io(sel, member_id).unwrap().y_pos += drag.y;
            }
        } else {
            io.y_pos += drag.y;
        }
    }
    pub fn remove_io(&mut self, sel: IoSel, id: u64) {
        let group_member = self.get_io(sel, id).unwrap().group_member;
        let Some(group_id) = group_member else {
        	self.remove_io_alone(sel, id);
        	return;
        };
        let members = self.get_io_group(sel, group_id).unwrap().members.clone();
        for member_id in members {
            self.remove_io_alone(sel, member_id);
        }
        self.remove_io_group(sel, group_id);
    }
    pub fn stack_io(&mut self, sel: IoSel, id: u64, settings: &Settings) {
        let io = self.get_io(sel, id).unwrap();
        let state = io.state;
        let name = io.name.clone();
        let y_pos = io.y_pos;

        fn new_name(name: &str, i: usize) -> String {
            if name.trim().is_empty() {
                return String::new();
            }
            format!("{}{}", name, i)
        }

        let sp = settings.board_io_col_w;
        if let Some(group_id) = io.group_member {
            let group = self.get_io_group(sel, group_id).unwrap();
            let first_member = self.get_io(sel, group.members[0]).unwrap();
            let new_name = new_name(&first_member.name, group.members.len());
            let bottom_y = self
                .get_io(sel, *group.members.last().unwrap())
                .unwrap()
                .y_pos;

            let group = self.mut_io_group(sel, group_id).unwrap();
            let new_id = rand_id();
            group.members.push(new_id);

            let io = Io {
                y_pos: bottom_y + sp,
                group_member: Some(group_id),
                name: new_name,
                state,
            };
            self.add_io(sel, new_id, io);
        } else {
            let group_id = rand_id();
            let new_id = rand_id();
            self.insert_io_group(sel, group_id, Group::new(vec![id, new_id]));
            self.mut_io(sel, id).unwrap().group_member = Some(group_id);

            let io = Io {
                y_pos: y_pos + sp,
                group_member: Some(group_id),
                name: new_name(&name, 1),
                state,
            };
            self.add_io(sel, new_id, io);
        }
    }
    pub fn unstack_io(&mut self, sel: IoSel, id: u64) {
        let Some(group_id) = self.get_io(sel, id).unwrap().group_member else {
        	return
        };
        let group = self.mut_io_group(sel, group_id).unwrap();
        let member = group.members.pop().unwrap();

        if group.members.len() == 1 {
            let last_member = group.members[0];
            self.remove_io_group(sel, group_id);
            self.mut_io(sel, id).unwrap().group_member = None;
            self.mut_io(sel, last_member).unwrap().group_member = None;
        }
        self.remove_io_alone(sel, member);
    }

    pub fn add_input(&mut self, y: f32) {
        self.inputs.insert(rand_id(), Input::new(Io::new(y)));
    }

    pub fn set_input(&mut self, input: u64, state: bool) {
        let Some(input) = self.inputs.get_mut(&input) else { return };
        input.io.state = state;
        for link in &input.links {
            self.write_queue.push(link.target, state);
        }
    }
    pub fn drag_input(&mut self, id: u64, drag: Vec2) {
        self.drag_io(IoSel::Input, id, drag)
    }
    pub fn remove_input(&mut self, id: u64) {
        self.remove_io(IoSel::Input, id)
    }
    pub fn stack_input(&mut self, id: u64, settings: &Settings) {
        self.stack_io(IoSel::Input, id, settings)
    }
    pub fn unstack_input(&mut self, id: u64) {
        self.unstack_io(IoSel::Input, id)
    }

    pub fn add_output(&mut self, y: f32) {
        self.outputs.insert(rand_id(), Output::new(Io::new(y)));
    }
    pub fn drag_output(&mut self, id: u64, drag: Vec2) {
        self.drag_io(IoSel::Output, id, drag)
    }
    pub fn remove_output(&mut self, id: u64) {
        self.remove_io(IoSel::Output, id)
    }
    pub fn stack_output(&mut self, id: u64, settings: &Settings) {
        self.stack_io(IoSel::Output, id, settings)
    }
    pub fn unstack_output(&mut self, id: u64) {
        self.unstack_io(IoSel::Output, id)
    }

    pub fn input_field(&self) -> BitField {
        let mut field = BitField::empty(self.inputs.len());
        let mut idx = 0;
        for (_, input) in &self.inputs {
            field.set(idx, input.io.state);
            idx += 1;
        }
        field
    }
    pub fn output_field(&self) -> BitField {
        let mut field = BitField::empty(self.outputs.len());
        let mut idx = 0;
        for (_, input) in &self.outputs {
            field.set(idx, input.io.state);
            idx += 1;
        }
        field
    }
    pub fn io_field(&self, sel: IoSel) -> BitField {
        match sel {
            IoSel::Input => self.input_field(),
            IoSel::Output => self.output_field(),
        }
    }

    pub fn inputs_sorted(&self) -> Vec<u64> {
        let mut keys: Vec<_> = self.inputs.keys().cloned().collect();
        keys.sort_by(|a, b| {
            let a_y = self.inputs.get(a).unwrap().io.y_pos;
            let b_y = self.inputs.get(b).unwrap().io.y_pos;
            a_y.partial_cmp(&b_y).unwrap()
        });
        keys
    }
    pub fn outputs_sorted(&self) -> Vec<u64> {
        let mut keys: Vec<_> = self.outputs.keys().cloned().collect();
        keys.sort_by(|a, b| {
            let a_y = self.outputs.get(a).unwrap().io.y_pos;
            let b_y = self.outputs.get(b).unwrap().io.y_pos;
            a_y.partial_cmp(&b_y).unwrap()
        });
        keys
    }
}
impl Board {
    pub fn add_link(&mut self, start: LinkStart<u64>, link: Link) {
        self.remove_link_to(link.target);
        let target = link.target;
        match start {
            LinkStart::Input(id) => {
                let input = self.inputs.get_mut(&id).unwrap();
                input.links.push(link);

                self.write_queue.push(target, input.io.state);
            }
            LinkStart::DeviceOutput(id, idx) => {
                let device = self.devices.get_mut(&id).unwrap();
                device.links[idx].push(link);
                let state = device.data.output().get(idx);

                self.write_queue.push(target, state);
            }
        }
    }

    #[inline(always)]
    pub fn link_target_state(&self, target: LinkTarget<u64>) -> Option<bool> {
        match target {
            LinkTarget::DeviceInput(device, input) => {
                let device = self.devices.get(&device)?;
                Some(device.data.input().get(input))
            }
            LinkTarget::Output(output) => Some(self.outputs.get(&output)?.io.state),
        }
    }
    #[inline(always)]
    pub fn link_start_state(&self, start: LinkStart<u64>) -> Option<bool> {
        match start {
            LinkStart::DeviceOutput(device, output) => {
                let device = self.devices.get(&device)?;
                Some(device.data.output().get(output))
            }
            LinkStart::Input(input) => Some(self.inputs.get(&input)?.io.state),
        }
    }

    pub fn remove_link_to(&mut self, target: LinkTarget<u64>) -> bool {
        for (_, input) in &mut self.inputs {
            for link_idx in 0..input.links.len() {
                if input.links[link_idx].target == target {
                    input.links.remove(link_idx);
                    return true;
                }
            }
        }
        for (_, device) in &mut self.devices {
            for links in &mut device.links {
                for link_idx in 0..links.len() {
                    if links[link_idx].target == target {
                        links.remove(link_idx);
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipDevice {
    pub links: Vec<Vec<LinkTarget<usize>>>,
    pub data: CombGate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chip {
    pub write_queue: WriteQueue<usize>,
    pub input: BitField,
    pub output: BitField,
    pub input_links: Vec<Vec<DeviceInput<usize>>>,
    pub devices: Vec<ChipDevice>,
}
impl Chip {
    pub fn from_preset(preset: &ChipPreset) -> Self {
        let input = BitField::empty(preset.inputs.len());
        let output = BitField::empty(preset.outputs.len());
        let input_links = preset.input_links.clone();

        let mut write_queue = WriteQueue::empty();
        let mut devices = Vec::new();

        for comb_gate in &preset.comb_gates {
            let (num_inputs, num_outputs) =
                (comb_gate.table.num_inputs, comb_gate.table.num_outputs);
            let output = comb_gate.table.get(0);

            // for any gate output that is on, queue a write for the links
            for i in 0..num_outputs {
                if !output.get(i) {
                    continue;
                }
                for target in &comb_gate.links[i] {
                    write_queue.push(*target, true);
                }
            }

            let data = CombGate {
                input: BitField::empty(num_inputs),
                output,
                table: comb_gate.table.clone(),
            };
            devices.push(ChipDevice {
                data,
                links: comb_gate.links.clone(),
            });
        }

        Self {
            write_queue,
            input,
            output,
            input_links,
            devices,
        }
    }

    pub fn update(&mut self) -> ChangedOutputs {
        let prev_output = self.output;
        while let Some(write) = self.write_queue.next() {
            self.set_link_target(write.target, write.state);
        }
        self.write_queue.update();
        self.write_queue.flush();
        ChangedOutputs::new(prev_output, self.output)
    }

    pub fn set_input(&mut self, input: usize, state: bool) {
        self.input.set(input, state);

        for DeviceInput(device, input) in self.input_links[input].clone() {
            self.set_device_input(device, input, state);
        }
    }

    #[inline(always)]
    fn set_link_target(&mut self, target: LinkTarget<usize>, state: bool) -> Option<ChangedOutput> {
        match target {
            LinkTarget::Output(output) => {
                self.output.set(output, state);
                Some(ChangedOutput { output, state })
            }
            LinkTarget::DeviceInput(device, input) => {
                self.set_device_input(device, input, state);
                None
            }
        }
    }

    #[inline(always)]
    fn set_device_input(&mut self, device: usize, input: usize, state: bool) {
        let device = &mut self.devices[device];

        let mut changed_outputs = device.data.set_input(input, state);
        while let Some((output, state)) = changed_outputs.next() {
            for target in &device.links[output] {
                self.write_queue.push(*target, state);
            }
        }
    }
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

    pub fn set_input(&mut self, input: usize, state: bool) -> ChangedOutputs {
        self.input.set(input, state);
        let result = self.table.get(self.input.data as usize);
        let prev_output = self.output;
        self.output = result;
        ChangedOutputs::new(prev_output, result)
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

    pub fn field(&self, board: &Board, sel: IoSel) -> BitField {
        let mut field = BitField::empty(self.members.len());
        for (idx, id) in self.members.iter().enumerate() {
            field.set(idx, board.get_io(sel, *id).unwrap().state);
        }
        field
    }

    pub fn display_value(&self, field: BitField) -> String {
        let mut value: i64 = 0;
        let mut bit_value: i64 = 1;
        let mut last_idx = 0;

        if self.lsb_top {
            for idx in 0..self.members.len() - 1 {
                if field.get(idx) {
                    value += bit_value;
                }
                bit_value *= 2;
            }
            last_idx = self.members.len() - 1;
        } else {
            for idx in (1..self.members.len()).rev() {
                if field.get(idx) {
                    value += bit_value;
                }
                bit_value *= 2;
            }
        }
        if field.get(last_idx) {
            if self.signed {
                bit_value *= -1;
            }
            value += bit_value;
        }
        if self.hex {
            format!("{:X}", value)
        } else {
            format!("{}", value)
        }
    }
}
