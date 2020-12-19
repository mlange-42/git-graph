use crate::settings::{BranchSettings, Settings};
use crate::text;
use git2::{BranchType, Commit, Error, Oid, Repository};
use std::collections::HashMap;

pub struct GitGraph {
    pub repository: Repository,
    pub commits: Vec<CommitInfo>,
    pub branches: Vec<BranchInfo>,
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

        let branches = assign_branches(&repository, &mut commits, indices, &settings.branches)?;
        let graph = GitGraph {
            repository,
            commits,
            branches,
        };

        Ok(graph)
    }

    pub fn commit(&self, id: Oid) -> Result<Commit, Error> {
        self.repository.find_commit(id)
    }
}

pub struct CommitInfo {
    pub oid: Oid,
    pub branches: Vec<usize>,
    pub branch_trace: Option<usize>,
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

fn assign_branches(
    repository: &Repository,
    commits: &mut Vec<CommitInfo>,
    indices: HashMap<Oid, usize>,
    settings: &BranchSettings,
) -> Result<Vec<BranchInfo>, Error> {
    let branches_ordered = extract_branches(repository, commits, settings)?;

    for (branch_idx, branch) in branches_ordered.iter().enumerate() {
        if let Some(&idx) = indices.get(&branch.target) {
            let trace_oid = {
                let info = &mut commits[idx];
                if !info.branches.contains(&branch_idx) {
                    info.branches.push(branch_idx);
                    Some(info.oid)
                } else {
                    None
                }
            };
            if let Some(oid) = trace_oid {
                trace_branch(repository, commits, &indices, oid, branch_idx)?;
            }
        }
    }

    Ok(branches_ordered)
}

fn trace_branch<'repo>(
    repository: &'repo Repository,
    commits: &mut Vec<CommitInfo>,
    indices: &HashMap<Oid, usize>,
    oid: Oid,
    branch: usize,
) -> Result<(), Error> {
    let mut curr_oid = oid;
    loop {
        let index = indices[&curr_oid];
        let info = &mut commits[index];
        if info.branch_trace.is_some() {
            break;
        } else {
            info.branch_trace = Some(branch);
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
    settings: &BranchSettings,
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

    valid_branches.sort_by_cached_key(|branch| branch_persistence(&branch.name, settings));

    Ok(valid_branches)
}

fn branch_persistence(name: &str, settings: &BranchSettings) -> usize {
    settings
        .persistence
        .iter()
        .position(|b| name.starts_with(b))
        .unwrap_or(settings.persistence.len())
}
