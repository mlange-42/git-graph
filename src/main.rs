use clap::{crate_version, App, Arg, SubCommand};
use git2::{Error, Repository};
use git_graph::graph::{CommitInfo, GitGraph};
use git_graph::print::svg::print_svg;
use git_graph::print::unicode::print_unicode;
use git_graph::settings::{
    BranchOrder, BranchSettings, BranchSettingsDef, Characters, MergePatterns, RepoSettings,
    Settings,
};
use platform_dirs::AppDirs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

fn main() {
    std::process::exit(match from_args() {
        Ok(_) => 0,
        Err(err) => {
            eprint!("{}", err);
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
            Arg::with_name("max-count")
                .long("max-count")
                .short("n")
                .help("Maximum number of commits")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("model")
                .long("model")
                .short("m")
                .help("Branching model. Available presets are [simple|git-flow|none]. Default: git-flow. Permanently set the model for a repository with `git-graph model <model>`.")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("local")
                .long("local")
                .short("l")
                .help("Show only local branches, no remotes.")
                .required(false)
                .takes_value(false),
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
            Arg::with_name("sparse")
                .long("sparse")
                .short("S")
                .help("Print a less compact graph: merge lines point to target lines rather than merge commits.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("no-color")
                .long("no-color").alias("mono")
                .short("M")
                .help("Print without colors. Missing color support should be detected automatically (e.g. when piping to a file).")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("style")
                .long("style")
                .short("s")
                .help("Output style. One of [normal|thin|round|bold|double|ascii].")
                .required(false)
                .takes_value(true),
        ).subcommand(SubCommand::with_name("model")
            .about("Prints or permanently sets the branching model for a repository.")
            .arg(
                Arg::with_name("model")
                    .help("The branching model to be used. Available presets are [simple|git-flow|none]. When not given, prints the currently set model.")
                    .value_name("model")
                    .takes_value(true)
                    .required(false)
                    .index(1))
            .arg(
                Arg::with_name("list")
                    .long("list")
                    .short("l")
                    .help("List all available branching models.")
                    .required(false)
                    .takes_value(false),
        ));

    let matches = app.clone().get_matches();

    if let Some(matches) = matches.subcommand_matches("model") {
        if matches.is_present("list") {
            print!("{}", itertools::join(get_available_models()?, "\n"));
            return Ok(());
        }
        match matches.value_of("model") {
            None => {
                let curr_model = get_model_name()?;
                match curr_model {
                    None => print!("No branching model set"),
                    Some(model) => print!("{}", model),
                }
            }
            Some(model) => set_model(model)?,
        };
        return Ok(());
    }

    let model = get_model(matches.value_of("model"))?;

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

    let include_remote = !matches.is_present("local");

    let svg = matches.is_present("svg");
    let colored = !matches.is_present("no-color");
    let compact = !matches.is_present("sparse");
    let debug = matches.is_present("debug");
    let style = matches
        .value_of("style")
        .map(|s| Characters::from_str(s))
        .unwrap_or_else(|| Ok(Characters::thin()))?;

    let settings = Settings {
        debug,
        colored,
        compact,
        include_remote,
        characters: style,
        branch_order: BranchOrder::ShortestFirst(true),
        branches: BranchSettings::from(model).map_err(|err| err.to_string())?,
        merge_patterns: MergePatterns::default(),
    };

    run(&settings, svg, commit_limit)
}

fn get_model_name() -> Result<Option<String>, String> {
    let path = ".";
    let repository = Repository::open(path).map_err(|err| err.to_string())?;

    let mut config_path = PathBuf::from(repository.path());
    config_path.push("git-graph.toml");

    if config_path.exists() {
        let repo_config: RepoSettings =
            toml::from_str(&std::fs::read_to_string(config_path).map_err(|err| err.to_string())?)
                .map_err(|err| err.to_string())?;

        Ok(Some(repo_config.model))
    } else {
        Ok(None)
    }
}

fn get_available_models() -> Result<Vec<String>, String> {
    let app_dir = AppDirs::new(Some("git-graph"), false).unwrap().config_dir;
    let mut models_dir = app_dir;
    models_dir.push("models");

    let models = std::fs::read_dir(&models_dir)
        .map_err(|err| err.to_string())?
        .filter_map(|e| match e {
            Ok(e) => {
                if let (Some(name), Some(ext)) = (e.path().file_name(), e.path().extension()) {
                    if ext == "toml" {
                        if let Some(name) = name.to_str() {
                            Some((&name[..(name.len() - 5)]).to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        })
        .collect::<Vec<_>>();

    Ok(models)
}

fn get_model(model: Option<&str>) -> Result<BranchSettingsDef, String> {
    match model {
        Some(model) => read_model(model),
        None => {
            let path = ".";
            let repository = Repository::open(path).map_err(|err| err.to_string())?;

            let mut config_path = PathBuf::from(repository.path());
            config_path.push("git-graph.toml");

            if config_path.exists() {
                let repo_config: RepoSettings = toml::from_str(
                    &std::fs::read_to_string(config_path).map_err(|err| err.to_string())?,
                )
                .map_err(|err| err.to_string())?;

                read_model(&repo_config.model)
            } else {
                Ok(read_model("git-flow").unwrap_or_else(|_| BranchSettingsDef::git_flow()))
            }
        }
    }
}

fn read_model(model: &str) -> Result<BranchSettingsDef, String> {
    let app_dir = AppDirs::new(Some("git-graph"), false).unwrap().config_dir;
    let mut models_dir = app_dir;
    models_dir.push("models");

    let mut model_file = PathBuf::from(&models_dir);
    model_file.push(format!("{}.toml", model));

    if model_file.exists() {
        toml::from_str::<BranchSettingsDef>(
            &std::fs::read_to_string(model_file).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())
    } else {
        let models = get_available_models()?;
        Err(format!(
            "ERROR: No branching model named '{}' found in {}\n       Available models are: {}",
            model,
            models_dir.display(),
            itertools::join(models, ", ")
        ))
    }
}

fn set_model(model: &str) -> Result<(), String> {
    let models = get_available_models()?;

    if !models.contains(&model.to_string()) {
        let app_dir = AppDirs::new(Some("git-graph"), false).unwrap().config_dir;
        let mut models_dir = app_dir;
        models_dir.push("models");
        return Err(format!(
            "ERROR: No branching model named '{}' found in {}\n       Available models are: {}",
            model,
            models_dir.display(),
            itertools::join(models, ", ")
        ));
    }

    let path = ".";
    let repository = Repository::open(path).map_err(|err| err.to_string())?;

    let mut config_path = PathBuf::from(repository.path());
    config_path.push("git-graph.toml");

    let config = RepoSettings {
        model: model.to_string(),
    };

    let str = toml::to_string_pretty(&config).map_err(|err| err.to_string())?;
    std::fs::write(&config_path, str).map_err(|err| err.to_string())?;

    eprint!("Branching model set to '{}'", model);

    Ok(())
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

fn run(settings: &Settings, svg: bool, max_commits: Option<usize>) -> Result<(), String> {
    let path = ".";

    let now = Instant::now();
    let graph = GitGraph::new(path, settings, max_commits)?;

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
        print!("{}", print_svg(&graph, &settings)?);
    } else {
        print!("{}", print_unicode(&graph, &settings)?);
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
