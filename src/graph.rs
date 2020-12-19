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

        let branches = assign_branches(&repository, &mut commits, &indices, &settings.branches)?;
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
    pub target_index: Option<usize>,
    pub name: String,
    pub order_group: usize,
    pub deleted: bool,
    pub range: (Option<usize>, Option<usize>),
}
impl BranchInfo {
    fn new(
        target: Oid,
        target_index: Option<usize>,
        name: String,
        order_group: usize,
        deleted: bool,
        end_index: Option<usize>,
    ) -> Self {
        BranchInfo {
            target,
            target_index,
            name,
            order_group,
            deleted,
            range: (end_index, None),
        }
    }
}

/// Extract braches from repository and merge summaries, assigns branches and branch traces to commits.
///
/// Algorithm:
/// * Find all actual branches (incl. target oid) and all extract branches from merge summaries (incl. parent oid)
/// * Sort all branches by persistence
/// * Iterating over all branches in persistence order, trace back over commit parents until a trace is already assigned
fn assign_branches(
    repository: &Repository,
    commits: &mut Vec<CommitInfo>,
    indices: &HashMap<Oid, usize>,
    settings: &BranchSettings,
) -> Result<Vec<BranchInfo>, Error> {
    let mut branch_idx = 0;
    let branches_ordered = extract_branches(repository, commits, &indices, settings)?
        .into_iter()
        .filter_map(|mut branch| {
            if let Some(&idx) = &indices.get(&branch.target) {
                let info = &mut commits[idx];
                if !branch.deleted {
                    info.branches.push(branch_idx);
                }
                let oid = info.oid;
                let any_assigned =
                    trace_branch(repository, commits, &indices, oid, &mut branch, branch_idx)
                        .ok()?;

                if any_assigned || !branch.deleted {
                    branch_idx += 1;
                    Some(branch)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Ok(branches_ordered)
}

fn trace_branch<'repo>(
    repository: &'repo Repository,
    commits: &mut Vec<CommitInfo>,
    indices: &HashMap<Oid, usize>,
    oid: Oid,
    branch: &mut BranchInfo,
    branch_index: usize,
) -> Result<bool, Error> {
    let mut curr_oid = oid;
    let start_index;
    let mut any_assigned = false;
    loop {
        let index = indices[&curr_oid];
        let info = &mut commits[index];
        if info.branch_trace.is_some() {
            if index > 0 {
                start_index = index - 1;
            } else {
                start_index = index
            }
            break;
        } else {
            info.branch_trace = Some(branch_index);
            any_assigned = true;
        }
        let commit = repository.find_commit(curr_oid)?;
        match commit.parent_count() {
            0 => {
                start_index = index;
                break;
            }
            _ => {
                curr_oid = commit.parent_id(0)?;
            }
        }
    }
    branch.range = (branch.range.0, Some(start_index));
    Ok(any_assigned)
}

fn extract_branches(
    repository: &Repository,
    commits: &[CommitInfo],
    indices: &HashMap<Oid, usize>,
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
                br.get().target().map(|t| {
                    let name = &n[11..];
                    let end_index = indices.get(&t).cloned();
                    BranchInfo::new(
                        t,
                        indices.get(&t).cloned(),
                        name.to_string(),
                        branch_order(name, &settings.order),
                        false,
                        end_index,
                    )
                })
            })
        })
        .collect::<Vec<_>>();

    for (idx, info) in commits.iter().enumerate() {
        let commit = repository.find_commit(info.oid)?;
        if commit.parent_count() > 1 {
            if let Some(summary) = commit.summary() {
                let branches = text::parse_merge_summary(summary);
                if let Some(branch) = branches.1 {
                    let parent_oid = commit.parent_id(1)?;
                    let pos = branch_order(&branch, &settings.order);
                    let branch_info = BranchInfo::new(
                        parent_oid,
                        indices.get(&parent_oid).cloned(),
                        branch,
                        pos,
                        true,
                        Some(idx + 1),
                    );
                    valid_branches.push(branch_info);
                }
            }
        }
    }

    valid_branches.sort_by_cached_key(|branch| branch_order(&branch.name, &settings.persistence));

    Ok(valid_branches)
}

fn branch_order(name: &str, order: &[String]) -> usize {
    order
        .iter()
        .position(|b| name.starts_with(b))
        .unwrap_or(order.len())
}
