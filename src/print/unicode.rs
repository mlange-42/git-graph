//! Create graphs in Unicode format with ANSI X3.64 / ISO 6429 colour codes

use crate::graph::{CommitInfo, GitGraph, HeadInfo};
use crate::print::format::CommitFormat;
use crate::settings::{Characters, Settings};
use itertools::Itertools;
use std::cmp::max;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::fmt::Write;
use textwrap::Options;
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

/**
UnicodeGraphInfo is a type alias for a tuple containing three elements:
graph-lines, text-lines, start-row

1.  graph_lines: `Vec<String>` - This represents the lines of the generated text-based graph
    visualization. Each `String` in this vector corresponds to a single row of
    the graph output, containing characters that form the visual representation
    of the commit history (like lines, dots, and branch intersections).

2.  text_lines: `Vec<String>`: This represents the lines of the commit messages or other
    textual information associated with each commit in the graph. Each `String`
    in this vector corresponds to a line of text that is displayed alongside
    the graph. This can include commit hashes, author information, commit
    messages, branch names, and tags, depending on the formatting settings.
    Some entries in this vector might be empty strings or correspond to
    inserted blank lines for visual spacing.

3.  start_row: `Vec<usize>`: Starting row for commit in the `graph.commits` vector.
*/
pub type UnicodeGraphInfo = (Vec<String>, Vec<String>, Vec<usize>);

/// Creates a text-based visual representation of a graph.
pub fn print_unicode(graph: &GitGraph, settings: &Settings) -> Result<UnicodeGraphInfo, String> {
    if graph.all_branches.is_empty() {
        return Ok((vec![], vec![], vec![]));
    }
    let num_cols = 2 * graph
        .all_branches
        .iter()
        .map(|b| b.visual.column.unwrap_or(0))
        .max()
        .unwrap()
        + 1;

    let head_idx = graph.indices.get(&graph.head.oid);

    let inserts = get_inserts(graph, settings.compact);

    let (indent1, indent2) = if let Some((_, ind1, ind2)) = settings.wrapping {
        (" ".repeat(ind1.unwrap_or(0)), " ".repeat(ind2.unwrap_or(0)))
    } else {
        ("".to_string(), "".to_string())
    };

    let wrap_options = if let Some((width, _, _)) = settings.wrapping {
        create_wrapping_options(width, &indent1, &indent2, num_cols + 4)?
    } else {
        None
    };

    // Compute commit text into text_lines and add blank rows
    // if needed to match branch graph inserts.
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

        let head = if head_idx == Some(&idx) {
            Some(&graph.head)
        } else {
            None
        };

        let lines = format(
            &settings.format,
            graph,
            info,
            head,
            settings.colored,
            &wrap_options,
        )?;

        let num_lines = if lines.is_empty() { 0 } else { lines.len() - 1 };
        let max_inserts = max(cnt_inserts, num_lines);
        let add_lines = max_inserts - num_lines;

        text_lines.extend(lines.into_iter().map(Some));
        text_lines.extend((0..add_lines).map(|_| None));

        offset += max_inserts;
    }

    let mut grid = Grid::new(
        num_cols,
        graph.commits.len() + offset,
        GridCell {
            character: SPACE,
            color: WHITE,
            pers: settings.branches.persistence.len() as u8 + 2,
        },
    );

    // Compute branch lines in grid
    for (idx, info) in graph.commits.iter().enumerate() {
        if let Some(trace) = info.branch_trace {
            let branch = &graph.all_branches[trace];
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
                        let par_branch = &graph.all_branches[par_info.branch_trace.unwrap()];
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
                            let split_index = super::get_deviate_index(graph, idx, *par_idx);
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

    if settings.reverse_commit_order {
        text_lines.reverse();
        grid.reverse();
    }

    let lines = print_graph(&settings.characters, &grid, text_lines, settings.colored);

    Ok((lines.0, lines.1, index_map))
}

/// Create `textwrap::Options` from width and indent.
fn create_wrapping_options<'a>(
    width: Option<usize>,
    indent1: &'a str,
    indent2: &'a str,
    graph_width: usize,
) -> Result<Option<Options<'a>>, String> {
    let wrapping = if let Some(width) = width {
        Some(
            textwrap::Options::new(width)
                .initial_indent(indent1)
                .subsequent_indent(indent2),
        )
    } else if atty::is(atty::Stream::Stdout) {
        let width = crossterm::terminal::size()
            .map_err(|err| err.to_string())?
            .0;
        let width = if width as usize > graph_width {
            width as usize - graph_width
        } else {
            1
        };
        Some(
            textwrap::Options::new(width)
                .initial_indent(indent1)
                .subsequent_indent(indent2),
        )
    } else {
        None
    };
    Ok(wrapping)
}

/// Draws a vertical line
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

/// Draws a horizontal line
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

/// Calculates required additional rows to visually connect commits that
/// are not direct descendants in the main commit list. These "inserts"
//  represent the horizontal lines in the graph.
///
/// # Arguments
///
/// * `graph`: A reference to the `GitGraph` structure containing the
//             commit and branch information.
/// * `compact`: A boolean indicating whether to use a compact layout,
//               potentially merging some insertions with commits.
///
/// # Returns
///
/// A `HashMap` where the keys are the indices of commits in the
/// `graph.commits` vector, and the values are vectors of vectors
/// of `Occ`. Each inner vector represents a potential row of
/// insertions needed *before* the commit at the key index. The
/// `Occ` enum describes what occupies a cell in that row
/// (either a commit or a range representing a connection).
///
fn get_inserts(graph: &GitGraph, compact: bool) -> HashMap<usize, Vec<Vec<Occ>>> {
    // Initialize an empty HashMap to store the required insertions. The key is the commit
    // index, and the value is a vector of rows, where each row is a vector of Occupations (`Occ`).
    let mut inserts: HashMap<usize, Vec<Vec<Occ>>> = HashMap::new();

    // First, for each commit, we initialize an entry in the `inserts`
    // map with a single row containing the commit itself. This ensures
    // that every commit has a position in the grid.
    for (idx, info) in graph.commits.iter().enumerate() {
        // Get the visual column assigned to the branch of this commit. Unwrap is safe here
        // because `branch_trace` should always point to a valid branch with an assigned column
        // for commits that are included in the filtered graph.
        let column = graph.all_branches[info.branch_trace.unwrap()]
            .visual
            .column
            .unwrap();

        inserts.insert(idx, vec![vec![Occ::Commit(idx, column)]]);
    }

    // Now, iterate through the commits again to identify connections
    // needed between parents that are not directly adjacent in the
    // `graph.commits` list.
    for (idx, info) in graph.commits.iter().enumerate() {
        // If the commit has a branch trace (meaning it belongs to a visualized branch).
        if let Some(trace) = info.branch_trace {
            // Get the `BranchInfo` for the current commit's branch.
            let branch = &graph.all_branches[trace];
            // Get the visual column of the current commit's branch. Unwrap is safe as explained above.
            let column = branch.visual.column.unwrap();

            // Iterate through the two possible parents of the current commit.
            for p in 0..2 {
                // If the commit has a parent at this index (0 for the first parent, 1 for the second).
                if let Some(par_oid) = info.parents[p] {
                    // Try to find the index of the parent commit in the `graph.commits` vector.
                    if let Some(par_idx) = graph.indices.get(&par_oid) {
                        let par_info = &graph.commits[*par_idx];
                        let par_branch = &graph.all_branches[par_info.branch_trace.unwrap()];
                        let par_column = par_branch.visual.column.unwrap();
                        // Determine the sorted range of columns between the current commit and its parent.
                        let column_range = sorted(column, par_column);

                        // If the column of the current commit is different from the column of its parent,
                        // it means we need to draw a horizontal line (an "insert") to connect them.
                        if column != par_column {
                            // Find the index in the `graph.commits` list where the visual connection
                            // should deviate from the parent's line. This helps in drawing the graph
                            // correctly when branches diverge or merge.
                            let split_index = super::get_deviate_index(graph, idx, *par_idx);
                            // Access the entry in the `inserts` map for the `split_index`.
                            match inserts.entry(split_index) {
                                // If there's already an entry at this `split_index` (meaning other
                                // insertions might be needed before this commit).
                                Occupied(mut entry) => {
                                    // Find the first available row in the existing vector of rows
                                    // where the new range doesn't overlap with existing occupations.
                                    let mut insert_at = entry.get().len();
                                    for (insert_idx, sub_entry) in entry.get().iter().enumerate() {
                                        let mut occ = false;
                                        // Check for overlaps with existing `Occ` in the current row.
                                        for other_range in sub_entry {
                                            // Check if the current column range overlaps with the other range.
                                            if other_range.overlaps(&column_range) {
                                                match other_range {
                                                    // If the other occupation is a commit.
                                                    Occ::Commit(target_index, _) => {
                                                        // In compact mode, we might allow overlap with the commit itself
                                                        // for merge commits (specifically the second parent) to keep the
                                                        // graph tighter.
                                                        if !compact
                                                            || !info.is_merge
                                                            || idx != *target_index
                                                            || p == 0
                                                        {
                                                            occ = true;
                                                            break;
                                                        }
                                                    }
                                                    // If the other occupation is a range (another connection).
                                                    Occ::Range(o_idx, o_par_idx, _, _) => {
                                                        // Avoid overlap with connections between the same commits.
                                                        if idx != *o_idx && par_idx != o_par_idx {
                                                            occ = true;
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        // If no overlap is found in this row, we can insert here.
                                        if !occ {
                                            insert_at = insert_idx;
                                            break;
                                        }
                                    }
                                    // Get a mutable reference to the vector of rows for this `split_index`.
                                    let vec = entry.get_mut();
                                    // If no suitable row was found, add a new row.
                                    if insert_at == vec.len() {
                                        vec.push(vec![Occ::Range(
                                            idx,
                                            *par_idx,
                                            column_range.0,
                                            column_range.1,
                                        )]);
                                    } else {
                                        // Otherwise, insert the new range into the found row.
                                        vec[insert_at].push(Occ::Range(
                                            idx,
                                            *par_idx,
                                            column_range.0,
                                            column_range.1,
                                        ));
                                    }
                                }
                                // If there's no entry at this `split_index` yet.
                                Vacant(entry) => {
                                    // Create a new entry with a single row containing the range.
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

    // Return the map of required insertions.
    inserts
}

/// Creates the complete graph visualization, incl. formatter commits.
fn print_graph(
    characters: &Characters,
    grid: &Grid,
    text_lines: Vec<Option<String>>,
    color: bool,
) -> (Vec<String>, Vec<String>) {
    let mut g_lines = vec![];
    let mut t_lines = vec![];

    for (row, line) in grid.data.chunks(grid.width).zip(text_lines.into_iter()) {
        let mut g_out = String::new();
        let mut t_out = String::new();

        if color {
            for cell in row {
                if cell.character == SPACE {
                    write!(g_out, "{}", cell.char(characters))
                } else {
                    write!(g_out, "{}", Paint::fixed(cell.color, cell.char(characters)))
                }
                .unwrap();
            }
        } else {
            let str = row
                .iter()
                .map(|cell| cell.char(characters))
                .collect::<String>();
            write!(g_out, "{}", str).unwrap();
        }

        if let Some(line) = line {
            write!(t_out, "{}", line).unwrap();
        }

        g_lines.push(g_out);
        t_lines.push(t_out);
    }

    (g_lines, t_lines)
}

/// Format a commit.
fn format(
    format: &CommitFormat,
    graph: &GitGraph,
    info: &CommitInfo,
    head: Option<&HeadInfo>,
    color: bool,
    wrapping: &Option<Options>,
) -> Result<Vec<String>, String> {
    let commit = graph
        .repository
        .find_commit(info.oid)
        .map_err(|err| err.message().to_string())?;

    let branch_str = format_branches(graph, info, head, color);

    let hash_color = if color { Some(HASH_COLOR) } else { None };

    crate::print::format::format(&commit, branch_str, wrapping, hash_color, format)
}

/// Format branches and tags.
pub fn format_branches(
    graph: &GitGraph,
    info: &CommitInfo,
    head: Option<&HeadInfo>,
    color: bool,
) -> String {
    let curr_color = info
        .branch_trace
        .map(|branch_idx| &graph.all_branches[branch_idx].visual.term_color);

    let mut branch_str = String::new();

    let head_str = "HEAD ->";
    if let Some(head) = head {
        if !head.is_branch {
            if color {
                write!(branch_str, " {}", Paint::fixed(HEAD_COLOR, head_str))
            } else {
                write!(branch_str, " {}", head_str)
            }
            .unwrap();
        }
    }

    if !info.branches.is_empty() {
        write!(branch_str, " (").unwrap();

        let branches = info.branches.iter().sorted_by_key(|br| {
            if let Some(head) = head {
                head.name != graph.all_branches[**br].name
            } else {
                false
            }
        });

        for (idx, branch_index) in branches.enumerate() {
            let branch = &graph.all_branches[*branch_index];
            let branch_color = branch.visual.term_color;

            if let Some(head) = head {
                if idx == 0 && head.is_branch {
                    if color {
                        write!(branch_str, "{} ", Paint::fixed(14, head_str))
                    } else {
                        write!(branch_str, "{} ", head_str)
                    }
                    .unwrap();
                }
            }

            if color {
                write!(branch_str, "{}", Paint::fixed(branch_color, &branch.name))
            } else {
                write!(branch_str, "{}", &branch.name)
            }
            .unwrap();

            if idx < info.branches.len() - 1 {
                write!(branch_str, ", ").unwrap();
            }
        }
        write!(branch_str, ")").unwrap();
    }

    if !info.tags.is_empty() {
        write!(branch_str, " [").unwrap();
        for (idx, tag_index) in info.tags.iter().enumerate() {
            let tag = &graph.all_branches[*tag_index];
            let tag_color = curr_color.unwrap_or(&tag.visual.term_color);

            if color {
                write!(branch_str, "{}", Paint::fixed(*tag_color, &tag.name[5..]))
            } else {
                write!(branch_str, "{}", &tag.name[5..])
            }
            .unwrap();

            if idx < info.tags.len() - 1 {
                write!(branch_str, ", ").unwrap();
            }
        }
        write!(branch_str, "]").unwrap();
    }

    branch_str
}

/// Occupied row ranges
enum Occ {
    /// Horizontal position of commit markers
    // First  field (usize): The index of a commit within the graph.commits vector.
    // Second field (usize): The visual column in the grid where this commit is located. This column is determined by the branch the commit belongs to.
    // Purpose: This variant of Occ signifies that a specific row in the grid is occupied by a commit marker (dot or circle) at a particular column.
    Commit(usize, usize), // index in Graph.commits, column

    /// Horizontal line connecting two commits
    // First  field (usize): The index of the starting commit of a visual connection (usually the child commit).
    // Second field (usize): The index of the ending commit of a visual connection (usually the parent commit).
    // Third  field (usize): The starting visual column of the range occupied by the connection line between the two commits. This is the minimum of the columns of the two connected commits.
    // Fourth field (usize): The ending visual column of the range occupied by the connection line between the two commits. This is the maximum of the columns of the two connected commits.
    // Purpose: This variant of Occ signifies that a range of columns in a particular row is occupied by a horizontal line segment connecting a commit to one of its parents. The range spans from the visual column of one commit to the visual column of the other.
    Range(usize, usize, usize, usize), // ?child index, parent index, leftmost column, rightmost column
}

impl Occ {
    fn overlaps(&self, (start, end): &(usize, usize)) -> bool {
        match self {
            Occ::Commit(_, col) => start <= col && end >= col,
            Occ::Range(_, _, s, e) => s <= end && e >= start,
        }
    }
}

/// Sorts two numbers in ascending order
fn sorted(v1: usize, v2: usize) -> (usize, usize) {
    if v2 > v1 {
        (v1, v2)
    } else {
        (v2, v1)
    }
}

/// One cell in a [Grid]
#[derive(Clone, Copy)]
struct GridCell {
    /// The symbol shown, encoded as in index into settings::Characters
    character: u8,
    /// Standard 8-bit terminal colour code
    color: u8,
    /// Persistence level. z-order, lower numbers take preceedence.
    pers: u8,
}

impl GridCell {
    pub fn char(&self, characters: &Characters) -> char {
        characters.chars[self.character as usize]
    }
}

/// Two-dimensional grid used to hold the graph layout.
///
/// This can be rendered as unicode text or as SVG.
struct Grid {
    width: usize,

    /// Grid cells are stored in row-major order.
    data: Vec<GridCell>,
}

impl Grid {
    pub fn new(width: usize, height: usize, initial: GridCell) -> Self {
        Grid {
            width,
            data: vec![initial; width * height],
        }
    }

    pub fn reverse(&mut self) {
        self.data.reverse();
    }
    /// Turn a 2D coordinate into an index of Grid.data
    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
    pub fn get_tuple(&self, x: usize, y: usize) -> (u8, u8, u8) {
        let v = self.data[self.index(x, y)];
        (v.character, v.color, v.pers)
    }
    pub fn set(&mut self, x: usize, y: usize, character: u8, color: u8, pers: u8) {
        let idx = self.index(x, y);
        self.data[idx] = GridCell {
            character,
            color,
            pers,
        };
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
        let cell = &mut self.data[idx];
        if let Some(character) = character {
            cell.character = character;
        }
        if let Some(color) = color {
            cell.color = color;
        }
        if let Some(pers) = pers {
            cell.pers = pers;
        }
    }
}
