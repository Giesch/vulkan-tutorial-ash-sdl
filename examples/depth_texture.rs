use ash_sdl_vulkan_tutorial::*;

fn main() -> Result<(), anyhow::Error> {
    run_game(DepthTexture::init())
}
