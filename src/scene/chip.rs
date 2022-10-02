use super::{ChangedOutput, CombGate};
use crate::preset::chip::Chip as PresetChip;

// a chip is a device that is placed in the scene.
// the order of inputs and order of internal devices can not change.
// such fact can lead to optimizations, such as storing the inputs and outputs of all internal chips in the same vector
#[derive(Default, Debug, Clone)]
pub struct Chip {
    pub writes: Vec<Write>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub bits: Vec<Bit>,
    pub comb_gates: Vec<CombGate>,
    pub lights: Vec<Light>,
}
impl Chip {
    pub fn from_preset(_preset: &PresetChip) -> Self {
        todo!()
    }

    #[inline(always)]
    pub fn num_inputs(&self) -> usize {
        self.inputs.len()
    }
    #[inline(always)]
    pub fn num_outputs(&self) -> usize {
        self.outputs.len()
    }

    #[inline(always)]
    pub fn get_input(&self, input: usize) -> bool {
        self.inputs[input].state
    }
    #[inline(always)]
    pub fn get_output(&self, output: usize) -> bool {
        self.outputs[output].state
    }

    #[allow(unused_variables)]
    pub fn update_listener(
        &mut self,
        l: Listener,
        state: bool,
        changed_outputs: &mut Vec<ChangedOutput>,
    ) {
        match l {
            Listener::CombGate(i) => todo!(),
            Listener::Light(i) => todo!(),
            Listener::Output(i) => todo!(),
            Listener::Bit(i) => todo!(),
        }
    }

    pub fn set_bit(&mut self, bit: usize, state: bool, changed_outputs: &mut Vec<ChangedOutput>) {
        self.bits[bit].state = state;
        for l in self.bits[bit].listeners.clone() {
            self.update_listener(l, state, changed_outputs);
        }
    }

    pub fn set_input(
        &mut self,
        input: usize,
        state: bool,
        changed_outputs: &mut Vec<ChangedOutput>,
    ) {
        if self.inputs[input].state == state {
            return;
        }

        self.inputs[input].state = state;
        for l in self.inputs[input].listeners.clone() {
            self.update_listener(l, state, changed_outputs);
        }
    }

    pub fn update(&mut self, changed_outputs: &mut Vec<ChangedOutput>) {
        let mut writes = Vec::with_capacity(self.writes.len());

        std::mem::swap(&mut writes, &mut self.writes);

        for write in writes {
            if write.delay > 0 {
                self.writes.push(write.dec_delay());
                continue;
            }

            match write.target {
                WriteTarget::Bit(i) => self.set_bit(i, write.state, changed_outputs),
                WriteTarget::Output(i) => {
                    self.outputs[i].state = write.state;
                    changed_outputs.push(ChangedOutput {
                        output: i,
                        state: write.state,
                    });
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bit {
    pub state: bool,
    pub listeners: Vec<Listener>,
}

// what can listen to a bit in a chip?
#[derive(Debug, Clone, Copy)]
pub enum Listener {
    CombGate(usize),
    Light(usize),
    Output(usize),
    Bit(usize),
}

#[derive(Debug, Clone)]
pub struct Light(bool);

#[derive(Debug, Clone, Copy)]
pub enum WriteTarget {
    Bit(usize),
    Output(usize),
}

#[derive(Debug, Clone)]
pub struct Write {
    pub delay: u8,
    pub target: WriteTarget,
    pub state: bool,
}
impl Write {
    pub fn dec_delay(&self) -> Self {
        Self {
            delay: self.delay - 1,
            ..*self
        }
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub state: bool,
    pub listeners: Vec<Listener>,
}
#[derive(Debug, Clone)]
pub struct Output {
    pub state: bool,
}
