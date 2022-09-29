//! Create graphs in SVG format (Scalable Vector Graphics).

use crate::graph::GitGraph;
use crate::settings::Settings;
use svg::node::element::path::Data;
use svg::node::element::{Circle, Line, Path};
use svg::Document;

/// Creates a SVG visual representation of a graph.
pub fn print_svg(graph: &GitGraph, settings: &Settings) -> Result<String, String> {
    let mut document = Document::new();

    let max_idx = graph.commits.len();
    let mut max_column = 0;

    if settings.debug {
        for branch in &graph.all_branches {
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
        if let Some(trace) = info.branch_trace {
            let branch = &graph.all_branches[trace];
            let branch_color = &branch.visual.svg_color;

            if branch.visual.column.unwrap() > max_column {
                max_column = branch.visual.column.unwrap();
            }

            for p in 0..2 {
                if let Some(par_oid) = info.parents[p] {
                    if let Some(par_idx) = graph.indices.get(&par_oid) {
                        let par_info = &graph.commits[*par_idx];
                        let par_branch = &graph.all_branches[par_info.branch_trace.unwrap()];

                        let color = if info.is_merge {
                            &par_branch.visual.svg_color
                        } else {
                            branch_color
                        };

                        if branch.visual.column == par_branch.visual.column {
                            document = document.add(line(
                                idx,
                                branch.visual.column.unwrap(),
                                *par_idx,
                                par_branch.visual.column.unwrap(),
                                color,
                            ));
                        } else {
                            let split_index = super::get_deviate_index(graph, idx, *par_idx);
                            document = document.add(path(
                                idx,
                                branch.visual.column.unwrap(),
                                *par_idx,
                                par_branch.visual.column.unwrap(),
                                split_index,
                                color,
                            ));
                        }
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
    }
    let (x_max, y_max) = commit_coord(max_idx + 1, max_column + 1);
    document = document
        .set("viewBox", (0, 0, x_max, y_max))
        .set("width", x_max)
        .set("height", y_max);

    let mut out: Vec<u8> = vec![];
    svg::write(&mut out, &document).map_err(|err| err.to_string())?;
    Ok(String::from_utf8(out).unwrap_or_else(|_| "Invalid UTF8 character.".to_string()))
}

fn commit_dot(index: usize, column: usize, color: &str, filled: bool) -> Circle {
    let (x, y) = commit_coord(index, column);
    Circle::new()
        .set("cx", x)
        .set("cy", y)
        .set("r", 4)
        .set("fill", if filled { color } else { "white" })
        .set("stroke", color)
        .set("stroke-width", 1)
}

fn line(index1: usize, column1: usize, index2: usize, column2: usize, color: &str) -> Line {
    let (x1, y1) = commit_coord(index1, column1);
    let (x2, y2) = commit_coord(index2, column2);
    Line::new()
        .set("x1", x1)
        .set("y1", y1)
        .set("x2", x2)
        .set("y2", y2)
        .set("stroke", color)
        .set("stroke-width", 1)
}

fn bold_line(index1: usize, column1: usize, index2: usize, column2: usize, color: &str) -> Line {
    let (x1, y1) = commit_coord(index1, column1);
    let (x2, y2) = commit_coord(index2, column2);
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
    split_idx: usize,
    color: &str,
) -> Path {
    let c0 = commit_coord(index1, column1);

    let c1 = commit_coord(split_idx, column1);
    let c2 = commit_coord(split_idx + 1, column2);

    let c3 = commit_coord(index2, column2);

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

fn commit_coord(index: usize, column: usize) -> (f32, f32) {
    (15.0 * (column as f32 + 1.0), 15.0 * (index as f32 + 1.0))
}
