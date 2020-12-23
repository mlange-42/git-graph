use regex::Regex;

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
    /// Include remote branches?
    pub include_remote: bool,
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
    pub color: Vec<(String, String, String)>,
    /// Color for branches not matching any of `colors`
    pub color_unknown: (String, String),
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
            color: vec![
                ("master".to_string(), "blue".to_string(), "blue".to_string()),
                ("main".to_string(), "blue".to_string(), "blue".to_string()),
                (
                    "develop".to_string(),
                    "orange".to_string(),
                    "yellow".to_string(),
                ),
                (
                    "dev".to_string(),
                    "orange".to_string(),
                    "yellow".to_string(),
                ),
                (
                    "feature".to_string(),
                    "purple".to_string(),
                    "magenta".to_string(),
                ),
                (
                    "release".to_string(),
                    "green".to_string(),
                    "green".to_string(),
                ),
                ("hotfix".to_string(), "red".to_string(), "red".to_string()),
                ("bugfix".to_string(), "red".to_string(), "red".to_string()),
            ],
            color_unknown: ("gray".to_string(), "white".to_string()),
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
                // GitLab pull rewuest
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
