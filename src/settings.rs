/// Top-leven settings
#[derive(Debug)]
pub struct Settings {
    /// Branch persistance
    pub branch_persistance: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            branch_persistance: vec![
                "master".to_string(),
                "main".to_string(),
                "develop".to_string(),
                "dev".to_string(),
                "feature".to_string(),
                "release".to_string(),
                "hotfix".to_string(),
                "bugfix".to_string(),
            ],
        }
    }
}
