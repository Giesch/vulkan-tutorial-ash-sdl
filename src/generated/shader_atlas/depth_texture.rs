// GENERATED FILE (do not edit directly)

//! TODO: docs

use serde::Serialize;

use crate::renderer::gpu_write::GPUWrite;

#[derive(Debug, Clone, Serialize)]
#[repr(C, align(16))]
pub struct DepthTexture {
    pub mvp: MVPMatrices,
}

impl GPUWrite for DepthTexture {}

#[derive(Debug, Clone, Serialize)]
#[repr(C, align(16))]
pub struct MVPMatrices {
    pub model: glam::Mat4,
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
}

impl GPUWrite for MVPMatrices {}

