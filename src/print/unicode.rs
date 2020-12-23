use crate::graph::GitGraph;
use crate::print::colors::to_term_color;
use crate::settings::BranchSettings;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use term_painter::Color::Custom;
use term_painter::ToStyle;

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

const WHITE: u8 = 7;

pub fn print_unicode(
    graph: &GitGraph,
    settings: &BranchSettings,
    color: bool,
    _debug: bool,
) -> Result<(), String> {
    let num_cols = 2 * graph
        .branches
        .iter()
        .map(|b| b.visual.column.unwrap_or(0))
        .max()
        .unwrap()
        + 1;

    let color_list = settings
        .color
        .iter()
        .map(|(_, _, color)| to_term_color(color))
        .collect::<Result<Vec<u8>, String>>()?;

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

    let mut grid = Grid::new(num_cols, graph.commits.len() + offset, SPACE);
    let mut colors = Grid::new(num_cols, graph.commits.len() + offset, WHITE);

    let color_unknown = to_term_color(&settings.color_unknown.1)?;

    for (idx, info) in graph.commits.iter().enumerate() {
        let branch = &graph.branches[info.branch_trace.unwrap()];
        let column = branch.visual.column.unwrap() * 2;
        let draw_idx = index_map[idx];
        let branch_color = color_list
            .get(branch.visual.color_group)
            .unwrap_or(&color_unknown);
        grid.set(column, draw_idx, if info.is_merge { CIRCLE } else { DOT });
        colors.set(column, draw_idx, *branch_color);
    }

    for (idx, info) in graph.commits.iter().enumerate() {
        if let Some(trace) = info.branch_trace {
            let branch = &graph.branches[trace];
            let column = branch.visual.column.unwrap();
            let idx_map = index_map[idx];

            let branch_color = color_list
                .get(branch.visual.color_group)
                .unwrap_or(&color_unknown);

            for p in 0..2 {
                if let Some(par_oid) = info.parents[p] {
                    let par_idx = graph.indices[&par_oid];
                    let par_idx_map = index_map[par_idx];
                    let par_info = &graph.commits[par_idx];
                    let par_branch = &graph.branches[par_info.branch_trace.unwrap()];
                    let par_column = par_branch.visual.column.unwrap();

                    let color = if info.is_merge {
                        color_list
                            .get(par_branch.visual.color_group)
                            .unwrap_or(&color_unknown)
                    } else {
                        branch_color
                    };

                    if branch.visual.column == par_branch.visual.column {
                        if par_idx_map > idx_map + 1 {
                            vline(
                                &mut grid,
                                &mut colors,
                                (idx_map, par_idx_map),
                                column,
                                *color,
                            );
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
                                                &mut colors,
                                                (idx_map, split_idx_map + insert_idx),
                                                column,
                                                *color,
                                            );
                                            hline(
                                                &mut grid,
                                                &mut colors,
                                                split_idx_map + insert_idx,
                                                (par_column, column),
                                                info.is_merge && p > 0,
                                                *color,
                                            );
                                            vline(
                                                &mut grid,
                                                &mut colors,
                                                (split_idx_map + insert_idx, par_idx_map),
                                                par_column,
                                                *color,
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

    let index_map_inv: HashMap<usize, usize> = index_map
        .iter()
        .enumerate()
        .map(|(idx, line)| (*line, idx))
        .collect();

    print_graph(
        &graph,
        &index_map_inv,
        &color_list,
        color_unknown,
        &grid,
        &colors,
        color,
    )
}

fn vline(
    grid: &mut Grid<char>,
    colors: &mut Grid<u8>,
    (from, to): (usize, usize),
    column: usize,
    color: u8,
) {
    for i in (from + 1)..to {
        let curr = grid.get(column * 2, i);
        match curr {
            HOR => {
                grid.set(column * 2, i, CROSS);
                colors.set(column * 2, i, color);
            }
            HOR_U | HOR_D => {
                grid.set(column * 2, i, CROSS);
                colors.set(column * 2, i, color);
            }
            CROSS | VER | VER_L | VER_R => {}
            L_D | L_U => {
                grid.set(column * 2, i, VER_L);
                colors.set(column * 2, i, color);
            }
            R_D | R_U => {
                grid.set(column * 2, i, VER_R);
                colors.set(column * 2, i, color);
            }
            _ => {
                grid.set(column * 2, i, VER);
                colors.set(column * 2, i, color);
            }
        }
    }
}

fn hline(
    grid: &mut Grid<char>,
    colors: &mut Grid<u8>,
    index: usize,
    (from, to): (usize, usize),
    merge: bool,
    color: u8,
) {
    if from == to {
        return;
    }
    let from_2 = from * 2;
    let to_2 = to * 2;
    if from < to {
        for column in (from_2 + 1)..to_2 {
            if merge && column == to_2 - 1 {
                grid.set(column, index, ARR_R);
                colors.set(column, index, color);
            } else {
                let curr = grid.get(column, index);
                match curr {
                    VER => grid.set(column, index, CROSS),
                    HOR | CROSS | HOR_U | HOR_D => {}
                    L_U | R_U => grid.set(column, index, HOR_U),
                    L_D | R_D => grid.set(column, index, HOR_D),
                    _ => {
                        grid.set(column, index, HOR);
                        colors.set(column, index, color);
                    }
                }
            }
        }
        let left = grid.get(from_2, index);
        match left {
            VER => grid.set(from_2, index, VER_R),
            VER_R => {}
            HOR | L_U => grid.set(from_2, index, HOR_U),
            _ => {
                grid.set(from_2, index, R_D);
                colors.set(from_2, index, color);
            }
        }
        let right = grid.get(to_2, index);
        match right {
            VER => grid.set(to_2, index, VER_L),
            VER_L | HOR_U => {}
            HOR | R_U => grid.set(to_2, index, HOR_U),
            _ => {
                grid.set(to_2, index, L_U);
                colors.set(to_2, index, color);
            }
        }
    } else {
        for column in (to_2 + 1)..from_2 {
            if merge && column == to_2 + 1 {
                grid.set(column, index, ARR_L);
                colors.set(column, index, color);
            } else {
                let curr = grid.get(column, index);
                match curr {
                    VER => grid.set(column, index, CROSS),
                    HOR | CROSS | HOR_U | HOR_D => {}
                    L_U | R_U => grid.set(column, index, HOR_U),
                    L_D | R_D => grid.set(column, index, HOR_D),
                    _ => {
                        grid.set(column, index, HOR);
                        colors.set(column, index, color);
                    }
                }
            }
        }
        let left = grid.get(to_2, index);
        match left {
            VER => grid.set(to_2, index, VER_R),
            VER_R => {}
            HOR | L_U => grid.set(to_2, index, HOR_U),
            _ => {
                grid.set(to_2, index, R_U);
                colors.set(to_2, index, color);
            }
        }
        let right = grid.get(from_2, index);
        match right {
            VER => grid.set(from_2, index, VER_L),
            VER_L => {}
            HOR | R_D => grid.set(from_2, index, HOR_D),
            _ => {
                grid.set(from_2, index, L_D);
                colors.set(from_2, index, color);
            }
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

fn print_graph(
    graph: &GitGraph,
    line_to_index: &HashMap<usize, usize>,
    colors: &[u8],
    color_unknown: u8,
    grid: &Grid<char>,
    color_grid: &Grid<u8>,
    color: bool,
) -> Result<(), String> {
    if color {
        let rows = grid.data.chunks(grid.width);
        let col_rows = color_grid.data.chunks(grid.width);

        for (line_idx, (row, cols)) in rows.zip(col_rows).enumerate() {
            let index = line_to_index.get(&line_idx);
            print_pre(&graph, index);
            for (&c, col) in row.iter().zip(cols) {
                if c == SPACE {
                    print!(" ");
                } else {
                    print_colored_char(c, *col);
                }
            }
            print_post(&graph, colors, color_unknown, index, color)?;
            println!();
        }
    } else {
        for (line_idx, row) in grid.data.chunks(grid.width).enumerate() {
            let index = line_to_index.get(&line_idx);
            print_pre(&graph, index);
            let str = row.iter().collect::<String>();
            print!("{}", str);
            print_post(&graph, colors, color_unknown, index, color)?;
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

fn print_post(
    graph: &GitGraph,
    colors: &[u8],
    color_unknown: u8,
    index: Option<&usize>,
    color: bool,
) -> Result<(), String> {
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
                let branch_color = colors
                    .get(branch.visual.color_group)
                    .unwrap_or(&color_unknown);
                if color {
                    print_colored_str(&branch.name, *branch_color);
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

pub struct Grid<T> {
    width: usize,
    data: Vec<T>,
}

impl<T: Copy> Grid<T> {
    pub fn new(width: usize, height: usize, initial: T) -> Self {
        Grid {
            width,
            data: vec![initial; width * height],
        }
    }
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
    pub fn get(&self, x: usize, y: usize) -> T {
        self.data[self.index(x, y)]
    }
    pub fn set(&mut self, x: usize, y: usize, value: T) {
        let idx = self.index(x, y);
        self.data[idx] = value;
    }
}
