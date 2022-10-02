pub mod chip;

use crate::{BitField, TruthTable};
use eframe::egui::Color32;

#[derive(Default, Clone, Debug)]
pub struct IoLabel {
    pub name: String,
    pub implicit: bool,
}
impl IoLabel {
    #[inline(always)]
    pub fn implicit_input() -> Self {
        Self {
            name: String::from("input"),
            implicit: true,
        }
    }
    #[inline(always)]
    pub fn implicit_output() -> Self {
        Self {
            name: String::from("output"),
            implicit: true,
        }
    }

    #[inline(always)]
    pub fn implicit(name: &str) -> Self {
        Self {
            name: String::from(name),
            implicit: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CombGate {
    pub name: String,
    pub color: Color32,
    pub inputs: Vec<IoLabel>,
    pub outputs: Vec<IoLabel>,
    pub table: TruthTable,
}
impl CombGate {
    #[inline(always)]
    pub fn num_inputs(&self) -> usize {
        self.table.num_inputs
    }
    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.table.num_outputs
    }

    #[inline(always)]
    pub fn get_input(&self, input: usize) -> &IoLabel {
        &self.inputs[input]
    }
    #[inline(always)]
    pub fn get_output(&self, output: usize) -> &IoLabel {
        &self.outputs[output]
    }
}

#[derive(Debug, Clone)]
pub enum Device {
    CombGate(CombGate),
    Chip(chip::Chip),
    Light,
    Switch,
}
impl Device {
    pub fn size(&self) -> (f32, f32) {
        const PORT_SIZE: f32 = 20.0;
        const PORT_SPACE: f32 = 5.0;

        let width = 60.0;
        let height = std::cmp::max(self.num_inputs(), self.num_outputs()) as f32
            * (PORT_SIZE + PORT_SPACE)
            + PORT_SPACE;

        (width, height)
    }

    pub fn num_inputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_inputs(),
            Self::Chip(e) => e.inputs.len(),
            Self::Light => 1,
            Self::Switch => 0,
        }
    }
    pub fn num_outputs(&self) -> usize {
        match self {
            Self::CombGate(e) => e.num_outputs(),
            Self::Chip(e) => e.outputs.len(),
            Self::Light => 0,
            Self::Switch => 1,
        }
    }

    pub fn get_input(&self, input: usize) -> IoLabel {
        match self {
            Self::CombGate(e) => e.get_input(input).clone(),
            Self::Chip(e) => e.inputs[input].label.clone(),
            Self::Light => {
                assert_eq!(input, 0);
                IoLabel::implicit_input()
            }
            Self::Switch => panic!("a switch doesnt have an input"),
        }
    }
    pub fn get_output(&self, output: usize) -> IoLabel {
        match self {
            Self::CombGate(e) => e.get_output(output).clone(),
            Self::Chip(e) => e.outputs[output].label.clone(),
            Self::Switch => {
                assert_eq!(output, 0);
                IoLabel::implicit_output()
            }
            Self::Light => panic!("a switch doesnt have an output"),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::CombGate(e) => &e.name,
            Self::Chip(e) => &e.name,
            Self::Light => "light",
            Self::Switch => "switch",
        }
    }

    pub fn color(&self) -> Option<Color32> {
        match self {
            Self::CombGate(e) => Some(e.color.clone()),
            Self::Chip(e) => Some(e.color.clone()),
            Self::Light => None,
            Self::Switch => None,
        }
    }
}

pub fn default_presets() -> [Device; 2] {
    [
        Device::CombGate(CombGate {
            name: String::from("And"),
            color: Color32::BLUE,
            inputs: vec![IoLabel::implicit("a"), IoLabel::implicit("b")],
            outputs: vec![IoLabel::implicit_output()],
            table: TruthTable {
                num_inputs: 2,
                num_outputs: 1,
                map: vec![
                    BitField(0), // 00
                    BitField(0), // 01
                    BitField(0), // 10
                    BitField(1), // 11
                ],
            },
        }),
        Device::CombGate(CombGate {
            name: String::from("Not"),
            color: Color32::GREEN,
            inputs: vec![IoLabel::implicit_input()],
            outputs: vec![IoLabel::implicit_output()],
            table: TruthTable {
                num_inputs: 1,
                num_outputs: 1,
                map: vec![
                    BitField(1), // 0
                    BitField(0), // 1
                ],
            },
        }),
    ]
}
