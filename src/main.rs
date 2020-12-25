use clap::{crate_version, App, Arg};
use git2::Error;
use git_graph::graph::{CommitInfo, GitGraph};
use git_graph::print::svg::print_svg;
use git_graph::print::unicode::print_unicode;
use git_graph::settings::{
    BranchOrder, BranchSettings, BranchSettingsDef, Characters, MergePatterns, Settings,
};
use platform_dirs::AppDirs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

fn main() {
    std::process::exit(match from_args() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    });
}

fn from_args() -> Result<(), String> {
    create_config()?;

    let app = App::new("git-graph")
        .version(crate_version!())
        .about(
            "Structured Git graphs for your branching model.\n  \
                  https://github.com/mlange-42/git-graph",
        )
        .arg(
            Arg::with_name("all")
                .long("all")
                .short("a")
                .help("Show the entire graph, not only from HEAD.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("max-count")
                .long("max-count")
                .short("n")
                .help("Maximum number of commits")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("svg")
                .long("svg")
                .help("Render graph as SVG instead of text-based.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .short("d")
                .help("Additional debug output and graphics.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("compact")
                .long("compact")
                .short("c")
                .help("Print compact graph: merges point to merge commits rather than connecting lines.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("no-color")
                .long("no-color").alias("mono")
                .short("m")
                .help("Print without colors.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("style")
                .long("style")
                .short("s")
                .help("Output style. One of [normal|thin|round|bold|double|ascii]")
                .required(false)
                .takes_value(true),
        );

    let matches = app.clone().get_matches();

    let all = matches.is_present("all");
    let commit_limit = match matches.value_of("max-count") {
        None => None,
        Some(str) => match str.parse::<usize>() {
            Ok(val) => Some(val),
            Err(_) => {
                return Err(format![
                    "Option max-count must be a positive number, but got '{}'",
                    str
                ])
            }
        },
    };
    let svg = matches.is_present("svg");
    let colored = !matches.is_present("no-color");
    let compact = matches.is_present("compact");
    let debug = matches.is_present("debug");
    let style = matches
        .value_of("style")
        .map(|s| Characters::from_str(s))
        .unwrap_or_else(|| Ok(Characters::thin()))?;

    let settings = Settings {
        debug,
        colored,
        compact,
        include_remote: true,
        characters: style,
        branch_order: BranchOrder::ShortestFirst(true),
        branches: BranchSettings::from(BranchSettingsDef::git_flow())
            .map_err(|err| err.to_string())?,
        merge_patterns: MergePatterns::default(),
    };

    run(&settings, svg, all, commit_limit)
}

fn create_config() -> Result<(), String> {
    let app_dir = AppDirs::new(Some("git-graph"), false).unwrap().config_dir;
    let mut models_dir = app_dir;
    models_dir.push("models");

    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir).map_err(|err| err.to_string())?;

        let models = [
            (BranchSettingsDef::git_flow(), "git-flow.toml"),
            (BranchSettingsDef::simple(), "simple.toml"),
            (BranchSettingsDef::none(), "none.toml"),
        ];
        for (model, file) in &models {
            let mut path = PathBuf::from(&models_dir);
            path.push(file);
            let str = toml::to_string_pretty(&model).map_err(|err| err.to_string())?;
            std::fs::write(&path, str).map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

fn run(
    settings: &Settings,
    svg: bool,
    all: bool,
    max_commits: Option<usize>,
) -> Result<(), String> {
    let path = ".";

    let now = Instant::now();
    let graph = GitGraph::new(path, settings, all, max_commits)?;

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
        print_svg(&graph, &settings)?
    } else {
        print_unicode(&graph, &settings)?
    };

    let duration_print = now.elapsed().as_micros();

    if settings.debug {
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
