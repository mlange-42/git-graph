use git2::Commit;
use lazy_static::lazy_static;
use std::fmt::Write;
use std::str::FromStr;
use yansi::Paint;

const FORMAT_ONELINE: &str = "%h%d %s";
const FORMAT_SHORT: &str = "%h%d %s";
const FORMAT_MEDIUM: &str = "%h%d %s";
const FORMAT_LONG: &str = "%h%d %s";

pub enum CommitFormat {
    OneLine,
    Short,
    Medium,
    Long,
    Format(String),
}

impl CommitFormat {
    pub fn get_format(&self) -> String {
        match self {
            CommitFormat::OneLine => FORMAT_ONELINE.to_string(),
            CommitFormat::Short => FORMAT_SHORT.to_string(),
            CommitFormat::Medium => FORMAT_MEDIUM.to_string(),
            CommitFormat::Long => FORMAT_LONG.to_string(),
            CommitFormat::Format(str) => str.clone(),
        }
    }
}

impl FromStr for CommitFormat {
    type Err = String;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "oneline" => Ok(CommitFormat::OneLine),
            "short" => Ok(CommitFormat::Short),
            "medium" => Ok(CommitFormat::Medium),
            "long" => Ok(CommitFormat::Long),
            str => Ok(CommitFormat::Format(str.to_string())),
        }
    }
}

const NEW_LINE: usize = 0;
const HASH: usize = 1;
const HASH_ABBREV: usize = 2;
const REFS: usize = 3;
const SUBJECT: usize = 4;

lazy_static! {
    pub static ref PLACEHOLDERS: Vec<&'static str> = vec!["%n", "%H", "%h", "%d", "%s"];
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
