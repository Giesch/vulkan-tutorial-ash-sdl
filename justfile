# run an example with Pop OS config for GPU
example example:
    __GLX_VENDOR_LIBRARY_NAME=nvidia __NV_PRIME_RENDER_OFFLOAD=1 cargo run --example {{example}}
