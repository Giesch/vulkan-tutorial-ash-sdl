use std::process::Command;

const SHADERS_SOURCE_DIR: &'static str = "./shaders/source";
const SHADERS_COMPILED_DIR: &'static str = "./shaders/compiled";

/// Compiles each glsl source shader into spv
/// Depends on an installed glslc (from the vulkan sdk)
pub fn main() {
    let shader_source_dir = std::fs::read_dir(SHADERS_SOURCE_DIR).unwrap();

    if !std::fs::exists(SHADERS_COMPILED_DIR).unwrap() {
        std::fs::create_dir(SHADERS_COMPILED_DIR).unwrap();
    }

    for entry in shader_source_dir {
        let entry = entry.unwrap();

        let in_path = entry.path().display().to_string();

        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        let out_file_name = file_name.replace("glsl", "spv");
        let out_path = format!("{SHADERS_COMPILED_DIR}/{out_file_name}");

        let shader_stage = if file_name.contains(".vert") {
            "vert"
        } else if file_name.contains(".frag") {
            "frag"
        } else {
            panic!("unable to determine shader stage for {}", file_name);
        };

        let output = Command::new("glslc")
            .arg(format!("-fshader-stage={shader_stage}"))
            .arg(&in_path)
            .arg("-o")
            .arg(&out_path)
            .output()
            .unwrap();

        if !output.status.success() {
            dbg!(&output);
            panic!("failed to compile shader");
        }
    }
}
