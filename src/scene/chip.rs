use super::{CombGate, WriteQueue};
use crate::preset::ChipPreset;
use crate::{BitField, ChangedOutput, ChangedOutputs, DeviceInput, LinkTarget};
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
