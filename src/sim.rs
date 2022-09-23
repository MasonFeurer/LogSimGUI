use crate::preset::CombGate;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BitField(pub u64);
impl BitField {
    pub fn set(&mut self, pos: usize, state: bool) {
        self.0 = (self.0 & !(1 << pos as u64)) | ((state as u64) << pos);
    }
    pub fn get(&self, pos: usize) -> bool {
        ((self.0 >> pos as u64) & 1) == 1
    }

    pub fn states(self, size: usize) -> Vec<bool> {
        let mut states = Vec::with_capacity(size);
        for i in 0..size {
            states.push(self.get(i));
        }
        states
    }
}

// **** DEVICE ****

pub struct ChangedOutput {
    output: usize,
    state: bool,
}

#[derive(Debug, Clone)]
pub enum Device {
    CombGate {
        input: BitField,
        output: BitField,
        comb_gate: CombGate,
    },
    Board(Board),
    Light(bool),
    Switch(bool),
}
impl Device {
    pub fn set_input(
        &mut self,
        input: usize,
        state: bool,
        changed_outputs: &mut Vec<ChangedOutput>,
    ) {
        match self {
            Self::CombGate {
                input: self_input,
                output: self_output,
                comb_gate,
            } => {
                // set input
                self_input.set(input, state);

                // generate output
                let result = comb_gate.get(self_input.0);

                // push changed outputs
                if result != *self_output {
                    for i in 0..comb_gate.outputs.len() {
                        if self_output.get(i) == result.get(i) {
                            continue;
                        }

                        changed_outputs.push(ChangedOutput {
                            output: i,
                            state: result.get(i),
                        });
                    }
                }

                // set output
                *self_output = result;
            }
            Self::Board(board) => {
                board.set_input(input, state);
            }
            Self::Light(self_state) => {
                *self_state = state;
            }
            Self::Switch(_) => panic!("a button doesn't have an input"),
        }
    }
    pub fn get_input(&self, input: usize) -> bool {
        match self {
            Self::CombGate {
                input: self_input, ..
            } => self_input.get(input),
            Self::Board(board) => board.get_input(input),
            Self::Light(state) => {
                assert_eq!(input, 0);
                *state
            }
            Self::Switch(_) => panic!("a button doesn't have an input"),
        }
    }
    pub fn get_output(&self, output: usize) -> bool {
        match self {
            Self::CombGate {
                output: self_output,
                ..
            } => self_output.get(output),
            Self::Board(board) => board.outputs[output].state,
            Self::Light(_) => panic!("a light doesn't have an output"),
            Self::Switch(state) => {
                assert_eq!(output, 0);
                *state
            }
        }
    }
}

// **** BOARD ****

#[derive(Debug, Clone, Copy)]
pub struct BoardLink {
    pub output: usize,
    pub target: BoardWriteTarget,
}

#[derive(Debug, Clone, Copy)]
pub enum BoardWriteTarget {
    BoardOutput(usize),
    DeviceInput(usize, usize),
}

#[derive(Debug, Clone)]
pub struct BoardWrite {
    pub target: BoardWriteTarget,
    pub state: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BoardInput {
    pub state: bool,
    pub links: Vec<BoardWriteTarget>,
}

#[derive(Debug, Clone, Default)]
pub struct BoardOutput {
    pub state: bool,
}

#[derive(Debug, Clone)]
pub struct BoardDevice {
    pub device: Device,
    pub links: Vec<Vec<BoardLink>>,
}

#[derive(Debug, Clone)]
pub struct Board {
    pub inputs: Vec<BoardInput>,
    pub outputs: Vec<BoardOutput>,
    pub devices: Vec<BoardDevice>,

    pub writes: Vec<BoardWrite>,
}
impl Board {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            devices: Vec::new(),
            writes: Vec::new(),
        }
    }

    pub fn add_input(&mut self) {}

    #[inline(always)]
    pub fn queue_write(&mut self, write: BoardWrite) {
        self.writes.push(write)
    }

    pub fn set_input(&mut self, input: usize, state: bool) {
        self.inputs[input].state = state;
        for target in self.inputs[input].links.clone() {
            self.queue_write(BoardWrite { target, state });
        }
    }
    #[inline(always)]
    pub fn get_input(&self, input: usize) -> bool {
        self.inputs[input].state
    }
    #[inline(always)]
    pub fn get_input_links(&mut self, input: usize) -> Vec<BoardWriteTarget> {
        self.inputs[input].links.clone()
    }

    pub fn exec_writes(&mut self, changed_outputs: &mut Vec<ChangedOutput>) {
        let mut new_writes = Vec::new();

        // if self.writes.len() > 0 {
        // 	println!("executing {} writes", self.writes.len());
        // }
        for write in &self.writes {
            // println!("  write: {:?}", write);

            match &write.target {
                BoardWriteTarget::BoardOutput(output) => {
                    if self.outputs[*output].state != write.state {
                        changed_outputs.push(ChangedOutput {
                            output: *output,
                            state: write.state,
                        });
                    }

                    self.outputs[*output].state = write.state;
                }
                BoardWriteTarget::DeviceInput(device, input) => {
                    let mut changed_outputs = Vec::new();

                    self.devices[*device].device.set_input(
                        *input,
                        write.state,
                        &mut changed_outputs,
                    );

                    // println!("writing resulted in set outputs: {:?}", set_outputs);

                    for changed_output in changed_outputs {
                        for link in &self.devices[*device].links[changed_output.output] {
                            new_writes.push(BoardWrite {
                                target: link.target,
                                state: changed_output.state,
                            });
                        }
                    }
                }
            }
        }

        for device in &mut self.devices {
            if let Device::Board(board) = &mut device.device {
                let mut changed_outputs = Vec::new();
                board.exec_writes(&mut changed_outputs);

                for changed_output in changed_outputs {
                    for link in &device.links[changed_output.output] {
                        new_writes.push(BoardWrite {
                            target: link.target,
                            state: changed_output.state,
                        });
                    }
                }
            }
        }

        self.writes = new_writes;
    }
}
