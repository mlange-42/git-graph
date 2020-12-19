use git2::Error;
use git_graph::graph::{CommitInfo, GitGraph};
use git_graph::settings::Settings;

struct Args {}

fn main() -> Result<(), Error> {
    let _args = Args {};
    let settings = Settings::default();
    run(&settings)?;
    Ok(())
}

fn run(settings: &Settings) -> Result<(), Error> {
    let path = ".";
    let graph = GitGraph::new(path, settings)?;
    for info in &graph.commits {
        print_commit_short(&graph, &info)?;
    }
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
                info.branches.iter().map(|idx| &graph.branches[*idx].name),
                ", "
            )
        )
    };
    let trace_str = if let Some(trace) = info.branch_trace {
        let name = &graph.branches[trace].name;
        format!(" [{}{}]", &name[0..1], &name[(name.len() - 1)..name.len()])
    } else {
        "".to_string()
    };

    println!(
        "{} {}{}{} {}",
        symbol,
        &commit.id().to_string()[0..7],
        trace_str,
        branch_str,
        &commit.summary().unwrap_or("---")
    );

    Ok(())
}
