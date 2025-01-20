_default:
    @just --choose

# Shows a list of all available recipes
help:
    @just --list

green := '\033[0;32m'
red := '\033[0;31m'
reset := '\033[0m'

# Checks if all requirements to work on this project are installed
check-requirements:
    @command -v cargo &>/dev/null && echo -e "{{ green }}✓{{ reset}} cargo installed" || echo -e "{{ red }}✖{{ reset }} cargo missing"

# Runs the same linters as the pipeline with fix option
lint:
    cargo fmt --all # Is in fix mode by default
    cargo clippy --all --all-targets --allow-dirty --fix

# Runs the same test as the pipeline but locally
test:
    cargo test --all

