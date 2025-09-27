use std::ffi::CString;

use shader_slang as slang;
use shader_slang::Downcast;

use ash::vk::{self, Handle};

use crate::util::*;

/// whether to use column-major or row-major matricies with slang
pub const COLUMN_MAJOR: bool = true;

pub fn precompiled_shaders() -> Result<CompiledShaderModule, BoxError> {
    // TODO glob for all .slang/.spv files
    let spv_file_name = "depth_texture.spv";
    let path = manifest_path(["shaders", "compiled", spv_file_name]);

    let shader_bytecode = std::fs::read(&path).unwrap();
    let byte_reader = &mut std::io::Cursor::new(shader_bytecode.as_slice());
    let _spv_bytes = ash::util::read_spv(byte_reader)?;

    todo!()
}

pub fn compile_slang_shaders() -> Result<CompiledShaderModule, BoxError> {
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

        _ => Err(format!(
            "failed to load vertex and/or fragment entry points for: {source_file_name}"
        )
        .into()),
    }
}

#[derive(Debug)]
pub struct CompiledShaderModule {
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

    let program_layout = linked_program.layout(0)?;

    let mut refl_entry_points = program_layout.entry_points();
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

// slang reflection based vulkan builders
// https://docs.shader-slang.org/en/latest/parameter-blocks.html#using-parameter-blocks-with-reflection

pub struct PipelineLayoutBuilder {
    device: ash::Device,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    push_constant_ranges: Vec<vk::PushConstantRange>,
}

impl PipelineLayoutBuilder {
    pub fn new(device: ash::Device) -> Self {
        Self {
            device,
            descriptor_set_layouts: vec![],
            push_constant_ranges: vec![],
        }
    }

    pub fn add_push_constatant_range_for_constant_buffer(
        &mut self,
        constant_buffer_type_layout: &slang::reflection::TypeLayout,
    ) {
        let element_type_layout = constant_buffer_type_layout.element_type_layout();
        let element_size = element_type_layout.size(slang::ParameterCategory::Uniform);

        if element_size == 0 {
            return;
        }

        // NOTE this relies on the way the slang compiler
        // only ever uses one push constant range per entry point
        let offset = 0;

        let vk_push_constant_range = vk::PushConstantRange::default()
            // TODO use correct stage flags
            .stage_flags(vk::ShaderStageFlags::ALL)
            .offset(offset)
            .size(element_size as u32);

        self.push_constant_ranges.push(vk_push_constant_range);
    }

    fn add_sub_object_ranges(
        &mut self,
        type_layout: &slang::reflection::TypeLayout,
    ) -> Result<(), BoxError> {
        for sub_object_range_index in 0..type_layout.sub_object_range_count() {
            self.add_sub_object_range(type_layout, sub_object_range_index)?;
        }

        Ok(())
    }

    fn add_sub_object_range(
        &mut self,
        type_layout: &slang::reflection::TypeLayout,
        sub_object_range_index: i64,
    ) -> Result<(), BoxError> {
        let binding_range_index =
            type_layout.sub_object_range_binding_range_index(sub_object_range_index);
        let binding_type = type_layout.binding_range_type(binding_range_index);

        use slang::BindingType;

        match binding_type {
            BindingType::ParameterBlock => {
                let parameter_block_type_layout =
                    type_layout.binding_range_leaf_type_layout(binding_range_index);
                self.add_descriptor_set_for_parameter_block(parameter_block_type_layout)?;
            }

            BindingType::PushConstant => {
                let constant_buffer_type_layout =
                    type_layout.binding_range_leaf_type_layout(binding_range_index);
                self.add_push_constatant_range_for_constant_buffer(constant_buffer_type_layout);
            }

            // BindingType::Unknown => todo!(),
            // BindingType::Sampler => todo!(),
            // BindingType::Texture => todo!(),
            // BindingType::ConstantBuffer => todo!(),
            // BindingType::TypedBuffer => todo!(),
            // BindingType::RawBuffer => todo!(),
            // BindingType::CombinedTextureSampler => todo!(),
            // BindingType::InputRenderTarget => todo!(),
            // BindingType::InlineUniformData => todo!(),
            // BindingType::RayTracingAccelerationStructure => todo!(),
            // BindingType::VaryingInput => todo!(),
            // BindingType::VaryingOutput => todo!(),
            // BindingType::ExistentialValue => todo!(),
            // BindingType::MutableFlag => todo!(),
            // BindingType::MutableTeture => todo!(),
            // BindingType::MutableTypedBuffer => todo!(),
            // BindingType::MutableRawBuffer => todo!(),
            // BindingType::BaseMask => todo!(),
            // BindingType::ExtMask => todo!(),
            _ => {}
        }

        Ok(())
    }

    pub fn add_descriptor_set_for_parameter_block(
        &mut self,
        parameter_block_type_layout: &slang::reflection::TypeLayout,
    ) -> Result<(), BoxError> {
        let mut descriptor_set_layout_builder = DescriptorSetLayoutBuilder::default();
        descriptor_set_layout_builder.add_descriptor_ranges_for_parameter_block_element(
            parameter_block_type_layout.element_type_layout(),
            self,
        )?;

        descriptor_set_layout_builder.build_and_add(self)?;

        Ok(())
    }

    // aka 'finishBuilding' in the docs
    pub fn build(&mut self) -> Result<vk::PipelineLayout, BoxError> {
        // a null here represents an unused reserved slot for a
        // ParameterBlock that ended up only containing other ParameterBlocks
        // https://docs.shader-slang.org/en/latest/parameter-blocks.html#empty-parameter-blocks
        self.descriptor_set_layouts.retain(|l| !l.is_null());

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&self.descriptor_set_layouts)
            .push_constant_ranges(&self.push_constant_ranges);

        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_info, None)?
        };

        Ok(pipeline_layout)
    }
}

#[derive(Default)]
pub struct DescriptorSetLayoutBuilder<'a> {
    set_index: usize,
    binding_ranges: Vec<vk::DescriptorSetLayoutBinding<'a>>,
}

impl<'a> DescriptorSetLayoutBuilder<'a> {
    pub fn new(pipeline_layout_builder: &mut PipelineLayoutBuilder) -> Self {
        // reserve a layout slot to be filled in later
        // this preserves the correct index order for nested ParameterBlocks
        // https://docs.shader-slang.org/en/latest/parameter-blocks.html#ordering-of-nested-parameter-blocks

        let set_index = pipeline_layout_builder.descriptor_set_layouts.len();

        pipeline_layout_builder
            .descriptor_set_layouts
            .push(vk::DescriptorSetLayout::null());

        let binding_ranges = vec![];

        Self {
            set_index,
            binding_ranges,
        }
    }

    pub fn add_descriptor_ranges_for_parameter_block_element(
        &mut self,
        element_layout: &slang::reflection::TypeLayout,
        pipeline_layout_builder: &mut PipelineLayoutBuilder,
    ) -> Result<(), BoxError> {
        // in the cpp header there's a default argument overload for Uniform
        if element_layout.size(slang::ParameterCategory::Uniform) > 0 {
            self.add_automatically_introduced_uniform_buffer();
        }

        self.add_descriptor_ranges(element_layout);
        pipeline_layout_builder.add_sub_object_ranges(element_layout)?;

        Ok(())
    }

    fn add_automatically_introduced_uniform_buffer(&mut self) {
        // this relies on using no manual binding annotations
        let vk_binding_index = self.binding_ranges.len();

        let binding = vk::DescriptorSetLayoutBinding::default()
            .stage_flags(vk::ShaderStageFlags::ALL)
            .binding(vk_binding_index as u32)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER);

        self.binding_ranges.push(binding)
    }

    fn add_descriptor_ranges(&mut self, type_layout: &slang::reflection::TypeLayout) {
        // NOTE this means we are only querying the first descriptor set
        // doing this is vulkan-specific
        let relative_set_index = 0;

        let range_count = type_layout.descriptor_set_descriptor_range_count(relative_set_index);

        for range_index in 0..range_count {
            self.add_descriptor_range(type_layout, relative_set_index, range_index);
        }
    }

    fn add_descriptor_range(
        &mut self,
        type_layout: &slang::reflection::TypeLayout,
        relative_set_index: i64,
        range_index: i64,
    ) {
        let binding_type =
            type_layout.descriptor_set_descriptor_range_type(relative_set_index, range_index);
        if binding_type == slang::BindingType::PushConstant {
            // this is accounted for in add_sub_object_range
            // TODO should this also skip a nested ParameterBlock?
            return;
        }

        let descriptor_count = type_layout
            .descriptor_set_descriptor_range_descriptor_count(relative_set_index, range_index);

        // TODO what goes in the '...' here?
        // https://docs.shader-slang.org/en/latest/parameter-blocks.html#descriptor-ranges

        // this relies on using no manual binding annotations
        let vk_binding_index = self.binding_ranges.len();
        let descriptor_type = map_slang_binding_type_to_vk_descriptor_type(binding_type);

        let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(vk_binding_index as u32)
            .descriptor_count(descriptor_count as u32)
            // TODO where to get these from? '_currentStageFlags' in the docs
            .stage_flags(vk::ShaderStageFlags::ALL)
            .descriptor_type(descriptor_type);

        self.binding_ranges.push(descriptor_set_layout_binding);
    }

    // aka 'finishBuilding' in the docs
    // creates a vulkan DescriptorSetLayout and adds it to the PipelineLayoutBuilder
    pub fn build_and_add(
        &self,
        pipeline_layout_builder: &mut PipelineLayoutBuilder,
    ) -> Result<(), BoxError> {
        if self.binding_ranges.is_empty() {
            return Ok(());
        }

        let create_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&self.binding_ranges);
        let layout = unsafe {
            pipeline_layout_builder
                .device
                .create_descriptor_set_layout(&create_info, None)?
        };

        pipeline_layout_builder.descriptor_set_layouts[self.set_index] = layout;

        Ok(())
    }
}

fn map_slang_binding_type_to_vk_descriptor_type(
    binding_type: slang::BindingType,
) -> vk::DescriptorType {
    use slang::BindingType;

    match binding_type {
        BindingType::Sampler => vk::DescriptorType::SAMPLER,
        BindingType::Texture => vk::DescriptorType::SAMPLED_IMAGE,

        BindingType::ConstantBuffer => vk::DescriptorType::UNIFORM_BUFFER,

        BindingType::ParameterBlock => todo!(),

        BindingType::VaryingInput => todo!(),
        BindingType::VaryingOutput => todo!(),
        BindingType::PushConstant => todo!(),
        BindingType::TypedBuffer => todo!(),
        BindingType::RawBuffer => todo!(),
        BindingType::CombinedTextureSampler => todo!(),
        BindingType::InputRenderTarget => todo!(),
        BindingType::InlineUniformData => todo!(),
        BindingType::RayTracingAccelerationStructure => todo!(),
        BindingType::ExistentialValue => todo!(),
        BindingType::MutableFlag => todo!(),
        BindingType::MutableTeture => todo!(),
        BindingType::MutableTypedBuffer => todo!(),
        BindingType::MutableRawBuffer => todo!(),
        BindingType::BaseMask => todo!(),
        BindingType::ExtMask => todo!(),
        BindingType::Unknown => todo!(),
    }
}

fn create_pipeline_layout(
    device: ash::Device,
    program_layout: &slang::reflection::Shader,
) -> Result<vk::PipelineLayout, BoxError> {
    let mut pipeline_layout_builder = PipelineLayoutBuilder::new(device);

    // this will hold top-level non-parameter-block shader parameters
    let default_descriptor_set_layout_builder =
        DescriptorSetLayoutBuilder::new(&mut pipeline_layout_builder);

    // TODO add these
    // default_descriptor_set_layout_builder.add_global_scope_parameters(program_layout);
    // default_descriptor_set_layout_builder.add_entry_point_parameters(program_layout);

    default_descriptor_set_layout_builder.build_and_add(&mut pipeline_layout_builder)?;

    pipeline_layout_builder.build()
}
