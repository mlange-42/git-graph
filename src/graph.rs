use crate::print::colors::to_terminal_color;
use crate::settings::{BranchOrder, BranchSettings, Settings};
use crate::text;
use git2::{BranchType, Commit, Error, Oid, Reference, Repository};
use itertools::Itertools;
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};

const ORIGIN: &str = "origin/";

/// Represents a git history graph.
pub struct GitGraph {
    pub repository: Repository,
    pub commits: Vec<CommitInfo>,
    pub indices: HashMap<Oid, usize>,
    pub branches: Vec<BranchInfo>,
    pub head: HeadInfo,
}

impl GitGraph {
    pub fn new(
        mut repository: Repository,
        settings: &Settings,
        max_count: Option<usize>,
    ) -> Result<Self, String> {
        let mut stashes = HashSet::new();
        repository
            .stash_foreach(|_, _, oid| {
                stashes.insert(*oid);
                true
            })
            .map_err(|err| err.message().to_string())?;

        let mut walk = repository
            .revwalk()
            .map_err(|err| err.message().to_string())?;

        walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
            .map_err(|err| err.message().to_string())?;

        walk.push_glob("*")
            .map_err(|err| err.message().to_string())?;

        let head = HeadInfo::new(&repository.head().map_err(|err| err.message().to_string())?)?;

        let mut commits = Vec::new();
        let mut indices = HashMap::new();
        let mut idx = 0;
        for oid in walk {
            if let Some(max) = max_count {
                if idx >= max {
                    break;
                }
            }
            let oid = oid.map_err(|err| err.message().to_string())?;

            if !stashes.contains(&oid) {
                let commit = repository.find_commit(oid).unwrap();

                commits.push(CommitInfo::new(&commit));
                indices.insert(oid, idx);
                idx += 1;
            }
        }
        assign_children(&mut commits, &indices);

        let mut branches = assign_branches(&repository, &mut commits, &indices, &settings)?;

        match settings.branch_order {
            BranchOrder::FirstComeFirstServed(forward) => {
                assign_branch_columns_fcfs(&commits, &mut branches, &settings.branches, forward)
            }
            BranchOrder::ShortestFirst(forward) => assign_branch_columns_branch_length(
                &commits,
                &indices,
                &mut branches,
                &settings.branches,
                true,
                forward,
            ),
            BranchOrder::LongestFirst(forward) => assign_branch_columns_branch_length(
                &commits,
                &indices,
                &mut branches,
                &settings.branches,
                false,
                forward,
            ),
        }

        let filtered_commits: Vec<CommitInfo> = commits
            .into_iter()
            .filter(|info| info.branch_trace.is_some())
            .collect();

        let filtered_indices: HashMap<Oid, usize> = filtered_commits
            .iter()
            .enumerate()
            .map(|(idx, info)| (info.oid, idx))
            .collect();

        let index_map: HashMap<usize, Option<&usize>> = indices
            .iter()
            .map(|(oid, index)| (*index, filtered_indices.get(oid)))
            .collect();

        for branch in branches.iter_mut() {
            if let Some(mut start_idx) = branch.range.0 {
                let mut idx0 = index_map[&start_idx];
                while idx0.is_none() {
                    start_idx += 1;
                    idx0 = index_map[&start_idx];
                }
                branch.range.0 = Some(*idx0.unwrap());
            }
            if let Some(mut end_idx) = branch.range.1 {
                let mut idx0 = index_map[&end_idx];
                while idx0.is_none() {
                    end_idx -= 1;
                    idx0 = index_map[&end_idx];
                }
                branch.range.1 = Some(*idx0.unwrap());
            }
        }

        Ok(GitGraph {
            repository,
            commits: filtered_commits,
            indices: filtered_indices,
            branches,
            head,
        })
    }

    pub fn commit(&self, id: Oid) -> Result<Commit, Error> {
        self.repository.find_commit(id)
    }
}

pub struct HeadInfo {
    pub oid: Oid,
    pub name: String,
    pub is_branch: bool,
}
impl HeadInfo {
    fn new(head: &Reference) -> Result<Self, String> {
        let name = head.name().ok_or_else(|| "No name for HEAD".to_string())?;
        let name = if name == "HEAD" {
            name.to_string()
        } else {
            name[11..].to_string()
        };

        let h = HeadInfo {
            oid: head.target().ok_or_else(|| "No id for HEAD".to_string())?,
            name,
            is_branch: head.is_branch(),
        };
        Ok(h)
    }
}

/// Represents a commit.
pub struct CommitInfo {
    pub oid: Oid,
    pub is_merge: bool,
    pub parents: [Option<Oid>; 2],
    pub children: Vec<Oid>,
    pub branches: Vec<usize>,
    pub tags: Vec<usize>,
    pub branch_trace: Option<usize>,
}

impl CommitInfo {
    fn new(commit: &Commit) -> Self {
        CommitInfo {
            oid: commit.id(),
            is_merge: commit.parent_count() > 1,
            parents: [commit.parent_id(0).ok(), commit.parent_id(1).ok()],
            children: Vec::new(),
            branches: Vec::new(),
            tags: Vec::new(),
            branch_trace: None,
        }
    }
}

/// Represents a branch (real or derived from merge summary).
pub struct BranchInfo {
    pub target: Oid,
    pub merge_target: Option<Oid>,
    pub name: String,
    pub persistence: u8,
    pub is_remote: bool,
    pub is_merged: bool,
    pub is_tag: bool,
    pub visual: BranchVis,
    pub range: (Option<usize>, Option<usize>),
}
impl BranchInfo {
    #[allow(clippy::too_many_arguments)]
    fn new(
        target: Oid,
        merge_target: Option<Oid>,
        name: String,
        persistence: u8,
        is_remote: bool,
        is_merged: bool,
        is_tag: bool,
        visual: BranchVis,
        end_index: Option<usize>,
    ) -> Self {
        BranchInfo {
            target,
            merge_target,
            name,
            persistence,
            is_remote,
            is_merged,
            is_tag,
            visual,
            range: (end_index, None),
        }
    }
}

/// Branch properties for visualization.
pub struct BranchVis {
    pub order_group: usize,
    pub term_color: u8,
    pub svg_color: String,
    pub column: Option<usize>,
}

impl BranchVis {
    fn new(order_group: usize, term_color: u8, svg_color: String) -> Self {
        BranchVis {
            order_group,
            term_color,
            svg_color,
            column: None,
        }
    }
}

/// Walks through the commits and adds each commit's Oid to the children of its parents.
fn assign_children(commits: &mut [CommitInfo], indices: &HashMap<Oid, usize>) {
    for idx in 0..commits.len() {
        let (oid, parents) = {
            let info = &commits[idx];
            (info.oid, info.parents)
        };
        for par_oid in &parents {
            if let Some(par_oid) = par_oid {
                if let Some(par_idx) = indices.get(par_oid) {
                    commits[*par_idx].children.push(oid);
                }
            }
        }
    }
}

/// Extracts branches from repository and merge summaries, assigns branches and branch traces to commits.
///
/// Algorithm:
/// * Find all actual branches (incl. target oid) and all extract branches from merge summaries (incl. parent oid)
/// * Sort all branches by persistence
/// * Iterating over all branches in persistence order, trace back over commit parents until a trace is already assigned
fn assign_branches(
    repository: &Repository,
    commits: &mut [CommitInfo],
    indices: &HashMap<Oid, usize>,
    settings: &Settings,
) -> Result<Vec<BranchInfo>, String> {
    let mut branch_idx = 0;

    let mut branches = extract_branches(repository, commits, &indices, settings)?;

    let mut index_map: Vec<_> = (0..branches.len())
        .map(|old_idx| {
            let (target, is_tag, is_merged) = {
                let branch = &branches[old_idx];
                (branch.target, branch.is_tag, branch.is_merged)
            };
            if let Some(&idx) = &indices.get(&target) {
                let info = &mut commits[idx];
                if is_tag {
                    info.tags.push(old_idx);
                } else if !is_merged {
                    info.branches.push(old_idx);
                }
                let oid = info.oid;
                let any_assigned =
                    trace_branch(repository, commits, &indices, &mut branches, oid, old_idx)
                        .unwrap_or(false);

                if any_assigned || !is_merged {
                    branch_idx += 1;
                    Some(branch_idx - 1)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let mut commit_count = vec![0; branches.len()];
    for info in commits.iter_mut() {
        if let Some(trace) = info.branch_trace {
            commit_count[trace] += 1;
        }
    }

    let mut count_skipped = 0;
    for (idx, branch) in branches.iter().enumerate() {
        if let Some(mapped) = index_map[idx] {
            if commit_count[idx] == 0 && branch.is_merged && !branch.is_tag {
                index_map[idx] = None;
                count_skipped += 1;
            } else {
                index_map[idx] = Some(mapped - count_skipped);
            }
        }
    }

    for info in commits.iter_mut() {
        if let Some(trace) = info.branch_trace {
            info.branch_trace = index_map[trace];
            for br in info.branches.iter_mut() {
                *br = index_map[*br].unwrap();
            }
            for tag in info.tags.iter_mut() {
                *tag = index_map[*tag].unwrap();
            }
        }
    }

    let branches: Vec<_> = branches
        .into_iter()
        .enumerate()
        .filter_map(|(arr_index, branch)| {
            if index_map[arr_index].is_some() {
                Some(branch)
            } else {
                None
            }
        })
        .collect();

    Ok(branches)
}

/// Extracts (real or derived from merge summary) and assigns basic properties.
fn extract_branches(
    repository: &Repository,
    commits: &[CommitInfo],
    indices: &HashMap<Oid, usize>,
    settings: &Settings,
) -> Result<Vec<BranchInfo>, String> {
    let filter = if settings.include_remote {
        None
    } else {
        Some(BranchType::Local)
    };
    let actual_branches = repository
        .branches(filter)
        .map_err(|err| err.message().to_string())?
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|err| err.message().to_string())?;

    let mut counter = 0;

    let mut valid_branches = actual_branches
        .iter()
        .filter_map(|(br, tp)| {
            br.get().name().and_then(|n| {
                br.get().target().map(|t| {
                    counter += 1;
                    let start_index = match tp {
                        BranchType::Local => 11,
                        BranchType::Remote => 13,
                    };
                    let name = &n[start_index..];
                    let end_index = indices.get(&t).cloned();

                    let term_color = match to_terminal_color(
                        &branch_color(
                            name,
                            &settings.branches.terminal_colors[..],
                            &settings.branches.terminal_colors_unknown,
                            counter,
                        )[..],
                    ) {
                        Ok(col) => col,
                        Err(err) => return Err(err),
                    };

                    Ok(BranchInfo::new(
                        t,
                        None,
                        name.to_string(),
                        branch_order(name, &settings.branches.persistence) as u8,
                        &BranchType::Remote == tp,
                        false,
                        false,
                        BranchVis::new(
                            branch_order(name, &settings.branches.order),
                            term_color,
                            branch_color(
                                name,
                                &settings.branches.svg_colors,
                                &settings.branches.svg_colors_unknown,
                                counter,
                            ),
                        ),
                        end_index,
                    ))
                })
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    for (idx, info) in commits.iter().enumerate() {
        let commit = repository
            .find_commit(info.oid)
            .map_err(|err| err.message().to_string())?;
        if info.is_merge {
            if let Some(summary) = commit.summary() {
                counter += 1;

                let parent_oid = commit
                    .parent_id(1)
                    .map_err(|err| err.message().to_string())?;

                let branch_name = text::parse_merge_summary(summary, &settings.merge_patterns)
                    .unwrap_or_else(|| "unknown".to_string());
                let persistence = branch_order(&branch_name, &settings.branches.persistence) as u8;

                let pos = branch_order(&branch_name, &settings.branches.order);

                let term_col = to_terminal_color(
                    &branch_color(
                        &branch_name,
                        &settings.branches.terminal_colors[..],
                        &settings.branches.terminal_colors_unknown,
                        counter,
                    )[..],
                )?;
                let svg_col = branch_color(
                    &branch_name,
                    &settings.branches.svg_colors,
                    &settings.branches.svg_colors_unknown,
                    counter,
                );

                let branch_info = BranchInfo::new(
                    parent_oid,
                    Some(info.oid),
                    branch_name,
                    persistence,
                    false,
                    true,
                    false,
                    BranchVis::new(pos, term_col, svg_col),
                    Some(idx + 1),
                );
                valid_branches.push(branch_info);
            }
        }
    }

    valid_branches.sort_by_cached_key(|branch| (branch.persistence, !branch.is_merged));

    let mut tags = Vec::new();

    repository
        .tag_foreach(|oid, name| {
            tags.push((oid, name.to_vec()));
            true
        })
        .map_err(|err| err.message().to_string())?;

    for (oid, name) in tags {
        let name = std::str::from_utf8(&name[5..]).map_err(|err| err.to_string())?;

        if let Ok(tag) = repository.find_tag(oid) {
            if let Some(target_index) = indices.get(&tag.target_id()) {
                counter += 1;
                let term_col = to_terminal_color(
                    &branch_color(
                        &name,
                        &settings.branches.terminal_colors[..],
                        &settings.branches.terminal_colors_unknown,
                        counter,
                    )[..],
                )?;
                let pos = branch_order(&name, &settings.branches.order);
                let svg_col = branch_color(
                    &name,
                    &settings.branches.svg_colors,
                    &settings.branches.svg_colors_unknown,
                    counter,
                );
                let tag_info = BranchInfo::new(
                    tag.target_id(),
                    None,
                    name.to_string(),
                    settings.branches.persistence.len() as u8 + 1,
                    false,
                    false,
                    true,
                    BranchVis::new(pos, term_col, svg_col),
                    Some(*target_index),
                );
                valid_branches.push(tag_info);
            }
        }
    }

    Ok(valid_branches)
}

/// Traces back branches by following 1st commit parent,
/// until a commit is reached that already has a trace.
fn trace_branch<'repo>(
    repository: &'repo Repository,
    commits: &mut [CommitInfo],
    indices: &HashMap<Oid, usize>,
    branches: &mut [BranchInfo],
    oid: Oid,
    branch_index: usize,
) -> Result<bool, Error> {
    let mut curr_oid = oid;
    let mut prev_index: Option<usize> = None;
    let mut start_index: Option<i32> = None;
    let mut any_assigned = false;
    while let Some(index) = indices.get(&curr_oid) {
        let info = &mut commits[*index];
        if let Some(old_trace) = info.branch_trace {
            let (old_name, old_term, old_svg, old_range) = {
                let old_branch = &branches[old_trace];
                (
                    &old_branch.name.clone(),
                    old_branch.visual.term_color,
                    old_branch.visual.svg_color.clone(),
                    old_branch.range,
                )
            };
            let new_name = &branches[branch_index].name;
            let old_end = old_range.0.unwrap_or(0);
            let new_end = branches[branch_index].range.0.unwrap_or(0);
            if new_name == old_name && old_end >= new_end {
                let old_branch = &mut branches[old_trace];
                if let Some(old_end) = old_range.1 {
                    if index > &old_end {
                        old_branch.range = (None, None);
                    } else {
                        old_branch.range = (Some(*index), old_branch.range.1);
                    }
                } else {
                    old_branch.range = (Some(*index), old_branch.range.1);
                }
            } else {
                let branch = &mut branches[branch_index];
                if branch.name.starts_with(ORIGIN) && branch.name[7..] == old_name[..] {
                    branch.visual.term_color = old_term;
                    branch.visual.svg_color = old_svg;
                }
                match prev_index {
                    None => start_index = Some(*index as i32 - 1),
                    Some(prev_index) => {
                        // TODO: in cases where no crossings occur, the rule for merge commits can also be applied to normal commits
                        // see also print::get_deviate_index()
                        if commits[prev_index].is_merge {
                            let mut temp_index = prev_index;
                            for sibling_oid in &commits[*index].children {
                                if sibling_oid != &curr_oid {
                                    let sibling_index = indices[&sibling_oid];
                                    if sibling_index > temp_index {
                                        temp_index = sibling_index;
                                    }
                                }
                            }
                            start_index = Some(temp_index as i32);
                        } else {
                            start_index = Some(*index as i32 - 1);
                        }
                    }
                }
                break;
            }
        }

        info.branch_trace = Some(branch_index);
        any_assigned = true;

        let commit = repository.find_commit(curr_oid)?;
        match commit.parent_count() {
            0 => {
                start_index = Some(*index as i32);
                break;
            }
            _ => {
                prev_index = Some(*index);
                curr_oid = commit.parent_id(0)?;
            }
        }
    }

    let branch = &mut branches[branch_index];
    if let Some(end) = branch.range.0 {
        if let Some(start_index) = start_index {
            if start_index < end as i32 {
                // TODO: find a better solution (bool field?) to identify non-deleted branches that were not assigned to any commits, and thus should not occupy a column.
                branch.range = (None, None);
            } else {
                branch.range = (branch.range.0, Some(start_index as usize));
            }
        } else {
            branch.range = (branch.range.0, None);
        }
    } else {
        branch.range = (branch.range.0, start_index.map(|si| si as usize));
    }
    Ok(any_assigned)
}

/// Sorts branches into columns for visualization, that all branches can be
/// visualizes linearly and without overlaps. Uses First-Come First-Served scheduling.
fn assign_branch_columns_fcfs(
    commits: &[CommitInfo],
    branches: &mut [BranchInfo],
    settings: &BranchSettings,
    forward: bool,
) {
    let mut occupied: Vec<Vec<bool>> = vec![vec![]; settings.order.len() + 1];

    let sort_factor = if forward { 1 } else { -1 };

    let mut start_queue: VecDeque<_> = branches
        .iter()
        .enumerate()
        .filter(|br| br.1.range.0.is_some() || br.1.range.1.is_some())
        .map(|(idx, br)| (idx, br.range.0.unwrap_or(0)))
        .sorted_by_key(|tup| tup.1 as i32 * sort_factor)
        .collect();

    let mut end_queue: VecDeque<_> = branches
        .iter()
        .enumerate()
        .filter(|br| br.1.range.0.is_some() || br.1.range.1.is_some())
        .map(|(idx, br)| (idx, br.range.1.unwrap_or(branches.len() - 1)))
        .sorted_by_key(|tup| tup.1 as i32 * sort_factor)
        .collect();

    if !forward {
        std::mem::swap(&mut start_queue, &mut end_queue);
    }

    for i in 0..commits.len() {
        let idx = if forward { i } else { commits.len() - 1 - i };

        loop {
            let start = start_queue.pop_front();

            if let Some(start) = start {
                if start.1 == idx {
                    let branch = &mut branches[start.0];
                    let group = &mut occupied[branch.visual.order_group];
                    let column = group
                        .iter()
                        .find_position(|val| !**val)
                        .unwrap_or_else(|| (group.len(), &false))
                        .0;
                    branch.visual.column = Some(column);
                    if column < group.len() {
                        group[column] = true;
                    } else {
                        group.push(true);
                    }
                } else {
                    start_queue.push_front(start);
                    break;
                }
            } else {
                break;
            }
        }

        loop {
            let end = end_queue.pop_front();
            if let Some(end) = end {
                if end.1 == idx {
                    let branch = &mut branches[end.0];
                    let group = &mut occupied[branch.visual.order_group];
                    if let Some(column) = branch.visual.column {
                        group[column] = false;
                    }
                } else {
                    end_queue.push_front(end);
                    break;
                }
            } else {
                break;
            }
        }
    }

    let group_offset: Vec<usize> = occupied
        .iter()
        .scan(0, |acc, group| {
            *acc += group.len();
            Some(*acc)
        })
        .collect();

    for branch in branches {
        if let Some(column) = branch.visual.column {
            let offset = if branch.visual.order_group == 0 {
                0
            } else {
                group_offset[branch.visual.order_group - 1]
            };
            branch.visual.column = Some(column + offset);
        }
    }
}

/// Sorts branches into columns for visualization, that all branches can be
/// visualizes linearly and without overlaps. Uses Shortest-First scheduling.
fn assign_branch_columns_branch_length(
    commits: &[CommitInfo],
    indices: &HashMap<Oid, usize>,
    branches: &mut [BranchInfo],
    settings: &BranchSettings,
    shortest_first: bool,
    forward: bool,
) {
    let mut occupied: Vec<Vec<Vec<(usize, usize)>>> = vec![vec![]; settings.order.len() + 1];

    let length_sort_factor = if shortest_first { 1 } else { -1 };
    let start_sort_factor = if forward { 1 } else { -1 };

    let branches_sort: VecDeque<_> = branches
        .iter()
        .enumerate()
        .filter(|(_idx, br)| br.range.0.is_some() || br.range.1.is_some())
        .map(|(idx, br)| {
            (
                idx,
                br.range.0.unwrap_or(0),
                br.range.1.unwrap_or(branches.len() - 1),
            )
        })
        .sorted_by_key(|tup| {
            (
                (tup.2 as i32 - tup.1 as i32) * length_sort_factor,
                tup.1 as i32 * start_sort_factor,
            )
        })
        .collect();

    for (branch_idx, start, end) in branches_sort {
        let branch = &branches[branch_idx];
        let group = branch.visual.order_group;
        let group_occ = &mut occupied[group];

        let mut found = group_occ.len();
        for (i, column_occ) in group_occ.iter().enumerate() {
            let mut occ = false;
            for (s, e) in column_occ {
                if start <= *e && end >= *s {
                    occ = true;
                    break;
                }
            }
            if !occ {
                if let Some(merge_trace) = branch
                    .merge_target
                    .and_then(|t| indices.get(&t))
                    .and_then(|t_idx| commits[*t_idx].branch_trace)
                {
                    let merge_branch = &branches[merge_trace];
                    if merge_branch.visual.order_group == branch.visual.order_group {
                        if let Some(merge_column) = merge_branch.visual.column {
                            if merge_column == i {
                                occ = true;
                            }
                        }
                    }
                }
            }
            if !occ {
                found = i;
                break;
            }
        }
        let branch = &mut branches[branch_idx];
        branch.visual.column = Some(found);
        if found == group_occ.len() {
            group_occ.push(vec![]);
        }
        group_occ[found].push((start, end));
    }

    let group_offset: Vec<usize> = occupied
        .iter()
        .scan(0, |acc, group| {
            *acc += group.len();
            Some(*acc)
        })
        .collect();

    for branch in branches {
        if let Some(column) = branch.visual.column {
            let offset = if branch.visual.order_group == 0 {
                0
            } else {
                group_offset[branch.visual.order_group - 1]
            };
            branch.visual.column = Some(column + offset);
        }
    }
}

/// Finds the index for a branch name from a slice of prefixes
fn branch_order(name: &str, order: &[Regex]) -> usize {
    order
        .iter()
        .position(|b| b.is_match(name) || (name.starts_with(ORIGIN) && b.is_match(&name[7..])))
        .unwrap_or(order.len())
}

/// Finds the svg color for a branch name.
fn branch_color<T: Clone>(
    name: &str,
    order: &[(Regex, Vec<T>)],
    unknown: &[T],
    counter: usize,
) -> T {
    let color = order
        .iter()
        .find_position(|(b, _)| {
            b.is_match(name) || (name.starts_with(ORIGIN) && b.is_match(&name[7..]))
        })
        .map(|(_pos, col)| &col.1[counter % col.1.len()])
        .unwrap_or_else(|| &unknown[counter % unknown.len()]);
    color.clone()
}
