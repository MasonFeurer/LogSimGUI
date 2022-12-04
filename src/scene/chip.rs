use super::{CombGate, SetOutput, WriteQueue};
use crate::preset::ChipPreset;
use crate::{BitField, DeviceInput, LinkTarget};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub links: Vec<Vec<LinkTarget<usize>>>,
    pub data: CombGate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chip {
    pub write_queue: WriteQueue<usize>,
    pub input: BitField,
    pub output: BitField,
    pub input_links: Vec<Vec<DeviceInput<usize>>>,
    pub devices: Vec<Device>,
}
impl Chip {
    pub fn from_preset(preset: &ChipPreset) -> Self {
        let input = BitField::empty(preset.inputs.len());
        let output = BitField::empty(preset.outputs.len());
        let input_links = preset.input_links.clone();

        let mut write_queue = WriteQueue::new();
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
                for target in &comb_gate.links[i as usize] {
                    write_queue.push(target.clone(), true);
                }
            }

            let data = CombGate {
                input: BitField::empty(num_inputs),
                output,
                table: comb_gate.table.clone(),
            };
            devices.push(Device {
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
                self.output.set(output, state);
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
                        self.write_queue.push(target, state);
                    }
                }
            }
        }
    }

    pub fn set_input(&mut self, input: usize, state: bool, set_outputs: &mut Vec<SetOutput>) {
        self.input.set(input, state);

        for link in self.input_links[input].clone() {
            self.set_link_target(link.wrap(), state, set_outputs);
        }
    }

    pub fn update(&mut self, set_outputs: &mut Vec<SetOutput>) {
        let mut ready_writes = Vec::new();
        self.write_queue.update(&mut ready_writes);
        for write in ready_writes {
            self.set_link_target(write.target, write.state, set_outputs);
        }
    }
}
