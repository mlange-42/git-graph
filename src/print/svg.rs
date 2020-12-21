use crate::graph::GitGraph;
use crate::settings::BranchSettings;
use lazy_static::lazy_static;
use std::cmp::max;
use svg::node::element::path::Data;
use svg::node::element::{Circle, Line, Path};
use svg::Document;

lazy_static! {
    static ref COLOR_UNKNOWN: (String, String) = (String::new(), "grey".to_string());
}

pub fn print_svg(
    graph: &GitGraph,
    settings: &BranchSettings,
    debug: bool,
) -> Result<String, String> {
    let mut document = Document::new();

    let max_idx = graph.commits.len();
    let mut max_column = 0;

    if debug {
        for branch in &graph.branches {
            if let (Some(start), Some(end)) = branch.range {
                document = document.add(bold_line(
                    start,
                    branch.visual.column.unwrap(),
                    end,
                    branch.visual.column.unwrap(),
                    "cyan",
                ));
            }
        }
    }

    for (idx, info) in graph.commits.iter().enumerate() {
        let branch = &graph.branches[info.branch_trace.unwrap()];
        let branch_color = &settings
            .color
            .get(branch.visual.color_group)
            .unwrap_or(&COLOR_UNKNOWN)
            .1;

        if branch.visual.column.unwrap() > max_column {
            max_column = branch.visual.column.unwrap();
        }

        for p in 0..2 {
            if let Some(par_oid) = info.parents[p] {
                let par_idx = graph.indices[&par_oid];
                let par_info = &graph.commits[par_idx];
                let par_branch = &graph.branches[par_info.branch_trace.unwrap()];

                let color = if info.is_merge {
                    &settings
                        .color
                        .get(par_branch.visual.color_group)
                        .unwrap_or(&COLOR_UNKNOWN)
                        .1
                } else {
                    branch_color
                };

                if branch.visual.column == par_branch.visual.column {
                    document = document.add(line(
                        idx,
                        branch.visual.column.unwrap(),
                        par_idx,
                        par_branch.visual.column.unwrap(),
                        color,
                    ));
                } else {
                    let mut min_split_idx = idx;
                    for sibling_oid in &graph.commits[par_idx].children {
                        let sibling_index = graph.indices[&sibling_oid];
                        let sibling_branch =
                            &graph.branches[graph.commits[sibling_index].branch_trace.unwrap()];
                        if sibling_oid != &info.oid
                            && sibling_branch.visual.column == par_branch.visual.column
                        {
                            if sibling_index > min_split_idx {
                                min_split_idx = sibling_index;
                            }
                        }
                    }
                    document = document.add(path(
                        idx,
                        branch.visual.column.unwrap(),
                        par_idx,
                        par_branch.visual.column.unwrap(),
                        info.is_merge,
                        min_split_idx,
                        color,
                    ));
                }
            }
        }

        document = document.add(commit_dot(
            idx,
            branch.visual.column.unwrap(),
            branch_color,
            !info.is_merge,
        ));
    }
    let (x_max, y_max) = commit_coord_u(max_idx + 1, max_column + 1);
    document = document
        .set("viewBox", (0, 0, x_max, y_max))
        .set("width", x_max)
        .set("height", y_max);

    let mut out: Vec<u8> = vec![];
    match svg::write(&mut out, &document) {
        Ok(_) => {}
        Err(err) => {
            return Err(err.to_string());
        }
    }
    String::from_utf8(out).map_err(|err| err.to_string())
}

fn commit_dot(index: usize, column: usize, color: &str, filled: bool) -> Circle {
    let (x, y) = commit_coord_u(index, column);
    Circle::new()
        .set("cx", x)
        .set("cy", y)
        .set("r", 4)
        .set("fill", if filled { color } else { "white" })
        .set("stroke", color)
        .set("stroke-width", 1)
}

fn line(index1: usize, column1: usize, index2: usize, column2: usize, color: &str) -> Line {
    let (x1, y1) = commit_coord_u(index1, column1);
    let (x2, y2) = commit_coord_u(index2, column2);
    Line::new()
        .set("x1", x1)
        .set("y1", y1)
        .set("x2", x2)
        .set("y2", y2)
        .set("stroke", color)
        .set("stroke-width", 1)
}

fn bold_line(index1: usize, column1: usize, index2: usize, column2: usize, color: &str) -> Line {
    let (x1, y1) = commit_coord_u(index1, column1);
    let (x2, y2) = commit_coord_u(index2, column2);
    Line::new()
        .set("x1", x1)
        .set("y1", y1)
        .set("x2", x2)
        .set("y2", y2)
        .set("stroke", color)
        .set("stroke-width", 5)
}

fn path(
    index1: usize,
    column1: usize,
    index2: usize,
    column2: usize,
    is_merge: bool,
    min_split_idx: usize,
    color: &str,
) -> Path {
    let c0 = commit_coord_u(index1, column1);
    let c1 = if is_merge {
        commit_coord_u(max(index1, min_split_idx), column1)
    } else {
        commit_coord_i(index2 as i32 - 1, column1 as i32)
    };
    let c2 = if is_merge {
        commit_coord_u(max(index1, min_split_idx) + 1, column2)
    } else {
        commit_coord_u(index2, column2)
    };
    let c3 = commit_coord_u(index2, column2);

    let m = (0.5 * (c1.0 + c2.0), 0.5 * (c1.1 + c2.1));

    let data = Data::new()
        .move_to(c0)
        .line_to(c1)
        .quadratic_curve_to((c1.0, m.1, m.0, m.1))
        .quadratic_curve_to((c2.0, m.1, c2.0, c2.1))
        .line_to(c3);

    Path::new()
        .set("d", data)
        .set("fill", "none")
        .set("stroke", color)
        .set("stroke-width", 1)
}

fn commit_coord_u(index: usize, column: usize) -> (f32, f32) {
    (15.0 * (column as f32 + 1.0), 15.0 * (index as f32 + 1.0))
}
fn commit_coord_i(index: i32, column: i32) -> (f32, f32) {
    (15.0 * (column as f32 + 1.0), 15.0 * (index as f32 + 1.0))
}
