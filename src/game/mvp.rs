use crate::renderer::gpu_write::GPUWrite;

#[derive(Debug, Clone)]
#[repr(C, align(16))]
pub struct MVPMatrices {
    pub model: glam::Mat4,
    pub view: glam::Mat4,
    pub projection: glam::Mat4,
}

impl GPUWrite for MVPMatrices {}
