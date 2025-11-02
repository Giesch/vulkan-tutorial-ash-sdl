use std::ffi::CString;
use std::io::Cursor;

use ash::util::read_spv;
use ash::vk;

use crate::renderer::vertex_description::VertexDescription;
use crate::renderer::{LayoutDescription, PipelineConfig, RawUniformBufferHandle};
use crate::shaders::ReflectionJson;
use crate::shaders::json::ReflectedPipelineLayout;

pub use crate::generated::shader_atlas::depth_texture::{
    DepthTexture, DepthTextureResources, MVPMatrices, Vertex,
};

pub struct DepthTextureShader {
    pub reflection_json: ReflectionJson,
}

impl DepthTextureShader {
    pub fn init() -> Self {
        let json_str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compiled/depth_texture.json"
        ));

        let reflection_json: ReflectionJson = serde_json::from_str(json_str).unwrap();

        Self { reflection_json }
    }

    pub fn pipeline_config(
        self,
        resources: DepthTextureResources<'_>,
    ) -> PipelineConfig<'_, Vertex> {
        // NOTE this must be in descriptor set layout order in the reflection json
        #[rustfmt::skip]
        let texture_handles = vec![
            resources.texture,
        ];

        // NOTE this must be in descriptor set layout order in the reflection json
        #[rustfmt::skip]
        let uniform_buffer_handles = vec![
            RawUniformBufferHandle::from_typed(resources.depth_texture_buffer),
        ];

        PipelineConfig {
            shader: Box::new(self),
            vertices: resources.vertices,
            indices: resources.indices,
            texture_handles,
            uniform_buffer_handles,
        }
    }

    fn vert_entry_point_name(&self) -> CString {
        let entry_point = self
            .reflection_json
            .vertex_entry_point
            .entry_point_name
            .clone();

        CString::new(entry_point).unwrap()
    }

    fn frag_entry_point_name(&self) -> CString {
        let entry_point = self
            .reflection_json
            .fragment_entry_point
            .entry_point_name
            .clone();

        CString::new(entry_point).unwrap()
    }

    fn vert_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compiled/depth_texture.vert.spv"
        ));
        let byte_reader = &mut Cursor::new(bytes);
        read_spv(byte_reader).expect("failed to convert spv byte layout")
    }

    fn frag_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compiled/depth_texture.frag.spv"
        ));
        let byte_reader = &mut Cursor::new(bytes);
        read_spv(byte_reader).expect("failed to convert spv byte layout")
    }
}

impl super::ShaderAtlasEntry for DepthTextureShader {
    fn source_file_name(&self) -> &str {
        &self.reflection_json.source_file_name
    }

    fn uniform_buffer_sizes(&self) -> Vec<u64> {
        vec![std::mem::size_of::<DepthTexture>() as u64]
    }

    fn vertex_binding_descriptions(&self) -> Vec<vk::VertexInputBindingDescription> {
        Vertex::binding_descriptions()
    }

    fn vertex_attribute_descriptions(&self) -> Vec<vk::VertexInputAttributeDescription> {
        Vertex::attribute_descriptions()
    }

    fn layout_bindings(&self) -> Vec<Vec<LayoutDescription>> {
        self.reflection_json.layout_bindings()
    }

    fn precompiled_shaders(&self) -> super::PrecompiledShaders {
        let vert = super::PrecompiledShader {
            entry_point_name: self.vert_entry_point_name(),
            spv_bytes: self.vert_spv(),
        };

        let frag = super::PrecompiledShader {
            entry_point_name: self.frag_entry_point_name(),
            spv_bytes: self.frag_spv(),
        };

        super::PrecompiledShaders { vert, frag }
    }

    fn pipeline_layout(&self) -> &ReflectedPipelineLayout {
        &self.reflection_json.pipeline_layout
    }
}
