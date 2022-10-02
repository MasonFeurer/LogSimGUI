use super::IoLabel;
use crate::scene;
use crate::SimId;
use eframe::egui::Color32;

#[derive(Debug, Clone)]
pub enum LinkTarget {
    ChipOutput(usize),
    DeviceInput(usize, usize),
}

#[derive(Default, Debug, Clone)]
pub struct Input {
    pub label: IoLabel,
    pub links: Vec<LinkTarget>,
}

#[derive(Default, Debug, Clone)]
pub struct Output {
    pub label: IoLabel,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub preset: SimId,
    pub links: Vec<Vec<LinkTarget>>,
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
        let map_links = |link: &scene::WriteTarget| -> LinkTarget {
            match link {
                scene::WriteTarget::DeviceInput(device, input) => {
                    let device = scene
                        .devices
                        .iter()
                        .position(|(id, _)| *id == *device)
                        .unwrap();
                    LinkTarget::DeviceInput(device, *input)
                }
                scene::WriteTarget::SceneOutput(output) => {
                    let output = scene
                        .outputs
                        .iter()
                        .position(|(id, _)| *id == *output)
                        .unwrap();
                    LinkTarget::ChipOutput(output)
                }
            }
        };

        let inputs = scene
            .inputs
            .iter()
            .map(|(_, input)| Input {
                label: input.label.clone(),
                links: input.links.iter().map(map_links).collect(),
            })
            .collect();
        let outputs = scene
            .outputs
            .iter()
            .map(|(_, output)| Output {
                label: output.label.clone(),
            })
            .collect();
        let devices = scene
            .devices
            .iter()
            .map(|(_, device)| {
                let links = device
                    .links
                    .iter()
                    .map(|links| links.iter().map(map_links).collect())
                    .collect();
                Device {
                    preset: device.preset,
                    links,
                }
            })
            .collect();
        let color = Color32::from_rgb(
            (scene.color[0] * 255.0) as u8,
            (scene.color[1] * 255.0) as u8,
            (scene.color[2] * 255.0) as u8,
        );
        Self {
            name: scene.name.clone(),
            color,
            inputs,
            outputs,
            devices,
        }
    }
}
