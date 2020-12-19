/// Top-level settings
pub struct Settings {
    /// Settings for branches
    pub branches: BranchSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            branches: BranchSettings {
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
            },
        }
    }
}

pub struct BranchSettings {
    /// Branch persistence
    pub persistence: Vec<String>,
}
