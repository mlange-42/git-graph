use git2::Error;
use git_graph::graph::{CommitInfo, GitGraph};
use git_graph::print::unicode::print_unicode;
use git_graph::settings::{BranchOrder, BranchSettings, MergePatterns, Settings};
use std::time::Instant;

struct Args {}

fn main() -> Result<(), Error> {
    let _args = Args {};

    let settings = Settings {
        debug: true,
        include_remote: true,
        branch_order: BranchOrder::ShortestFirst(true),
        branches: BranchSettings::git_flow(),
        merge_patterns: MergePatterns::default(),
    };

    run(&settings)?;
    Ok(())
}

fn run(settings: &Settings) -> Result<(), Error> {
    let path = ".";

    let now = Instant::now();
    let graph = GitGraph::new(path, settings)?;
    let duration_graph = now.elapsed().as_micros();

    if settings.debug {
        for branch in &graph.branches {
            eprintln!(
                "{} (col {}) ({:?}) {}",
                branch.name,
                branch.visual.column.unwrap_or(99),
                branch.range,
                if branch.is_merged { "m" } else { "" }
            );
        }
        eprintln!("---------------------------------------------");
        for info in &graph.commits {
            if info.branch_trace.is_some() {
                print_commit_short(&graph, &info)?;
            }
        }
    }

    let now = Instant::now();

    //print_svg(&graph, &settings.branches, settings.debug).unwrap()
    println!(
        "{}",
        print_unicode(&graph, &settings.branches, settings.debug)
    );

    let duration_print = now.elapsed().as_micros();

    eprintln!(
        "Graph construction: {:.1} ms, printing: {:.1} ms ({} commits)",
        duration_graph as f32 / 1000.0,
        duration_print as f32 / 1000.0,
        graph.commits.len()
    );

    Ok(())
}

fn print_commit_short(graph: &GitGraph, info: &CommitInfo) -> Result<(), Error> {
    let commit = &graph.commit(info.oid)?;
    let symbol = if commit.parents().len() > 1 { "o" } else { "*" };

    let branch_str = if info.branches.is_empty() {
        "".to_string()
    } else {
        format!(
            " ({})",
            itertools::join(
                info.branches
                    .iter()
                    .map(|idx| { graph.branches[*idx].name.to_string() }),
                ", "
            )
        )
    };
    let (trace_str, indent) = if let Some(trace) = info.branch_trace {
        let branch = &graph.branches[trace];
        let name = &branch.name;
        (
            format!(
                " [{}{}-{}]",
                &name[0..1],
                &name[(name.len() - 1)..name.len()],
                trace,
            ),
            std::iter::repeat(" ")
                .take(branch.visual.column.unwrap())
                .collect::<String>(),
        )
    } else {
        ("".to_string(), "".to_string())
    };

    eprintln!(
        "{}{} {}{}{} {}",
        indent,
        symbol,
        &commit.id().to_string()[0..7],
        trace_str,
        branch_str,
        &commit.summary().unwrap_or("---")
    );

    Ok(())
}
