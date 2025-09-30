set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# NOTE this does nothing on windows
set dotenv-load := true


# compiler watch
check:
    bacon check-all


# run dev build with shader hot reload
[unix]
run:
    cargo run

# run with shader printf and vk validation layers at 'info'
[unix]
shader-debug:
    RUST_LOG=info VK_LAYER_PRINTF_ONLY_PRESET=1 cargo run

# run dev build with shader hot reload
[windows]
run:
    $Env:SLANG_DIR = "$PWD\vendor\slang"; cargo run


# run a release build
[unix]
run-release: prepare-shaders
    cargo run --release

# run a release build
[windows]
run-release:
    $Env:SLANG_DIR = "$PWD\vendor\slang"; cargo run --release


slang_version := "2025.17.2"

# download Slang shader compiler
[linux]
setup:
    rm -rf vendor/slang
    mkdir -p vendor/slang
    wget -O vendor/slang.tar.gz "https://github.com/shader-slang/slang/releases/download/v{{slang_version}}/slang-{{slang_version}}-linux-x86_64.tar.gz"
    tar xzf vendor/slang.tar.gz --directory=vendor/slang
    rm vendor/slang.tar.gz

# download Slang shader compiler
[windows]
setup:
    if (Test-Path -Path vendor\slang) { Remove-Item vendor\slang -Recurse }
    mkdir vendor\slang > $null
    Invoke-WebRequest -OutFile vendor\slang.zip -Uri "https://github.com/shader-slang/slang/releases/download/v{{slang_version}}/slang-{{slang_version}}-windows-x86_64.zip"
    Expand-Archive -Path vendor\slang.zip -DestinationPath vendor\slang
    Remove-Item vendor\slang.zip


# write precompiled shaders & metadata to disk
prepare-shaders:
    cargo run --bin prepare_shaders
