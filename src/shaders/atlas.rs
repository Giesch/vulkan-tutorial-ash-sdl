mod depth_texture;
pub use depth_texture::*;

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
