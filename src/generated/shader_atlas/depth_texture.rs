// GENERATED FILE (do not edit directly)

use serde::Serialize;

use crate::renderer::gpu_write::GPUWrite;
use crate::renderer::vertex_description::VertexDescription;
use crate::renderer::*;

#[derive(Debug, Clone, Serialize)]
#[repr(C, align(16))]
pub struct DepthTexture {
    pub mvp: MVPMatrices,
}

impl GPUWrite for DepthTexture {}

#[derive(Debug, Clone, Serialize)]
#[repr(C, align(16))]
pub struct MVPMatrices {
    pub model: glam::Mat4,
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
}

impl GPUWrite for MVPMatrices {}

#[derive(Debug, Clone, Serialize)]
#[repr(C, align(16))]
pub struct Vertex {
    pub position: glam::Vec3,
    pub color: glam::Vec3,
    pub tex_coord: glam::Vec2,
}

impl GPUWrite for Vertex {}

pub struct DepthTextureResources<'a> {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub texture: &'a TextureHandle,
    pub depth_texture: &'a UniformBufferHandle<DepthTexture>,
}

impl VertexDescription for Vertex {
    fn binding_descriptions() -> Vec<ash::vk::VertexInputBindingDescription> {
        let binding_description = ash::vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(ash::vk::VertexInputRate::VERTEX);

        vec![binding_description]
    }

    fn attribute_descriptions() -> Vec<ash::vk::VertexInputAttributeDescription> {
        vec![
            ash::vk::VertexInputAttributeDescription::default()
                .offset(std::mem::offset_of!(Vertex, position) as u32)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .binding(0)
                .location(0),
            ash::vk::VertexInputAttributeDescription::default()
                .offset(std::mem::offset_of!(Vertex, color) as u32)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .binding(0)
                .location(1),
            ash::vk::VertexInputAttributeDescription::default()
                .offset(std::mem::offset_of!(Vertex, tex_coord) as u32)
                .format(ash::vk::Format::R32G32_SFLOAT)
                .binding(0)
                .location(2),
        ]
    }
}
