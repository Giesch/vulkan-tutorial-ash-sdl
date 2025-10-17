#[cfg_attr(debug_assertions, expect(unused))]
use std::ffi::CString;

use crate::shaders::ReflectionJson;

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
            use crate::shaders::json::GlobalParameter;

            let mut parameter_blocks =
                shader
                    .reflection_json
                    .global_parameters
                    .iter()
                    .map(|global_parameter| match global_parameter {
                        GlobalParameter::ParameterBlock(p) => p,
                    });

            assert!(parameter_blocks.len() == 1);

            let reflected_uniform_buffer_size =
                parameter_blocks.next().unwrap().element_type.uniform_size;

            assert!(reflected_uniform_buffer_size == shader.uniform_buffer_size());
        }

        shader
    }

    pub fn uniform_buffer_size(&self) -> usize {
        std::mem::size_of::<MVPMatrices>()
    }

    // dev only

    #[cfg(debug_assertions)]
    pub fn source_file_name(&self) -> &str {
        &self.reflection_json.source_file_name
    }

    // release only

    #[cfg(not(debug_assertions))]
    pub fn vert_entry_point_name(&self) -> CString {
        let entry_point = self
            .reflection_json
            .vertex_entry_point
            .entry_point_name
            .clone();
        CString::new(entry_point).unwrap()
    }

    #[cfg(not(debug_assertions))]
    pub fn frag_entry_point_name(&self) -> CString {
        let entry_point = self
            .reflection_json
            .fragment_entry_point
            .entry_point_name
            .clone();
        CString::new(entry_point).unwrap()
    }

    #[cfg(not(debug_assertions))]
    pub fn vert_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!("../../../shaders/compiled/depth_texture.vert.spv");
        let byte_reader = &mut std::io::Cursor::new(bytes);
        ash::util::read_spv(byte_reader).expect("failed to convert spv byte layout")
    }

    #[cfg(not(debug_assertions))]
    pub fn frag_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!("../../../shaders/compiled/depth_texture.frag.spv");
        let byte_reader = &mut std::io::Cursor::new(bytes);
        ash::util::read_spv(byte_reader).expect("failed to convert spv byte layout")
    }
}
