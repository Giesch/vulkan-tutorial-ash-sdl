use std::ffi::c_void;

use crate::shaders::atlas::DepthTextureShader;

use super::{vertex_description::VertexDescription, ShaderPipelineLayout, TextureHandle};
use ash::vk;

#[derive(Debug)]
pub struct PipelineHandle(usize);

pub(super) struct PipelineStorage(Vec<Option<RendererPipeline>>);

// TODO docs w/ panics sections
impl PipelineStorage {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn add(&mut self, pipeline: RendererPipeline) -> PipelineHandle {
        let handle = PipelineHandle(self.0.len());
        self.0.push(Some(pipeline));

        handle
    }

    pub fn get(&self, handle: &PipelineHandle) -> &RendererPipeline {
        self.0[handle.0].as_ref().unwrap()
    }

    // used only for hot reload
    #[cfg(debug_assertions)]
    pub fn get_mut(&mut self, handle: &PipelineHandle) -> &mut RendererPipeline {
        self.0[handle.0].as_mut().unwrap()
    }

    pub fn take(&mut self, handle: PipelineHandle) -> RendererPipeline {
        self.0[handle.0].take().unwrap()
    }
}

pub(super) struct RendererPipeline {
    pub layout: ShaderPipelineLayout,
    pub pipeline: vk::Pipeline,

    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,

    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
    pub uniform_buffers_mapped: Vec<*mut c_void>,

    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,

    pub index_count: usize,
    #[cfg_attr(not(debug_assertions), expect(unused))]
    pub shader: DepthTextureShader,
}

/// arguments for creating a pipeline
pub struct PipelineConfig<'a, V: VertexDescription> {
    pub shader: DepthTextureShader,
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
    // TODO make this a vec/iterable
    pub texture_handle: &'a TextureHandle,
}
