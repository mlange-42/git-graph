//! Formatting of commits.

use chrono::{FixedOffset, Local, TimeZone};
use git2::{Commit, Time};
use lazy_static::lazy_static;
use std::fmt::Write;
use std::str::FromStr;
use textwrap::Options;
use yansi::Paint;

/// Commit formatting options.
#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum CommitFormat {
    OneLine,
    Short,
    Medium,
    Full,
    Format(String),
}

impl FromStr for CommitFormat {
    type Err = String;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "oneline" | "o" => Ok(CommitFormat::OneLine),
            "short" | "s" => Ok(CommitFormat::Short),
            "medium" | "m" => Ok(CommitFormat::Medium),
            "full" | "f" => Ok(CommitFormat::Full),
            str => Ok(CommitFormat::Format(str.to_string())),
        }
    }
}

const NEW_LINE: usize = 0;
const HASH: usize = 1;
const HASH_ABBREV: usize = 2;
const PARENT_HASHES: usize = 3;
const PARENT_HASHES_ABBREV: usize = 4;
const REFS: usize = 5;
const SUBJECT: usize = 6;
const AUTHOR: usize = 7;
const AUTHOR_EMAIL: usize = 8;
const AUTHOR_DATE: usize = 9;
const AUTHOR_DATE_SHORT: usize = 10;
const COMMITTER: usize = 11;
const COMMITTER_EMAIL: usize = 12;
const COMMITTER_DATE: usize = 13;
const COMMITTER_DATE_SHORT: usize = 14;
const BODY: usize = 15;
const BODY_RAW: usize = 16;

const MODE_SPACE: usize = 1;
const MODE_PLUS: usize = 2;
const MODE_MINUS: usize = 3;

lazy_static! {
    pub static ref PLACEHOLDERS: Vec<[String; 4]> = {
        let base = vec![
            "n", "H", "h", "P", "p", "d", "s", "an", "ae", "ad", "as", "cn", "ce", "cd", "cs", "b",
            "B",
        ];
        base.iter()
            .map(|b| {
                [
                    format!("%{}", b),
                    format!("% {}", b),
                    format!("%+{}", b),
                    format!("%-{}", b),
                ]
            })
            .collect()
    };
}

/// Format a commit for `CommitFormat::Format(String)`.
pub fn format_commit(
    format: &str,
    commit: &Commit,
    branches: String,
    wrapping: &Option<Options>,
    hash_color: Option<u8>,
) -> Result<Vec<String>, String> {
    let mut replacements = vec![];

    for (idx, arr) in PLACEHOLDERS.iter().enumerate() {
        let mut curr = 0;
        loop {
            let mut found = false;
            for (mode, str) in arr.iter().enumerate() {
                if let Some(start) = &format[curr..format.len()].find(str) {
                    replacements.push((curr + start, str.len(), idx, mode));
                    curr += start + str.len();
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }
    }

    replacements.sort_by_key(|p| p.0);

    let mut lines = vec![];
    let mut out = String::new();
    if replacements.is_empty() {
        write!(out, "{}", format).unwrap();
        add_line(&mut lines, &mut out, wrapping);
    } else {
        let mut curr = 0;
        for (start, len, idx, mode) in replacements {
            if idx == NEW_LINE {
                write!(out, "{}", &format[curr..start]).unwrap();
                add_line(&mut lines, &mut out, wrapping);
            } else {
                write!(out, "{}", &format[curr..start]).unwrap();
                match idx {
                    HASH => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        if let Some(color) = hash_color {
                            write!(out, "{}", Paint::fixed(color, commit.id()))
                        } else {
                            write!(out, "{}", commit.id())
                        }
                    }
                    HASH_ABBREV => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        if let Some(color) = hash_color {
                            write!(
                                out,
                                "{}",
                                Paint::fixed(color, &commit.id().to_string()[..7])
                            )
                        } else {
                            write!(out, "{}", &commit.id().to_string()[..7])
                        }
                    }
                    PARENT_HASHES => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        for i in 0..commit.parent_count() {
                            write!(out, "{}", commit.parent_id(i).unwrap()).unwrap();
                            if i < commit.parent_count() - 1 {
                                write!(out, " ").unwrap();
                            }
                        }
                        Ok(())
                    }
                    PARENT_HASHES_ABBREV => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        for i in 0..commit.parent_count() {
                            write!(
                                out,
                                "{}",
                                &commit
                                    .parent_id(i)
                                    .map_err(|err| err.to_string())?
                                    .to_string()[..7]
                            )
                            .unwrap();
                            if i < commit.parent_count() - 1 {
                                write!(out, " ").unwrap();
                            }
                        }
                        Ok(())
                    }
                    REFS => {
                        match mode {
                            MODE_SPACE => {
                                if !branches.is_empty() {
                                    write!(out, " ").unwrap()
                                }
                            }
                            MODE_PLUS => {
                                if !branches.is_empty() {
                                    add_line(&mut lines, &mut out, wrapping)
                                }
                            }
                            MODE_MINUS => {
                                if branches.is_empty() {
                                    out = remove_empty_lines(&mut lines, out)
                                }
                            }
                            _ => {}
                        }
                        write!(out, "{}", branches)
                    }
                    SUBJECT => {
                        let summary = commit.summary().unwrap_or("");
                        match mode {
                            MODE_SPACE => {
                                if !summary.is_empty() {
                                    write!(out, " ").unwrap()
                                }
                            }
                            MODE_PLUS => {
                                if !summary.is_empty() {
                                    add_line(&mut lines, &mut out, wrapping)
                                }
                            }
                            MODE_MINUS => {
                                if summary.is_empty() {
                                    out = remove_empty_lines(&mut lines, out)
                                }
                            }
                            _ => {}
                        }
                        write!(out, "{}", summary)
                    }
                    AUTHOR => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        write!(out, "{}", &commit.author().name().unwrap_or(""))
                    }
                    AUTHOR_EMAIL => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        write!(out, "{}", &commit.author().email().unwrap_or(""))
                    }
                    AUTHOR_DATE => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        write!(
                            out,
                            "{}",
                            format_date(commit.author().when(), "%a %b %e %H:%M:%S %Y %z")
                        )
                    }
                    AUTHOR_DATE_SHORT => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        write!(out, "{}", format_date(commit.author().when(), "%F"))
                    }
                    COMMITTER => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        write!(out, "{}", &commit.committer().name().unwrap_or(""))
                    }
                    COMMITTER_EMAIL => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        write!(out, "{}", &commit.committer().email().unwrap_or(""))
                    }
                    COMMITTER_DATE => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        write!(
                            out,
                            "{}",
                            format_date(commit.committer().when(), "%a %b %e %H:%M:%S %Y %z")
                        )
                    }
                    COMMITTER_DATE_SHORT => {
                        match mode {
                            MODE_SPACE => write!(out, " ").unwrap(),
                            MODE_PLUS => add_line(&mut lines, &mut out, wrapping),
                            _ => {}
                        }
                        write!(out, "{}", format_date(commit.committer().when(), "%F"))
                    }
                    BODY => {
                        let message = commit
                            .message()
                            .unwrap_or("")
                            .lines()
                            .collect::<Vec<&str>>();

                        let num_parts = message.len();
                        match mode {
                            MODE_SPACE => {
                                if num_parts > 2 {
                                    write!(out, " ").unwrap()
                                }
                            }
                            MODE_PLUS => {
                                if num_parts > 2 {
                                    add_line(&mut lines, &mut out, wrapping)
                                }
                            }
                            MODE_MINUS => {
                                if num_parts <= 2 {
                                    out = remove_empty_lines(&mut lines, out)
                                }
                            }
                            _ => {}
                        }
                        for (cnt, line) in message.iter().enumerate() {
                            if cnt > 1 && (cnt < num_parts - 1 || !line.is_empty()) {
                                write!(out, "{}", line).unwrap();
                                add_line(&mut lines, &mut out, wrapping);
                            }
                        }
                        Ok(())
                    }
                    BODY_RAW => {
                        let message = commit
                            .message()
                            .unwrap_or("")
                            .lines()
                            .collect::<Vec<&str>>();

                        let num_parts = message.len();

                        match mode {
                            MODE_SPACE => {
                                if !message.is_empty() {
                                    write!(out, " ").unwrap()
                                }
                            }
                            MODE_PLUS => {
                                if !message.is_empty() {
                                    add_line(&mut lines, &mut out, wrapping)
                                }
                            }
                            MODE_MINUS => {
                                if message.is_empty() {
                                    out = remove_empty_lines(&mut lines, out)
                                }
                            }
                            _ => {}
                        }
                        for (cnt, line) in message.iter().enumerate() {
                            if cnt < num_parts - 1 || !line.is_empty() {
                                write!(out, "{}", line).unwrap();
                                add_line(&mut lines, &mut out, wrapping);
                            }
                        }
                        Ok(())
                    }
                    x => return Err(format!("No commit field at index {}", x)),
                }
                .unwrap();
            }
            curr = start + len;
        }
        write!(out, "{}", &format[curr..(format.len())]).unwrap();
        if !out.is_empty() {
            add_line(&mut lines, &mut out, wrapping);
        }
    }
    Ok(lines)
}

/// Format a commit for `CommitFormat::OneLine`.
pub fn format_oneline(
    commit: &Commit,
    branches: String,
    wrapping: &Option<Options>,
    hash_color: Option<u8>,
) -> Vec<String> {
    let mut out = String::new();
    if let Some(color) = hash_color {
        write!(
            out,
            "{}",
            Paint::fixed(color, &commit.id().to_string()[..7])
        )
    } else {
        write!(out, "{}", &commit.id().to_string()[..7])
    }
    .unwrap();

    write!(out, "{} {}", branches, commit.summary().unwrap_or("")).unwrap();

    if let Some(wrap) = wrapping {
        textwrap::fill(&out, wrap)
            .lines()
            .map(|str| str.to_string())
            .collect()
    } else {
        vec![out]
    }
}

/// Format a commit for `CommitFormat::Short`, `CommitFormat::Medium` or `CommitFormat::Full`.
pub fn format(
    commit: &Commit,
    branches: String,
    wrapping: &Option<Options>,
    hash_color: Option<u8>,
    format: &CommitFormat,
) -> Result<Vec<String>, String> {
    match format {
        CommitFormat::OneLine => return Ok(format_oneline(commit, branches, wrapping, hash_color)),
        CommitFormat::Format(format) => {
            return format_commit(format, commit, branches, wrapping, hash_color)
        }
        _ => {}
    }

    let mut out_vec = vec![];
    let mut out = String::new();

    if let Some(color) = hash_color {
        write!(out, "commit {}", Paint::fixed(color, &commit.id()))
    } else {
        write!(out, "commit {}", &commit.id())
    }
    .map_err(|err| err.to_string())?;

    write!(out, "{}", branches).map_err(|err| err.to_string())?;
    append_wrapped(&mut out_vec, out, wrapping);

    if commit.parent_count() > 1 {
        out = String::new();
        write!(
            out,
            "Merge: {} {}",
            &commit.parent_id(0).unwrap().to_string()[..7],
            &commit.parent_id(1).unwrap().to_string()[..7]
        )
        .map_err(|err| err.to_string())?;
        append_wrapped(&mut out_vec, out, wrapping);
    }

    out = String::new();
    write!(
        out,
        "Author: {} <{}>",
        commit.author().name().unwrap_or(""),
        commit.author().email().unwrap_or("")
    )
    .map_err(|err| err.to_string())?;
    append_wrapped(&mut out_vec, out, wrapping);

    if format > &CommitFormat::Medium {
        out = String::new();
        write!(
            out,
            "Commit: {} <{}>",
            commit.committer().name().unwrap_or(""),
            commit.committer().email().unwrap_or("")
        )
        .map_err(|err| err.to_string())?;
        append_wrapped(&mut out_vec, out, wrapping);
    }

    if format > &CommitFormat::Short {
        out = String::new();
        write!(
            out,
            "Date:   {}",
            format_date(commit.author().when(), "%a %b %e %H:%M:%S %Y %z")
        )
        .map_err(|err| err.to_string())?;
        append_wrapped(&mut out_vec, out, wrapping);
    }

    if format == &CommitFormat::Short {
        out_vec.push("".to_string());
        append_wrapped(
            &mut out_vec,
            format!("    {}", commit.summary().unwrap_or("")),
            wrapping,
        );
        out_vec.push("".to_string());
    } else {
        out_vec.push("".to_string());
        let mut add_line = true;
        for line in commit.message().unwrap_or("").lines() {
            if line.is_empty() {
                out_vec.push(line.to_string());
            } else {
                append_wrapped(&mut out_vec, format!("    {}", line), wrapping);
            }
            add_line = !line.trim().is_empty();
        }
        if add_line {
            out_vec.push("".to_string());
        }
    }

    Ok(out_vec)
}

pub fn format_date(time: Time, format: &str) -> String {
    let date =
        Local::from_offset(&FixedOffset::east(time.offset_minutes())).timestamp(time.seconds(), 0);
    format!("{}", date.format(format))
}

fn append_wrapped(vec: &mut Vec<String>, str: String, wrapping: &Option<Options>) {
    if str.is_empty() {
        vec.push(str);
    } else if let Some(wrap) = wrapping {
        vec.extend(
            textwrap::fill(&str, wrap)
                .lines()
                .map(|str| str.to_string()),
        )
    } else {
        vec.push(str);
    }
}

fn add_line(lines: &mut Vec<String>, line: &mut String, wrapping: &Option<Options>) {
    let mut temp = String::new();
    std::mem::swap(&mut temp, line);
    append_wrapped(lines, temp, wrapping);
}

fn remove_empty_lines(lines: &mut Vec<String>, mut line: String) -> String {
    while !lines.is_empty() && lines.last().unwrap().is_empty() {
        line = lines.remove(lines.len() - 1);
    }
    if !lines.is_empty() {
        line = lines.remove(lines.len() - 1);
    }
    line
}
