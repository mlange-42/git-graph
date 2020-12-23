use clap::{crate_version, App, Arg};
use git2::Error;
use git_graph::graph::{CommitInfo, GitGraph};
use git_graph::print::svg::print_svg;
use git_graph::print::unicode::print_unicode;
use git_graph::settings::{BranchOrder, BranchSettings, MergePatterns, Settings};
use std::time::Instant;

fn main() {
    let app = App::new("git-graph")
        .version(crate_version!())
        .about(
            "Structured Git graphs for your branching model\n  \
                  https://github.com/mlange-42/git-graph",
        )
        .arg(
            Arg::with_name("svg")
                .long("svg")
                .help("Render graph as SVG instead of text-based")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .short("d")
                .help("Additional debug output and graphics")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("no-color")
                .long("no-color")
                .short("n")
                .help("Print without colors")
                .required(false)
                .takes_value(false),
        );

    let matches = app.clone().get_matches();
    let svg = matches.is_present("svg");
    let color = !matches.is_present("no-color");
    let debug = matches.is_present("debug");

    let settings = Settings {
        debug,
        include_remote: true,
        branch_order: BranchOrder::ShortestFirst(true),
        branches: BranchSettings::git_flow(),
        merge_patterns: MergePatterns::default(),
    };

    std::process::exit(match run(&settings, svg, color, debug) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    });
}

fn run(settings: &Settings, svg: bool, color: bool, debug: bool) -> Result<(), String> {
    let path = ".";

    let now = Instant::now();
    let graph = match GitGraph::new(path, settings) {
        Ok(graph) => graph,
        Err(err) => return Err(err.to_string()),
    };
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
                match print_commit_short(&graph, &info) {
                    Ok(_) => {}
                    Err(err) => return Err(err.to_string()),
                }
            }
        }
    }

    let now = Instant::now();

    if svg {
        print_svg(&graph, &settings.branches, settings.debug)?
    } else {
        print_unicode(&graph, &settings.branches, color, settings.debug)?
    };

    let duration_print = now.elapsed().as_micros();

    if debug {
        eprintln!(
            "Graph construction: {:.1} ms, printing: {:.1} ms ({} commits)",
            duration_graph as f32 / 1000.0,
            duration_print as f32 / 1000.0,
            graph.commits.len()
        );
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
