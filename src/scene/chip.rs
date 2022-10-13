use super::{CombGate, SetOutput};
use crate::{preset, BitField, IntId, LinkTarget, WithLinks};

#[derive(Debug, Clone)]
pub enum DeviceData {
    CombGate(CombGate),
}
impl DeviceData {
    pub fn set_input(&mut self, input: usize, state: bool, set_outputs: &mut Vec<SetOutput>) {
        match self {
            Self::CombGate(e) => e.set_input(input, state, set_outputs),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    pub preset: IntId,
    pub links: Vec<Vec<LinkTarget<usize>>>,
    pub data: DeviceData,
}

#[derive(Debug, Clone)]
pub struct Io {
    pub state: bool,
}

#[derive(Default, Debug, Clone)]
pub struct Chip {
    pub writes: Vec<Write>,
    pub inputs: Vec<WithLinks<Io, usize>>,
    pub outputs: Vec<Io>,
    pub devices: Vec<Device>,
}
impl Chip {
    pub fn from_preset(preset: &preset::chip::Chip, presets: &preset::Presets) -> Self {
        let inputs = preset
            .inputs
            .iter()
            .map(|input| input.map_item(|_| Io { state: false }))
            .collect();
        let outputs = preset.outputs.iter().map(|_| Io { state: false }).collect();

        let mut writes = Vec::new();
        let mut devices = Vec::new();

        for device in &preset.devices {
            let device_preset = presets.get_preset(device.preset).unwrap();
            let data = match device_preset {
                preset::Preset::CombGate(e) => {
                    let output = e.table.get(BitField::single(0));
                    // for any gate output that is on, queue a write for the links
                    for i in 0..e.table.num_outputs {
                        if !output.get(i) {
                            continue;
                        }
                        writes.extend(device.links[i as usize].iter().map(Write::new_on));
                    }

                    DeviceData::CombGate(CombGate {
                        preset: device.preset,
                        input: BitField::single(0),
                        output,
                        table: e.table.clone(),
                    })
                }
                _ => unreachable!(),
            };
            devices.push(Device {
                preset: device.preset,
                data,
                links: device.links.clone(),
            });
        }

        Self {
            writes,
            inputs,
            outputs,
            devices,
        }
    }

    pub fn set_link_target(
        &mut self,
        target: LinkTarget<usize>,
        state: bool,
        set_outputs: &mut Vec<SetOutput>,
    ) {
        // `set_outputs`: outputs of *this chip* that were set by this link
        match target {
            LinkTarget::Output(output) => {
                set_outputs.push(SetOutput { output, state });
                self.outputs[output].state = state;
            }
            LinkTarget::DeviceInput(device, input) => {
                let device = &mut self.devices[device];

                // these `new_set_outputs` are not outputs of this chip
                // so they are not pushed to `set_outputs`.
                // they are instead stored as writes, to be handled next update
                let mut new_set_outputs = Vec::new();
                device.data.set_input(input, state, &mut new_set_outputs);

                for SetOutput { output, state } in new_set_outputs {
                    for target in device.links[output].clone() {
                        self.writes.push(Write::new(target, state));
                    }
                }
            }
        }
    }

    pub fn set_input(&mut self, input: usize, state: bool, set_outputs: &mut Vec<SetOutput>) {
        self.inputs[input].item.state = state;

        for link in self.inputs[input].links.clone() {
            self.set_link_target(link, state, set_outputs);
        }
    }

    pub fn update(&mut self, set_outputs: &mut Vec<SetOutput>) {
        // most writes will have a delay > 0,
        // so it's more efficient to allocate space for them all here
        let mut writes = Vec::with_capacity(self.writes.len());

        std::mem::swap(&mut writes, &mut self.writes);

        for write in writes {
            if write.delay > 0 {
                self.writes.push(write.dec_delay());
                continue;
            }
            self.set_link_target(write.target, write.state, set_outputs);
        }
    }
}
/// GETTERS
impl Chip {
    #[inline(always)]
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }
    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    #[inline(always)]
    pub fn get_input(&self, input: usize) -> Option<bool> {
        Some(self.inputs.get(input)?.item.state)
    }
    #[inline(always)]
    pub fn get_output(&self, output: usize) -> Option<bool> {
        Some(self.outputs.get(output)?.state)
    }
}

#[derive(Debug, Clone)]
pub struct Write {
    pub delay: u8,
    pub target: LinkTarget<usize>,
    pub state: bool,
}
impl Write {
    #[inline(always)]
    pub fn new(target: LinkTarget<usize>, state: bool) -> Self {
        Self {
            delay: fastrand::u8(1..4),
            target,
            state,
        }
    }

    #[inline(always)]
    pub fn new_on(target: &LinkTarget<usize>) -> Self {
        Self::new(target.clone(), true)
    }

    #[inline(always)]
    pub fn dec_delay(&self) -> Self {
        Self {
            delay: self.delay - 1,
            target: self.target.clone(),
            state: self.state,
        }
    }
}
