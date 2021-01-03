use clap::{crate_version, App, Arg, SubCommand};
use crossterm::cursor::MoveToColumn;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{ErrorKind, ExecutableCommand};
use git2::Repository;
use git_graph::graph::GitGraph;
use git_graph::print::format::CommitFormat;
use git_graph::print::svg::print_svg;
use git_graph::print::unicode::print_unicode;
use git_graph::settings::{
    BranchOrder, BranchSettings, BranchSettingsDef, Characters, MergePatterns, RepoSettings,
    Settings,
};
use platform_dirs::AppDirs;
use std::ffi::OsStr;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;

const REPO_CONFIG_FILE: &str = "git-graph.toml";

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
    let app_dir = AppDirs::new(Some("git-graph"), false).unwrap().config_dir;
    let mut models_dir = app_dir;
    models_dir.push("models");

    create_config(&models_dir)?;

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
                .takes_value(true)
                .value_name("n"),
        )
        .arg(
            Arg::with_name("model")
                .long("model")
                .short("m")
                .help("Branching model. Available presets are [simple|git-flow|none].\n\
                       Default: git-flow. \n\
                       Permanently set the model for a repository with\n\
                         > git-graph model <model>")
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
                .help("Print a less compact graph: merge lines point to target lines\n\
                       rather than merge commits.")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("color")
                .long("color")
                .help("Specify when colors should be used. One of [auto|always|never].\n\
                       Default: auto.")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("no-color")
                .long("no-color")
                .help("Print without colors. Missing color support should be detected\n\
                       automatically (e.g. when piping to a file).\n\
                       Overrides option '--color'")
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
                .help("Output style. One of [normal/thin|round|bold|double|ascii].\n  \
                         (First character can be used as abbreviation, e.g. '-s r')")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("wrap")
                .long("wrap")
                .short("w")
                .help("Line wrapping for formatted commit text. Default: 'auto 0 8'\n\
                       Argument format: [<width>|auto|none[ <indent1>[ <indent2>]]]\n\
                       For examples, consult 'git-graph --help'")
                .long_help("Line wrapping for formatted commit text. Default: 'auto 0 8'\n\
                       Argument format: [<width>|auto|none[ <indent1>[ <indent2>]]]\n\
                       Examples:\n    \
                           git-graph --wrap auto\n    \
                           git-graph --wrap auto 0 8\n    \
                           git-graph --wrap none\n    \
                           git-graph --wrap 80\n    \
                           git-graph --wrap 80 0 8\n\
                       'auto' uses the terminal's width if on a terminal.")
                .required(false)
                .min_values(0)
                .max_values(3),
        )
        .arg(
            Arg::with_name("format")
                .long("format")
                .short("f")
                .help("Commit format. One of [oneline|short|medium|full|\"<string>\"].\n  \
                         (First character can be used as abbreviation, e.g. '-f m')\n\
                       Default: oneline.\n\
                       For placeholders supported in \"<string>\", consult 'git-graph --help'")
                .long_help("Commit format. One of [oneline|short|medium|full|\"<string>\"].\n  \
                              (First character can be used as abbreviation, e.g. '-f m')\n\
                            Formatting placeholders for \"<string>\":\n    \
                                %n    newline\n    \
                                %H    commit hash\n    \
                                %h    abbreviated commit hash\n    \
                                %P    parent commit hashes\n    \
                                %p    abbreviated parent commit hashes\n    \
                                %d    refs (branches, tags)\n    \
                                %s    commit summary\n    \
                                %b    commit message body\n    \
                                %B    raw body (subject and body)\n    \
                                %an   author name\n    \
                                %ae   author email\n    \
                                %ad   author date\n    \
                                %as   author date in short format 'YYYY-MM-DD'\n    \
                                %cn   committer name\n    \
                                %ce   committer email\n    \
                                %cd   committer date\n    \
                                %cs   committer date in short format 'YYYY-MM-DD'\n    \
                                \n    \
                                If you add a + (plus sign) after % of a placeholder,\n       \
                                   a line-feed is inserted immediately before the expansion if\n       \
                                   and only if the placeholder expands to a non-empty string.\n    \
                                If you add a - (minus sign) after % of a placeholder, all\n       \
                                   consecutive line-feeds immediately preceding the expansion are\n       \
                                   deleted if and only if the placeholder expands to an empty string.\n    \
                                If you add a ' ' (space) after % of a placeholder, a space is\n       \
                                   inserted immediately before the expansion if and only if\n       \
                                   the placeholder expands to a non-empty string.\n\
                            \n    \
                                See also the respective git help: https://git-scm.com/docs/pretty-formats\n")
                .required(false)
                .takes_value(true),
        )
        .subcommand(SubCommand::with_name("model")
            .about("Prints or permanently sets the branching model for a repository.")
            .arg(
                Arg::with_name("model")
                    .help("The branching model to be used. Available presets are [simple|git-flow|none].\n\
                           When not given, prints the currently set model.")
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
            println!(
                "{}",
                itertools::join(get_available_models(&models_dir)?, "\n")
            );
            return Ok(());
        }
    }

    let path = matches.value_of("path").unwrap_or(".");
    let repository = Repository::discover(path)
        .map_err(|err| format!("ERROR: {}\n       Navigate into a repository before running git-graph, or use option --path", err.message()))?;

    if let Some(matches) = matches.subcommand_matches("model") {
        match matches.value_of("model") {
            None => {
                let curr_model = get_model_name(&repository, REPO_CONFIG_FILE)?;
                match curr_model {
                    None => print!("No branching model set"),
                    Some(model) => print!("{}", model),
                }
            }
            Some(model) => set_model(&repository, model, REPO_CONFIG_FILE, &models_dir)?,
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
    let pager = !matches.is_present("no-pager");
    let compact = !matches.is_present("sparse");
    let debug = matches.is_present("debug");
    let style = matches
        .value_of("style")
        .map(|s| Characters::from_str(s))
        .unwrap_or_else(|| Ok(Characters::thin()))?;

    let model = get_model(
        &repository,
        matches.value_of("model"),
        REPO_CONFIG_FILE,
        &models_dir,
    )?;

    let format = match matches.value_of("format") {
        None => CommitFormat::OneLine,
        Some(str) => CommitFormat::from_str(str)?,
    };

    let colored = if matches.is_present("no-color") {
        false
    } else if let Some(mode) = matches.value_of("color") {
        match mode {
            "auto" => {
                atty::is(atty::Stream::Stdout)
                    && (!cfg!(windows) || yansi::Paint::enable_windows_ascii())
            }
            "always" => {
                if cfg!(windows) {
                    yansi::Paint::enable_windows_ascii();
                }
                true
            }
            "never" => false,
            other => {
                return Err(format!(
                    "Unknown color mode '{}'. Supports [auto|always|never].",
                    other
                ))
            }
        }
    } else {
        atty::is(atty::Stream::Stdout) && (!cfg!(windows) || yansi::Paint::enable_windows_ascii())
    };

    let wrapping = if let Some(wrap_values) = matches.values_of("wrap") {
        let strings = wrap_values.collect::<Vec<_>>();
        if strings.is_empty() {
            Some((None, Some(0), Some(8)))
        } else {
            match strings[0] {
                "none" => None,
                "auto" => {
                    let wrap = strings
                        .iter()
                        .skip(1)
                        .map(|str| str.parse::<usize>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| {
                            format!(
                                "ERROR: Can't parse option --wrap '{}' to integers.",
                                strings.join(" ")
                            )
                        })?;
                    Some((None, wrap.get(0).cloned(), wrap.get(1).cloned()))
                }
                _ => {
                    let wrap = strings
                        .iter()
                        .map(|str| str.parse::<usize>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| {
                            format!(
                                "ERROR: Can't parse option --wrap '{}' to integers.",
                                strings.join(" ")
                            )
                        })?;
                    Some((
                        wrap.get(0).cloned(),
                        wrap.get(1).cloned(),
                        wrap.get(2).cloned(),
                    ))
                }
            }
        }
    } else {
        Some((None, Some(0), Some(8)))
    };

    let settings = Settings {
        debug,
        colored,
        compact,
        include_remote,
        format,
        wrapping,
        characters: style,
        branch_order: BranchOrder::ShortestFirst(true),
        branches: BranchSettings::from(model).map_err(|err| err.to_string())?,
        merge_patterns: MergePatterns::default(),
    };

    run(repository, &settings, svg, commit_limit, pager)
}

/// Get the currently set branching model for a repo.
fn get_model_name(repository: &Repository, file_name: &str) -> Result<Option<String>, String> {
    let mut config_path = PathBuf::from(repository.path());
    config_path.push(file_name);

    if config_path.exists() {
        let repo_config: RepoSettings =
            toml::from_str(&std::fs::read_to_string(config_path).map_err(|err| err.to_string())?)
                .map_err(|err| err.to_string())?;

        Ok(Some(repo_config.model))
    } else {
        Ok(None)
    }
}

/// Get models available in `APP_DATA/git-graph/models`.
fn get_available_models<P: AsRef<Path>>(app_model_path: &P) -> Result<Vec<String>, String> {
    let models = std::fs::read_dir(app_model_path)
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

/// Try to get the branch settings for a given model.
/// If no model name is given, returns the branch settings set fot the repo, or the default otherwise.
fn get_model<P: AsRef<Path> + AsRef<OsStr>>(
    repository: &Repository,
    model: Option<&str>,
    repo_config_file: &str,
    app_model_path: &P,
) -> Result<BranchSettingsDef, String> {
    match model {
        Some(model) => read_model(model, app_model_path),
        None => {
            let mut config_path = PathBuf::from(repository.path());
            config_path.push(repo_config_file);

            if config_path.exists() {
                let repo_config: RepoSettings = toml::from_str(
                    &std::fs::read_to_string(config_path).map_err(|err| err.to_string())?,
                )
                .map_err(|err| err.to_string())?;

                read_model(&repo_config.model, app_model_path)
            } else {
                Ok(read_model("git-flow", app_model_path)
                    .unwrap_or_else(|_| BranchSettingsDef::git_flow()))
            }
        }
    }
}

/// Read a branching model file.
fn read_model<P: AsRef<Path> + AsRef<OsStr>>(
    model: &str,
    app_model_path: &P,
) -> Result<BranchSettingsDef, String> {
    let mut model_file = PathBuf::from(&app_model_path);
    model_file.push(format!("{}.toml", model));

    if model_file.exists() {
        toml::from_str::<BranchSettingsDef>(
            &std::fs::read_to_string(model_file).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())
    } else {
        let models = get_available_models(&app_model_path)?;
        let path: &Path = app_model_path.as_ref();
        Err(format!(
            "ERROR: No branching model named '{}' found in {}\n       Available models are: {}",
            model,
            path.display(),
            itertools::join(models, ", ")
        ))
    }
}
/// Permanently sets the branching model for a repository
fn set_model<P: AsRef<Path>>(
    repository: &Repository,
    model: &str,
    repo_config_file: &str,
    app_model_path: &P,
) -> Result<(), String> {
    let models = get_available_models(&app_model_path)?;

    if !models.contains(&model.to_string()) {
        return Err(format!(
            "ERROR: No branching model named '{}' found in {}\n       Available models are: {}",
            model,
            app_model_path.as_ref().display(),
            itertools::join(models, ", ")
        ));
    }

    let mut config_path = PathBuf::from(repository.path());
    config_path.push(repo_config_file);

    let config = RepoSettings {
        model: model.to_string(),
    };

    let str = toml::to_string_pretty(&config).map_err(|err| err.to_string())?;
    std::fs::write(&config_path, str).map_err(|err| err.to_string())?;

    eprint!("Branching model set to '{}'", model);

    Ok(())
}

/// Creates the directory `APP_DATA/git-graph/models` if it does not exist,
/// and writes the files for built-in branching models there.
fn create_config<P: AsRef<Path> + AsRef<OsStr>>(app_model_path: &P) -> Result<(), String> {
    let path: &Path = app_model_path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(app_model_path).map_err(|err| err.to_string())?;

        let models = [
            (BranchSettingsDef::git_flow(), "git-flow.toml"),
            (BranchSettingsDef::simple(), "simple.toml"),
            (BranchSettingsDef::none(), "none.toml"),
        ];
        for (model, file) in &models {
            let mut path = PathBuf::from(&app_model_path);
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
                "{} (col {}) ({:?}) {} s: {:?}, t: {:?}",
                branch.name,
                branch.visual.column.unwrap_or(99),
                branch.range,
                if branch.is_merged { "m" } else { "" },
                branch.visual.source_order_group,
                branch.visual.target_order_group
            );
        }
    }

    let now = Instant::now();

    if svg {
        println!("{}", print_svg(&graph, &settings)?);
    } else {
        let (lines, _indices) = print_unicode(&graph, &settings)?;
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

/// Print the graph, paged (i.e. wait for user input once the terminal is filled).
fn print_paged(lines: &[String]) -> Result<(), ErrorKind> {
    let (width, height) = crossterm::terminal::size()?;
    let width = width as usize;

    let mut line_idx = 0;
    let mut print_lines = height - 2;
    let mut clear = false;
    let mut abort = false;

    let help = " >>> Down: line, PgDown/Enter: page, End: all, Esc/Q/^C: quit";
    let help = if help.len() > width {
        &help[0..width]
    } else {
        help
    };

    while line_idx < lines.len() {
        if print_lines > 0 {
            if clear {
                stdout()
                    .execute(Clear(ClearType::CurrentLine))?
                    .execute(MoveToColumn(0))?;
            }

            stdout().execute(Print(format!("{}\n", lines[line_idx])))?;

            if print_lines == 1 && line_idx < lines.len() - 1 {
                stdout().execute(Print(help))?;
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

/// Print the graph, un-paged.
fn print_unpaged(lines: &[String]) {
    for line in lines {
        println!("{}", line);
    }
}
