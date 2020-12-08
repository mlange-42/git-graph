use git2::{Commit, Error};
use git_graph::graph::GitGraph;

struct Args {}

fn main() -> Result<(), Error> {
    let args = Args {};
    run(&args)?;
    Ok(())
}

fn run(_args: &Args) -> Result<(), Error> {
    let path = ".";
    let graph = GitGraph::new(path)?;
    for info in &graph.commits {
        let commit = &graph.commit(info.oid)?;
        print_commit_short(commit, &info.branches);
    }
    Ok(())
}

fn print_commit_short(commit: &Commit, branches: &[String]) {
    let symbol = if commit.parents().len() > 1 {
        "\u{25CB}"
    } else {
        "\u{25CF}"
    };
    let branch_str = if branches.is_empty() {
        "".to_string()
    } else {
        format!(" ({})", branches.join(", "))
    };

    println!(
        "{} {}{} {}",
        symbol,
        &commit.id().to_string()[0..7],
        branch_str,
        &commit.summary().unwrap_or("---")
    );
}
