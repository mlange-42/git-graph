use crate::settings::Settings;
use crate::text;
use git2::{BranchType, Commit, Error, Oid, Repository};
use std::collections::HashMap;

pub struct GitGraph {
    pub repository: Repository,
    pub commits: Vec<CommitInfo>,
}

impl GitGraph {
    pub fn new(path: &str, settings: &Settings) -> Result<Self, Error> {
        let repository = Repository::open(path)?;
        let mut walk = repository.revwalk()?;

        walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        walk.push_glob("*")?;

        let mut commits = Vec::new();
        let mut indices = HashMap::new();
        for (idx, oid) in walk.enumerate() {
            let oid = oid?;
            let commit = repository.find_commit(oid).unwrap();
            commits.push(CommitInfo::new(&commit));
            indices.insert(oid, idx);
        }

        let mut graph = GitGraph {
            repository,
            commits,
        };
        graph.assign_branches(indices, settings)?;

        Ok(graph)
    }
    fn assign_branches(
        &mut self,
        indices: HashMap<Oid, usize>,
        settings: &Settings,
    ) -> Result<(), Error> {
        let valid_branches = extract_branches(&self.repository, &self.commits)?;
        let branches_ordered = branches_persistence_order(valid_branches, settings);

        for branch in branches_ordered {
            if let Some(&idx) = indices.get(&branch.target) {
                let trace_oid = {
                    let info = &mut self.commits[idx];
                    if !info.branches.contains(&branch.name) {
                        info.branches.push(branch.name.to_owned());
                        Some(info.oid)
                    } else {
                        None
                    }
                };
                if let Some(oid) = trace_oid {
                    trace_branch(
                        &self.repository,
                        &mut self.commits,
                        &indices,
                        oid,
                        &branch.name,
                    )?;
                }
            }
        }

        Ok(())
    }

    pub fn commit(&self, id: Oid) -> Result<Commit, Error> {
        self.repository.find_commit(id)
    }
}

pub struct CommitInfo {
    pub oid: Oid,
    pub branches: Vec<String>,
    pub branch_trace: Option<String>,
}

impl CommitInfo {
    fn new(commit: &Commit) -> Self {
        CommitInfo {
            oid: commit.id(),
            branches: Vec::new(),
            branch_trace: None,
        }
    }
}

pub struct BranchInfo {
    pub target: Oid,
    pub name: String,
    pub deleted: bool,
}
impl BranchInfo {
    fn new(target: Oid, name: String, deleted: bool) -> Self {
        BranchInfo {
            target,
            name,
            deleted,
        }
    }
}

fn trace_branch<'repo>(
    repository: &'repo Repository,
    commits: &mut Vec<CommitInfo>,
    indices: &HashMap<Oid, usize>,
    oid: Oid,
    branch: &str,
) -> Result<(), Error> {
    let mut curr_oid = oid;
    loop {
        let index = indices[&curr_oid];
        let info = &mut commits[index];
        if info.branch_trace.is_some() {
            break;
        } else {
            info.branch_trace = Some(branch.to_string());
        }
        let commit = repository.find_commit(curr_oid)?;
        match commit.parent_count() {
            0 => break,
            _ => {
                curr_oid = commit.parent_id(0)?;
            }
        }
    }
    Ok(())
}

fn extract_branches(
    repository: &Repository,
    commits: &[CommitInfo],
) -> Result<Vec<BranchInfo>, Error> {
    let actual_branches = repository
        .branches(Some(BranchType::Local))?
        .map(|bt| bt.map(|bt| bt.0))
        .collect::<Result<Vec<_>, Error>>()?;

    let mut valid_branches = actual_branches
        .iter()
        .filter_map(|br| {
            br.get().name().and_then(|n| {
                br.get()
                    .target()
                    .map(|t| BranchInfo::new(t, n[11..].to_string(), false))
            })
        })
        .collect::<Vec<_>>();

    for info in commits {
        let commit = repository.find_commit(info.oid)?;
        if commit.parent_count() > 1 {
            if let Some(summary) = commit.summary() {
                let branches = text::parse_merge_summary(summary);
                if let Some(branch) = branches.1 {
                    let parent_oid = commit.parent_id(1)?;
                    valid_branches.push(BranchInfo::new(parent_oid, branch, true));
                }
            }
        }
    }

    Ok(valid_branches)
}

fn branches_persistence_order(
    mut branches: Vec<BranchInfo>,
    settings: &Settings,
) -> Vec<BranchInfo> {
    branches.sort_by_cached_key(|branch| {
        settings
            .branch_persistance
            .iter()
            .position(|b| branch.name.starts_with(b))
            .unwrap_or(settings.branch_persistance.len())
    });
    branches
}
