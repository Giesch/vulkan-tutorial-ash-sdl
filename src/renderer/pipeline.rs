use ash::vk;

use crate::shaders::atlas::ShaderAtlasEntry;

use super::vertex_description::VertexDescription;
use super::TextureHandle;
use super::{AllocatedUniformBuffers, ShaderPipelineLayout};

#[derive(Debug)]
pub struct PipelineHandle {
    index: usize,
}

pub(super) struct PipelineStorage(Vec<Option<RendererPipeline>>);

// TODO docs w/ panics sections
impl PipelineStorage {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn add(&mut self, pipeline: RendererPipeline) -> PipelineHandle {
        let index = self.0.len();
        let handle = PipelineHandle { index };
        self.0.push(Some(pipeline));

        handle
    }

    pub fn get(&self, handle: &PipelineHandle) -> &RendererPipeline {
        self.0[handle.index].as_ref().unwrap()
    }

    // used only for hot reload
    #[cfg(debug_assertions)]
    pub fn get_mut(&mut self, handle: &PipelineHandle) -> &mut RendererPipeline {
        self.0[handle.index].as_mut().unwrap()
    }

    pub fn take(&mut self, handle: PipelineHandle) -> RendererPipeline {
        self.0[handle.index].take().unwrap()
    }
}

pub(super) struct RendererPipeline {
    pub layout: ShaderPipelineLayout,
    pub pipeline: vk::Pipeline,

    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,

    pub all_uniform_buffers: Vec<AllocatedUniformBuffers>,

    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,

    pub index_count: usize,
    #[cfg_attr(not(debug_assertions), expect(unused))]
    pub shader: Box<dyn ShaderAtlasEntry>,
}

/// the generic arguments for creating a pipeline
pub struct PipelineConfig<'t, V: VertexDescription> {
    pub shader: Box<dyn ShaderAtlasEntry>,
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
    pub texture_handles: Vec<&'t TextureHandle>,
}
