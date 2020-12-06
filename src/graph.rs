use git2::{Commit, Error, Oid, Repository};

#[allow(dead_code)]
pub struct GitGraph {
    pub repository: Repository,
    pub commits: Vec<CommitInfo>,
}

impl GitGraph {
    pub fn new(path: &str) -> Result<Self, Error> {
        let repository = Repository::open(path)?;
        let mut revwalk = repository.revwalk()?;

        revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repository.find_commit(oid).unwrap();
            commits.push(CommitInfo::new(&commit));
        }

        Ok(GitGraph {
            repository,
            commits,
        })
    }
    pub fn commit(&self, id: Oid) -> Result<Commit, Error> {
        self.repository.find_commit(id)
    }
}

#[allow(dead_code)]
pub struct CommitInfo {
    pub oid: Oid,
}

impl CommitInfo {
    fn new(commit: &Commit) -> Self {
        CommitInfo { oid: commit.id() }
    }
}
