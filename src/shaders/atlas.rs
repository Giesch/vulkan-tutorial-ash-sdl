#[cfg_attr(debug_assertions, expect(unused))]
use std::ffi::CString;

#[cfg_attr(debug_assertions, expect(unused))]
use crate::shaders::ReflectionJson;

pub struct ShaderAtlas {
    pub depth_texture: DepthTextureShader,
}

impl ShaderAtlas {
    pub fn init() -> Self {
        Self {
            depth_texture: DepthTextureShader::init(),
        }
    }
}

pub struct DepthTextureShader {
    #[cfg(not(debug_assertions))]
    pub reflection_json: ReflectionJson,
}

impl DepthTextureShader {
    // dev and release

    const SOURCE_FILE_NAME: &str = "depth_texture.slang";

    pub fn source_file_name(&self) -> &str {
        Self::SOURCE_FILE_NAME
    }

    // dev only

    #[cfg(debug_assertions)]
    pub fn init() -> Self {
        Self {}
    }

    // release only

    #[cfg(not(debug_assertions))]
    pub fn init() -> Self {
        let json_str = include_str!("../../shaders/compiled/depth_texture.json");
        let reflection_json: ReflectionJson = serde_json::from_str(json_str).unwrap();

        Self { reflection_json }
    }

    #[cfg(not(debug_assertions))]
    pub fn vert_entry_point_name(&self) -> CString {
        let entry_point = self.reflection_json.vertex_entry_point.name.clone();
        CString::new(entry_point).unwrap()
    }

    #[cfg(not(debug_assertions))]
    pub fn frag_entry_point_name(&self) -> CString {
        let entry_point = self.reflection_json.fragment_entry_point.name.clone();
        CString::new(entry_point).unwrap()
    }

    #[cfg(not(debug_assertions))]
    pub fn vert_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!("../../shaders/compiled/depth_texture.vert.spv");
        let byte_reader = &mut std::io::Cursor::new(bytes);
        ash::util::read_spv(byte_reader).expect("failed to convert spv byte layout")
    }

    #[cfg(not(debug_assertions))]
    pub fn frag_spv(&self) -> Vec<u32> {
        let bytes = include_bytes!("../../shaders/compiled/depth_texture.frag.spv");
        let byte_reader = &mut std::io::Cursor::new(bytes);
        ash::util::read_spv(byte_reader).expect("failed to convert spv byte layout")
    }
}
