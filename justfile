# run with Pop OS env vars for Nvidia GPU
run:
    __GLX_VENDOR_LIBRARY_NAME=nvidia __NV_PRIME_RENDER_OFFLOAD=1 cargo run
