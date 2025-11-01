use std::ffi::CString;

use ash::vk;

use crate::renderer::vertex_description::VertexDescription;
use crate::renderer::{
    LayoutDescription, PipelineConfig, RawUniformBufferHandle, TextureDescription,
    UniformBufferDescription,
};
use crate::shaders::ReflectionJson;
use crate::shaders::json::ReflectedPipelineLayout;

#[cfg_attr(not(debug_assertions), expect(unused))]
use super::ShaderAtlasEntry;

pub use crate::generated::shader_atlas::depth_texture::{
    DepthTexture, DepthTextureResources, MVPMatrices, Vertex,
};

pub struct DepthTextureShader {
    pub reflection_json: ReflectionJson,
}

impl DepthTextureShader {
    // dev and release

    pub fn init() -> Self {
        let json_str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compiled/depth_texture.json"
        ));
        let reflection_json: ReflectionJson = serde_json::from_str(json_str).unwrap();

        let shader = Self { reflection_json };

        // assertions that static values match shader reflection
        #[cfg(debug_assertions)]
        {
            let const_uniform_buffer_sizes = shader.uniform_buffer_sizes();

            let layout_bindings = shader.layout_bindings();
            let reflected_uniform_buffer_sizes: Vec<u64> = layout_bindings
                .iter()
                .flat_map(|descriptions| {
                    descriptions.iter().filter_map(|ld| match ld {
                        LayoutDescription::Uniform(u) => Some(u.size),
                        _ => None,
                    })
                })
                .collect();

            assert!(reflected_uniform_buffer_sizes == const_uniform_buffer_sizes);
        }

        shader
    }

    pub fn pipeline_config(
        self,
        resources: DepthTextureResources<'_>,
    ) -> PipelineConfig<'_, Vertex> {
        let uniform_buffer_handle = RawUniformBufferHandle::from_typed(resources.depth_texture);

        PipelineConfig {
            shader: Box::new(self),
            vertices: resources.vertices,
            indices: resources.indices,
            // NOTE if there are multipe textures or multipe buffers,
            // these must match the order of ['pipelineLayout']['descriptorSetLayouts'] in the reflection json
            // codegen will need to handle this ordering
            texture_handles: vec![resources.texture],
            uniform_buffer_handles: vec![uniform_buffer_handle],
        }
    }

    // release only

    fn vert_entry_point_name(&self) -> CString {
        let entry_point = self
            .reflection_json
            .vertex_entry_point
            .entry_point_name
            .clone();

        CString::new(entry_point).unwrap()
    }

    fn vert_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compiled/depth_texture.vert.spv"
        ));
        let byte_reader = &mut std::io::Cursor::new(bytes);
        ash::util::read_spv(byte_reader).expect("failed to convert spv byte layout")
    }

    fn frag_entry_point_name(&self) -> CString {
        let entry_point = self
            .reflection_json
            .fragment_entry_point
            .entry_point_name
            .clone();

        CString::new(entry_point).unwrap()
    }

    fn frag_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compiled/depth_texture.frag.spv"
        ));
        let byte_reader = &mut std::io::Cursor::new(bytes);
        ash::util::read_spv(byte_reader).expect("failed to convert spv byte layout")
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
        self.reflection_json
            .pipeline_layout
            .descriptor_set_layouts
            .iter()
            .map(|dsl| {
                use crate::shaders::json::ReflectedBindingType;

                // NOTE this depends on the order from 'pipeline_config'
                // exactly matching the order of layout descriptions
                dsl.binding_ranges
                    .iter()
                    .map(|b| match b.descriptor_type {
                        ReflectedBindingType::ConstantBuffer => {
                            LayoutDescription::Uniform(UniformBufferDescription {
                                size: b.size as u64,
                                binding: b.binding,
                                descriptor_count: 1,
                            })
                        }

                        ReflectedBindingType::CombinedTextureSampler => {
                            LayoutDescription::Texture(TextureDescription {
                                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                                binding: b.binding,
                                descriptor_count: 1,
                            })
                        }

                        b => todo!("unhandled binding type: {b:?}"),
                    })
                    .collect()
            })
            .collect()
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
