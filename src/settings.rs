use regex::Regex;
use std::str::FromStr;

/// Ordering policy for branches in visual columns.
pub enum BranchOrder {
    /// Recommended! Shortest branches are inserted left-most.
    ///
    /// For branches with equal length, branches ending last are inserted first.
    /// Reverse (arg = false): Branches ending first are inserted first.
    ShortestFirst(bool),
    /// Longest branches are inserted left-most.
    ///
    /// For branches with equal length, branches ending last are inserted first.
    /// Reverse (arg = false): Branches ending first are inserted first.
    LongestFirst(bool),
    /// Branches ending last are inserted left-most.
    ///
    /// Reverse (arg = false): Branches starting first are inserted left-most.
    FirstComeFirstServed(bool),
}

/// Top-level settings
pub struct Settings {
    /// Debug printing and drawing
    pub debug: bool,
    /// Compact text-based graph
    pub compact: bool,
    /// Colored text-based graph
    pub colored: bool,
    /// Include remote branches?
    pub include_remote: bool,
    /// Characters to use for text-based graph
    pub characters: Characters,
    /// Branch column sorting algorithm
    pub branch_order: BranchOrder,
    /// Settings for branches
    pub branches: BranchSettings,
    /// Regex patterns for finding branch names in merge commit summaries
    pub merge_patterns: MergePatterns,
}

pub struct BranchSettings {
    /// Branch persistence
    pub persistence: Vec<String>,
    /// Branch ordering
    pub order: Vec<String>,
    /// Branch colors
    pub terminal_colors: Vec<(String, Vec<String>)>,
    /// Colors for branches not matching any of `colors`
    pub terminal_colors_unknown: Vec<String>,
    /// Branch colors for SVG output
    pub svg_colors: Vec<(String, Vec<String>)>,
    /// Colors for branches not matching any of `colors` for SVG output
    pub svg_colors_unknown: Vec<String>,
}

impl BranchSettings {
    pub fn git_flow() -> Self {
        BranchSettings {
            persistence: vec![
                "master".to_string(),
                "main".to_string(),
                "develop".to_string(),
                "dev".to_string(),
                "feature".to_string(),
                "release".to_string(),
                "hotfix".to_string(),
                "bugfix".to_string(),
            ],
            order: vec![
                "master".to_string(),
                "main".to_string(),
                "hotfix".to_string(),
                "release".to_string(),
                "develop".to_string(),
                "dev".to_string(),
            ],
            terminal_colors: vec![
                ("master".to_string(), vec!["blue".to_string()]),
                ("main".to_string(), vec!["blue".to_string()]),
                ("develop".to_string(), vec!["yellow".to_string()]),
                ("dev".to_string(), vec!["yellow".to_string()]),
                (
                    "feature".to_string(),
                    vec!["magenta".to_string(), "cyan".to_string()],
                ),
                ("release".to_string(), vec!["green".to_string()]),
                ("hotfix".to_string(), vec!["red".to_string()]),
                ("bugfix".to_string(), vec!["red".to_string()]),
            ],
            terminal_colors_unknown: vec!["white".to_string()],

            svg_colors: vec![
                ("master".to_string(), vec!["blue".to_string()]),
                ("main".to_string(), vec!["blue".to_string()]),
                ("develop".to_string(), vec!["orange".to_string()]),
                ("dev".to_string(), vec!["orange".to_string()]),
                (
                    "feature".to_string(),
                    vec!["purple".to_string(), "turquoise".to_string()],
                ),
                ("release".to_string(), vec!["green".to_string()]),
                ("hotfix".to_string(), vec!["red".to_string()]),
                ("bugfix".to_string(), vec!["red".to_string()]),
            ],
            svg_colors_unknown: vec!["gray".to_string()],
        }
    }

    pub fn simple() -> Self {
        BranchSettings {
            persistence: vec!["master".to_string(), "main".to_string()],
            order: vec!["master".to_string(), "main".to_string()],
            terminal_colors: vec![
                ("master".to_string(), vec!["blue".to_string()]),
                ("main".to_string(), vec!["blue".to_string()]),
            ],
            terminal_colors_unknown: vec![
                "yellow".to_string(),
                "green".to_string(),
                "red".to_string(),
                "magenta".to_string(),
                "cyan".to_string(),
            ],

            svg_colors: vec![
                ("master".to_string(), vec!["blue".to_string()]),
                ("main".to_string(), vec!["blue".to_string()]),
            ],
            svg_colors_unknown: vec![
                "orange".to_string(),
                "green".to_string(),
                "red".to_string(),
                "purple".to_string(),
                "turquoise".to_string(),
            ],
        }
    }
}

pub struct MergePatterns {
    pub patterns: Vec<Regex>,
}

impl Default for MergePatterns {
    fn default() -> Self {
        MergePatterns {
            patterns: vec![
                // GitLab pull request
                Regex::new(r"^Merge branch '(.+)' into '.+'$").unwrap(),
                // Git default
                Regex::new(r"^Merge branch '(.+)' into .+$").unwrap(),
                // Git default into main branch
                Regex::new(r"^Merge branch '(.+)'$").unwrap(),
                // GitHub pull request
                Regex::new(r"^Merge pull request #[0-9]+ from .[^/]+/(.+)$").unwrap(),
                // GitHub pull request (from fork?)
                Regex::new(r"^Merge branch '(.+)' of .+$").unwrap(),
                // BitBucket pull request
                Regex::new(r"^Merged in (.+) \(pull request #[0-9]+\)$").unwrap(),
            ],
        }
    }
}

pub struct Characters {
    pub chars: Vec<char>,
}

impl FromStr for Characters {
    type Err = String;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "normal" | "thin" => Ok(Characters::thin()),
            "round" => Ok(Characters::round()),
            "bold" => Ok(Characters::bold()),
            "double" => Ok(Characters::double()),
            "ascii" => Ok(Characters::ascii()),
            _ => Err(format!("Unknown characters/style '{}'. Must be one of [normal|thin|round|bold|double|ascii]", str)),
        }
    }
}

impl Characters {
    pub fn thin() -> Self {
        Characters {
            chars: " ●○│─┼└┌┐┘┤├┴┬<>".chars().collect(),
        }
    }
    pub fn round() -> Self {
        Characters {
            chars: " ●○│─┼╰╭╮╯┤├┴┬<>".chars().collect(),
        }
    }
    pub fn bold() -> Self {
        Characters {
            chars: " ●○┃━╋┗┏┓┛┫┣┻┳<>".chars().collect(),
        }
    }
    pub fn double() -> Self {
        Characters {
            chars: " ●○║═╬╚╔╗╝╣╠╩╦<>".chars().collect(),
        }
    }
    pub fn ascii() -> Self {
        Characters {
            chars: " *o|-+'..'||++<>".chars().collect(),
        }
    }
}
