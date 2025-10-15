use std::ffi::c_void;

use super::{texture::Texture, ShaderPipelineLayout};
use ash::vk;

// TODO remove clone after finishing refactor
#[derive(Debug, Clone)]
pub struct PipelineHandle(usize);

pub struct PipelineStorage(Vec<RendererPipeline>);

impl PipelineStorage {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn add(&mut self, pipeline: RendererPipeline) -> PipelineHandle {
        let handle = PipelineHandle(self.0.len());
        self.0.push(pipeline);

        handle
    }

    pub fn get(&self, handle: &PipelineHandle) -> &RendererPipeline {
        &self.0[handle.0]
    }

    pub fn get_mut(&mut self, handle: &mut PipelineHandle) -> &mut RendererPipeline {
        &mut self.0[handle.0]
    }
}

pub struct RendererPipeline {
    pub(super) texture: Texture,

    pub(super) layout: ShaderPipelineLayout,
    pub(super) pipeline: vk::Pipeline,

    pub(super) vertex_buffer: vk::Buffer,
    pub(super) vertex_buffer_memory: vk::DeviceMemory,

    pub(super) index_buffer: vk::Buffer,
    pub(super) index_buffer_memory: vk::DeviceMemory,

    pub(super) uniform_buffers: Vec<vk::Buffer>,
    pub(super) uniform_buffers_memory: Vec<vk::DeviceMemory>,
    pub(super) uniform_buffers_mapped: Vec<*mut c_void>,

    pub(super) descriptor_pool: vk::DescriptorPool,
    pub(super) descriptor_sets: Vec<vk::DescriptorSet>,
}
