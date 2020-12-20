use crate::graph::GitGraph;
use crate::settings::Settings;
use svg::node::element::path::Data;
use svg::node::element::{Circle, Line, Path};
use svg::Document;

pub fn print_svg(graph: &GitGraph, _settings: &Settings) -> Result<String, String> {
    let mut document = Document::new();

    let max_idx = graph.commits.len();
    let mut max_column = 0;

    for (idx, info) in graph.commits.iter().enumerate() {
        let branch = &graph.branches[info.branch_trace.unwrap()];

        if branch.column.unwrap() > max_column {
            max_column = branch.column.unwrap();
        }

        for p in 0..2 {
            if let Some(par_oid) = info.parents[p] {
                let par_idx = graph.indices[&par_oid];
                let par_info = &graph.commits[par_idx];
                let par_branch = &graph.branches[par_info.branch_trace.unwrap()];
                if branch.column == par_branch.column {
                    document = document.add(line(
                        idx,
                        branch.column.unwrap(),
                        par_idx,
                        par_branch.column.unwrap(),
                        "red",
                    ));
                } else {
                    document = document.add(path(
                        idx,
                        branch.column.unwrap(),
                        par_idx,
                        par_branch.column.unwrap(),
                        info.is_merge,
                        "red",
                    ));
                }
            }
        }

        document = document.add(commit_dot(
            idx,
            branch.column.unwrap(),
            "red",
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

fn path(
    index1: usize,
    column1: usize,
    index2: usize,
    column2: usize,
    is_merge: bool,
    color: &str,
) -> Path {
    let c0 = commit_coord_u(index1, column1);
    let c1 = if is_merge {
        commit_coord_u(index1, column1)
    } else {
        commit_coord_i(index2 as i32 - 1, column1 as i32)
    };
    let c2 = if is_merge {
        commit_coord_u(index1 + 1, column2)
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
