use git2::Error;
use git_graph::graph::{CommitInfo, GitGraph};
use git_graph::print::svg::print_svg;
use git_graph::settings::Settings;

struct Args {}

fn main() -> Result<(), Error> {
    let _args = Args {};
    let settings = Settings::git_flow();
    run(&settings)?;
    Ok(())
}

fn run(settings: &Settings) -> Result<(), Error> {
    let path = ".";
    let graph = GitGraph::new(path, settings)?;

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
    println!("{}", print_svg(&graph, &settings.branches, true).unwrap());
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
