use super::{CombGate, SetOutput};
use crate::{preset, BitField, DeviceInput, LinkTarget};

#[derive(Debug, Clone)]
pub struct Device {
    pub links: Vec<Vec<LinkTarget<usize>>>,
    pub data: CombGate,
}

#[derive(Debug, Clone)]
pub struct Input {
    pub state: bool,
    pub links: Vec<DeviceInput<usize>>,
}
#[derive(Debug, Clone)]
pub struct Output {
    pub state: bool,
}

#[derive(Default, Debug, Clone)]
pub struct Chip {
    pub writes: Vec<Write>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub devices: Vec<Device>,
}
impl Chip {
    pub fn from_preset(preset: &preset::chip::Chip) -> Self {
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

        for comb_gate in &preset.comb_gates {
            let (num_inputs, num_outputs) =
                (comb_gate.table.num_inputs, comb_gate.table.num_outputs);
            let output = comb_gate.table.get(0);
            // for any gate output that is on, queue a write for the links
            for i in 0..num_outputs {
                if !output.get(i) {
                    continue;
                }
                writes.extend(comb_gate.links[i as usize].iter().map(Write::new_on));
            }

            let data = CombGate {
                input: BitField::empty(num_inputs as u8),
                output,
                table: comb_gate.table.clone(),
            };
            devices.push(Device {
                data,
                links: comb_gate.links.clone(),
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
        self.inputs[input].state = state;

        for link in self.inputs[input].links.clone() {
            self.set_link_target(link.wrap(), state, set_outputs);
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
        Some(self.inputs.get(input)?.state)
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
