use ash::vk::{self, Handle};
use shader_slang as slang;

use crate::util::*;

// slang-reflection-based vulkan builders
// https://docs.shader-slang.org/en/latest/parameter-blocks.html#using-parameter-blocks-with-reflection

pub fn create_pipeline_layout(
    device: ash::Device,
    program_layout: &slang::reflection::Shader,
) -> Result<(vk::PipelineLayout, Vec<vk::DescriptorSetLayout>), BoxError> {
    let mut pipeline_layout_builder = PipelineLayoutBuilder::new(device);

    let mut default_descriptor_set_layout_builder =
        DescriptorSetLayoutBuilder::reserve_slot(&mut pipeline_layout_builder);

    // TODO should pipeline_layout_builder just be a desc builder field?
    // do we ever need multiple of these going at once, or is that an error?
    default_descriptor_set_layout_builder
        .add_global_scope_parameters(program_layout, &mut pipeline_layout_builder)?;
    default_descriptor_set_layout_builder
        .add_entry_point_parameters(program_layout, &mut pipeline_layout_builder)?;

    default_descriptor_set_layout_builder.build_and_add(&mut pipeline_layout_builder)?;

    let layouts = pipeline_layout_builder.build()?;

    Ok(layouts)
}

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
            //   I guess '_currentStageFlags' is a cpp global and not a builder field
            // TODO move it to a field on this builder,
            //   and make this builder a field in the desc set one?
            // that breaks lifetimes; need the top level and the nested ref
            //   can we move it up and just pass it every time? doesn't look like it
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

        match binding_type {
            slang::BindingType::ParameterBlock => {
                let parameter_block_type_layout =
                    type_layout.binding_range_leaf_type_layout(binding_range_index);
                self.add_descriptor_set_for_parameter_block(parameter_block_type_layout)?;
            }

            slang::BindingType::PushConstant => {
                let constant_buffer_type_layout =
                    type_layout.binding_range_leaf_type_layout(binding_range_index);
                self.add_push_constatant_range_for_constant_buffer(constant_buffer_type_layout);
            }

            // slang::BindingType::Unknown => todo!(),
            // slang::BindingType::Sampler => todo!(),
            // slang::BindingType::Texture => todo!(),
            // slang::BindingType::ConstantBuffer => todo!(),
            // slang::BindingType::TypedBuffer => todo!(),
            // slang::BindingType::RawBuffer => todo!(),
            // slang::BindingType::CombinedTextureSampler => todo!(),
            // slang::BindingType::InputRenderTarget => todo!(),
            // slang::BindingType::InlineUniformData => todo!(),
            // slang::BindingType::RayTracingAccelerationStructure => todo!(),
            // slang::BindingType::VaryingInput => todo!(),
            // slang::BindingType::VaryingOutput => todo!(),
            // slang::BindingType::ExistentialValue => todo!(),
            // slang::BindingType::MutableFlag => todo!(),
            // slang::BindingType::MutableTeture => todo!(),
            // slang::BindingType::MutableTypedBuffer => todo!(),
            // slang::BindingType::MutableRawBuffer => todo!(),
            // slang::BindingType::BaseMask => todo!(),
            // slang::BindingType::ExtMask => todo!(),
            _ => {}
        }

        Ok(())
    }

    pub fn add_descriptor_set_for_parameter_block(
        &mut self,
        parameter_block_type_layout: &slang::reflection::TypeLayout,
    ) -> Result<(), BoxError> {
        let mut descriptor_set_layout_builder = DescriptorSetLayoutBuilder::reserve_slot(self);
        descriptor_set_layout_builder.add_descriptor_ranges_for_parameter_block_element(
            parameter_block_type_layout.element_type_layout(),
            self,
        )?;

        descriptor_set_layout_builder.build_and_add(self)?;

        Ok(())
    }

    // aka 'finishBuilding' in the docs
    pub fn build(mut self) -> Result<(vk::PipelineLayout, Vec<vk::DescriptorSetLayout>), BoxError> {
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

        Ok((pipeline_layout, self.descriptor_set_layouts))
    }
}

pub struct DescriptorSetLayoutBuilder<'a> {
    set_index: usize,
    binding_ranges: Vec<vk::DescriptorSetLayoutBinding<'a>>,
    current_stage_flags: vk::ShaderStageFlags,
}

impl<'a> DescriptorSetLayoutBuilder<'a> {
    pub fn reserve_slot(pipeline_layout_builder: &mut PipelineLayoutBuilder) -> Self {
        // reserve a layout slot to be filled in later
        // this preserves the correct index order for nested ParameterBlocks
        // https://docs.shader-slang.org/en/latest/parameter-blocks.html#ordering-of-nested-parameter-blocks
        let set_index = pipeline_layout_builder.descriptor_set_layouts.len();
        pipeline_layout_builder
            .descriptor_set_layouts
            .push(vk::DescriptorSetLayout::null());

        Self {
            set_index,
            binding_ranges: vec![],
            current_stage_flags: vk::ShaderStageFlags::ALL,
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
            .stage_flags(self.current_stage_flags)
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
            //   and the other sub object types
            return;
        }

        let descriptor_count = type_layout
            .descriptor_set_descriptor_range_descriptor_count(relative_set_index, range_index);

        // TODO what goes in the '...' here?
        // https://docs.shader-slang.org/en/latest/parameter-blocks.html#descriptor-ranges

        // this relies on using no manual binding annotations
        let vk_binding_index = self.binding_ranges.len();
        let descriptor_type = slang_to_vk_descriptor_type(binding_type);

        let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(vk_binding_index as u32)
            .descriptor_count(descriptor_count as u32)
            .stage_flags(self.current_stage_flags)
            .descriptor_type(descriptor_type);

        self.binding_ranges.push(descriptor_set_layout_binding);
    }

    pub fn add_global_scope_parameters(
        &mut self,
        program_layout: &slang::reflection::Shader,
        pipeline_layout_builder: &mut PipelineLayoutBuilder,
    ) -> Result<(), BoxError> {
        // NOTE we could also track usage using the reflection API
        //   but it's simpler to just only use entry point args and shared globals
        // could also use a slang user attribute for this?
        self.current_stage_flags = vk::ShaderStageFlags::ALL;
        self.add_descriptor_ranges_for_parameter_block_element(
            program_layout.global_params_type_layout(),
            pipeline_layout_builder,
        )?;

        Ok(())
    }

    pub fn add_entry_point_parameters(
        &mut self,
        program_layout: &slang::reflection::Shader,
        pipeline_layout_builder: &mut PipelineLayoutBuilder,
    ) -> Result<(), BoxError> {
        for entry_point in program_layout.entry_points() {
            self.current_stage_flags = slang_to_vk_stage_flags(entry_point.stage());
            self.add_descriptor_ranges_for_parameter_block_element(
                entry_point.type_layout(),
                pipeline_layout_builder,
            )?;
        }

        Ok(())
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

// cpp mapSlangBindingTypeToVulkanDescriptorType
fn slang_to_vk_descriptor_type(binding_type: slang::BindingType) -> vk::DescriptorType {
    match binding_type {
        slang::BindingType::Sampler => vk::DescriptorType::SAMPLER,
        slang::BindingType::Texture => vk::DescriptorType::SAMPLED_IMAGE,
        slang::BindingType::ConstantBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        slang::BindingType::CombinedTextureSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,

        slang::BindingType::ParameterBlock => todo!(),
        slang::BindingType::VaryingInput => todo!(),
        slang::BindingType::VaryingOutput => todo!(),
        slang::BindingType::PushConstant => todo!(),
        slang::BindingType::TypedBuffer => todo!(),
        slang::BindingType::RawBuffer => todo!(),
        slang::BindingType::InputRenderTarget => todo!(),
        slang::BindingType::InlineUniformData => todo!(),
        slang::BindingType::RayTracingAccelerationStructure => todo!(),
        slang::BindingType::ExistentialValue => todo!(),
        slang::BindingType::MutableFlag => todo!(),
        slang::BindingType::MutableTeture => todo!(),
        slang::BindingType::MutableTypedBuffer => todo!(),
        slang::BindingType::MutableRawBuffer => todo!(),
        slang::BindingType::BaseMask => todo!(),
        slang::BindingType::ExtMask => todo!(),
        slang::BindingType::Unknown => todo!(),
    }
}

// cpp getShaderStageFlags
fn slang_to_vk_stage_flags(stage: slang::Stage) -> vk::ShaderStageFlags {
    match stage {
        // general
        shader_slang::Stage::Vertex => vk::ShaderStageFlags::VERTEX,
        shader_slang::Stage::Fragment => vk::ShaderStageFlags::FRAGMENT,
        shader_slang::Stage::Compute => vk::ShaderStageFlags::COMPUTE,
        shader_slang::Stage::None => vk::ShaderStageFlags::empty(),

        // raytracing
        shader_slang::Stage::RayGeneration => vk::ShaderStageFlags::RAYGEN_KHR,
        shader_slang::Stage::Intersection => vk::ShaderStageFlags::INTERSECTION_KHR,
        shader_slang::Stage::AnyHit => vk::ShaderStageFlags::ANY_HIT_KHR,
        shader_slang::Stage::ClosestHit => vk::ShaderStageFlags::CLOSEST_HIT_KHR,
        shader_slang::Stage::Miss => vk::ShaderStageFlags::MISS_KHR,
        shader_slang::Stage::Callable => vk::ShaderStageFlags::CALLABLE_KHR,

        // mesh
        shader_slang::Stage::Mesh => vk::ShaderStageFlags::MESH_EXT,
        shader_slang::Stage::Amplification => vk::ShaderStageFlags::TASK_EXT,

        // tesselation, dispatch, & count
        _ => unimplemented!(),
    }
}
