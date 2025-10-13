use ash::vk;

use crate::game::Game;

pub struct RendererConfig {
    pub uniform_buffer_size: u64,
    // TODO make this a generic
    // maybe as a pipeline create arg instead of renderer init arg
    pub vertices: Vec<crate::game::Vertex>,
    pub indices: Vec<u32>,
    pub image: image::DynamicImage,

    // TODO make these generic
    // TODO and not direct vk types
    pub vertex_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
}

impl RendererConfig {
    pub fn from_game(game: &dyn Game) -> anyhow::Result<Self> {
        let uniform_buffer_size = game.uniform_buffer_size() as vk::DeviceSize;
        let (vertices, indices) = game.load_vertices()?;
        let image = game.load_texture()?;
        let vertex_binding_descriptions = game.vertex_binding_descriptions();
        let vertex_attribute_descriptions = game.vertex_attribute_descriptions();

        Ok(Self {
            uniform_buffer_size,
            vertices,
            indices,
            image,
            vertex_binding_descriptions,
            vertex_attribute_descriptions,
        })
    }
}

pub trait RendererVertex: super::GPUWrite {
    fn binding_descriptions() -> Vec<ash::vk::VertexInputBindingDescription>;
    fn attribute_descriptions() -> Vec<ash::vk::VertexInputAttributeDescription>;
}
