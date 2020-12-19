use git2::Error;
use git_graph::graph::{CommitInfo, GitGraph};
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
        println!("{}", branch.name);
    }
    println!("---------------------------------------------");
    for (idx, info) in graph.commits.iter().enumerate() {
        print_commit_short(&graph, &info, idx)?;
    }
    Ok(())
}

fn print_commit_short(graph: &GitGraph, info: &CommitInfo, index: usize) -> Result<(), Error> {
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
                " {}{}-{}/{}-{}]",
                &name[0..1],
                &name[(name.len() - 1)..name.len()],
                trace,
                branch.range.0.unwrap_or(0),
                branch.range.1.unwrap_or(0),
            ),
            std::iter::repeat(" ")
                .take(branch.order_group)
                .collect::<String>(),
        )
    } else {
        ("".to_string(), "".to_string())
    };

    println!(
        "{} {}{} {}{}{} {}",
        index,
        indent,
        symbol,
        &commit.id().to_string()[0..7],
        trace_str,
        branch_str,
        &commit.summary().unwrap_or("---")
    );

    Ok(())
}
