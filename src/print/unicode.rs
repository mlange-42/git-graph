use crate::graph::{CommitInfo, GitGraph, HeadInfo};
use crate::print::format::{format_commit, format_multiline, format_oneline, CommitFormat};
use crate::settings::{Characters, Settings};
use itertools::Itertools;
use std::cmp::max;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::fmt::Write;
use yansi::Paint;

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
const HEAD_COLOR: u8 = 14;
const HASH_COLOR: u8 = 11;

pub fn print_unicode(graph: &GitGraph, settings: &Settings) -> Result<Vec<String>, String> {
    let num_cols = 2 * graph
        .branches
        .iter()
        .map(|b| b.visual.column.unwrap_or(0))
        .max()
        .unwrap()
        + 1;

    let head_idx = graph.indices[&graph.head.oid];

    let inserts = get_inserts(graph, settings.compact);

    let mut index_map = vec![];
    let mut text_lines = vec![];
    let mut offset = 0;
    for (idx, info) in graph.commits.iter().enumerate() {
        index_map.push(idx + offset);
        let cnt_inserts = if let Some(inserts) = inserts.get(&idx) {
            inserts
                .iter()
                .filter(|vec| {
                    vec.iter().all(|occ| match occ {
                        Occ::Commit(_, _) => false,
                        Occ::Range(_, _, _, _) => true,
                    })
                })
                .count()
        } else {
            0
        };

        let head = if idx == head_idx {
            Some(&graph.head)
        } else {
            None
        };

        let lines = format(&settings.format, &graph, &info, head, settings.colored)?;
        let max_inserts = max(cnt_inserts, lines.len() - 1);
        let add_lines = max_inserts - (lines.len() - 1);

        for line in lines.into_iter() {
            text_lines.push(Some(line));
        }
        for _ in 0..add_lines {
            text_lines.push(None);
        }

        offset += max_inserts;
    }

    let mut grid = Grid::new(
        num_cols,
        graph.commits.len() + offset,
        [SPACE, WHITE, settings.branches.persistence.len() as u8 + 2],
    );

    for (idx, info) in graph.commits.iter().enumerate() {
        if let Some(trace) = info.branch_trace {
            let branch = &graph.branches[trace];
            let column = branch.visual.column.unwrap();
            let idx_map = index_map[idx];

            let branch_color = branch.visual.term_color;

            grid.set(
                column * 2,
                idx_map,
                if info.is_merge { CIRCLE } else { DOT },
                branch_color,
                branch.persistence,
            );

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

    print_graph(&settings.characters, &grid, text_lines, settings.colored)
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
            DOT | CIRCLE => {}
            HOR => {
                grid.set_opt(column * 2, i, Some(CROSS), Some(color), Some(pers));
            }
            HOR_U | HOR_D => {
                grid.set_opt(column * 2, i, Some(CROSS), Some(color), Some(pers));
            }
            CROSS | VER | VER_L | VER_R => grid.set_opt(column * 2, i, None, new_col, new_pers),
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
                    DOT | CIRCLE => {}
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
            DOT | CIRCLE => {}
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
            DOT | CIRCLE => {}
            VER => grid.set_opt(to_2, index, Some(VER_L), None, None),
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
                    DOT | CIRCLE => {}
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
            DOT | CIRCLE => {}
            VER => grid.set_opt(to_2, index, Some(VER_R), None, None),
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
            DOT | CIRCLE => {}
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
    characters: &Characters,
    grid: &Grid,
    text_lines: Vec<Option<String>>,
    color: bool,
) -> Result<Vec<String>, String> {
    let mut lines = vec![];

    for (row, line) in grid.data.chunks(grid.width).zip(text_lines.into_iter()) {
        let mut out = String::new();

        write!(out, " ").map_err(|err| err.to_string())?;

        if color {
            for arr in row {
                if arr[0] == SPACE {
                    write!(out, "{}", characters.chars[arr[0] as usize])
                } else {
                    write!(
                        out,
                        "{}",
                        Paint::fixed(arr[1], characters.chars[arr[0] as usize])
                    )
                }
                .map_err(|err| err.to_string())?;
            }
        } else {
            let str = row
                .iter()
                .map(|arr| characters.chars[arr[0] as usize])
                .collect::<String>();
            write!(out, "{}", str).map_err(|err| err.to_string())?;
        }

        if let Some(line) = line {
            write!(out, "  {}", line).map_err(|err| err.to_string())?;
        }

        lines.push(out);
    }

    Ok(lines)
}

fn format(
    format: &CommitFormat,
    graph: &GitGraph,
    info: &CommitInfo,
    head: Option<&HeadInfo>,
    color: bool,
) -> Result<Vec<String>, String> {
    let commit = graph
        .repository
        .find_commit(info.oid)
        .map_err(|err| err.message().to_string())?;

    let branch_str = format_branches(&graph, &info, head, color)?;

    let hash_color = if color { Some(HASH_COLOR) } else { None };
    match format {
        CommitFormat::OneLine => format_oneline(&commit, branch_str, hash_color),
        CommitFormat::Short => format_multiline(&commit, branch_str, hash_color, 0),
        CommitFormat::Medium => format_multiline(&commit, branch_str, hash_color, 1),
        CommitFormat::Full => format_multiline(&commit, branch_str, hash_color, 2),
        CommitFormat::Format(format) => format_commit(format, &commit, branch_str, hash_color),
    }
}

fn format_branches(
    graph: &GitGraph,
    info: &CommitInfo,
    head: Option<&HeadInfo>,
    color: bool,
) -> Result<String, String> {
    let curr_color = info
        .branch_trace
        .map(|branch_idx| &graph.branches[branch_idx].visual.term_color);

    let mut branch_str = String::new();

    let head_str = "HEAD -> ";
    if let Some(head) = head {
        if !head.is_branch {
            if color {
                write!(branch_str, "{}", Paint::fixed(HEAD_COLOR, head_str))
            } else {
                write!(branch_str, "{}", head_str)
            }
            .map_err(|err| err.to_string())?;
        }
    }

    if !info.branches.is_empty() {
        write!(branch_str, " (").map_err(|err| err.to_string())?;

        let branches = info.branches.iter().sorted_by_key(|br| {
            if let Some(head) = head {
                head.name != graph.branches[**br].name
            } else {
                false
            }
        });

        for (idx, branch_index) in branches.enumerate() {
            let branch = &graph.branches[*branch_index];
            let branch_color = branch.visual.term_color;

            if let Some(head) = head {
                if idx == 0 && head.is_branch {
                    if color {
                        write!(branch_str, "{}", Paint::fixed(14, head_str))
                    } else {
                        write!(branch_str, "{}", head_str)
                    }
                    .map_err(|err| err.to_string())?;
                }
            }

            if color {
                write!(branch_str, "{}", Paint::fixed(branch_color, &branch.name))
            } else {
                write!(branch_str, "{}", &branch.name)
            }
            .map_err(|err| err.to_string())?;

            if idx < info.branches.len() - 1 {
                write!(branch_str, ", ").map_err(|err| err.to_string())?;
            }
        }
        write!(branch_str, ")").map_err(|err| err.to_string())?;
    }

    if !info.tags.is_empty() {
        write!(branch_str, " [").map_err(|err| err.to_string())?;
        for (idx, tag_index) in info.tags.iter().enumerate() {
            let tag = &graph.branches[*tag_index];
            let tag_color = curr_color.unwrap_or(&tag.visual.term_color);

            if color {
                write!(branch_str, "{}", Paint::fixed(*tag_color, &tag.name[5..]))
            } else {
                write!(branch_str, "{}", &tag.name[5..])
            }
            .map_err(|err| err.to_string())?;

            if idx < info.tags.len() - 1 {
                write!(branch_str, ", ").map_err(|err| err.to_string())?;
            }
        }
        write!(branch_str, "]").map_err(|err| err.to_string())?;
    }

    Ok(branch_str)
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

#[allow(dead_code)]
pub struct Grid {
    width: usize,
    height: usize,
    data: Vec<[u8; 3]>,
}

impl Grid {
    pub fn new(width: usize, height: usize, initial: [u8; 3]) -> Self {
        Grid {
            width,
            height,
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
