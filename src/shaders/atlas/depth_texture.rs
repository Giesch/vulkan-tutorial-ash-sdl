use std::ffi::CString;

use ash::vk;

use crate::renderer::vertex_description::VertexDescription;
use crate::renderer::{LayoutDescription, TextureDescription, UniformBufferDescription};
use crate::shaders::json::ReflectedPipelineLayout;
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
    // dev and release

    pub fn init() -> Self {
        let json_str = include_str!("../../../shaders/compiled/depth_texture.json");
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

impl super::ShaderAtlasEntry for DepthTextureShader {
    fn source_file_name(&self) -> &str {
        &self.reflection_json.source_file_name
    }

    fn uniform_buffer_sizes(&self) -> Vec<u64> {
        vec![std::mem::size_of::<MVPMatrices>() as u64]
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
                                size: b.size as u64,
                                binding: b.binding,
                                descriptor_count: 1,
                            })
                        }

                        // TODO how do we associate the texture with this?
                        // expecially when there's more than one texture
                        //
                        // need one method on here that takes a texture id / struct of one,
                        // and returns all the binding info
                        // that resources struct could be a generated associated type on the trait
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
