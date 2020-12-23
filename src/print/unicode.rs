use crate::graph::GitGraph;
use crate::settings::BranchSettings;
use itertools::join;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

const SPACE: char = ' ';

const VER: char = '│';
const VER_L: char = '┤';
const VER_R: char = '├';
const HOR: char = '─';
const CROSS: char = '┼';
const HOR_U: char = '┴';
const HOR_D: char = '┬';
const L_U: char = '┘';
const L_D: char = '┐';
const R_U: char = '└';
const R_D: char = '┌';

const DOT: char = '●';
const CIRCLE: char = '○';

const ARR_L: char = '<';
const ARR_R: char = '>';

pub fn print_unicode(graph: &GitGraph, _settings: &BranchSettings, _debug: bool) -> String {
    let num_cols = 2 * graph
        .branches
        .iter()
        .map(|b| b.visual.column.unwrap_or(0))
        .max()
        .unwrap()
        + 1;

    let inserts = get_inserts(graph);

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

    let mut grid = CharGrid::new(num_cols, graph.commits.len() + offset);

    for (idx, info) in graph.commits.iter().enumerate() {
        let branch = &graph.branches[info.branch_trace.unwrap()];
        grid.set(
            branch.visual.column.unwrap() * 2,
            index_map[idx],
            if info.is_merge { CIRCLE } else { DOT },
        );
    }

    for (idx, info) in graph.commits.iter().enumerate() {
        if let Some(trace) = info.branch_trace {
            let branch = &graph.branches[trace];
            let column = branch.visual.column.unwrap();
            let idx_map = index_map[idx];

            for p in 0..2 {
                if let Some(par_oid) = info.parents[p] {
                    let par_idx = graph.indices[&par_oid];
                    let par_idx_map = index_map[par_idx];
                    let par_info = &graph.commits[par_idx];
                    let par_branch = &graph.branches[par_info.branch_trace.unwrap()];
                    let par_column = par_branch.visual.column.unwrap();

                    if branch.visual.column == par_branch.visual.column {
                        if par_idx_map > idx_map + 1 {
                            vline(&mut grid, (idx_map, par_idx_map), column);
                        }
                    } else {
                        let split_index = super::get_deviate_index(&graph, idx, par_idx);
                        let split_idx_map = index_map[split_index];
                        let inserts = &inserts[&split_index];
                        for (insert_idx, sub_entry) in inserts.iter().enumerate() {
                            for occ in sub_entry {
                                match occ {
                                    Occ::Commit(_, _) => {}
                                    Occ::Range(i1, i2, _, _) => {
                                        if *i1 == idx && *i2 == par_idx {
                                            vline(
                                                &mut grid,
                                                (idx_map, split_idx_map + insert_idx),
                                                column,
                                            );
                                            vline(
                                                &mut grid,
                                                (split_idx_map + insert_idx, par_idx_map),
                                                par_column,
                                            );
                                            hline(
                                                &mut grid,
                                                split_idx_map + insert_idx,
                                                (par_column, column),
                                                info.is_merge && p > 0,
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

    grid.to_string_block()
}

fn vline(grid: &mut CharGrid, (from, to): (usize, usize), column: usize) {
    for i in (from + 1)..to {
        let curr = grid.get(column * 2, i);
        match curr {
            HOR => grid.set(column * 2, i, CROSS),
            HOR_U | HOR_D | CROSS => {}
            L_D | L_U => grid.set(column * 2, i, VER_L),
            R_D | R_U => grid.set(column * 2, i, VER_R),
            _ => grid.set(column * 2, i, VER),
        }
    }
}

fn hline(grid: &mut CharGrid, index: usize, (from, to): (usize, usize), merge: bool) {
    if from == to {
        return;
    }
    let from_2 = from * 2;
    let to_2 = to * 2;
    if from < to {
        for column in (from_2 + 1)..to_2 {
            if merge && column == to_2 - 1 {
                grid.set(column, index, ARR_R);
            } else {
                let curr = grid.get(column, index);
                match curr {
                    VER => grid.set(column, index, CROSS),
                    HOR | CROSS | HOR_U | HOR_D => {}
                    L_U | R_U => grid.set(column, index, HOR_U),
                    L_D | R_D => grid.set(column, index, HOR_D),
                    _ => grid.set(column, index, HOR),
                }
            }
        }
        let left = grid.get(from_2, index);
        match left {
            VER => grid.set(from_2, index, VER_R),
            VER_R => {}
            HOR | L_U => grid.set(from_2, index, HOR_U),
            _ => grid.set(from_2, index, R_D),
        }
        let right = grid.get(to_2, index);
        match right {
            VER => grid.set(to_2, index, VER_L),
            VER_L | HOR_U => {}
            HOR | R_U => grid.set(to_2, index, HOR_U),
            _ => grid.set(to_2, index, L_U),
        }
    } else {
        for column in (to_2 + 1)..from_2 {
            if merge && column == to_2 + 1 {
                grid.set(column, index, ARR_L);
            } else {
                let curr = grid.get(column, index);
                match curr {
                    VER => grid.set(column, index, CROSS),
                    HOR | CROSS | HOR_U | HOR_D => {}
                    L_U | R_U => grid.set(column, index, HOR_U),
                    L_D | R_D => grid.set(column, index, HOR_D),
                    _ => grid.set(column, index, HOR),
                }
            }
        }
        let left = grid.get(to_2, index);
        match left {
            VER => grid.set(to_2, index, VER_R),
            VER_R => {}
            HOR | L_U => grid.set(to_2, index, HOR_U),
            _ => grid.set(to_2, index, R_U),
        }
        let right = grid.get(from_2, index);
        match right {
            VER => grid.set(from_2, index, VER_L),
            VER_L => {}
            HOR | R_D => grid.set(from_2, index, HOR_D),
            _ => grid.set(from_2, index, L_D),
        }
    }
}

fn get_inserts(graph: &GitGraph) -> HashMap<usize, Vec<Vec<Occ>>> {
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
                    let par_idx = graph.indices[&par_oid];
                    let par_info = &graph.commits[par_idx];
                    let par_branch = &graph.branches[par_info.branch_trace.unwrap()];
                    let par_column = par_branch.visual.column.unwrap();
                    let column_range = sorted(column, par_column);

                    if column != par_column {
                        let split_index = super::get_deviate_index(&graph, idx, par_idx);
                        match inserts.entry(split_index) {
                            Occupied(mut entry) => {
                                let mut insert_at = entry.get().len();
                                for (insert_idx, sub_entry) in entry.get().iter().enumerate() {
                                    let mut occ = false;
                                    for other_range in sub_entry {
                                        if other_range.overlaps(&column_range) {
                                            match other_range {
                                                Occ::Commit(_, _) => {
                                                    occ = true;
                                                    break;
                                                }
                                                Occ::Range(o_idx, o_par_idx, _, _) => {
                                                    if idx != *o_idx && par_idx != *o_par_idx {
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
                                        par_idx,
                                        column_range.0,
                                        column_range.1,
                                    )]);
                                } else {
                                    vec[insert_at].push(Occ::Range(
                                        idx,
                                        par_idx,
                                        column_range.0,
                                        column_range.1,
                                    ));
                                }
                            }
                            Vacant(entry) => {
                                entry.insert(vec![vec![Occ::Range(
                                    idx,
                                    par_idx,
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

    inserts
}

#[derive(Debug)]
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

#[allow(dead_code)]
pub struct CharGrid {
    width: usize,
    height: usize,
    data: Vec<char>,
}

impl CharGrid {
    pub fn new(width: usize, height: usize) -> Self {
        CharGrid {
            width,
            height,
            data: vec![SPACE; width * height],
        }
    }
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
    pub fn get(&self, x: usize, y: usize) -> char {
        self.data[self.index(x, y)]
    }
    pub fn set(&mut self, x: usize, y: usize, value: char) {
        let idx = self.index(x, y);
        self.data[idx] = value;
    }
    pub fn to_string_block(&self) -> String {
        let rows = self
            .data
            .chunks(self.width)
            .map(|row| row.iter().collect::<String>());
        join(rows, "\n")
    }
}
