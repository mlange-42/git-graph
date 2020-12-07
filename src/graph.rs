use git2::{BranchType, Commit, Error, Oid, Repository};
use std::collections::HashMap;

#[allow(dead_code)]
pub struct GitGraph {
    pub repository: Repository,
    pub commits: Vec<CommitInfo>,
    pub indices: HashMap<Oid, usize>,
}

impl GitGraph {
    pub fn new(path: &str) -> Result<Self, Error> {
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
            indices,
        };
        graph.assign_branches()?;

        Ok(graph)
    }
    fn assign_branches(&mut self) -> Result<(), Error> {
        let branches = self.repository.branches(None)?;
        for branch in branches {
            let (branch, branch_type) = branch?;
            if branch_type == BranchType::Local {
                let reference = branch.get();
                if let Some(oid) = reference.target() {
                    let idx = self.indices[&oid];
                    if let Some(name) = reference.name() {
                        self.commits[idx].branches.push(name[11..].to_string());
                    }
                }
            }
        }

        Ok(())
    }
    pub fn commit(&self, id: Oid) -> Result<Commit, Error> {
        self.repository.find_commit(id)
    }
}

#[allow(dead_code)]
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
