use ash::vk;

use crate::shaders::atlas::DepthTextureShader;
use crate::shaders::atlas::Vertex;

pub struct RendererConfig {
    // TODO make this a generic
    // maybe as a pipeline create arg instead of renderer init arg
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,

    // TODO make these generic
    // TODO and not direct vk types
    pub vertex_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,

    // TODO pass this when creating a pipeline,
    // instead of when creating the renderer
    pub shader: DepthTextureShader,
}

pub trait RendererVertex: super::GPUWrite {
    fn binding_descriptions() -> Vec<ash::vk::VertexInputBindingDescription>;
    fn attribute_descriptions() -> Vec<ash::vk::VertexInputAttributeDescription>;
}
