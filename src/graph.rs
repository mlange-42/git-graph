use crate::settings::Settings;
use git2::{Branch, BranchType, Commit, Error, Oid, Repository};
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
        walk.push_head()?;

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
        let branches_ordered = branches_persistence_order(&self.repository, settings)?;

        for branch in branches_ordered {
            let reference = branch.get();
            if let Some(name) = reference.name() {
                if let Some(oid) = reference.target() {
                    let idx = indices[&oid];
                    self.commits[idx].branches.push(name[11..].to_string());
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
    pub branch_traces: Vec<String>,
}

impl CommitInfo {
    fn new(commit: &Commit) -> Self {
        CommitInfo {
            oid: commit.id(),
            branches: Vec::new(),
            branch_traces: Vec::new(),
        }
    }
}

fn branches_persistence_order<'repo>(
    repository: &'repo Repository,
    settings: &Settings,
) -> Result<Vec<Branch<'repo>>, Error> {
    let mut branches = repository
        .branches(Some(BranchType::Local))?
        .map(|bt| bt.map(|bt| bt.0))
        .collect::<Result<Vec<_>, Error>>()?;

    branches.sort_by_cached_key(|branch| {
        settings
            .branch_persistance
            .iter()
            .position(|b| branch.get().name().unwrap_or("refs/heads/-")[11..].starts_with(b))
            .unwrap_or(settings.branch_persistance.len())
    });

    Ok(branches)
}
