use chrono::{FixedOffset, Local, TimeZone};
use git2::Commit;
use lazy_static::lazy_static;
use std::fmt::Write;
use std::str::FromStr;
use yansi::Paint;

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
            "oneline" => Ok(CommitFormat::OneLine),
            "short" => Ok(CommitFormat::Short),
            "medium" => Ok(CommitFormat::Medium),
            "full" => Ok(CommitFormat::Full),
            str => Ok(CommitFormat::Format(str.to_string())),
        }
    }
}

const NEW_LINE: usize = 0;
const HASH: usize = 1;
const HASH_ABBREV: usize = 2;
const REFS: usize = 3;
const SUBJECT: usize = 4;
const AUTHOR: usize = 5;
const AUTHOR_EMAIL: usize = 6;

lazy_static! {
    pub static ref PLACEHOLDERS: Vec<&'static str> =
        vec!["%n", "%H", "%h", "%d", "%s", "%an", "%ae"];
}

#[allow(dead_code)]
pub fn format_commit(
    format: &str,
    commit: &Commit,
    branches: String,
    hash_color: Option<u8>,
) -> Result<Vec<String>, String> {
    let mut replacements = vec![];

    for (idx, str) in PLACEHOLDERS.iter().enumerate() {
        let mut curr = 0;
        while let Some(start) = &format[curr..format.len()].find(str) {
            replacements.push((curr + start, str.len(), idx));
            curr += start + str.len();
        }
    }

    replacements.sort_by_key(|p| p.0);

    let mut lines = vec![];
    let mut out = String::new();
    if replacements.is_empty() {
        write!(out, "{}", format).map_err(|err| err.to_string())?;
        lines.push(out);
    } else {
        let mut curr = 0;
        for (start, len, idx) in replacements {
            if idx == NEW_LINE {
                let mut temp = String::new();
                std::mem::swap(&mut temp, &mut out);
                lines.push(temp);
            } else {
                let prefix = &format[curr..start];
                match idx {
                    HASH => {
                        if let Some(color) = hash_color {
                            write!(out, "{}{}", prefix, Paint::fixed(color, commit.id()))
                        } else {
                            write!(out, "{}{}", prefix, commit.id())
                        }
                    }
                    HASH_ABBREV => {
                        if let Some(color) = hash_color {
                            write!(
                                out,
                                "{}{}",
                                prefix,
                                Paint::fixed(color, &commit.id().to_string()[..7])
                            )
                        } else {
                            write!(out, "{}{}", prefix, &commit.id().to_string()[..7])
                        }
                    }
                    REFS => write!(out, "{}{}", prefix, branches),
                    SUBJECT => write!(out, "{}{}", prefix, commit.summary().unwrap_or("")),
                    AUTHOR => write!(out, "{}{}", prefix, &commit.author().name().unwrap_or("")),
                    AUTHOR_EMAIL => {
                        write!(out, "{}{}", prefix, &commit.author().email().unwrap_or(""))
                    }
                    x => return Err(format!("No commit field at index {}", x)),
                }
                .map_err(|err| err.to_string())?;
            }
            curr = start + len;
        }
        write!(out, "{}", &format[curr..(format.len())]).map_err(|err| err.to_string())?;

        let mut temp = String::new();
        std::mem::swap(&mut temp, &mut out);
        lines.push(temp);
    }
    Ok(lines)
}

pub fn format_oneline(
    commit: &Commit,
    branches: String,
    hash_color: Option<u8>,
) -> Result<Vec<String>, String> {
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
    .map_err(|err| err.to_string())?;

    write!(out, "{} {}", branches, commit.summary().unwrap_or(""))
        .map_err(|err| err.to_string())?;

    Ok(vec![out])
}

pub fn format_short(
    commit: &Commit,
    branches: String,
    hash_color: Option<u8>,
) -> Result<Vec<String>, String> {
    let mut out_vec = vec![];
    let mut out = String::new();
    if let Some(color) = hash_color {
        write!(out, "commit {}", Paint::fixed(color, &commit.id()))
    } else {
        write!(out, "commit {}", &commit.id())
    }
    .map_err(|err| err.to_string())?;

    write!(out, " {}", branches).map_err(|err| err.to_string())?;
    out_vec.push(out);

    if commit.parent_count() > 1 {
        out = String::new();
        write!(
            out,
            "Merge: {} {}",
            &commit.parent_id(0).unwrap().to_string()[..7],
            &commit.parent_id(1).unwrap().to_string()[..7]
        )
        .map_err(|err| err.to_string())?;
        out_vec.push(out);
    }

    out = String::new();
    write!(
        out,
        "Author: {} <{}>",
        commit.author().name().unwrap_or(""),
        commit.author().email().unwrap_or("")
    )
    .map_err(|err| err.to_string())?;
    out_vec.push(out);

    out_vec.push("".to_string());
    let mut add_line = true;
    for line in commit.message().unwrap_or("").split('\n') {
        out_vec.push(format!("    {}", line));
        add_line = !line.trim().is_empty();
    }
    if add_line {
        out_vec.push("".to_string());
    }

    Ok(out_vec)
}

pub fn format_medium(
    commit: &Commit,
    branches: String,
    hash_color: Option<u8>,
) -> Result<Vec<String>, String> {
    let mut out_vec = vec![];
    let mut out = String::new();
    if let Some(color) = hash_color {
        write!(out, "commit {}", Paint::fixed(color, &commit.id()))
    } else {
        write!(out, "commit {}", &commit.id())
    }
    .map_err(|err| err.to_string())?;

    write!(out, " {}", branches).map_err(|err| err.to_string())?;
    out_vec.push(out);

    if commit.parent_count() > 1 {
        out = String::new();
        write!(
            out,
            "Merge: {} {}",
            &commit.parent_id(0).unwrap().to_string()[..7],
            &commit.parent_id(1).unwrap().to_string()[..7]
        )
        .map_err(|err| err.to_string())?;
        out_vec.push(out);
    }

    out = String::new();
    write!(
        out,
        "Author: {} <{}>",
        commit.author().name().unwrap_or(""),
        commit.author().email().unwrap_or("")
    )
    .map_err(|err| err.to_string())?;
    out_vec.push(out);

    out = String::new();
    let time = commit.author().when();
    let date = Local::from_offset(&FixedOffset::east(time.offset_minutes()))
        .timestamp(commit.author().when().seconds(), 0);
    write!(out, "Date:   {}", date.format("%a %b %e %H:%M:%S %Y %z"))
        .map_err(|err| err.to_string())?;
    out_vec.push(out);

    out_vec.push("".to_string());
    let mut add_line = true;
    for line in commit.message().unwrap_or("").split('\n') {
        out_vec.push(format!("    {}", line));
        add_line = !line.trim().is_empty();
    }
    if add_line {
        out_vec.push("".to_string());
    }

    Ok(out_vec)
}

pub fn format_full(
    commit: &Commit,
    branches: String,
    hash_color: Option<u8>,
) -> Result<Vec<String>, String> {
    let mut out_vec = vec![];
    let mut out = String::new();
    if let Some(color) = hash_color {
        write!(out, "commit {}", Paint::fixed(color, &commit.id()))
    } else {
        write!(out, "commit {}", &commit.id())
    }
    .map_err(|err| err.to_string())?;

    write!(out, " {}", branches).map_err(|err| err.to_string())?;
    out_vec.push(out);

    if commit.parent_count() > 1 {
        out = String::new();
        write!(
            out,
            "Merge: {} {}",
            &commit.parent_id(0).unwrap().to_string()[..7],
            &commit.parent_id(1).unwrap().to_string()[..7]
        )
        .map_err(|err| err.to_string())?;
        out_vec.push(out);
    }

    out = String::new();
    write!(
        out,
        "Author: {} <{}>",
        commit.author().name().unwrap_or(""),
        commit.author().email().unwrap_or("")
    )
    .map_err(|err| err.to_string())?;
    out_vec.push(out);

    out = String::new();
    write!(
        out,
        "Commit: {} <{}>",
        commit.committer().name().unwrap_or(""),
        commit.committer().email().unwrap_or("")
    )
    .map_err(|err| err.to_string())?;
    out_vec.push(out);

    out = String::new();
    let time = commit.author().when();
    let date = Local::from_offset(&FixedOffset::east(time.offset_minutes()))
        .timestamp(commit.author().when().seconds(), 0);
    write!(out, "Date:   {}", date.format("%a %b %e %H:%M:%S %Y %z"))
        .map_err(|err| err.to_string())?;
    out_vec.push(out);

    out_vec.push("".to_string());
    let mut add_line = true;
    for line in commit.message().unwrap_or("").split('\n') {
        out_vec.push(format!("    {}", line));
        add_line = !line.trim().is_empty();
    }
    if add_line {
        out_vec.push("".to_string());
    }

    Ok(out_vec)
}
