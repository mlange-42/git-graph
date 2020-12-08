# git-graph

A command line tool to visualize Git history graphs in a comprehensible way, following different branching models.

The aim is a structured graph of the branches. An example using the [GitFlow model](https://nvie.com/posts/a-successful-git-branching-model/) looks like this:

```
,-------- master
| ,------ release/...
| | ,---- develop
| | | ,-- feature/...
| | | |

    *    dd90e4f (HEAD -> develop) Merge branch 'release/0.1.1' into develop
*  /|    f26955e (master) Merge branch 'release/0.1.1'
|\| |
| * |    02dc52c increment version number
|  \|
|   *    e7d3f60 Merge branch 'feature/feature-for-next-release' into develop
|   |\
|   | *  ac6fefe and even more work on the feature
|   | *  cc1b000 more work on the feature
|   | *  75ab692 work on the first feature for v0.1.1
|   |/
|   *    d3611c3 Merge branch 'release/0.1.0' into develop
*  /|    3c84307 Merge branch 'release/0.1.0'
|\| |
| * |    63dce4b set version number
|  \|
|   *    2f2d8df Merge branch 'feature/second-feature' into develop
|   |\
|   | *  7e82d8c second commit on second feature
|   | *  70adae1 second feature, first commit
|   |/
|   *    c1a5e2d Merge branch 'feature/first-feature' into develop
|   |\
|   | *  9b33812 a second edit for the first feature
|   | *  5b31453 a first edit forthe first feature
|   |/
|   *    1867d53 initialize develop branch
|  /
| /
|/
*        487c782 initial project setup
```

While this is the same graph as shown by `git` 

```
*   dd90e4f (HEAD -> develop) Merge branch 'release/0.1.1' into develop
|\
| | *   f26955e (master) Merge branch 'release/0.1.1'
| | |\
| | |/
| |/|
| * | 02dc52c increment version number
|/ /
* |   e7d3f60 Merge branch 'feature/feature-for-next-release' into develop
|\ \
| * | ac6fefe and even more work on the feature
| * | cc1b000 more work on the feature
| * | 75ab692 work on the first feature for v0.1.1
|/ /
* |   d3611c3 Merge branch 'release/0.1.0' into develop
|\ \
| | *   3c84307 Merge branch 'release/0.1.0'
| | |\
| | |/
| |/|
| * | 63dce4b set version number
|/ /
* |   2f2d8df Merge branch 'feature/second-feature' into develop
|\ \
| * | 7e82d8c second commit on second feature
| * | 70adae1 second feature, first commit
|/ /
* |   c1a5e2d Merge branch 'feature/first-feature' into develop
|\ \
| * | 9b33812 a second edit for the first feature
| * | 5b31453 a first edit forthe first feature
|/ /
* / 1867d53 initialize develop branch
|/
* 487c782 initial project setup
```
