use cached::proc_macro::cached;
use itertools::Itertools;
use std::collections::HashMap;
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

#[derive(Copy, Clone, Debug)]
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

        for &value in input.iter() {
            let col = value / 10;
            let row = value % 10;
            if !is_valid_location((row, col)) {
                panic!("Invalid row or col input");
            }
            println!("Will drop cell ({} - {})", row, col);
            board.drop_cell(row, col);
        }
        board
    }

    fn remove_unreachable_blocks(&mut self) {
        let mut removing_cells: Vec<Cell> = Vec::new();

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
            }
        }

        for cell in &removing_cells {
            self.drop_cell(cell.row, cell.col);
        }
    }

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
            println!("Dropping cells: {:?}", removing_cells);
            for cell in &removing_cells {
                self.drop_cell(cell.row, cell.col);
            }
            removing_cells.clear();
        }
    }

    // fn check_move(&self, from: &Cell, to: &Cell) -> bool {
    //     if !self.cells[from.row][from.col] || !self.cells[to.row][to.col] {
    //         return false;
    //     }
    // }
    //
    fn get_available_blocks(&mut self) -> Vec<Cell> {
        let mut available_blocks: Vec<Cell> = Vec::new();
        for row in 1..ROWS + 1 {
            for col in 1..COLS + 1 {
                let cell = Cell {
                    row: row as u8,
                    col: col as u8,
                };
                if self.value_at_cell(&cell)
                    && row != MIMIC_INITIAL_ROW
                    && col != MIMIC_INITIAL_COL
                    && !cell.is_outer()
                {
                    available_blocks.push(cell);
                }
            }
        }
        available_blocks
    }

    fn count_living_cells(&self) -> usize {
        return self.cells.iter().flatten().filter(|&&x| x).count();
    }

    fn calc_benefit(&mut self, removing_cells: &Vec<Cell>) -> (isize, Vec<Cell>) {
        let num_initial_living_cells: usize = self.count_living_cells();
        let mut imaginery_board = self.create_imagine_board(removing_cells);
        imaginery_board.remove_unreachable_blocks();

        let num_total_removing_cells: usize =
            self.map_live_outer_cells.len() + removing_cells.len();
        if num_total_removing_cells > MAX_BLOCKS_TO_REMOVE {
            return (-1, Vec::new());
        }

        let total_removing_cells: Vec<Cell> = imaginery_board
            .map_live_outer_cells
            .values()
            .cloned()
            .into_iter()
            .chain(removing_cells.clone().into_iter())
            .collect();

        return (
            (num_initial_living_cells - num_total_removing_cells) as isize,
            total_removing_cells,
        );
    }

    fn solve(&mut self) -> (isize, Vec<Vec<Cell>>) {
        let available_blocks: Vec<Cell> = self.get_available_blocks();

        let mut combinations: Vec<Vec<Cell>> = Vec::new();
        for size in 1..11 {
            let combos: Vec<Vec<Cell>> = available_blocks
                .clone()
                .into_iter()
                .combinations(size)
                .collect();
            combinations.extend(combos);
        }

        let mut max_benefit_combinations: Vec<Vec<Cell>> = Vec::new();
        let mut max_benefit: isize = 0;
        for combination in combinations {
            let (benefit, removing_cells) = self.calc_benefit(&combination);
            if benefit < max_benefit {
                continue;
            } else if benefit == max_benefit {
                max_benefit_combinations.push(removing_cells);
            } else {
                max_benefit = benefit;
                max_benefit_combinations.clear();
                max_benefit_combinations.push(removing_cells);
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
    println!("{:?}", board);
    println!("{}", board);
    println!("{:?}", board.remove_redundant_blocks());
    let (benefit, combinations) = board.solve();
    println!("The maximum benefit is {}", benefit);
    println!("All combinations: {:?}", combinations);
}
