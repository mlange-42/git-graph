
# Overview

To generate a graph, [GitGraph::new()] will read the repository
and assign every commit to a single branch.

It takes the following steps to generate the graph

- Identify branches
- Sort branches by persistence
- Trace branches to commits
- Filtering and indexing

## Identify branches
Local and remote git-branches and tags are used as candidates for branches.
A branch can be identified by a merge commit, even though no current git-branch
refers to it.

## Sort branches by persistence
Each branch is assigned a persistence which can be configured by settings.
Think of persistence as z-order where lower values take preceedence.
**TODO** Merge branch

## Trace branches to commits
The branches now get to pick their commits, in order of persistence. Each
branch starts with a head, and follow the primary parent while it is
available. It stops when the parent is a commit already assigned to a branch.
**TODO** Duplicate branch names
**TODO** Handle visual artifacts on merge

## Filtering and indexing
Commits that have not been assigned a branch is filtered out.
An *index_map* is created to map from original commit index, to filtered
commit index.
**TODO** what? why? Would it not be better to track from child/heads instead of every single commit in repo?




# Branch sorting
The goal of this algorithm is to assign a column number to each tracked branch so that they can be visualized linearly without overlapping in the graph. It uses a shortest-first scheduling strategy (optionally longest-first and with forward/backward start sorting).

## Initialization
- occupied: A vector of vectors of vectors of tuples. 
The outer vector is indexed by the branch's order_group (determined by branch_order based on the settings.branches.order). 
Each inner vector represents a column within that order group, 
and the tuples (start, end) store the range of commits occupied by a branch in that column. 

## Preparing Branches for Sorting
- It creates branches_sort, a vector of tuples containing the branch index, its start commit index (range.0), its end commit index (range.1), its source order group, and its target order group. 
- It filters out branches that don't have a defined range (meaning they weren't associated with any commits). 
## Sorting Branches
- The branches_sort vector is sorted based on a key that prioritizes: 
    1. The maximum of the source and target order groups. This likely aims to keep related branches (e.g., those involved in merges) closer together. 
    2. The length of the branch's lifespan (end - start commit index), either shortest-first or longest-first based on the shortest_first setting. 
    3. The starting commit index, either forward or backward based on the forward setting. 
