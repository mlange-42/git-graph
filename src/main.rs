use git2::{Commit, Error};
use git_graph::graph::GitGraph;
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
