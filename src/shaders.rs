use std::ffi::CString;

use atlas::DepthTextureShader;
use shader_slang as slang;
use shader_slang::Downcast;

use crate::util::*;

pub mod atlas;
mod descriptor_set_reflection;
pub mod json;

use json::*;

/// whether to use column-major or row-major matricies with slang
pub const COLUMN_MAJOR: bool = true;

pub fn write_precompiled_shaders() -> Result<(), anyhow::Error> {
    let shaders_source_dir = manifest_path(["shaders", "source"]);
    let slang_file_names: Vec<_> = std::fs::read_dir(shaders_source_dir)?
        .filter_map(|entry_res| entry_res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter(|path| path.extension().map_or(false, |ext| ext == "slang"))
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

pub struct ReflectedShader {
    pub vertex_shader: CompiledShader,
    pub fragment_shader: CompiledShader,
    pub reflection_json: ReflectionJson,
}

fn prepare_reflected_shader(source_file_name: &str) -> Result<ReflectedShader, anyhow::Error> {
    let global_session = slang::GlobalSession::new().unwrap();
    let search_path = CString::new("shaders/source").unwrap();

    let session_options = slang::CompilerOptions::default()
        .vulkan_use_entry_point_name(true)
        .language(slang::SourceLanguage::Slang)
        .optimization(slang::OptimizationLevel::High)
        .emit_spirv_directly(true);
    let session_options = if COLUMN_MAJOR {
        session_options.matrix_layout_column(true)
    } else {
        session_options.matrix_layout_row(true)
    };

    let target_desc = slang::TargetDesc::default()
        .format(slang::CompileTarget::Spirv)
        .profile(global_session.find_profile("glsl_450+spirv_1_6"));

    let targets = [target_desc];
    let search_paths = [search_path.as_ptr()];
    let session_desc = slang::SessionDesc::default()
        .targets(&targets)
        .search_paths(&search_paths)
        .options(&session_options);

    let session = global_session.create_session(&session_desc).unwrap();

    let module = session.load_module(source_file_name)?;

    // the examples have 1 vert and 1 frag shader
    debug_assert!(module.entry_points().len() == 2);

    let mut components = vec![module.downcast().clone()];
    let mut vertex_shader: Option<(String, CompiledShader)> = None;
    let mut fragment_shader: Option<(String, CompiledShader)> = None;
    for entry_point in module.entry_points() {
        let entry_point_name = entry_point.function_reflection().name().to_owned();
        let compiled_shader = compile_shader(&entry_point, &session, &module)?;

        if compiled_shader.stage == slang::Stage::Vertex {
            vertex_shader = Some((entry_point_name, compiled_shader))
        } else if compiled_shader.stage == slang::Stage::Fragment {
            fragment_shader = Some((entry_point_name, compiled_shader))
        }

        components.push(entry_point.downcast().clone());
    }
    let (vertex_entry_point, vertex_shader) = vertex_shader
        .unwrap_or_else(|| panic!("failed to load vertex entry point for: {source_file_name}"));
    let (fragment_entry_point, fragment_shader) = fragment_shader
        .unwrap_or_else(|| panic!("failed to load fragment entry point for: {source_file_name}"));

    let program = session.create_composite_component_type(&components)?;
    let linked_program = program.link()?;
    let program_layout = linked_program.layout(0)?;

    let reflected_pipeline_layout =
        descriptor_set_reflection::reflect_pipeline_layout(program_layout);
    let reflection_json = ReflectionJson {
        source_file_name: source_file_name.to_string(),
        vertex_entry_point,
        fragment_entry_point,
        pipeline_layout: reflected_pipeline_layout,
    };

    let reflected_shader = ReflectedShader {
        vertex_shader,
        fragment_shader,
        reflection_json,
    };

    Ok(reflected_shader)
}

pub fn dev_compile_slang_shaders(
    shader: &DepthTextureShader,
) -> Result<ReflectedShader, anyhow::Error> {
    prepare_reflected_shader(&shader.reflection_json.source_file_name)
}

pub struct CompiledShader {
    pub entry_point_name: CString,
    pub stage: slang::Stage,
    pub shader_bytecode: Vec<u8>,
}

impl CompiledShader {
    /// converts compiled spv to vulkan-readable u32s
    pub fn spv_bytes(&self) -> Result<Vec<u32>, std::io::Error> {
        let byte_reader = &mut std::io::Cursor::new(self.shader_bytecode.as_slice());
        ash::util::read_spv(byte_reader)
    }
}

impl std::fmt::Debug for CompiledShader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledShader")
            .field("entry_point_name", &self.entry_point_name)
            .field("stage", &self.stage)
            .finish()
    }
}

fn compile_shader(
    entry_point: &slang::EntryPoint,
    session: &slang::Session,
    module: &slang::Module,
) -> Result<CompiledShader, anyhow::Error> {
    let program = session.create_composite_component_type(&[
        module.downcast().clone(),
        entry_point.downcast().clone(),
    ])?;

    let linked_program = program.link()?;

    let program_layout = linked_program.layout(0)?;

    let mut refl_entry_points = program_layout.entry_points();
    assert!(refl_entry_points.len() == 1);
    let reflection_entry_point = refl_entry_points.next().unwrap();
    let stage = reflection_entry_point.stage();

    let shader_bytecode: slang::Blob = linked_program.entry_point_code(0, 0)?;
    let shader_bytecode = shader_bytecode.as_slice().to_vec();

    let entry_point_name = CString::new(reflection_entry_point.name())?;

    Ok(CompiledShader {
        entry_point_name,
        stage,
        shader_bytecode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_texture_reflection() {
        let shader = prepare_reflected_shader("depth_texture.slang").unwrap();
        insta::assert_json_snapshot!(shader.reflection_json);
    }
}
