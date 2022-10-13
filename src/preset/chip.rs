use super::Io;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Device {
    pub preset: IntId,
    pub links: Vec<Vec<LinkTarget<usize>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chip {
    pub name: String,
    pub color: Color,
    pub inputs: Vec<WithLinks<Io, usize>>,
    pub outputs: Vec<Io>,
    pub devices: Vec<Device>,
}
impl Chip {
    #[allow(unused_variables)]
    pub fn from_scene(name: &str, color: Color, scene: &scene::Scene) -> Self {
        todo!()
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

    pub fn get_input(&self, input: usize) -> Option<&Io> {
        Some(&self.inputs.get(input)?.item)
    }
    pub fn get_output(&self, output: usize) -> Option<&Io> {
        Some(self.outputs.get(output)?)
    }
}
