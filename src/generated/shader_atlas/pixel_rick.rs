// GENERATED FILE (do not edit directly)

//! generated from slang shader: pixel_rick.slang

use std::ffi::CString;
use std::io::Cursor;

use ash::util::read_spv;
use ash::vk;
use serde::Serialize;

use crate::renderer::gpu_write::GPUWrite;
use crate::renderer::vertex_description::VertexDescription;
use crate::renderer::*;
use crate::shaders::atlas::{PrecompiledShader, PrecompiledShaders, ShaderAtlasEntry};
use crate::shaders::json::{ReflectedPipelineLayout, ReflectionJson};

#[derive(Debug, Clone, Serialize)]
#[repr(C, align(16))]
pub struct PixelRick {
    pub circle: Circle,
}

impl GPUWrite for PixelRick {}

#[derive(Debug, Clone, Serialize)]
#[repr(C, align(16))]
pub struct Circle {
    pub position: glam::Vec2,
    pub radius: f32,
}

impl GPUWrite for Circle {}

#[derive(Debug, Clone, Serialize)]
#[repr(C, align(16))]
pub struct PixelRickVertex {
    pub position: glam::Vec3,
    pub color: glam::Vec3,
    pub tex_coord: glam::Vec2,
}

impl GPUWrite for PixelRickVertex {}

pub struct PixelRickResources<'a> {
    pub vertices: Vec<PixelRickVertex>,
    pub indices: Vec<u32>,
    pub pixel_rick_buffer: &'a UniformBufferHandle<PixelRick>,
}

impl VertexDescription for PixelRickVertex {
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
                .offset(std::mem::offset_of!(PixelRickVertex, position) as u32)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .binding(0)
                .location(0),
            ash::vk::VertexInputAttributeDescription::default()
                .offset(std::mem::offset_of!(PixelRickVertex, color) as u32)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .binding(0)
                .location(1),
            ash::vk::VertexInputAttributeDescription::default()
                .offset(std::mem::offset_of!(PixelRickVertex, tex_coord) as u32)
                .format(ash::vk::Format::R32G32_SFLOAT)
                .binding(0)
                .location(2),
        ]
    }
}

pub struct PixelRickShader {
    pub reflection_json: ReflectionJson,
}

impl PixelRickShader {
    pub fn init() -> Self {
        let json_str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compiled/pixel_rick.json"
        ));

        let reflection_json: ReflectionJson = serde_json::from_str(json_str).unwrap();

        Self { reflection_json }
    }

    pub fn pipeline_config(
        self,
        resources: PixelRickResources<'_>,
    ) -> PipelineConfig<'_, PixelRickVertex> {
        // NOTE this must be in descriptor set layout order in the reflection json
        #[rustfmt::skip]
        let texture_handles = vec![
        ];

        // NOTE this must be in descriptor set layout order in the reflection json
        #[rustfmt::skip]
        let uniform_buffer_handles = vec![
            RawUniformBufferHandle::from_typed(resources.pixel_rick_buffer),
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
            "/shaders/compiled/pixel_rick.vert.spv"
        ));
        let byte_reader = &mut Cursor::new(bytes);
        read_spv(byte_reader).expect("failed to convert spv byte layout")
    }

    fn frag_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/shaders/compiled/pixel_rick.frag.spv"
        ));
        let byte_reader = &mut Cursor::new(bytes);
        read_spv(byte_reader).expect("failed to convert spv byte layout")
    }
}

impl ShaderAtlasEntry for PixelRickShader {
    fn source_file_name(&self) -> &str {
        &self.reflection_json.source_file_name
    }

    fn uniform_buffer_sizes(&self) -> Vec<u64> {
        vec![std::mem::size_of::<PixelRick>() as u64]
    }

    fn vertex_binding_descriptions(&self) -> Vec<vk::VertexInputBindingDescription> {
        PixelRickVertex::binding_descriptions()
    }

    fn vertex_attribute_descriptions(&self) -> Vec<vk::VertexInputAttributeDescription> {
        PixelRickVertex::attribute_descriptions()
    }

    fn layout_bindings(&self) -> Vec<Vec<LayoutDescription>> {
        self.reflection_json.layout_bindings()
    }

    fn precompiled_shaders(&self) -> PrecompiledShaders {
        let vert = PrecompiledShader {
            entry_point_name: self.vert_entry_point_name(),
            spv_bytes: self.vert_spv(),
        };

        let frag = PrecompiledShader {
            entry_point_name: self.frag_entry_point_name(),
            spv_bytes: self.frag_spv(),
        };

        PrecompiledShaders { vert, frag }
    }

    fn pipeline_layout(&self) -> &ReflectedPipelineLayout {
        &self.reflection_json.pipeline_layout
    }
}
