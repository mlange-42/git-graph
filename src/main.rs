use git2::{Commit, Error, Time};
use git_graph::graph::GitGraph;

struct Args {}

fn main() {
    let args = Args {};
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}

fn run(_args: &Args) -> Result<(), Error> {
    let path = ".";
    let graph = GitGraph::new(path)?;
    for info in &graph.commits {
        print_commit_short(&graph.commit(info.oid)?);
    }
    Ok(())
}

#[allow(dead_code)]
fn print_commit_short(commit: &Commit) {
    let symbol = if commit.parents().len() > 1 {
        "\u{25CB}"
    } else {
        "\u{25CF}"
    };
    let str = format!(
        "{} {} {}",
        symbol,
        &commit.id().to_string()[0..7],
        &commit.summary().unwrap_or("---")
    );
    println!("{}", str);
}

#[allow(dead_code)]
fn print_commit(commit: &Commit) {
    println!("commit {}", commit.id());

    if commit.parents().len() > 1 {
        print!("Merge:");
        for id in commit.parent_ids() {
            print!(" {:.8}", id);
        }
        println!();
    }

    let author = commit.author();
    println!("Author: {}", author);
    print_time(&author.when(), "Date:   ");
    println!();

    for line in String::from_utf8_lossy(commit.message_bytes()).lines() {
        println!("    {}", line);
    }
    println!();
}

#[allow(dead_code)]
fn print_time(time: &Time, prefix: &str) {
    let (offset, sign) = match time.offset_minutes() {
        n if n < 0 => (-n, '-'),
        n => (n, '+'),
    };
    let (hours, minutes) = (offset / 60, offset % 60);
    let ts = time::Timespec::new(time.seconds() + (time.offset_minutes() as i64) * 60, 0);
    let time = time::at(ts);

    println!(
        "{}{} {}{:02}{:02}",
        prefix,
        time.strftime("%a %b %e %T %Y").unwrap(),
        sign,
        hours,
        minutes
    );
}
