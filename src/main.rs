use cached::proc_macro::cached;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Write};

const ROWS: usize = 7;
const COLS: usize = 7;
const MIMIC_INITIAL_ROW: usize = 4;
const MIMIC_INITIAL_COL: usize = 4;
const MAX_BLOCKS_TO_REMOVE: usize = 10;

#[cached(size = 81)]
fn is_valid_location(loc: (u8, u8)) -> bool {
    match loc {
        (row, col) => row > 0 && row < 8 && col > 0 && col < 8,
    }
}

#[cached(size = 49)]
fn is_outer(row: u8, col: u8) -> bool {
    row % 6 == 1 || col % 6 == 1
}

#[cached(size = 49)]
fn get_neighbors(row: u8, col: u8) -> Vec<Cell> {
    let locations: [(u8, u8); 6] = if col % 2 == 0 {
        [
            (row - 1, col),
            (row, col - 1),
            (row, col + 1),
            (row + 1, col - 1),
            (row + 1, col + 1),
            (row + 1, col),
        ]
    } else {
        [
            (row - 1, col),
            (row - 1, col - 1),
            (row - 1, col + 1),
            (row, col - 1),
            (row, col + 1),
            (row + 1, col),
        ]
    };
    let valid_locations: Vec<(u8, u8)> = locations
        .iter()
        .filter(|&&loc| is_valid_location(loc))
        .cloned()
        .collect();

    valid_locations
        .iter()
        .map(|&(row, col)| Cell { row, col })
        .collect()
}

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
struct Cell {
    row: u8,
    col: u8,
}

impl Cell {
    fn is_outer(&self) -> bool {
        is_outer(self.row, self.col)
    }

    fn get_neighbors(&self) -> Vec<Cell> {
        get_neighbors(self.row, self.col)
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}-{})", self.col, self.row)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct Board {
    cells: [[bool; COLS]; ROWS],
    map_live_outer_cells: HashMap<(u8, u8), Cell>,
}

impl Board {
    fn new() -> Self {
        Board {
            cells: [[true; COLS]; ROWS],
            ..Default::default()
        }
    }

    fn drop_cell(&mut self, row: u8, col: u8) {
        self.cells[(row - 1) as usize][(col - 1) as usize] = false;
        if is_outer(row, col) {
            self.map_live_outer_cells.remove(&(row, col));
        }
    }

    fn create_imagine_board(&mut self, removing_cells: &Vec<Cell>) -> Self {
        let mut new_board = Board {
            cells: self.cells.clone(),
            map_live_outer_cells: self.map_live_outer_cells.clone(),
        };

        for cell in removing_cells {
            new_board.drop_cell(cell.row, cell.col);
        }
        new_board
    }

    fn value_at(&self, row: u8, col: u8) -> bool {
        self.cells[(row - 1) as usize][(col - 1) as usize]
    }

    fn value_at_cell(&self, cell: &Cell) -> bool {
        self.cells[(cell.row - 1) as usize][(cell.col - 1) as usize]
    }

    fn from_input(input: Vec<u8>) -> Self {
        let mut board = Board::new();
        board.gen_map_live_outer_cells();
        let mut dropped_cells: Vec<(u8, u8)> = Vec::new();

        for &value in input.iter() {
            let col = value / 10;
            let row = value % 10;
            if !is_valid_location((row, col)) {
                panic!("Invalid row or col input");
            }
            dropped_cells.push((row, col));
            board.drop_cell(row, col);
        }

        println!("Will drop cells {:?}", dropped_cells);
        board
    }

    // fn remove_unreachable_blocks(&mut self) {
    //     let mut removing_cells: Vec<Cell> = Vec::new();

    //     for cell in self.map_live_outer_cells.values() {
    //         let live_neighbors: Vec<Cell> = cell
    //             .get_neighbors()
    //             .iter()
    //             .filter(|&c| self.value_at_cell(c))
    //             .cloned()
    //             .collect();

    //         if live_neighbors.len() > 1 {
    //             continue;
    //         }

    //         if live_neighbors.len() == 0 {
    //             removing_cells.push(*cell);
    //         }
    //     }

    //     for cell in &removing_cells {
    //         self.drop_cell(cell.row, cell.col);
    //     }
    // }

    fn remove_redundant_blocks(&mut self) {
        let mut blocks_removed: u8 = u8::MAX;
        let mut removing_cells: Vec<Cell> = Vec::new();

        while blocks_removed != 0 {
            blocks_removed = 0;
            for cell in self.map_live_outer_cells.values() {
                let live_neighbors: Vec<Cell> = cell
                    .get_neighbors()
                    .iter()
                    .filter(|&c| self.value_at_cell(c))
                    .cloned()
                    .collect();

                if live_neighbors.len() > 1 {
                    continue;
                }

                if live_neighbors.len() == 0 {
                    removing_cells.push(*cell);
                    blocks_removed += 1;
                    continue;
                }

                let neighbor = live_neighbors.first().unwrap();
                if neighbor.is_outer() {
                    removing_cells.push(*cell);
                    blocks_removed += 1;
                }
            }
            for cell in &removing_cells {
                self.drop_cell(cell.row, cell.col);
            }
            removing_cells.clear();
        }
    }

    fn get_available_blocks(&mut self) -> Vec<Cell> {
        let mut available_blocks: Vec<Cell> = Vec::new();
        for row in 1..ROWS + 1 {
            for col in 1..COLS + 1 {
                let cell = Cell {
                    row: row as u8,
                    col: col as u8,
                };
                if self.value_at_cell(&cell)
                    && (row != MIMIC_INITIAL_ROW || col != MIMIC_INITIAL_COL)
                    && !cell.is_outer()
                {
                    available_blocks.push(cell);
                }
            }
        }
        available_blocks
    }

    fn calc_benefit(&mut self, removing_cells: &Vec<Cell>) -> (isize, Vec<Cell>) {
        let mut imaginery_board = self.create_imagine_board(removing_cells);
        // imaginery_board.remove_unreachable_blocks();
        imaginery_board.remove_redundant_blocks();

        let reachable_cells: Vec<Cell> = imaginery_board.get_reachable_cells();
        let border_cells: Vec<Cell> = reachable_cells
            .iter()
            .filter(|&&cell| cell.is_outer())
            .cloned()
            .collect();

        let num_total_removing_cells: usize = border_cells.len() + removing_cells.len();
        if num_total_removing_cells > MAX_BLOCKS_TO_REMOVE {
            return (-1, Vec::new());
        }

        let total_removing_cells: Vec<Cell> = border_cells
            .clone()
            .into_iter()
            .chain(removing_cells.clone().into_iter())
            .collect();

        return (
            (reachable_cells.len() - border_cells.len()) as isize,
            total_removing_cells,
        );
    }

    fn solve(&mut self) -> (isize, Vec<Vec<Cell>>) {
        let available_blocks: Vec<Cell> = self.get_available_blocks();
        println!("Available blocks: {}", available_blocks.iter().join(", "));

        let mut map_size_combinations: BTreeMap<u8, Vec<Vec<Cell>>> = BTreeMap::new();
        map_size_combinations.insert(0, vec![vec![]]);

        let mut num_combos: usize = 1;
        for size in 1..11 {
            let combos: Vec<Vec<Cell>> = available_blocks
                .clone()
                .into_iter()
                .combinations(size)
                .collect();
            num_combos += combos.len();
            map_size_combinations.insert(size as u8, combos);
        }

        println!("Found total {} combinations", num_combos);

        let mut max_benefit_combinations: Vec<Vec<Cell>> = Vec::new();
        let mut max_benefit: isize = 0;
        let mut countdown: u8 = 2;

        let mut map_size_benefit: HashMap<u8, isize> = HashMap::new();
        let mut current_combinations: Vec<Vec<Cell>> = Vec::new();

        for (size, combinations) in &map_size_combinations {
            if countdown == 0 {
                break;
            }

            println!("Calculating benefit for combination size = {}", size);
            for combination in combinations {
                let (benefit, removing_cells) = self.calc_benefit(&combination);

                match map_size_benefit.get(&size) {
                    Some(&value) => {
                        if benefit > value {
                            map_size_benefit.insert(*size, benefit);
                            current_combinations.clear();
                            current_combinations.push(removing_cells);
                        } else if benefit == value {
                            current_combinations.push(removing_cells);
                        }
                    }
                    None => {
                        map_size_benefit.insert(*size, benefit);
                    }
                }
            }

            let &benefit = map_size_benefit.get(&size).unwrap();
            if benefit < max_benefit {
                if benefit != -1 {
                    println!("Benefit is decreasing, counting down ({})", countdown);
                    countdown -= 1;
                }
            } else if benefit == max_benefit {
                println!("Benefit similar to the last loop");
                max_benefit_combinations.extend(current_combinations.clone());
                countdown = 2;
            } else {
                println!("Update new max benefit = {}", benefit);
                max_benefit = benefit;
                max_benefit_combinations.clear();
                max_benefit_combinations.extend(current_combinations.clone());
            }
        }
        return (max_benefit, max_benefit_combinations);
    }

    fn gen_map_live_outer_cells(&mut self) -> &Self {
        for row in 1..ROWS + 1 {
            for col in 1..COLS + 1 {
                if self.value_at(row as u8, col as u8) && (row % 6 == 1 || col % 6 == 1) {
                    self.map_live_outer_cells.insert(
                        (row as u8, col as u8),
                        Cell {
                            row: row as u8,
                            col: col as u8,
                        },
                    );
                }
            }
        }
        self
    }

    fn get_reachable_cells(&mut self) -> Vec<Cell> {
        let mut queue: VecDeque<Cell> = VecDeque::new();
        let mut visited: HashMap<Cell, bool> = HashMap::new();
        let mut queued: HashMap<Cell, bool> = HashMap::new();

        queue.push_back(Cell {
            col: MIMIC_INITIAL_COL as u8,
            row: MIMIC_INITIAL_ROW as u8,
        });

        while !queue.is_empty() {
            let cell = queue.pop_front().unwrap();
            visited.insert(cell, true);
            for neighbor_cell in cell.get_neighbors() {
                if !self.value_at_cell(&neighbor_cell) {
                    continue;
                }

                if cell.is_outer() && neighbor_cell.is_outer() {
                    continue;
                }

                if visited.contains_key(&neighbor_cell) || queued.contains_key(&neighbor_cell) {
                    continue;
                }

                queue.push_back(neighbor_cell);
                queued.insert(neighbor_cell, true);
            }
        }

        return visited.keys().cloned().collect();
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in self.cells.iter() {
            for &cell in row.iter() {
                write!(f, "{} ", if cell { "1" } else { "0" })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn parse_input() -> Vec<u8> {
    print!("Enter cells: ");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    input
        .trim()
        .split_whitespace()
        .filter_map(|s| s.parse::<u8>().ok())
        .collect()
}

fn main() {
    let input: Vec<u8> = parse_input();
    let mut board = Board::from_input(input);
    board.remove_redundant_blocks();
    println!("{}", board);
    println!("Live outer has {} cells", board.map_live_outer_cells.len());
    let (benefit, combinations) = board.solve();
    println!("The maximum benefit is {}", benefit);
    println!("All combinations:");
    for combination in combinations {
        println!("Cells: {}", combination.iter().join(", "));
    }
}
