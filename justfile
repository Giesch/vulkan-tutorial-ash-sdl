set dotenv-load := true

check:
    bacon check-all

run:
    cargo run

# run with shader printf and vk validation layers at 'info'
shader-debug:
    RUST_LOG=info VK_LAYER_PRINTF_ONLY_PRESET=1 cargo run

run-release:
    cargo run --release

slang_version := "2025.12.1"

# download Slang shader compiler
[linux]
setup:
    mkdir -p vendor/slang
    wget -O vendor/slang.tar.gz https://github.com/shader-slang/slang/releases/download/v{{slang_version}}/slang-{{slang_version}}-linux-x86_64.tar.gz
    tar xzf vendor/slang.tar.gz --directory=vendor/slang
    rm vendor/slang.tar.gz
