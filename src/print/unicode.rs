use crate::graph::GitGraph;
use crate::settings::BranchSettings;
use itertools::join;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

const SPACE: char = ' ';
/*
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
 */
const DOT: char = '●';
const CIRCLE: char = '⦾';
/*
const ARR_L: char = '<';
const ARR_R: char = '>';
 */

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

    grid.to_string()
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

                    if branch.visual.column != par_branch.visual.column {
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
    pub fn to_string(&self) -> String {
        let rows = self
            .data
            .chunks(self.width)
            .map(|row| row.iter().collect::<String>());
        join(rows, "\n")
    }
}
