check:
    bacon check-all

run:
    cargo run

# NOTE: this depends on an installed glslc from the vulkan sdk
# compile shaders to spv
[unix]
shaders:
    cargo run --bin shaders
