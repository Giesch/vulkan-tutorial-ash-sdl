use ash_sdl_vulkan_tutorial::build_tasks;

pub fn main() {
    let arg = std::env::var("GENERATE_RUST_SOURCE").ok();

    let generate_rust_source = match arg {
        None => false,
        Some(s) if s.is_empty() => false,
        Some(s) if s.to_lowercase() == "false" => false,
        _ => true,
    };

    build_tasks::write_precompiled_shaders(generate_rust_source).unwrap();
}
