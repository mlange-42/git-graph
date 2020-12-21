/// Top-level settings
pub struct Settings {
    /// Settings for branches
    pub branches: BranchSettings,
}

impl Settings {
    pub fn git_flow() -> Self {
        Self {
            branches: BranchSettings {
                include_remote: true,
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
                    ("master".to_string(), "blue".to_string()),
                    ("main".to_string(), "blue".to_string()),
                    ("develop".to_string(), "orange".to_string()),
                    ("dev".to_string(), "orange".to_string()),
                    ("feature".to_string(), "purple".to_string()),
                    ("release".to_string(), "green".to_string()),
                    ("hotfix".to_string(), "red".to_string()),
                    ("bugfix".to_string(), "red".to_string()),
                ],
            },
        }
    }
}

pub struct BranchSettings {
    /// Include remote branches?
    pub include_remote: bool,
    /// Branch persistence
    pub persistence: Vec<String>,
    /// Branch ordering
    pub order: Vec<String>,
    /// Branch colors
    pub color: Vec<(String, String)>,
}
