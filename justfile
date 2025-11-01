set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# NOTE this does nothing on windows
set dotenv-load := true

# list all available just recipes
list:
    @ just --list --unsorted


# compiler/linter watch via bacon
check:
    bacon check-all
alias c := check


# run dev build with shader hot reload
dev:
    cargo run
alias d := dev


# run with shader printf and vk validation layers at 'info'
[unix]
shader-debug:
    RUST_LOG=info VK_LAYER_PRINTF_ONLY_PRESET=1 cargo run


# run a release build
release: shaders
    cargo run --release
alias r := release


# write precompiled shader bytecode & json metadata to disk
shaders:
    cargo run --bin prepare_shaders
    cargo fmt
alias s := shaders


# run all unit tests
test:
    INSTA_UPDATE=no cargo test
alias t := test

# run and review snapshot tests interactively
insta: shaders
  cargo insta test --review
alias i := insta


# lint in debug and release with warnings denied
lint:
    cargo clippy -- -D warnings
    cargo clippy --release -- -D warnings
alias l := lint


# set up git pre-commit hook
[unix]
setup:
    cp scripts/pre-commit.sh .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit

# lint and test for git pre-commit hook
pre-commit: shaders && lint test
    git add shaders/compiled
