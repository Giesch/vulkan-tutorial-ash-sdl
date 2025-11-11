pub mod basic_triangle;
pub mod depth_texture;

pub struct ShaderAtlas {
    pub basic_triangle: basic_triangle::BasicTriangleShader,
    pub depth_texture: depth_texture::DepthTextureShader,
}

impl ShaderAtlas {
    pub fn init() -> Self {
        Self {
            basic_triangle: basic_triangle::BasicTriangleShader::init(),
            depth_texture: depth_texture::DepthTextureShader::init(),
        }
    }
}
