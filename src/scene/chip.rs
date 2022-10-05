use super::{CombGate, SetOutput};
use crate::{preset, BitField, LinkTarget, Presets, SimId};

#[derive(Debug, Clone)]
pub struct Device {
    pub preset: SimId,
    pub links: Vec<Vec<LinkTarget<usize>>>,
    // DeviceData::CombGate is illegal
    pub data: DeviceData,
}
pub type DeviceData = crate::DeviceData<bool, !, CombGate>;
impl DeviceData {
    pub fn write_input(&mut self, input: usize, state: bool, set_outputs: &mut Vec<SetOutput>) {
        match self {
            Self::CombGate(e) => e.write_input(input, state, set_outputs),
            Self::Light(e) => *e = state,
            Self::Switch(_) => panic!("a switch doesnt have inputs"),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Chip {
    pub writes: Vec<Write>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub devices: Vec<Device>,
}
impl Chip {
    pub fn from_preset(preset: &preset::chip::Chip, presets: &Presets) -> Self {
        let inputs = preset
            .inputs
            .iter()
            .map(|input| Input {
                state: false,
                links: input.links.clone(),
            })
            .collect();
        let outputs = preset
            .outputs
            .iter()
            .map(|_| Output { state: false })
            .collect();

        let mut writes = Vec::new();
        let mut devices = Vec::new();
        for device in &preset.devices {
            let device_preset = presets.get(&device.preset).unwrap();
            let data = match device_preset {
                preset::DeviceData::CombGate(e) => {
                    let output = e.table.get(BitField(0));
                    for i in 0..e.table.num_outputs {
                        if output.get(i) {
                            writes.extend(
                                device.links[i]
                                    .iter()
                                    .map(|link| Write::new(link.clone(), true)),
                            );
                        }
                    }

                    DeviceData::CombGate(CombGate {
                        preset: device.preset,
                        input: BitField(0),
                        output,
                        table: e.table.clone(),
                    })
                }
                preset::DeviceData::Chip(_) => panic!("a chip preset shouldn't contain a chip"),
                preset::DeviceData::Light(_) => DeviceData::Light(false),
                preset::DeviceData::Switch(_) => DeviceData::Switch(false),
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

    pub fn update_link(
        &mut self,
        link: LinkTarget<usize>,
        state: bool,
        set_outputs: &mut Vec<SetOutput>,
        writes: &mut Vec<Write>,
    ) {
        match link {
            LinkTarget::Output(output) => {
                set_outputs.push(SetOutput { output, state });
                self.outputs[output].state = state;
            }
            LinkTarget::DeviceInput(device, input) => {
                let device = &mut self.devices[device];

                let mut device_set_outputs = Vec::new();
                device
                    .data
                    .write_input(input, state, &mut device_set_outputs);

                for set_output in device_set_outputs {
                    for link in device.links[set_output.output].clone() {
                        writes.push(Write::new(link, set_output.state));
                    }
                }
            }
        }
    }

    pub fn write_input(&mut self, input: usize, state: bool, set_outputs: &mut Vec<SetOutput>) {
        if self.inputs[input].state == state {
            return;
        }

        self.inputs[input].state = state;

        let mut new_writes = Vec::new();
        for link in self.inputs[input].links.clone() {
            self.update_link(link, state, set_outputs, &mut new_writes);
        }
        self.writes.extend(new_writes);
    }

    pub fn update(&mut self, set_outputs: &mut Vec<SetOutput>) {
        let mut writes = Vec::with_capacity(self.writes.len());

        std::mem::swap(&mut writes, &mut self.writes);

        let mut new_writes = Vec::new();

        for write in writes {
            if write.delay > 0 {
                self.writes.push(write.dec_delay());
                continue;
            }

            self.update_link(write.target, write.state, set_outputs, &mut new_writes);
        }
        self.writes.extend(new_writes);
    }
}
impl crate::IoAccess<bool> for Chip {
    #[inline(always)]
    fn num_inputs(&self) -> usize {
        self.inputs.len()
    }
    #[inline(always)]
    fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    #[inline(always)]
    fn get_input(&self, input: usize) -> bool {
        self.inputs[input].state
    }
    #[inline(always)]
    fn get_output(&self, output: usize) -> bool {
        self.outputs[output].state
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
    pub fn dec_delay(&self) -> Self {
        Self {
            delay: self.delay - 1,
            target: self.target.clone(),
            state: self.state,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub state: bool,
    pub links: Vec<LinkTarget<usize>>,
}
#[derive(Debug, Clone)]
pub struct Output {
    pub state: bool,
}
