use super::IoLabel;
use crate::scene::{self, Scene};
use crate::{LinkTarget, SimId};
use eframe::egui::{Color32, Pos2};

#[derive(Debug)]
pub struct UnnestedChip {
    pub id: SimId,
    pub chip: scene::chip::Chip,
    pub output_links: Vec<Vec<LinkTarget<SimId>>>,
    pub device_ids: Vec<SimId>,
}
impl UnnestedChip {
    pub fn map_link(&self, link: &LinkTarget<usize>, out: &mut Vec<LinkTarget<SimId>>) {
        match link {
            LinkTarget::Output(output) => {
                out.extend(self.output_links[*output].clone());
            }
            LinkTarget::DeviceInput(device, input) => {
                out.push(LinkTarget::DeviceInput(self.device_ids[*device], *input));
            }
        }
    }
}

#[derive(Debug)]
pub struct Unnester<'a> {
    pub scene: &'a mut Scene,
    pub chips: Vec<UnnestedChip>,
}
impl<'a> Unnester<'a> {
    pub fn new(scene: &'a mut Scene) -> Self {
        Self {
            scene,
            chips: Vec::new(),
        }
    }

    pub fn move_chips(&mut self) {
        let chip_ids: Vec<_> = self
            .scene
            .devices
            .iter()
            .filter_map(|(id, device)| {
                if let scene::DeviceData::Chip(_) = &device.data {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        let chips = chip_ids
            .iter()
            .map(|id| {
                let (chip, output_links) = {
                    let scene::Device { data, links, .. } = self.scene.devices.remove(id).unwrap();
                    let crate::DeviceData::Chip(chip) = data else { panic!() };
                    (chip, links)
                };
                let mut device_ids = Vec::with_capacity(chip.devices.len());
                for _ in 0..chip.devices.len() {
                    device_ids.push(SimId::new());
                }
                UnnestedChip {
                    id: *id,
                    chip,
                    output_links,
                    device_ids,
                }
            })
            .collect();
        self.chips = chips;
    }

    pub fn update_links(&mut self) {
        let device_ids: Vec<_> = self.scene.devices.keys().cloned().collect();
        for device_id in device_ids {
            let device = &self.scene.devices.get(&device_id).unwrap();
            let num_outputs = device.data.num_outputs();
            let links = device.links.clone();

            for output in 0..num_outputs {
                let mut new_links = Vec::with_capacity(links[output].len());

                for link in &links[output] {
                    self.map_link(link, &mut new_links);
                }

                self.scene.devices.get_mut(&device_id).unwrap().links[output] = new_links;
            }
        }
        let input_ids: Vec<_> = self.scene.inputs.keys().cloned().collect();
        for input_id in input_ids {
            let input = &self.scene.inputs.get(&input_id).unwrap();

            let mut new_links = Vec::with_capacity(input.links.len());

            for link in &input.links {
                self.map_link(link, &mut new_links);
            }

            self.scene.inputs.get_mut(&input_id).unwrap().links = new_links;
        }
        for chip_idx in 0..self.chips.len() {
            let chip = &self.chips[chip_idx];
            let num_outputs = chip.output_links.len();
            let links = chip.output_links.clone();

            for output in 0..num_outputs {
                let mut new_links = Vec::with_capacity(links[output].len());

                for link in &links[output] {
                    self.map_link(link, &mut new_links);
                }

                self.chips[chip_idx].output_links[output] = new_links;
            }
        }
    }

    pub fn join_chip_devices(&mut self) {
        for un_chip in &self.chips {
            for (device_idx, device_id) in un_chip.device_ids.iter().enumerate() {
                let device = &un_chip.chip.devices[device_idx];

                let data = match &device.data {
                    scene::chip::DeviceData::CombGate(e) => scene::DeviceData::CombGate(e.clone()),
                    scene::chip::DeviceData::Light(e) => scene::DeviceData::Light(*e),
                    scene::chip::DeviceData::Switch(e) => scene::DeviceData::Switch(*e),
                };

                let num_outputs = device.data.num_outputs();
                let mut links = vec![Vec::new(); num_outputs];

                for output in 0..num_outputs {
                    for link in &device.links[output] {
                        un_chip.map_link(link, &mut links[output]);
                    }
                }

                let device = scene::Device {
                    data,
                    links,
                    input_locs: Vec::new(),
                    output_locs: Vec::new(),
                    pos: Pos2::ZERO,
                    preset: device.preset,
                };
                self.scene.devices.insert(*device_id, device);
            }
        }
        self.chips.clear();
    }

    pub fn get_link_indices(&self, links: &[LinkTarget<SimId>]) -> Vec<LinkTarget<usize>> {
        assert!(self.chips.is_empty(), "there are unjoined chips");
        links
            .iter()
            .map(|link| match link {
                LinkTarget::Output(output) => LinkTarget::Output(
                    self.scene
                        .outputs
                        .iter()
                        .position(|(key, _)| *key == *output)
                        .unwrap(),
                ),
                LinkTarget::DeviceInput(device, input) => LinkTarget::DeviceInput(
                    self.scene
                        .devices
                        .iter()
                        .position(|(key, _)| *key == *device)
                        .unwrap(),
                    *input,
                ),
            })
            .collect()
    }
    pub fn map_link(&self, link: &LinkTarget<SimId>, out: &mut Vec<LinkTarget<SimId>>) {
        match link {
            // device links to output of scene
            LinkTarget::Output(output) => {
                out.push(LinkTarget::Output(*output));
            }

            // device links to input of device in scene
            LinkTarget::DeviceInput(device, input) => {
                // device links to chip input
                if let Some(chip_idx) = self.chips.iter().position(|e| e.id == *device) {
                    let un_chip = &self.chips[chip_idx];
                    for link in &un_chip.chip.inputs[*input].links {
                        un_chip.map_link(link, out);
                    }
                }
                // device links to non-chip device
                else {
                    out.push(LinkTarget::DeviceInput(*device, *input));
                }
            }
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Input {
    pub label: IoLabel,
    pub links: Vec<LinkTarget<usize>>,
}

#[derive(Default, Debug, Clone)]
pub struct Output {
    pub label: IoLabel,
}

pub type DeviceData = crate::DeviceData<(), !, SimId>;
#[derive(Debug, Clone)]
pub struct Device {
    pub preset: SimId,
    pub data: DeviceData,
    pub links: Vec<Vec<LinkTarget<usize>>>,
}

#[derive(Debug, Clone)]
pub struct Chip {
    pub name: String,
    pub color: Color32,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub devices: Vec<Device>,
}
impl Chip {
    pub fn from_scene(scene: &scene::Scene) -> Self {
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let color = Color32::from_rgb(
            (scene.color[0] * 255.0) as u8,
            (scene.color[1] * 255.0) as u8,
            (scene.color[2] * 255.0) as u8,
        );
        let name = scene.name.clone();

        let mut scene = scene.clone();

        let mut unnester = Unnester::new(&mut scene);

        // move chips from `scene` to `chips`
        unnester.move_chips();

        // any devices that now link to the chips input, needs it's links updated
        unnester.update_links();

        // move devices in each chip of `chips` into `scene`
        unnester.join_chip_devices();

        // DONE

        let devices = unnester
            .scene
            .devices
            .iter()
            .map(|(_, device)| {
                let links = device
                    .links
                    .iter()
                    .map(|links| {
                        let mut out = Vec::new();
                        links
                            .iter()
                            .for_each(|link| unnester.map_link(link, &mut out));
                        unnester.get_link_indices(&out)
                    })
                    .collect();
                let data = match &device.data {
                    crate::DeviceData::CombGate(e) => DeviceData::CombGate(e.preset),
                    crate::DeviceData::Chip(_) => panic!("unexpected chip"),
                    crate::DeviceData::Light(_) => DeviceData::Light(()),
                    crate::DeviceData::Switch(_) => DeviceData::Switch(()),
                };
                Device {
                    preset: device.preset,
                    data,
                    links,
                }
            })
            .collect();

        let inputs = unnester
            .scene
            .inputs
            .iter()
            .map(|(_, input)| Input {
                label: input.label.clone(),
                links: {
                    let mut out = Vec::new();
                    input
                        .links
                        .iter()
                        .for_each(|link| unnester.map_link(link, &mut out));
                    let out = unnester.get_link_indices(&out);
                    out
                },
            })
            .collect();
        let outputs = unnester
            .scene
            .outputs
            .iter()
            .map(|(_, output)| Output {
                label: output.label.clone(),
            })
            .collect();

        Self {
            name,
            color,
            inputs,
            outputs,
            devices,
        }
    }
}
impl crate::IoAccess<()> for Chip {
    #[inline(always)]
    fn num_inputs(&self) -> usize {
        self.inputs.len()
    }
    #[inline(always)]
    fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    fn get_input(&self, input: usize) -> () {
        assert!(input < self.inputs.len());
        ()
    }
    fn get_output(&self, output: usize) -> () {
        assert!(output < self.outputs.len());
        ()
    }
}
