use crate::preset::{DevicePreset, PresetData, PresetSource};
use crate::scene::{Device, Group, Input, Io, Output, Scene, WriteQueue};
use crate::DeviceInput;
use egui::Rect;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OldInput {
    pub name: String,
    pub y_pos: f32,
    pub state: bool,
    pub links: Vec<DeviceInput<u64>>,
    pub group_member: Option<u64>,
}
impl OldInput {
    pub fn update(self) -> Input {
        Input {
            links: self.links,
            io: Io {
                name: self.name,
                y_pos: self.y_pos,
                state: self.state,
                group_member: self.group_member,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OldScene {
    pub rect: Rect,
    pub write_queue: WriteQueue<u64>,

    pub inputs: HashMap<u64, OldInput>,
    pub outputs: HashMap<u64, OldOutput>,
    pub devices: HashMap<u64, Device>,

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
        Scene {
            rect: self.rect,
            write_queue: self.write_queue,
            inputs,
            outputs,
            devices: self.devices,
            input_groups: self.input_groups,
            output_groups: self.output_groups,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
