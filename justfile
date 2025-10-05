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
[unix]
dev:
    cargo run
alias d := dev

# run dev build with shader hot reload
[windows]
dev:
    $Env:SLANG_DIR = "$PWD\vendor\slang"; cargo run


# run with shader printf and vk validation layers at 'info'
[unix]
shader-debug:
    RUST_LOG=info VK_LAYER_PRINTF_ONLY_PRESET=1 cargo run


# run a release build
[unix]
release: shaders
    cargo run --release
alias r := release

# run a release build
[windows]
release:
    $Env:SLANG_DIR = "$PWD\vendor\slang"; cargo run --release


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


slang_version := "2025.17.2"

# download the Slang shader compiler
[linux]
setup:
    rm -rf vendor/slang
    mkdir -p vendor/slang
    wget -O vendor/slang.tar.gz "https://github.com/shader-slang/slang/releases/download/v{{slang_version}}/slang-{{slang_version}}-linux-x86_64.tar.gz"
    tar xzf vendor/slang.tar.gz --directory=vendor/slang
    rm vendor/slang.tar.gz

# download the Slang shader compiler
[windows]
setup:
    if (Test-Path -Path vendor\slang) { Remove-Item vendor\slang -Recurse }
    New-Item -Path vendor\slang -ItemType Directory | Out-Null
    $ProgressPreference='SilentlyContinue'; Invoke-WebRequest -OutFile vendor\slang.zip -Uri "https://github.com/shader-slang/slang/releases/download/v{{slang_version}}/slang-{{slang_version}}-windows-x86_64.zip"
    Expand-Archive -Path vendor\slang.zip -DestinationPath vendor\slang
    Remove-Item vendor\slang.zip


