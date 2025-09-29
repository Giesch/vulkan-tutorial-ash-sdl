use std::ffi::CString;

use descriptor_set_reflection::ReflectedPipelineLayout;
use shader_slang as slang;
use shader_slang::Downcast;

use crate::util::*;

mod descriptor_set_reflection;

/// whether to use column-major or row-major matricies with slang
pub const COLUMN_MAJOR: bool = true;

pub fn load_precompiled_shaders() -> Result<CompiledShaderModule, BoxError> {
    // TODO glob for all .slang files
    let source_file_name = "depth_texture.slang";

    let reflection_json_file_name = source_file_name.replace(".slang", ".json");
    let json_path = manifest_path(["shaders", "compiled", &reflection_json_file_name]);
    let json = std::fs::read_to_string(&json_path).unwrap();
    let _reflected_pipeline_layout: ReflectedPipelineLayout = serde_json::from_str(&json)?;

    let spv_frag_file_name = source_file_name.replace(".slang", ".frag.spv");
    let frag_path = manifest_path(["shaders", "compiled", &spv_frag_file_name]);
    let _frag_shader_bytecode = std::fs::read(&frag_path).unwrap();

    let spv_vert_file_name = source_file_name.replace(".slang", ".vert.spv");
    let vert_path = manifest_path(["shaders", "compiled", &spv_vert_file_name]);
    let _vert_shader_bytecode = std::fs::read(&vert_path).unwrap();

    // TODO entry point names

    todo!()
}

pub fn write_precompiled_shaders() -> Result<(), BoxError> {
    let PreparedShader {
        source_file_name,
        vertex_shader,
        fragment_shader,
        reflected_pipeline_layout,
    } = prepare_compiled_shaders()?;

    let compiled_shaders_dir = manifest_path(["shaders", "compiled"]);
    std::fs::create_dir_all(&compiled_shaders_dir)?;

    let reflection_json = serde_json::to_string_pretty(&reflected_pipeline_layout)?;
    let reflection_json_file_name = source_file_name.replace(".slang", ".json");
    let json_path = manifest_path(["shaders", "compiled", &reflection_json_file_name]);
    std::fs::write(json_path, reflection_json)?;

    let spv_vert_file_name = source_file_name.replace(".slang", ".vert.spv");
    let vert_path = manifest_path(["shaders", "compiled", &spv_vert_file_name]);
    std::fs::write(vert_path, &vertex_shader.shader_bytecode.as_slice())?;

    let spv_frag_file_name = source_file_name.replace(".slang", ".frag.spv");
    let frag_path = manifest_path(["shaders", "compiled", &spv_frag_file_name]);
    std::fs::write(frag_path, &fragment_shader.shader_bytecode.as_slice())?;

    Ok(())
}

struct PreparedShader {
    source_file_name: String,
    vertex_shader: CompiledShader,
    fragment_shader: CompiledShader,
    reflected_pipeline_layout: ReflectedPipelineLayout,
}

fn prepare_compiled_shaders() -> Result<PreparedShader, BoxError> {
    let global_session = slang::GlobalSession::new().unwrap();
    let search_path = CString::new("shaders/source")?;

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
    // TODO glob for all .slang files
    let source_file_name = "depth_texture.slang";
    let module = session.load_module(source_file_name)?;

    // the examples have 1 vert and 1 frag shader
    debug_assert!(module.entry_points().len() == 2);

    let mut components = vec![module.downcast().clone()];
    let mut vertex_shader: Option<CompiledShader> = None;
    let mut fragment_shader: Option<CompiledShader> = None;
    for entry_point in module.entry_points() {
        let compiled_shader = compile_shader(&entry_point, &session, &module)?;

        if compiled_shader.stage == slang::Stage::Vertex {
            vertex_shader = Some(compiled_shader)
        } else if compiled_shader.stage == slang::Stage::Fragment {
            fragment_shader = Some(compiled_shader)
        }

        components.push(entry_point.downcast().clone());
    }
    let vertex_shader: CompiledShader = vertex_shader.expect(&format!(
        "failed to load vertex entry point for: {source_file_name}"
    ));
    let fragment_shader: CompiledShader = fragment_shader.expect(&format!(
        "failed to load fragment entry point for: {source_file_name}"
    ));

    let program = session.create_composite_component_type(&components)?;
    let linked_program = program.link()?;
    let program_layout = linked_program.layout(0)?;
    let reflected_pipeline_layout =
        descriptor_set_reflection::reflect_pipeline_layout(program_layout);

    let prepared_shaders = PreparedShader {
        source_file_name: source_file_name.to_string(),
        vertex_shader,
        fragment_shader,
        reflected_pipeline_layout,
    };

    Ok(prepared_shaders)
}

pub fn dev_compile_slang_shaders(device: ash::Device) -> Result<CompiledShaderModule, BoxError> {
    let PreparedShader {
        source_file_name,
        vertex_shader: vert,
        fragment_shader: frag,
        reflected_pipeline_layout,
    } = prepare_compiled_shaders()?;

    let (vk_pipeline_layout, vk_descriptor_set_layouts) =
        unsafe { reflected_pipeline_layout.vk_create(&device)? };

    Ok(CompiledShaderModule {
        source_file_name: source_file_name.into(),
        vertex_shader: vert,
        fragment_shader: frag,
        vk_pipeline_layout,
        vk_descriptor_set_layouts,
    })
}

pub struct CompiledShaderModule {
    pub source_file_name: String,
    pub vertex_shader: CompiledShader,
    pub fragment_shader: CompiledShader,

    // NOTE the renderer is expected to free these fields correctly
    pub vk_pipeline_layout: ash::vk::PipelineLayout,
    pub vk_descriptor_set_layouts: Vec<ash::vk::DescriptorSetLayout>,
}

impl std::fmt::Debug for CompiledShaderModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledShaderModule")
            .field("source_file_name", &self.source_file_name)
            .field("vertex_shader", &self.vertex_shader)
            .field("fragment_shader", &self.fragment_shader)
            .field("vk_pipeline_layout", &self.vk_pipeline_layout)
            .field("vk_descriptor_set_layouts", &self.vk_descriptor_set_layouts)
            .finish()
    }
}

// TODO add reflection metadata needed by vulkan
// need to generate the Vertex struct and its methods or const values
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
) -> Result<CompiledShader, BoxError> {
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
