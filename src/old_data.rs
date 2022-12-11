use crate::preset::{DevicePreset, PresetData, PresetSource};
use crate::scene::{Device, DeviceData, Group, Input, Io, Output, Scene, WriteQueue};
use crate::{DeviceInput, Link, LinkTarget};
use egui::{Color32, Pos2, Rect};
use hashbrown::HashMap;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OldInput {
    pub name: String,
    pub y_pos: f32,
    pub state: bool,
    pub links: Vec<DeviceInput<u64>>,
    pub group_member: Option<u64>,
}
impl OldInput {
    pub fn update(self) -> Input {
        let links = self
            .links
            .iter()
            .map(|device_input| Link::new(device_input.wrap()))
            .collect();
        Input {
            links,
            io: Io {
                name: self.name,
                y_pos: self.y_pos,
                state: self.state,
                group_member: self.group_member,
            },
        }
    }
}

#[derive(Deserialize)]
pub struct OldOutput {
    pub name: String,
    pub y_pos: f32,
    pub state: bool,
    pub group_member: Option<u64>,
}
impl OldOutput {
    pub fn update(self) -> Output {
        Output {
            io: Io {
                name: self.name,
                y_pos: self.y_pos,
                state: self.state,
                group_member: self.group_member,
            },
        }
    }
}

#[derive(Deserialize)]
pub struct OldDevice {
    pub pos: Pos2,
    pub data: DeviceData,
    pub links: Vec<Vec<LinkTarget<u64>>>,
    pub name: String,
    pub color: Color32,
    pub input_names: Vec<String>,
    pub output_names: Vec<String>,
}
impl OldDevice {
    pub fn update(self) -> Device {
        let links = self
            .links
            .into_iter()
            .map(|links| links.into_iter().map(|target| Link::new(target)).collect())
            .collect();
        Device {
            pos: self.pos,
            data: self.data,
            links,
            name: self.name,
            color: self.color,
            input_names: self.input_names,
            output_names: self.output_names,
        }
    }
}

#[derive(Deserialize)]
pub struct OldScene {
    pub rect: Rect,
    pub write_queue: WriteQueue<u64>,

    pub inputs: HashMap<u64, OldInput>,
    pub outputs: HashMap<u64, OldOutput>,
    pub devices: HashMap<u64, OldDevice>,

    pub input_groups: HashMap<u64, Group>,
    pub output_groups: HashMap<u64, Group>,
}
impl OldScene {
    pub fn update(self) -> Scene {
        let inputs = self
            .inputs
            .into_iter()
            .map(|(id, input)| (id, input.update()))
            .collect();
        let outputs = self
            .outputs
            .into_iter()
            .map(|(id, outputs)| (id, outputs.update()))
            .collect();
        let devices = self
            .devices
            .into_iter()
            .map(|(id, device)| (id, device.update()))
            .collect();
        Scene {
            rect: self.rect,
            write_queue: self.write_queue,
            inputs,
            outputs,
            devices,
            input_groups: self.input_groups,
            output_groups: self.output_groups,
        }
    }
}

#[derive(Deserialize)]
pub struct OldDevicePreset {
    pub name: String,
    pub cat: String,
    pub color: [u8; 4],
    pub data: PresetData,
    pub src: OldPresetSource,
}
impl OldDevicePreset {
    pub fn update(self) -> DevicePreset {
        println!("updating preset {:?}", self.name);
        DevicePreset {
            name: self.name,
            cat: self.cat,
            color: self.color,
            data: self.data,
            src: self.src.update(),
        }
    }
}

#[derive(Deserialize)]
pub enum OldPresetSource {
    Default,
    Scene(Option<OldScene>),
}
impl OldPresetSource {
    pub fn update(self) -> PresetSource {
        match self {
            Self::Default => PresetSource::Default,
            Self::Scene(None) => PresetSource::Scene(None),
            Self::Scene(Some(scene)) => PresetSource::Scene(Some(scene.update())),
        }
    }
}
