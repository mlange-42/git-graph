use clap::{crate_version, Arg, Command};
use crossterm::cursor::MoveToColumn;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{ErrorKind, ExecutableCommand};
use git2::Repository;
use git_graph::config::{
    create_config, get_available_models, get_model, get_model_name, set_model,
};
use git_graph::get_repo;
use git_graph::graph::GitGraph;
use git_graph::print::format::CommitFormat;
use git_graph::print::svg::print_svg;
use git_graph::print::unicode::print_unicode;
use git_graph::settings::{BranchOrder, BranchSettings, Characters, MergePatterns, Settings};
use platform_dirs::AppDirs;
use std::io::stdout;
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

    let app = Command::new("git-graph")
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
            Arg::new("path")
                .long("path")
                .short('p')
                .help("Open repository from this path or above. Default '.'")
                .required(false)
                .num_args(1),
        )
        .arg(
            Arg::new("max-count")
                .long("max-count")
                .short('n')
                .help("Maximum number of commits")
                .required(false)
                .num_args(1)
                .value_name("n"),
        )
        .arg(
            Arg::new("model")
                .long("model")
                .short('m')
                .help("Branching model. Available presets are [simple|git-flow|none].\n\
                       Default: git-flow. \n\
                       Permanently set the model for a repository with\n\
                         > git-graph model <model>")
                .required(false)
                .num_args(1),
        )
        .arg(
            Arg::new("local")
                .long("local")
                .short('l')
                .help("Show only local branches, no remotes.")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("svg")
                .long("svg")
                .help("Render graph as SVG instead of text-based.")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .short('d')
                .help("Additional debug output and graphics.")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("sparse")
                .long("sparse")
                .short('S')
                .help("Print a less compact graph: merge lines point to target lines\n\
                       rather than merge commits.")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("color")
                .long("color")
                .help("Specify when colors should be used. One of [auto|always|never].\n\
                       Default: auto.")
                .required(false)
                .num_args(1),
        )
        .arg(
            Arg::new("no-color")
                .long("no-color")
                .help("Print without colors. Missing color support should be detected\n\
                       automatically (e.g. when piping to a file).\n\
                       Overrides option '--color'")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("no-pager")
                .long("no-pager")
                .help("Use no pager (print everything at once without prompt).")
                .required(false)
                .num_args(0),
        )
        .arg(
            Arg::new("style")
                .long("style")
                .short('s')
                .help("Output style. One of [normal/thin|round|bold|double|ascii].\n  \
                         (First character can be used as abbreviation, e.g. '-s r')")
                .required(false)
                .num_args(1),
        )
        .arg(
            Arg::new("wrap")
                .long("wrap")
                .short('w')
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
                .num_args(0..=3),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
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
                .num_args(1),
        )
        .subcommand(Command::new("model")
            .about("Prints or permanently sets the branching model for a repository.")
            .arg(
                Arg::new("model")
                    .help("The branching model to be used. Available presets are [simple|git-flow|none].\n\
                           When not given, prints the currently set model.")
                    .value_name("model")
                    .num_args(1)
                    .required(false)
                    .index(1))
            .arg(
                Arg::new("list")
                    .long("list")
                    .short('l')
                    .help("List all available branching models.")
                    .required(false)
                    .num_args(0),
        ));

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("model") {
        if matches.get_flag("list") {
            println!(
                "{}",
                itertools::join(get_available_models(&models_dir)?, "\n")
            );
            return Ok(());
        }
    }

    let dot = ".".to_string();
    let path = matches.get_one::<String>("path").unwrap_or(&dot);
    let repository = get_repo(path)
        .map_err(|err| format!("ERROR: {}\n       Navigate into a repository before running git-graph, or use option --path", err.message()))?;

    if let Some(matches) = matches.subcommand_matches("model") {
        match matches.get_one::<String>("model") {
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

    let commit_limit = match matches.get_one::<String>("max-count") {
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

    let include_remote = !matches.get_flag("local");

    let svg = matches.get_flag("svg");
    let pager = !matches.get_flag("no-pager");
    let compact = !matches.get_flag("sparse");
    let debug = matches.get_flag("debug");
    let style = matches
        .get_one::<String>("style")
        .map(|s| Characters::from_str(s))
        .unwrap_or_else(|| Ok(Characters::thin()))?;

    let model = get_model(
        &repository,
        matches.get_one::<String>("model").map(|s| &s[..]),
        REPO_CONFIG_FILE,
        &models_dir,
    )?;

    let format = match matches.get_one::<String>("format") {
        None => CommitFormat::OneLine,
        Some(str) => CommitFormat::from_str(str)?,
    };

    let colored = if matches.get_flag("no-color") {
        false
    } else if let Some(mode) = matches.get_one::<String>("color") {
        match &mode[..] {
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

    let wrapping = if let Some(wrap_values) = matches.get_many::<String>("wrap") {
        let strings = wrap_values.map(|s| s.as_str()).collect::<Vec<_>>();
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
                    Some((None, wrap.first().cloned(), wrap.get(1).cloned()))
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
                        wrap.first().cloned(),
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
        for branch in &graph.all_branches {
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
        println!("{}", print_svg(&graph, settings)?);
    } else {
        let (g_lines, t_lines, _indices) = print_unicode(&graph, settings)?;
        if pager && atty::is(atty::Stream::Stdout) {
            print_paged(&g_lines, &t_lines).map_err(|err| err.to_string())?;
        } else {
            print_unpaged(&g_lines, &t_lines);
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
fn print_paged(graph_lines: &[String], text_lines: &[String]) -> Result<(), ErrorKind> {
    let (width, height) = crossterm::terminal::size()?;
    let width = width as usize;

    let mut line_idx = 0;
    let mut print_lines = height - 2;
    let mut clear = false;
    let mut abort = false;

    let help = "\r >>> Down: line, PgDown/Enter: page, End: all, Esc/Q/^C: quit\r";
    let help = if help.len() > width {
        &help[0..width]
    } else {
        help
    };

    while line_idx < graph_lines.len() {
        if print_lines > 0 {
            if clear {
                stdout()
                    .execute(Clear(ClearType::CurrentLine))?
                    .execute(MoveToColumn(0))?;
            }

            stdout().execute(Print(format!(
                " {}  {}\n",
                graph_lines[line_idx], text_lines[line_idx]
            )))?;

            if print_lines == 1 && line_idx < graph_lines.len() - 1 {
                stdout().execute(Print(help))?;
            }
            print_lines -= 1;
            line_idx += 1;
        } else {
            enable_raw_mode()?;
            let input = crossterm::event::read()?;
            if let Event::Key(evt) = input {
                match evt.code {
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
                        print_lines = graph_lines.len() as u16;
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
                }
            }
        }
    }
    if abort {
        stdout()
            .execute(Clear(ClearType::CurrentLine))?
            .execute(MoveToColumn(0))?
            .execute(Print(" ...\n"))?;
    }
    disable_raw_mode()?;

    Ok(())
}

/// Print the graph, un-paged.
fn print_unpaged(graph_lines: &[String], text_lines: &[String]) {
    for (g_line, t_line) in graph_lines.iter().zip(text_lines.iter()) {
        println!(" {}  {}", g_line, t_line);
    }
}
