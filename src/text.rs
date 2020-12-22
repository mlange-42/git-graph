use crate::settings::MergePatterns;

pub fn parse_merge_summary(summary: &str, patterns: &MergePatterns) -> Option<String> {
    for regex in &patterns.patterns {
        if let Some(captures) = regex.captures(summary) {
            if captures.len() == 2 && captures.get(1).is_some() {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::settings::MergePatterns;

    #[test]
    fn parse_merge_summary() {
        let patterns = MergePatterns::default();

        let gitlab_pull = "Merge branch 'feature/my-feature' into 'master'";
        let git_default = "Merge branch 'feature/my-feature' into dev";
        let git_master = "Merge branch 'feature/my-feature'";
        let github_pull = "Merge pull request #1 from user-x/feature/my-feature";
        let github_pull_2 = "Merge branch 'feature/my-feature' of github.com:user-x/repo";
        let bitbucket_pull = "Merged in feature/my-feature (pull request #1)";

        assert_eq!(
            super::parse_merge_summary(&gitlab_pull, &patterns),
            Some("feature/my-feature".to_string()),
        );
        assert_eq!(
            super::parse_merge_summary(&git_default, &patterns),
            Some("feature/my-feature".to_string()),
        );
        assert_eq!(
            super::parse_merge_summary(&git_master, &patterns),
            Some("feature/my-feature".to_string()),
        );
        assert_eq!(
            super::parse_merge_summary(&github_pull, &patterns),
            Some("feature/my-feature".to_string()),
        );
        assert_eq!(
            super::parse_merge_summary(&github_pull_2, &patterns),
            Some("feature/my-feature".to_string()),
        );
        assert_eq!(
            super::parse_merge_summary(&bitbucket_pull, &patterns),
            Some("feature/my-feature".to_string()),
        );
    }
}
