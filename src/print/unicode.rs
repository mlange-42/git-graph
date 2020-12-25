use crate::graph::GitGraph;
use crate::settings::{Characters, Settings};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use term_painter::Color::Custom;
use term_painter::ToStyle;

const SPACE: u8 = 0;
const DOT: u8 = 1;
const CIRCLE: u8 = 2;
const VER: u8 = 3;
const HOR: u8 = 4;
const CROSS: u8 = 5;
const R_U: u8 = 6;
const R_D: u8 = 7;
const L_D: u8 = 8;
const L_U: u8 = 9;
const VER_L: u8 = 10;
const VER_R: u8 = 11;
const HOR_U: u8 = 12;
const HOR_D: u8 = 13;

const ARR_L: u8 = 14;
const ARR_R: u8 = 15;

const WHITE: u8 = 7;

pub fn print_unicode(graph: &GitGraph, settings: &Settings) -> Result<(), String> {
    let num_cols = 2 * graph
        .branches
        .iter()
        .map(|b| b.visual.column.unwrap_or(0))
        .max()
        .unwrap()
        + 1;

    let inserts = get_inserts(graph, settings.compact);

    let mut index_map = vec![];

    let mut offset = 0;
    for idx in 0..graph.commits.len() {
        index_map.push(idx + offset);
        if let Some(inserts) = inserts.get(&idx) {
            offset += inserts
                .iter()
                .filter(|vec| {
                    vec.iter().all(|occ| match occ {
                        Occ::Commit(_, _) => false,
                        Occ::Range(_, _, _, _) => true,
                    })
                })
                .count();
        }
    }

    let mut grid = Grid::new(
        num_cols,
        graph.commits.len() + offset,
        [SPACE, WHITE, settings.branches.persistence.len() as u8 + 1],
    );

    for (idx, info) in graph.commits.iter().enumerate() {
        let branch = &graph.branches[info.branch_trace.unwrap()];
        let column = branch.visual.column.unwrap() * 2;
        let draw_idx = index_map[idx];
        let branch_color = branch.visual.term_color;

        grid.set(
            column,
            draw_idx,
            if info.is_merge { CIRCLE } else { DOT },
            branch_color,
            branch.persistence,
        );
    }

    for (idx, info) in graph.commits.iter().enumerate() {
        if let Some(trace) = info.branch_trace {
            let branch = &graph.branches[trace];
            let column = branch.visual.column.unwrap();
            let idx_map = index_map[idx];

            let branch_color = branch.visual.term_color;

            for p in 0..2 {
                if let Some(par_oid) = info.parents[p] {
                    if let Some(par_idx) = graph.indices.get(&par_oid) {
                        let par_idx_map = index_map[*par_idx];
                        let par_info = &graph.commits[*par_idx];
                        let par_branch = &graph.branches[par_info.branch_trace.unwrap()];
                        let par_column = par_branch.visual.column.unwrap();

                        let (color, pers) = if info.is_merge {
                            (par_branch.visual.term_color, par_branch.persistence)
                        } else {
                            (branch_color, branch.persistence)
                        };

                        if branch.visual.column == par_branch.visual.column {
                            if par_idx_map > idx_map + 1 {
                                vline(&mut grid, (idx_map, par_idx_map), column, color, pers);
                            }
                        } else {
                            let split_index = super::get_deviate_index(&graph, idx, *par_idx);
                            let split_idx_map = index_map[split_index];
                            let inserts = &inserts[&split_index];
                            for (insert_idx, sub_entry) in inserts.iter().enumerate() {
                                for occ in sub_entry {
                                    match occ {
                                        Occ::Commit(_, _) => {}
                                        Occ::Range(i1, i2, _, _) => {
                                            if *i1 == idx && i2 == par_idx {
                                                vline(
                                                    &mut grid,
                                                    (idx_map, split_idx_map + insert_idx),
                                                    column,
                                                    color,
                                                    pers,
                                                );
                                                hline(
                                                    &mut grid,
                                                    split_idx_map + insert_idx,
                                                    (par_column, column),
                                                    info.is_merge && p > 0,
                                                    color,
                                                    pers,
                                                );
                                                vline(
                                                    &mut grid,
                                                    (split_idx_map + insert_idx, par_idx_map),
                                                    par_column,
                                                    color,
                                                    pers,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let index_map_inv: HashMap<usize, usize> = index_map
        .iter()
        .enumerate()
        .map(|(idx, line)| (*line, idx))
        .collect();

    print_graph(
        &graph,
        &index_map_inv,
        &settings.characters,
        &grid,
        settings.colored,
    )
}

fn vline(grid: &mut Grid, (from, to): (usize, usize), column: usize, color: u8, pers: u8) {
    for i in (from + 1)..to {
        let (curr, _, old_pers) = grid.get_tuple(column * 2, i);
        let (new_col, new_pers) = if pers < old_pers {
            (Some(color), Some(pers))
        } else {
            (None, None)
        };
        match curr {
            HOR => {
                grid.set_opt(column * 2, i, Some(CROSS), new_col, new_pers);
            }
            HOR_U | HOR_D => {
                grid.set_opt(column * 2, i, Some(CROSS), new_col, new_pers);
            }
            CROSS | VER | VER_L | VER_R => {}
            L_D | L_U => {
                grid.set_opt(column * 2, i, Some(VER_L), new_col, new_pers);
            }
            R_D | R_U => {
                grid.set_opt(column * 2, i, Some(VER_R), new_col, new_pers);
            }
            _ => {
                grid.set_opt(column * 2, i, Some(VER), new_col, new_pers);
            }
        }
    }
}

fn hline(
    grid: &mut Grid,
    index: usize,
    (from, to): (usize, usize),
    merge: bool,
    color: u8,
    pers: u8,
) {
    if from == to {
        return;
    }
    let from_2 = from * 2;
    let to_2 = to * 2;
    if from < to {
        for column in (from_2 + 1)..to_2 {
            if merge && column == to_2 - 1 {
                grid.set(column, index, ARR_R, color, pers);
            } else {
                let (curr, _, old_pers) = grid.get_tuple(column, index);
                let (new_col, new_pers) = if pers < old_pers {
                    (Some(color), Some(pers))
                } else {
                    (None, None)
                };
                match curr {
                    VER => grid.set_opt(column, index, Some(CROSS), None, None),
                    HOR | CROSS | HOR_U | HOR_D => {
                        grid.set_opt(column, index, None, new_col, new_pers)
                    }
                    L_U | R_U => grid.set_opt(column, index, Some(HOR_U), new_col, new_pers),
                    L_D | R_D => grid.set_opt(column, index, Some(HOR_D), new_col, new_pers),
                    _ => {
                        grid.set_opt(column, index, Some(HOR), new_col, new_pers);
                    }
                }
            }
        }

        let (left, _, old_pers) = grid.get_tuple(from_2, index);
        let (new_col, new_pers) = if pers < old_pers {
            (Some(color), Some(pers))
        } else {
            (None, None)
        };
        match left {
            VER => grid.set_opt(from_2, index, Some(VER_R), new_col, new_pers),
            VER_L => grid.set_opt(from_2, index, Some(CROSS), None, None),
            VER_R => {}
            HOR | L_U => grid.set_opt(from_2, index, Some(HOR_U), new_col, new_pers),
            _ => {
                grid.set_opt(from_2, index, Some(R_D), new_col, new_pers);
            }
        }

        let (right, _, old_pers) = grid.get_tuple(to_2, index);
        let (new_col, new_pers) = if pers < old_pers {
            (Some(color), Some(pers))
        } else {
            (None, None)
        };
        match right {
            VER => grid.set_opt(to_2, index, Some(VER_L), None, None),
            CIRCLE | DOT => {}
            VER_L | HOR_U => grid.set_opt(to_2, index, None, new_col, new_pers),
            HOR | R_U => grid.set_opt(to_2, index, Some(HOR_U), new_col, new_pers),
            _ => {
                grid.set_opt(to_2, index, Some(L_U), new_col, new_pers);
            }
        }
    } else {
        for column in (to_2 + 1)..from_2 {
            if merge && column == to_2 + 1 {
                grid.set(column, index, ARR_L, color, pers);
            } else {
                let (curr, _, old_pers) = grid.get_tuple(column, index);
                let (new_col, new_pers) = if pers < old_pers {
                    (Some(color), Some(pers))
                } else {
                    (None, None)
                };
                match curr {
                    VER => grid.set_opt(column, index, Some(CROSS), None, None),
                    HOR | CROSS | HOR_U | HOR_D => {
                        grid.set_opt(column, index, None, new_col, new_pers)
                    }
                    L_U | R_U => grid.set_opt(column, index, Some(HOR_U), new_col, new_pers),
                    L_D | R_D => grid.set_opt(column, index, Some(HOR_D), new_col, new_pers),
                    _ => {
                        grid.set_opt(column, index, Some(HOR), new_col, new_pers);
                    }
                }
            }
        }

        let (left, _, old_pers) = grid.get_tuple(to_2, index);
        let (new_col, new_pers) = if pers < old_pers {
            (Some(color), Some(pers))
        } else {
            (None, None)
        };
        match left {
            VER => grid.set_opt(to_2, index, Some(VER_R), None, None),
            CIRCLE | DOT => {}
            VER_R => grid.set_opt(to_2, index, None, new_col, new_pers),
            HOR | L_U => grid.set_opt(to_2, index, Some(HOR_U), new_col, new_pers),
            _ => {
                grid.set_opt(to_2, index, Some(R_U), new_col, new_pers);
            }
        }

        let (right, _, old_pers) = grid.get_tuple(from_2, index);
        let (new_col, new_pers) = if pers < old_pers {
            (Some(color), Some(pers))
        } else {
            (None, None)
        };
        match right {
            VER => grid.set_opt(from_2, index, Some(VER_L), new_col, new_pers),
            VER_R => grid.set_opt(from_2, index, Some(CROSS), None, None),
            VER_L => grid.set_opt(from_2, index, None, new_col, new_pers),
            HOR | R_D => grid.set_opt(from_2, index, Some(HOR_D), new_col, new_pers),
            _ => {
                grid.set_opt(from_2, index, Some(L_D), new_col, new_pers);
            }
        }
    }
}

fn get_inserts(graph: &GitGraph, compact: bool) -> HashMap<usize, Vec<Vec<Occ>>> {
    let mut inserts: HashMap<usize, Vec<Vec<Occ>>> = HashMap::new();

    for (idx, info) in graph.commits.iter().enumerate() {
        let column = graph.branches[info.branch_trace.unwrap()]
            .visual
            .column
            .unwrap();

        inserts.insert(idx, vec![vec![Occ::Commit(idx, column)]]);
    }

    for (idx, info) in graph.commits.iter().enumerate() {
        if let Some(trace) = info.branch_trace {
            let branch = &graph.branches[trace];
            let column = branch.visual.column.unwrap();

            for p in 0..2 {
                if let Some(par_oid) = info.parents[p] {
                    if let Some(par_idx) = graph.indices.get(&par_oid) {
                        let par_info = &graph.commits[*par_idx];
                        let par_branch = &graph.branches[par_info.branch_trace.unwrap()];
                        let par_column = par_branch.visual.column.unwrap();
                        let column_range = sorted(column, par_column);

                        if column != par_column {
                            let split_index = super::get_deviate_index(&graph, idx, *par_idx);
                            match inserts.entry(split_index) {
                                Occupied(mut entry) => {
                                    let mut insert_at = entry.get().len();
                                    for (insert_idx, sub_entry) in entry.get().iter().enumerate() {
                                        let mut occ = false;
                                        for other_range in sub_entry {
                                            if other_range.overlaps(&column_range) {
                                                match other_range {
                                                    Occ::Commit(target_index, _) => {
                                                        if !compact
                                                            || !info.is_merge
                                                            || idx != *target_index
                                                            || p == 0
                                                        {
                                                            occ = true;
                                                            break;
                                                        }
                                                    }
                                                    Occ::Range(o_idx, o_par_idx, _, _) => {
                                                        if idx != *o_idx && par_idx != o_par_idx {
                                                            occ = true;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        if !occ {
                                            insert_at = insert_idx;
                                            break;
                                        }
                                    }
                                    let vec = entry.get_mut();
                                    if insert_at == vec.len() {
                                        vec.push(vec![Occ::Range(
                                            idx,
                                            *par_idx,
                                            column_range.0,
                                            column_range.1,
                                        )]);
                                    } else {
                                        vec[insert_at].push(Occ::Range(
                                            idx,
                                            *par_idx,
                                            column_range.0,
                                            column_range.1,
                                        ));
                                    }
                                }
                                Vacant(entry) => {
                                    entry.insert(vec![vec![Occ::Range(
                                        idx,
                                        *par_idx,
                                        column_range.0,
                                        column_range.1,
                                    )]]);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    inserts
}

fn print_graph(
    graph: &GitGraph,
    line_to_index: &HashMap<usize, usize>,
    characters: &Characters,
    grid: &Grid,
    color: bool,
) -> Result<(), String> {
    if color {
        for (line_idx, row) in grid.data.chunks(grid.width).enumerate() {
            let index = line_to_index.get(&line_idx);
            print_pre(&graph, index);
            for arr in row {
                if arr[0] == SPACE {
                    print!("{}", characters.chars[arr[0] as usize]);
                } else {
                    print_colored_char(characters.chars[arr[0] as usize], arr[1]);
                }
            }
            print_post(&graph, index, color)?;
            println!();
        }
    } else {
        for (line_idx, row) in grid.data.chunks(grid.width).enumerate() {
            let index = line_to_index.get(&line_idx);
            print_pre(&graph, index);
            let str = row
                .iter()
                .map(|arr| characters.chars[arr[0] as usize])
                .collect::<String>();
            print!("{}", str);
            print_post(&graph, index, color)?;
            println!();
        }
    }
    Ok(())
}

fn print_pre(graph: &GitGraph, index: Option<&usize>) {
    if let Some(index) = index {
        let info = &graph.commits[*index];
        print!(" {} ", &info.oid.to_string()[..7]);
    } else {
        print!("         ");
    }
}

fn print_post(graph: &GitGraph, index: Option<&usize>, color: bool) -> Result<(), String> {
    if let Some(index) = index {
        let info = &graph.commits[*index];
        let commit = match graph.repository.find_commit(info.oid) {
            Ok(c) => c,
            Err(err) => return Err(err.to_string()),
        };
        print!("  ");
        if !info.branches.is_empty() {
            print!("(");
            for (idx, branch_index) in info.branches.iter().enumerate() {
                let branch = &graph.branches[*branch_index];
                let branch_color = branch.visual.term_color;
                if color {
                    print_colored_str(&branch.name, branch_color);
                } else {
                    print!("{}", &branch.name);
                }
                if idx < info.branches.len() - 1 {
                    print!(", ");
                }
            }
            print!(") ");
        }
        print!("{}", commit.summary().unwrap_or(""));
    }
    Ok(())
}

fn print_colored_char(character: char, color: u8) {
    let str = Custom(color as u32).paint(character);
    print!("{}", str);
}

fn print_colored_str(string: &str, color: u8) {
    let str = Custom(color as u32).paint(string);
    print!("{}", str);
}

enum Occ {
    Commit(usize, usize),
    Range(usize, usize, usize, usize),
}

impl Occ {
    fn overlaps(&self, (start, end): &(usize, usize)) -> bool {
        match self {
            Occ::Commit(_, col) => start <= col && end >= col,
            Occ::Range(_, _, s, e) => s <= end && e >= start,
        }
    }
}

fn sorted(v1: usize, v2: usize) -> (usize, usize) {
    if v2 > v1 {
        (v1, v2)
    } else {
        (v2, v1)
    }
}

pub struct Grid {
    width: usize,
    data: Vec<[u8; 3]>,
}

impl Grid {
    pub fn new(width: usize, height: usize, initial: [u8; 3]) -> Self {
        Grid {
            width,
            data: vec![initial; width * height],
        }
    }
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
    pub fn get(&self, x: usize, y: usize) -> &[u8; 3] {
        &self.data[self.index(x, y)]
    }
    pub fn get_tuple(&self, x: usize, y: usize) -> (u8, u8, u8) {
        let v = self.data[self.index(x, y)];
        (v[0], v[1], v[2])
    }
    pub fn get_char(&self, x: usize, y: usize) -> u8 {
        self.data[self.index(x, y)][0]
    }
    pub fn get_col(&self, x: usize, y: usize) -> u8 {
        self.data[self.index(x, y)][1]
    }
    pub fn get_pers(&self, x: usize, y: usize) -> u8 {
        self.data[self.index(x, y)][1]
    }

    pub fn set(&mut self, x: usize, y: usize, character: u8, color: u8, pers: u8) {
        let idx = self.index(x, y);
        self.data[idx] = [character, color, pers];
    }
    pub fn set_opt(
        &mut self,
        x: usize,
        y: usize,
        character: Option<u8>,
        color: Option<u8>,
        pers: Option<u8>,
    ) {
        let idx = self.index(x, y);
        let arr = &mut self.data[idx];
        if let Some(character) = character {
            arr[0] = character;
        }
        if let Some(color) = color {
            arr[1] = color;
        }
        if let Some(pers) = pers {
            arr[2] = pers;
        }
    }
}
