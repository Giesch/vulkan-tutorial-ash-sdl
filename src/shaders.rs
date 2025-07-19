use std::ffi::CString;

use slang::Downcast;

use crate::BoxError;

pub fn compile_slang_shaders() -> Result<CompiledShaderModule, BoxError> {
    let global_session = slang::GlobalSession::new().unwrap();
    let search_path = CString::new("shaders/source")?;

    let session_options = slang::CompilerOptions::default()
        .vulkan_use_entry_point_name(true)
        .language(slang::SourceLanguage::Slang)
        .optimization(slang::OptimizationLevel::High)
        .emit_spirv_directly(true)
        .matrix_layout_column(true);

    let target_desc = slang::TargetDesc::default()
        .format(slang::CompileTarget::Spirv)
        // TODO also need to specify glsl_450?
        .profile(global_session.find_profile("spirv_1_5"));

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

    let mut vert: Option<CompiledShader> = None;
    let mut frag: Option<CompiledShader> = None;
    for entry_point in module.entry_points() {
        let compiled_shader = compile_shader(&entry_point, &session, &module)?;

        if compiled_shader.stage == slang::Stage::Vertex {
            vert = Some(compiled_shader)
        } else if compiled_shader.stage == slang::Stage::Fragment {
            frag = Some(compiled_shader)
        }
    }

    match (vert, frag) {
        (Some(vertex_shader), Some(fragment_shader)) => Ok(CompiledShaderModule {
            source_file_name: source_file_name.into(),
            vertex_shader,
            fragment_shader,
        }),

        _ => {
            Err(format!("failed to load vert and frag entry points for: {source_file_name}").into())
        }
    }
}

#[derive(Debug)]
pub struct CompiledShaderModule {
    #[expect(unused)]
    pub source_file_name: String,
    pub vertex_shader: CompiledShader,
    pub fragment_shader: CompiledShader,
}

// TODO add reflection metadata needed by vulkan
// need to generate the Vertex struct and its methods or const values
pub struct CompiledShader {
    pub entry_point_name: CString,
    pub stage: slang::Stage,
    pub spv_bytes: Vec<u32>,
}

impl std::fmt::Debug for CompiledShader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledShader")
            .field("entry_point_name", &self.entry_point_name)
            .field("stage", &self.stage)
            .field("spv_bytes.len()", &self.spv_bytes.len())
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

    let reflection = linked_program.layout(0)?;

    // for param in reflection.parameters() {
    //     let binding = param.binding_index();
    //     let category = param.category();
    //     let type_layout = param.type_layout();
    //     let kind = type_layout.kind();
    //     dbg!(&binding, &category, &kind, type_layout.name());
    // }

    let mut refl_entry_points = reflection.entry_points();
    assert!(refl_entry_points.len() == 1);
    let reflection_entry_point = refl_entry_points.next().unwrap();
    let stage = reflection_entry_point.stage();

    let shader_bytecode: slang::Blob = linked_program.entry_point_code(0, 0)?;
    let byte_reader = &mut std::io::Cursor::new(shader_bytecode.as_slice());
    let spv_bytes = ash::util::read_spv(byte_reader)?;

    let entry_point_name = CString::new(reflection_entry_point.name())?;

    Ok(CompiledShader {
        entry_point_name,
        stage,
        spv_bytes,
    })
}
