//! Command line tool to show clear git graphs arranged for your branching model.

use git2::Repository;
use std::path::Path;

pub mod config;
pub mod graph;
pub mod print;
pub mod settings;

pub fn get_repo<P: AsRef<Path>>(path: P, skip_repo_owner_validation: bool) -> Result<Repository, git2::Error> {
    if(skip_repo_owner_validation) {
        unsafe { git2::opts::set_verify_owner_validation(false)? }
    }
    Repository::discover(path)
}
