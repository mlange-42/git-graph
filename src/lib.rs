//! git-graph shows clear git graphs arranged for your branching model.
//!
//! It provides both a library and a command line tool.
//!
//! The main steps are:
//! 1. Read branching model configuration (See [config] and [settings])
//! 2. Lay out the graph structure according to the branching model (See [graph])
//! 3. Render the layout to text or SVG (See [mod@print])

/* TODO git-graph has some complex functions, which make it hard to
understand and modify the code. The code should be made simpler
so the following warnings can be enabled without triggering.

// Configure clippy to look for complex functions
#![warn(clippy::cognitive_complexity)]
#![warn(clippy::too_many_lines)]
*/

use git2::Repository;
use std::path::Path;

pub mod config;
pub mod graph;
pub mod print;
pub mod settings;

pub fn get_repo<P: AsRef<Path>>(
    path: P,
    skip_repo_owner_validation: bool,
) -> Result<Repository, git2::Error> {
    if skip_repo_owner_validation {
        unsafe { git2::opts::set_verify_owner_validation(false)? }
    }
    Repository::discover(path)
}
