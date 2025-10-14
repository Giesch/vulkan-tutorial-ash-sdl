set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# NOTE this does nothing on windows
set dotenv-load := true

# compiler/linter watch via bacon
check:
    bacon check-all
alias c := check


# list all available just recipes
list:
    @just --list --unsorted
alias l := list


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


# write precompiled shader bytecode & metadata to disk
shaders:
    cargo run --bin prepare_shaders
alias s := shaders


# run all unit tests
test: shaders
    INSTA_UPDATE=no cargo test
alias t := test

# run and review snapshot tests
insta: shaders
  cargo insta test --review
alias i := insta

