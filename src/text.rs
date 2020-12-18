use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref REGEX_GITLAB_PULL: Regex = Regex::new(r"^Merge branch '(.+)' into '(.+)'$").unwrap();
    static ref REGEX_GIT_DEFAULT: Regex = Regex::new(r"^Merge branch '(.+)' into (.+)$").unwrap();
    static ref REGEX_GIT_MASTER: Regex = Regex::new(r"^Merge branch '(.+)'$").unwrap();
    static ref REGEX_GITHUB_PULL: Regex =
        Regex::new(r"^Merge pull request #[0-9]+ from .[^/]+/(.+)$").unwrap();
    static ref REGEX_BITBUCKET_PULL: Regex =
        Regex::new(r"^Merged in (.+) \(pull request #[0-9]+\)$").unwrap();
}

#[allow(dead_code)]
fn parse_merge_summary(summary: &str) -> (Option<String>, Option<String>) {
    if let Some(captures) = REGEX_GITLAB_PULL.captures(summary) {
        if captures.len() == 3 && captures.get(2).is_some() && captures.get(1).is_some() {
            return (
                captures.get(2).map(|m| m.as_str().to_string()),
                captures.get(1).map(|m| m.as_str().to_string()),
            );
        }
    }

    if let Some(captures) = REGEX_GIT_DEFAULT.captures(summary) {
        if captures.len() == 3 && captures.get(2).is_some() && captures.get(1).is_some() {
            return (
                captures.get(2).map(|m| m.as_str().to_string()),
                captures.get(1).map(|m| m.as_str().to_string()),
            );
        }
    }

    if let Some(captures) = REGEX_GIT_MASTER.captures(summary) {
        if captures.len() == 2 && captures.get(1).is_some() {
            return (None, captures.get(1).map(|m| m.as_str().to_string()));
        }
    }

    if let Some(captures) = REGEX_GITHUB_PULL.captures(summary) {
        if captures.len() == 2 && captures.get(1).is_some() {
            return (None, captures.get(1).map(|m| m.as_str().to_string()));
        }
    }

    if let Some(captures) = REGEX_BITBUCKET_PULL.captures(summary) {
        if captures.len() == 2 && captures.get(1).is_some() {
            return (None, captures.get(1).map(|m| m.as_str().to_string()));
        }
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_merge_summary() {
        let gitlab_pull = "Merge branch 'feature/my-feature' into 'master'";
        let git_default = "Merge branch 'feature/my-feature' into dev";
        let git_master = "Merge branch 'feature/my-feature'";
        let github_pull = "Merge pull request #1 from user-x/feature/my-feature";
        let bitbucket_pull = "Merged in feature/my-feature (pull request #1)";

        assert_eq!(
            super::parse_merge_summary(&gitlab_pull),
            (
                Some("master".to_string()),
                Some("feature/my-feature".to_string())
            )
        );
        assert_eq!(
            super::parse_merge_summary(&git_default),
            (
                Some("dev".to_string()),
                Some("feature/my-feature".to_string()),
            )
        );
        assert_eq!(
            super::parse_merge_summary(&git_master),
            (None, Some("feature/my-feature".to_string()))
        );
        assert_eq!(
            super::parse_merge_summary(&github_pull),
            (None, Some("feature/my-feature".to_string()))
        );
        assert_eq!(
            super::parse_merge_summary(&bitbucket_pull),
            (None, Some("feature/my-feature".to_string()))
        );
    }
}
