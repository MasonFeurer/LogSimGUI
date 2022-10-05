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
impl<'a> crate::IoAccess<()> for CombGate {
    #[inline(always)]
    fn num_inputs(&self) -> usize {
        self.table.num_inputs
    }
    #[inline(always)]
    fn num_outputs(&self) -> usize {
        self.table.num_outputs
    }

    #[inline(always)]
    fn get_input(&self, input: usize) -> () {
        assert!(input < self.inputs.len());
        ()
    }
    #[inline(always)]
    fn get_output(&self, output: usize) -> () {
        assert!(output < self.outputs.len());
        ()
    }
}
impl CombGate {
    #[inline(always)]
    pub fn get_input_label(&self, input: usize) -> &IoLabel {
        &self.inputs[input]
    }
    #[inline(always)]
    pub fn get_output_label(&self, output: usize) -> &IoLabel {
        &self.outputs[output]
    }
}

pub type DeviceData = crate::DeviceData<(), chip::Chip, CombGate>;
impl DeviceData {
    pub fn size(&self) -> (f32, f32) {
        const PORT_SIZE: f32 = 20.0;
        const PORT_SPACE: f32 = 5.0;

        let width = 60.0;
        let height = std::cmp::max(self.num_inputs(), self.num_outputs()) as f32
            * (PORT_SIZE + PORT_SPACE)
            + PORT_SPACE;

        (width, height)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::CombGate(e) => &e.name,
            Self::Chip(e) => &e.name,
            Self::Light(_) => "light",
            Self::Switch(_) => "switch",
        }
    }

    pub fn color(&self) -> Option<Color32> {
        match self {
            Self::CombGate(e) => Some(e.color.clone()),
            Self::Chip(e) => Some(e.color.clone()),
            Self::Light(_) => None,
            Self::Switch(_) => None,
        }
    }
}
impl DeviceData {
    pub fn get_input_label(&self, input: usize) -> IoLabel {
        match self {
            Self::CombGate(e) => e.get_input_label(input).clone(),
            Self::Chip(e) => e.inputs[input].label.clone(),
            Self::Light(_) => {
                assert_eq!(input, 0);
                IoLabel::implicit_input()
            }
            Self::Switch(_) => panic!("a switch doesnt have an input"),
        }
    }
    pub fn get_output_label(&self, output: usize) -> IoLabel {
        match self {
            Self::CombGate(e) => e.get_output_label(output).clone(),
            Self::Chip(e) => e.outputs[output].label.clone(),
            Self::Switch(_) => {
                assert_eq!(output, 0);
                IoLabel::implicit_output()
            }
            Self::Light(_) => panic!("a light doesnt have an output"),
        }
    }
}

pub fn default_presets() -> [DeviceData; 2] {
    [
        DeviceData::CombGate(CombGate {
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
        DeviceData::CombGate(CombGate {
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
