//! Command line tool to show clear git graphs arranged for your branching model.

use git2::Repository;
use std::path::Path;

pub mod config;
pub mod graph;
pub mod print;
pub mod settings;

pub fn get_repo<P: AsRef<Path>>(path: P) -> Result<Repository, git2::Error> {
    Repository::discover(path)
}
