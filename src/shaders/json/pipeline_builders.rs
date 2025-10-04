use ash::vk;
use serde::{Deserialize, Serialize};
use shader_slang as slang;

/// reflected data for creating a vulkan PipelineLayout
#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectedPipelineLayout {
    pub descriptor_set_layouts: Vec<ReflectedDescriptorSetLayout>,
    pub push_constant_ranges: Vec<ReflectedPushConstantRange>,
}

/// reflected data for creating a vulkan DescriptorSetLayout
#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectedDescriptorSetLayout {
    pub binding_ranges: Vec<ReflectedDescriptorSetLayoutBinding>,
}

/// reflected data for creating a vulkan DescriptorSetLayoutBinding
/// samplers are deliberately excluded
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReflectedDescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: ReflectedBindingType,
    pub descriptor_count: u32,
    pub stage_flags: ReflectedStageFlags,
}

impl ReflectedDescriptorSetLayoutBinding {
    pub fn to_vk(&self) -> vk::DescriptorSetLayoutBinding<'static> {
        vk::DescriptorSetLayoutBinding::default()
            .stage_flags(self.stage_flags.to_vk())
            .binding(self.binding)
            .descriptor_count(self.descriptor_count)
            .descriptor_type(self.descriptor_type.to_vk())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectedPushConstantRange {
    pub stage_flags: ReflectedStageFlags,
    pub offset: u32,
    pub size: u32,
}

impl ReflectedPushConstantRange {
    pub fn to_vk(&self) -> vk::PushConstantRange {
        vk::PushConstantRange::default()
            .stage_flags(self.stage_flags.to_vk())
            .offset(self.size)
            .size(self.size)
    }
}

// a slang BindingType or vulkan DescriptorType
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ReflectedBindingType {
    Sampler,
    Texture,
    ConstantBuffer,
    CombinedTextureSampler,
}

impl ReflectedBindingType {
    // cpp mapSlangBindingTypeToVulkanDescriptorType
    pub fn from_slang(binding_type: slang::BindingType) -> Self {
        match binding_type {
            slang::BindingType::Sampler => Self::Sampler,
            slang::BindingType::Texture => Self::Texture,
            slang::BindingType::ConstantBuffer => Self::ConstantBuffer,
            slang::BindingType::CombinedTextureSampler => Self::CombinedTextureSampler,

            slang::BindingType::PushConstant => todo!(),
            slang::BindingType::ParameterBlock => todo!(),

            slang::BindingType::VaryingInput => todo!(),
            slang::BindingType::VaryingOutput => todo!(),
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

    pub fn to_vk(&self) -> vk::DescriptorType {
        match self {
            Self::Sampler => vk::DescriptorType::SAMPLER,
            Self::Texture => vk::DescriptorType::SAMPLED_IMAGE,
            Self::ConstantBuffer => vk::DescriptorType::UNIFORM_BUFFER,
            Self::CombinedTextureSampler => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ReflectedStageFlags {
    Vertex,
    Fragment,
    Compute,
    All,
    Empty,
}

impl ReflectedStageFlags {
    // cpp getShaderStageFlags
    pub fn from_slang(stage: slang::Stage) -> Self {
        match stage {
            slang::Stage::Vertex => Self::Vertex,
            slang::Stage::Fragment => Self::Fragment,
            slang::Stage::Compute => Self::Compute,
            slang::Stage::None => Self::Empty,

            // raytracing, mesh, tesselation, dispatch, & count
            _ => unimplemented!(),
        }
    }

    pub fn to_vk(self) -> vk::ShaderStageFlags {
        match self {
            Self::Vertex => vk::ShaderStageFlags::VERTEX,
            Self::Fragment => vk::ShaderStageFlags::FRAGMENT,
            Self::Compute => vk::ShaderStageFlags::COMPUTE,
            Self::All => vk::ShaderStageFlags::ALL,
            Self::Empty => vk::ShaderStageFlags::empty(),
        }
    }
}
