use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CombGate {
    pub table: TruthTable,
    pub links: Vec<Vec<LinkTarget<usize>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChipPreset {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub input_links: Vec<Vec<DeviceInput<usize>>>,
    pub comb_gates: Vec<CombGate>,
}
impl ChipPreset {
    pub fn from_board(board: &board::Board) -> Self {
        let step1 = step1::exec(board);
        let step2 = step2::exec(&step1);

        Self {
            inputs: step2.inputs,
            outputs: step2.outputs,
            input_links: step2.input_links,
            comb_gates: step2.comb_gates,
        }
    }
}

pub fn map_links(links: &[Link]) -> Vec<LinkTarget<u64>> {
    links.iter().map(|link| link.target).collect()
}

// When unnesting occurs.
// New ID's are created for nested CombGates when they are unnested,
// and all links pointing at that CombGate is changed to the new ID
mod step1 {
    use super::map_links;
    use crate::*;
    use hashbrown::HashMap;

    #[derive(Debug)]
    pub struct CombGate {
        pub table: TruthTable,
        pub links: Vec<Vec<LinkTarget<u64>>>,
    }

    #[derive(Debug)]
    pub struct Input {
        pub y_pos: f32,
        pub name: String,
        pub links: Vec<DeviceInput<u64>>,
    }
    #[derive(Debug)]
    pub struct Output {
        pub y_pos: f32,
        pub name: String,
    }

    #[derive(Debug)]
    pub struct Board {
        pub inputs: HashMap<u64, Input>,
        pub outputs: HashMap<u64, Output>,
        pub comb_gates: HashMap<u64, CombGate>,
    }

    pub struct MovedChip {
        pub input_links: Vec<Vec<DeviceInput<u64>>>,
    }

    pub fn exec(board: &board::Board) -> Board {
        let mut comb_gates = HashMap::with_capacity(board.devices.len());
        let mut moved_chips = HashMap::new();

        // --- UN-NEST CHIPS ---
        for (id, board_device) in &board.devices {
            match &board_device.data {
                board::DeviceData::CombGate(comb_gate) => {
                    comb_gates.insert(
                        *id,
                        CombGate {
                            table: comb_gate.table.clone(),
                            links: board_device
                                .links
                                .iter()
                                .map(|links| map_links(links))
                                .collect(),
                        },
                    );
                }
                board::DeviceData::Chip(chip) => {
                    let mut device_ids = Vec::with_capacity(chip.devices.len());
                    for _ in 0..chip.devices.len() {
                        device_ids.push(rand_id());
                    }

                    let input_links = chip
                        .input_links
                        .iter()
                        .map(|links| {
                            links
                                .iter()
                                .map(|DeviceInput(device, input)| {
                                    DeviceInput(device_ids[*device], *input)
                                })
                                .collect()
                        })
                        .collect();

                    moved_chips.insert(*id, MovedChip { input_links });

                    for (idx, chip_device) in chip.devices.iter().enumerate() {
                        // if the link goes to the chip output, use the corresponding output links
                        // if the link goes to a contained device
                        let links = chip_device
                            .links
                            .iter()
                            .map(|links| {
                                let mut new_links = Vec::new();
                                for link in links {
                                    match link {
                                        LinkTarget::DeviceInput(device, input) => new_links.push(
                                            LinkTarget::DeviceInput(device_ids[*device], *input),
                                        ),
                                        LinkTarget::Output(output) => new_links
                                            .extend(map_links(&board_device.links[*output])),
                                    }
                                }
                                new_links
                            })
                            .collect();

                        comb_gates.insert(
                            device_ids[idx],
                            CombGate {
                                table: chip_device.data.table.clone(),
                                links,
                            },
                        );
                    }
                }
            }
        }

        // --- UPDATE LINKS TO ANY CHIPS ---
        for (_, comb_gate) in &mut comb_gates {
            for links in &mut comb_gate.links {
                let mut new_links = Vec::new();

                for link in &*links {
                    // we don't care about links to a Board output
                    let LinkTarget::DeviceInput(device, input) = link else {
        				new_links.push(link.clone());
        				continue
        			};
                    // we only care about links to Chips
                    let Some(moved_chip) = moved_chips.get(device) else {
        				new_links.push(link.clone());
        				continue
        			};

                    new_links.extend(moved_chip.input_links[*input].iter().map(DeviceInput::wrap));
                }

                *links = new_links;
            }
        }

        // --- INPUTS ---
        let inputs = board
            .inputs
            .iter()
            .map(|(id, input)| {
                let mut links = Vec::with_capacity(input.links.len());

                for link in &input.links {
                    let LinkTarget::DeviceInput(device, input) = link.target else {
                		panic!("Invalid board: input links to output");
                	};
                    match moved_chips.get(&device) {
                        // links to chip input (because all chips are in `moved_chips`)
                        Some(moved_chip) => links.extend(moved_chip.input_links[input].clone()),
                        // doesn't link to chip input
                        None => links.push(DeviceInput(device, input)),
                    }
                }
                let input = Input {
                    y_pos: input.io.y_pos,
                    name: input.io.name.clone(),
                    links,
                };
                (*id, input)
            })
            .collect();

        // --- OUTPUT ---
        let outputs = board
            .outputs
            .iter()
            .map(|(id, output)| {
                let output = Output {
                    y_pos: output.io.y_pos,
                    name: output.io.name.clone(),
                };
                (*id, output)
            })
            .collect();

        Board {
            inputs,
            outputs,
            comb_gates,
        }
    }
}

// When the u64's are mapped to indices
mod step2 {
    use super::CombGate;
    use crate::*;
    use hashbrown::HashMap;

    #[derive(Debug)]
    pub struct Board {
        pub inputs: Vec<String>,
        pub outputs: Vec<String>,
        pub input_links: Vec<Vec<DeviceInput<usize>>>,
        pub comb_gates: Vec<CombGate>,
    }

    pub fn exec(board: &super::step1::Board) -> Board {
        let mut output_indices = HashMap::with_capacity(board.outputs.len());
        let mut outputs = Vec::with_capacity(board.outputs.len());

        let mut board_outputs: Vec<_> = board.outputs.iter().collect();
        board_outputs.sort_by(|(_, a), (_, b)| a.y_pos.partial_cmp(&b.y_pos).unwrap());

        for (idx, (id, output)) in board_outputs.into_iter().enumerate() {
            outputs.push(output.name.clone());
            output_indices.insert(*id, idx);
        }

        let mut comb_gate_indices = HashMap::with_capacity(board.comb_gates.len());
        let mut comb_gates = Vec::with_capacity(board.comb_gates.len());

        for (idx, (id, _)) in board.comb_gates.iter().enumerate() {
            comb_gates.push(None);
            comb_gate_indices.insert(*id, idx);
        }

        let map_links = |links: &Vec<LinkTarget<u64>>| -> Vec<LinkTarget<usize>> {
            let mut new_links = Vec::with_capacity(links.len());

            for link in links {
                new_links.push(match link {
                    LinkTarget::Output(output) => {
                        LinkTarget::Output(*output_indices.get(output).unwrap())
                    }
                    LinkTarget::DeviceInput(device, input) => {
                        LinkTarget::DeviceInput(*comb_gate_indices.get(device).unwrap(), *input)
                    }
                });
            }
            new_links
        };

        let mut board_inputs: Vec<_> = board.inputs.iter().collect();
        board_inputs.sort_by(|(_, a), (_, b)| a.y_pos.partial_cmp(&b.y_pos).unwrap());

        let input_links: Vec<_> = board_inputs
            .iter()
            .map(|(_, input)| {
                let mut new_links = Vec::with_capacity(input.links.len());

                for DeviceInput(device, input) in &input.links {
                    new_links.push(DeviceInput(*comb_gate_indices.get(device).unwrap(), *input));
                }
                new_links
            })
            .collect();

        let inputs = board_inputs
            .into_iter()
            .map(|(_, input)| input.name.clone())
            .collect();

        for (id, comb_gate) in &board.comb_gates {
            let index = *comb_gate_indices.get(id).unwrap();
            let links = comb_gate.links.iter().map(map_links).collect();
            comb_gates[index] = Some(CombGate {
                table: comb_gate.table.clone(),
                links,
            });
        }

        let comb_gates = comb_gates.into_iter().map(Option::unwrap).collect();
        Board {
            inputs,
            outputs,
            input_links,
            comb_gates,
        }
    }
}
