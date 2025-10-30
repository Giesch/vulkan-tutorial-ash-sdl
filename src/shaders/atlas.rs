use std::ffi::CString;

use ash::vk;

mod depth_texture;
pub use depth_texture::*;

use crate::renderer::LayoutDescription;

use super::json::ReflectedPipelineLayout;

pub struct ShaderAtlas {
    pub depth_texture: DepthTextureShader,
}

impl ShaderAtlas {
    pub fn init() -> Self {
        Self {
            depth_texture: DepthTextureShader::init(),
        }
    }
}

pub trait ShaderAtlasEntry {
    // dev only

    // used in hot reload
    fn source_file_name(&self) -> &str;

    // dev and release

    fn uniform_buffer_sizes(&self) -> Vec<u64>;

    fn vertex_binding_descriptions(&self) -> Vec<vk::VertexInputBindingDescription>;
    fn vertex_attribute_descriptions(&self) -> Vec<vk::VertexInputAttributeDescription>;

    // one set of descriptions per descriptor set
    fn layout_bindings(&self) -> Vec<Vec<LayoutDescription>>;

    // release only

    fn precompiled_shaders(&self) -> PrecompiledShaders;

    fn pipeline_layout(&self) -> &ReflectedPipelineLayout;
}

pub struct PrecompiledShaders {
    pub vert: PrecompiledShader,
    pub frag: PrecompiledShader,
}

pub struct PrecompiledShader {
    pub entry_point_name: CString,
    pub spv_bytes: Vec<u32>,
}
