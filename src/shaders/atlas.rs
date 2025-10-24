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

// TODO need to allow optional vert/frag entrypoints
// how does that work with vertex descriptions?

pub trait ShaderAtlasEntry {
    // dev only

    // used in hot reload
    #[cfg_attr(not(debug_assertions), expect(unused))]
    fn source_file_name(&self) -> &str;

    // dev and release

    fn uniform_buffer_size(&self) -> usize;

    fn vertex_binding_descriptions(&self) -> Vec<vk::VertexInputBindingDescription>;
    fn vertex_attribute_descriptions(&self) -> Vec<vk::VertexInputAttributeDescription>;

    // one set of descriptions per descriptor set
    fn layout_bindings(&self) -> Vec<Vec<LayoutDescription>>;

    // release only

    #[cfg_attr(debug_assertions, expect(unused))]
    fn precompiled_shaders(&self) -> PrecompiledShaders;

    #[cfg_attr(debug_assertions, expect(unused))]
    fn pipeline_layout(&self) -> &ReflectedPipelineLayout;
}

#[cfg_attr(debug_assertions, expect(unused))]
pub struct PrecompiledShaders {
    pub vert: PrecompiledShader,
    pub frag: PrecompiledShader,
}

#[cfg_attr(debug_assertions, expect(unused))]
pub struct PrecompiledShader {
    pub entry_point_name: CString,
    pub spv_bytes: Vec<u32>,
}
}
