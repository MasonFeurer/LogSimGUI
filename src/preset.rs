use crate::graphics;
use crate::sim::{self, BitField};
use eframe::egui::{Pos2, Vec2};

#[derive(Clone, Debug)]
pub enum DeviceData {
    CombGate(CombGate),
    Board(Board),
    Light,
    Switch,
}
impl DeviceData {
    pub fn sim(&self) -> sim::Device {
        match self {
            Self::CombGate(comb_gate) => sim::Device::CombGate {
                input: BitField(0),
                output: comb_gate.table[0],
                comb_gate: comb_gate.clone(),
            },
            Self::Board(board) => sim::Device::Board(board.sim()),
            Self::Light => sim::Device::Light(false),
            Self::Switch => sim::Device::Switch(false),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Device {
    pub name: String,
    pub color: [f32; 3],
    pub data: DeviceData,
}
impl Device {
    pub fn num_inputs(&self) -> usize {
        match &self.data {
            DeviceData::CombGate(comb_gate) => comb_gate.inputs.len(),
            DeviceData::Board(board) => board.inputs.len(),
            DeviceData::Light => 1,
            DeviceData::Switch => 0,
        }
    }
    pub fn num_outputs(&self) -> usize {
        match &self.data {
            DeviceData::CombGate(comb_gate) => comb_gate.outputs.len(),
            DeviceData::Board(board) => board.outputs.len(),
            DeviceData::Light => 0,
            DeviceData::Switch => 1,
        }
    }
    pub fn get_input_label(&self, input: usize) -> Option<IoLabel> {
        match &self.data {
            DeviceData::CombGate(comb_gate) => Some(comb_gate.inputs[input].clone()),
            DeviceData::Board(board) => Some(board.inputs[input].label.clone()),
            DeviceData::Light => {
                assert_eq!(input, 0);
                None
            }
            DeviceData::Switch => panic!("a button doesn't have an input"),
        }
    }
    pub fn get_output_label(&self, output: usize) -> Option<IoLabel> {
        match &self.data {
            DeviceData::CombGate(comb_gate) => Some(comb_gate.outputs[output].clone()),
            DeviceData::Board(board) => Some(board.outputs[output].label.clone()),
            DeviceData::Light => panic!("a button doesn't have an output"),
            DeviceData::Switch => {
                assert_eq!(output, 0);
                None
            }
        }
    }

    pub fn size(&self) -> (f32, f32) {
        const PORT_SIZE: f32 = 20.0;
        const PORT_SPACE: f32 = 5.0;

        let width = 60.0;
        let height = std::cmp::max(self.num_inputs(), self.num_outputs()) as f32
            * (PORT_SIZE + PORT_SPACE)
            + PORT_SPACE;

        (width, height)
    }

    #[inline(always)]
    pub fn sim(&self) -> crate::sim::Device {
        self.data.sim()
    }
}

#[derive(Clone, Debug, Default)]
pub struct IoLabel {
    pub name: String,
    pub implicit: bool,
}

#[derive(Debug, Clone)]
pub struct CombGate {
    pub name: String,
    pub inputs: Vec<IoLabel>,
    pub outputs: Vec<IoLabel>,
    pub table: Vec<BitField>,
}
impl CombGate {
    pub fn get(&self, input: u64) -> BitField {
        self.table[input as usize]
    }
}

#[derive(Clone, Debug, Default)]
pub struct BoardInput {
    pub label: IoLabel,
    pub links: Vec<sim::BoardWriteTarget>,
}
impl BoardInput {
    pub fn sim(&self) -> sim::BoardInput {
        sim::BoardInput {
            state: false,
            links: self.links.clone(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BoardOutput {
    pub label: IoLabel,
}
impl BoardOutput {
    pub fn sim(&self) -> sim::BoardOutput {
        sim::BoardOutput { state: false }
    }
}

#[derive(Clone, Debug)]
pub struct BoardDevice {
    pub device: Device,
    pub pos: Pos2,
    pub links: Vec<Vec<sim::BoardLink>>,
    pub input_locs: Vec<Pos2>,
    pub output_locs: Vec<Pos2>,
}
impl BoardDevice {
    pub fn sim(&self) -> sim::BoardDevice {
        sim::BoardDevice {
            device: self.device.sim(),
            links: self.links.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Board {
    pub inputs: Vec<BoardInput>,
    pub outputs: Vec<BoardOutput>,
    pub devices: Vec<BoardDevice>,
}
impl Board {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            devices: Vec::new(),
        }
    }

    pub fn sim(&self) -> sim::Board {
        sim::Board {
            inputs: self.inputs.iter().map(BoardInput::sim).collect(),
            outputs: self.outputs.iter().map(BoardOutput::sim).collect(),
            devices: self.devices.iter().map(BoardDevice::sim).collect(),
            writes: Vec::new(),
        }
    }

    pub fn get_target_loc(&self, target: sim::BoardWriteTarget) -> Pos2 {
        match target {
            sim::BoardWriteTarget::BoardOutput(output) => {
                use graphics::{EDITOR_IO_SIZE, EDITOR_IO_SP, EDITOR_POS, EDITOR_SIZE};

                let output_locs = graphics::calc_io_unsized_locs(
                    EDITOR_POS + Vec2::new(EDITOR_SIZE.x - EDITOR_IO_SIZE.x, 0.0),
                    self.outputs.len(),
                    EDITOR_IO_SP,
                );
                output_locs[output]
            }
            sim::BoardWriteTarget::DeviceInput(device, input) => {
                self.devices[device].input_locs[input]
            }
        }
    }
}

// **** DEFAULTS ****
pub fn default_presets() -> [Device; 6] {
    [
        Device {
            name: "Light".to_owned(),
            color: [1.0, 1.0, 0.0],
            data: DeviceData::Light,
        },
        Device {
            name: "Switch".to_owned(),
            color: [1.0, 1.0, 0.0],
            data: DeviceData::Switch,
        },
        Device {
            name: "And".to_owned(),
            color: [0.0, 0.0, 1.0],
            data: DeviceData::CombGate(CombGate {
                name: "And".to_owned(),
                inputs: vec![
                    IoLabel {
                        name: "a".to_owned(),
                        implicit: true,
                    },
                    IoLabel {
                        name: "b".to_owned(),
                        implicit: true,
                    },
                ],
                outputs: vec![IoLabel {
                    name: "out".to_owned(),
                    implicit: true,
                }],
                table: vec![
                    BitField(0), // 00
                    BitField(0), // 01
                    BitField(0), // 10
                    BitField(1), // 11
                ],
            }),
        },
        Device {
            name: "Not".to_owned(),
            color: [0.0, 1.0, 0.0],
            data: DeviceData::CombGate(CombGate {
                name: "Not".to_owned(),
                inputs: vec![IoLabel {
                    name: "in".to_owned(),
                    implicit: true,
                }],
                outputs: vec![IoLabel {
                    name: "out".to_owned(),
                    implicit: true,
                }],
                table: vec![
                    BitField(1), // 0
                    BitField(0), // 1
                ],
            }),
        },
        Device {
            name: "Nor".to_owned(),
            color: [1.0, 1.0, 0.0],
            data: DeviceData::CombGate(CombGate {
                name: "Nor".to_owned(),
                inputs: vec![
                    IoLabel {
                        name: "a".to_owned(),
                        implicit: true,
                    },
                    IoLabel {
                        name: "b".to_owned(),
                        implicit: true,
                    },
                ],
                outputs: vec![IoLabel {
                    name: "out".to_owned(),
                    implicit: true,
                }],
                table: vec![
                    BitField(1), // 00
                    BitField(0), // 01
                    BitField(0), // 10
                    BitField(0), // 11
                ],
            }),
        },
        Device {
            name: "Or".to_owned(),
            color: [1.0, 0.0, 0.0],
            data: DeviceData::CombGate(CombGate {
                name: "Or".to_owned(),
                inputs: vec![
                    IoLabel {
                        name: "a".to_owned(),
                        implicit: true,
                    },
                    IoLabel {
                        name: "b".to_owned(),
                        implicit: true,
                    },
                ],
                outputs: vec![IoLabel {
                    name: "out".to_owned(),
                    implicit: true,
                }],
                table: vec![
                    BitField(0), // 00
                    BitField(1), // 01
                    BitField(1), // 10
                    BitField(1), // 11
                ],
            }),
        },
    ]
}
