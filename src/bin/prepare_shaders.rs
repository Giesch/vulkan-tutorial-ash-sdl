use std::fs;
use std::process::Command;

const SHADERS_SOURCE_DIR: &str = "./shaders/source";
const SHADERS_COMPILED_DIR: &str = "./shaders/compiled";

pub fn main() {
    let shader_source_dir = fs::read_dir(SHADERS_SOURCE_DIR).unwrap();

    if !fs::exists(SHADERS_COMPILED_DIR).unwrap() {
        fs::create_dir(SHADERS_COMPILED_DIR).unwrap();
    }

    for entry in shader_source_dir {
        let entry = entry.unwrap();

        let in_path = entry.path().display().to_string();
        if !in_path.ends_with(".slang") {
            panic!("non-slang file in shaders source dir: {in_path}");
        }

        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        let out_file_name = file_name.replace("slang", "spv");
        let json_file_name = file_name.replace("slang", "json");
        let out_path = format!("{SHADERS_COMPILED_DIR}/{out_file_name}");
        let json_path = format!("{SHADERS_COMPILED_DIR}/{json_file_name}");

        // slangc hello-world.slang -target spirv -o hello-world.spv
        let output = Command::new("slangc")
            .arg(&in_path)
            .arg("-target")
            .arg("spirv")
            .arg("-o")
            .arg(&out_path)
            .arg("-reflection-json")
            .arg(&json_path)
            .output()
            .unwrap();

        if !output.status.success() {
            dbg!(&output);
            panic!("failed to compile shader");
        }
    }
}
