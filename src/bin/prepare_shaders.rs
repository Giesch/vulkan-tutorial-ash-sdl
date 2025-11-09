use ash_sdl_vulkan_tutorial::build_tasks::{self, Config};
use ash_sdl_vulkan_tutorial::util::manifest_path;

pub fn main() {
    let arg = std::env::var("GENERATE_RUST_SOURCE").ok();

    let generate_rust_source = match arg {
        None => false,
        Some(s) if s.is_empty() => false,
        Some(s) if s.to_lowercase() == "false" => false,
        _ => true,
    };

    let config = Config {
        generate_rust_source,
        rust_source_dir: manifest_path(["src"]),
        shaders_source_dir: manifest_path(["shaders", "source"]),
        compiled_shaders_dir: manifest_path(["shaders", "compiled"]),
    };

    build_tasks::write_precompiled_shaders(config).unwrap();
}
