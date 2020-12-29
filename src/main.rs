use clap::{crate_version, App, Arg, SubCommand};
use crossterm::cursor::MoveToColumn;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{ErrorKind, ExecutableCommand};
use git2::{Error, Repository};
use git_graph::graph::{CommitInfo, GitGraph};
use git_graph::print::svg::print_svg;
use git_graph::print::unicode::print_unicode;
use git_graph::settings::{
    BranchOrder, BranchSettings, BranchSettingsDef, Characters, MergePatterns, RepoSettings,
    Settings,
};
use platform_dirs::AppDirs;
use std::io::stdout;
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
            "Structured Git graphs for your branching model.\n    \
                 https://github.com/mlange-42/git-graph\n\
             \n\
             EXAMPES:\n    \
                 git-graph                   -> Show graph\n    \
                 git-graph --style round     -> Show graph in a different style\n    \
                 git-graph --model <model>   -> Show graph using a certain <model>\n    \
                 git-graph model --list      -> List available branching models\n    \
                 git-graph model             -> Show repo's current branching models\n    \
                 git-graph model <model>     -> Permanently set model <model> for this repo",
        )
        .arg(
            Arg::with_name("path")
                .long("path")
                .short("p")
                .help("Open repository from this path or above. Default '.'")
                .required(false)
                .takes_value(true),
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
                .long("no-color")
                .help("Print without colors. Missing color support should be detected automatically (e.g. when piping to a file).")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("no-pager")
                .long("no-pager")
                .help("Use no pager (print everything at once without prompt).")
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
            println!("{}", itertools::join(get_available_models()?, "\n"));
            return Ok(());
        }
    }

    let path = matches.value_of("path").unwrap_or(".");
    let repository = Repository::discover(path)
        .map_err(|err| format!("ERROR: {}\n       Navigate into a repository before running git-graph, or use option --path", err.message()))?;

    if let Some(matches) = matches.subcommand_matches("model") {
        match matches.value_of("model") {
            None => {
                let curr_model = get_model_name(&repository)?;
                match curr_model {
                    None => print!("No branching model set"),
                    Some(model) => print!("{}", model),
                }
            }
            Some(model) => set_model(&repository, model)?,
        };
        return Ok(());
    }

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
    let pager = !matches.is_present("no-pager");
    let compact = !matches.is_present("sparse");
    let debug = matches.is_present("debug");
    let style = matches
        .value_of("style")
        .map(|s| Characters::from_str(s))
        .unwrap_or_else(|| Ok(Characters::thin()))?;

    let model = get_model(&repository, matches.value_of("model"))?;

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

    run(repository, &settings, svg, commit_limit, pager)
}

fn get_model_name(repository: &Repository) -> Result<Option<String>, String> {
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

fn get_model(repository: &Repository, model: Option<&str>) -> Result<BranchSettingsDef, String> {
    match model {
        Some(model) => read_model(model),
        None => {
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

fn set_model(repository: &Repository, model: &str) -> Result<(), String> {
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

fn run(
    repository: Repository,
    settings: &Settings,
    svg: bool,
    max_commits: Option<usize>,
    pager: bool,
) -> Result<(), String> {
    let now = Instant::now();
    let graph = GitGraph::new(repository, settings, max_commits)?;

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
                    Err(err) => return Err(err.message().to_string()),
                }
            }
        }
    }

    let now = Instant::now();

    if svg {
        println!("{}", print_svg(&graph, &settings)?);
    } else {
        let lines = print_unicode(&graph, &settings)?;
        if pager && atty::is(atty::Stream::Stdout) {
            print_paged(&lines).map_err(|err| err.to_string())?;
        } else {
            print_unpaged(&lines);
        }
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

fn print_paged(lines: &[String]) -> Result<(), ErrorKind> {
    let height = crossterm::terminal::size()?.1;

    let mut line_idx = 0;
    let mut print_lines = height - 2;
    let mut clear = false;
    let mut abort = false;

    while line_idx < lines.len() {
        if print_lines > 0 {
            if clear {
                stdout()
                    .execute(Clear(ClearType::CurrentLine))?
                    .execute(MoveToColumn(0))?;
            }

            stdout().execute(Print(format!("{}\n", lines[line_idx])))?;

            if print_lines == 1 && line_idx < lines.len() - 1 {
                stdout().execute(Print(
                    "Down: line, PgDown/Enter: page, End: all, Esc/Q/^C: quit",
                ))?;
            }
            print_lines -= 1;
            line_idx += 1;
        } else {
            let input = crossterm::event::read()?;
            match input {
                Event::Key(evt) => match evt.code {
                    KeyCode::Down => {
                        clear = true;
                        print_lines = 1;
                    }
                    KeyCode::Enter | KeyCode::PageDown => {
                        clear = true;
                        print_lines = height - 2;
                    }
                    KeyCode::End => {
                        clear = true;
                        print_lines = lines.len() as u16;
                    }
                    KeyCode::Char(c) => match c {
                        'q' => {
                            abort = true;
                            break;
                        }
                        'c' if evt.modifiers == KeyModifiers::CONTROL => {
                            abort = true;
                            break;
                        }
                        _ => {}
                    },
                    KeyCode::Esc => {
                        abort = true;
                        break;
                    }
                    _ => {}
                },
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
            }
        }
    }
    if abort {
        stdout()
            .execute(Clear(ClearType::CurrentLine))?
            .execute(MoveToColumn(0))?
            .execute(Print(" ...\n"))?;
    }

    Ok(())
}

fn print_unpaged(lines: &[String]) {
    for line in lines {
        println!("{}", line);
    }
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
