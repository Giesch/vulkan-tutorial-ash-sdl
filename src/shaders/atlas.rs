use std::ffi::CString;

use ash::vk;

mod depth_texture;
pub use depth_texture::*;

use crate::renderer::LayoutDescription;

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

// TODO need to allow optional vert/frag
// how does that work with vertex descriptions?

pub trait ShaderAtlasEntry {
    // dev only

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
    fn vert_entry_point_name(&self) -> CString;

    #[cfg_attr(debug_assertions, expect(unused))]
    fn vert_spv(&self) -> Vec<u32>;

    #[cfg_attr(debug_assertions, expect(unused))]
    fn frag_entry_point_name(&self) -> CString;

    #[cfg_attr(debug_assertions, expect(unused))]
    fn frag_spv(&self) -> Vec<u32>;
}
