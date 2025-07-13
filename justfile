set dotenv-load := true

check:
    bacon check-all

run:
    cargo run

# NOTE: this depends on an installed glslc from the vulkan sdk
# compile glsl shaders to spv
[unix]
shaders:
    cargo run --bin shaders

slang_version := "2025.12.1"

# download Slang
[linux]
setup:
    mkdir -p vendor/slang
    wget -O vendor/slang.tar.gz https://github.com/shader-slang/slang/releases/download/v{{slang_version}}/slang-{{slang_version}}-linux-x86_64.tar.gz
    tar xzf vendor/slang.tar.gz --directory=vendor/slang
    rm vendor/slang.tar.gz
