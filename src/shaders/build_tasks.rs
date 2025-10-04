use crate::util::*;

use super::{prepare_reflected_shader, ReflectedShader};

pub fn write_precompiled_shaders() -> Result<(), anyhow::Error> {
    let shaders_source_dir = manifest_path(["shaders", "source"]);
    let slang_file_names: Vec<_> = std::fs::read_dir(shaders_source_dir)?
        .filter_map(|entry_res| entry_res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "slang"))
        .filter_map(|path| {
            path.file_name()
                .and_then(|os_str| os_str.to_str())
                .map(|s| s.to_string())
        })
        .collect();

    for slang_file_name in &slang_file_names {
        let ReflectedShader {
            vertex_shader,
            fragment_shader,
            reflection_json,
        } = prepare_reflected_shader(slang_file_name)?;

        let source_file_name = &reflection_json.source_file_name;

        let compiled_shaders_dir = manifest_path(["shaders", "compiled"]);
        std::fs::create_dir_all(&compiled_shaders_dir)?;

        let reflection_json = serde_json::to_string_pretty(&reflection_json)?;
        let reflection_json_file_name = source_file_name.replace(".slang", ".json");
        let json_path = manifest_path(["shaders", "compiled", &reflection_json_file_name]);
        std::fs::write(json_path, reflection_json)?;

        let spv_vert_file_name = source_file_name.replace(".slang", ".vert.spv");
        let vert_path = manifest_path(["shaders", "compiled", &spv_vert_file_name]);
        std::fs::write(vert_path, vertex_shader.shader_bytecode.as_slice())?;

        let spv_frag_file_name = source_file_name.replace(".slang", ".frag.spv");
        let frag_path = manifest_path(["shaders", "compiled", &spv_frag_file_name]);
        std::fs::write(frag_path, fragment_shader.shader_bytecode.as_slice())?;
    }

    Ok(())
}
