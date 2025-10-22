use std::ffi::CString;

use ash::vk;

use crate::renderer::vertex_description::VertexDescription;
use crate::renderer::{LayoutDescription, TextureDescription, UniformBufferDescription};
use crate::shaders::ReflectionJson;

#[cfg_attr(not(debug_assertions), expect(unused))]
use super::ShaderAtlasEntry;

mod mvp_matrices;
pub use mvp_matrices::*;

mod vertex;
pub use vertex::*;

pub struct DepthTextureShader {
    pub reflection_json: ReflectionJson,
}

impl DepthTextureShader {
    pub fn init() -> Self {
        let json_str = include_str!("../../../shaders/compiled/depth_texture.json");
        let reflection_json: ReflectionJson = serde_json::from_str(json_str).unwrap();

        let shader = Self { reflection_json };

        // assertions that static values match shader reflection
        #[cfg(debug_assertions)]
        {
            use crate::shaders::json::GlobalParameter::ParameterBlock;

            let mut parameter_blocks = shader
                .reflection_json
                .global_parameters
                .iter()
                .map(|ParameterBlock(p)| p);

            assert!(parameter_blocks.len() == 1);

            let reflected_uniform_buffer_size =
                parameter_blocks.next().unwrap().element_type.uniform_size;

            assert!(reflected_uniform_buffer_size == shader.uniform_buffer_size());
        }

        shader
    }
}

impl super::ShaderAtlasEntry for DepthTextureShader {
    fn source_file_name(&self) -> &str {
        &self.reflection_json.source_file_name
    }

    fn uniform_buffer_size(&self) -> usize {
        std::mem::size_of::<MVPMatrices>()
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

                dsl.binding_ranges
                    .iter()
                    .map(|b| match b.descriptor_type {
                        ReflectedBindingType::ConstantBuffer => {
                            LayoutDescription::Uniform(UniformBufferDescription {
                                // TODO associate these in the json
                                // maybe like the top level 'bindings' key in slangc's
                                size: self.uniform_buffer_size() as u64,
                                binding: b.binding,
                                descriptor_count: 1,
                            })
                        }

                        // TODO how do we associate the texture with this?
                        // expecially when there's more than one texture
                        ReflectedBindingType::CombinedTextureSampler => {
                            LayoutDescription::Texture(TextureDescription {
                                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                                binding: b.binding,
                                descriptor_count: 1,
                            })
                        }

                        ReflectedBindingType::Sampler => todo!(),
                        ReflectedBindingType::Texture => todo!(),
                    })
                    .collect()
            })
            .collect()
    }

    fn vert_entry_point_name(&self) -> CString {
        let entry_point = self
            .reflection_json
            .vertex_entry_point
            .entry_point_name
            .clone();

        CString::new(entry_point).unwrap()
    }

    fn vert_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!("../../../shaders/compiled/depth_texture.vert.spv");
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
        let bytes = include_bytes!("../../../shaders/compiled/depth_texture.frag.spv");
        let byte_reader = &mut std::io::Cursor::new(bytes);
        ash::util::read_spv(byte_reader).expect("failed to convert spv byte layout")
    }
}
