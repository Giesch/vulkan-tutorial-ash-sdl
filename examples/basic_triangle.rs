use glam::{Mat4, Vec3};

use ash_sdl_vulkan_tutorial::game::Game;
use ash_sdl_vulkan_tutorial::renderer::{PipelineHandle, Renderer, UniformBufferHandle};
use ash_sdl_vulkan_tutorial::shaders::COLUMN_MAJOR;
use ash_sdl_vulkan_tutorial::shaders::atlas::basic_triangle::*;
use ash_sdl_vulkan_tutorial::shaders::atlas::*;

fn main() -> Result<(), anyhow::Error> {
    BasicTriangle::run()
}

pub struct BasicTriangle {
    pipeline: PipelineHandle,
    uniform_buffer: UniformBufferHandle<MVPMatrices>,
}

impl Game for BasicTriangle {
    fn setup(renderer: &mut Renderer) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let vertices = vec![
            Vertex {
                position: Vec3::new(-1.0, -1.0, 0.0),
                color: Vec3::new(1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(1.0, -1.0, 0.0),
                color: Vec3::new(0.0, 1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.0, 1.0, 0.0),
                color: Vec3::new(0.0, 0.0, 1.0),
            },
        ];
        let indices = vec![0, 1, 2];

        let uniform_buffer = renderer.create_uniform_buffer::<MVPMatrices>()?;

        let resources = Resources {
            vertices,
            indices,
            mvp_buffer: &uniform_buffer,
        };

        let shader = ShaderAtlas::init().basic_triangle;
        let pipeline_config = shader.pipeline_config(resources);
        let pipeline = renderer.create_pipeline(pipeline_config)?;

        Ok(Self {
            pipeline,
            uniform_buffer,
        })
    }

    fn draw_frame(&mut self, renderer: &mut Renderer) -> anyhow::Result<()> {
        let aspect_ratio = renderer.aspect_ratio();
        let mvp = make_basic_mvp_matrices(aspect_ratio, COLUMN_MAJOR);

        renderer.draw_frame(&self.pipeline, |gpu| {
            gpu.write_uniform(&mut self.uniform_buffer, mvp);
        })
    }
}

fn make_basic_mvp_matrices(aspect_ratio: f32, column_major: bool) -> MVPMatrices {
    let model = Mat4::IDENTITY;

    let eye = Vec3::new(0.0, 0.0, 6.0);
    let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);

    let fov_degrees: f32 = 45.0;
    let proj = Mat4::perspective_rh(fov_degrees.to_radians(), aspect_ratio, 0.1, 10.0);

    normalize_mvp(MVPMatrices { model, view, proj }, column_major)
}

fn normalize_mvp(mut mvp: MVPMatrices, column_major: bool) -> MVPMatrices {
    // "GLM was originally designed for OpenGL,
    // where the Y coordinate of the clip coordinates is inverted.
    // The easiest way to compensate for that is to flip the sign
    // on the scaling factor of the Y axis in the projection matrix.
    // If you don’t do this, then the image will be rendered upside down."
    // https://docs.vulkan.org/tutorial/latest/05_Uniform_buffers/00_Descriptor_set_layout_and_buffer.html
    mvp.proj.y_axis.y *= -1.0;

    // GLM & glam use column-major matrices, but D3D12 and Slang use row-major by default
    // it's also possible to avoid the transpose by reversing the mul() calls in shaders
    // https://discord.com/channels/1303735196696445038/1395879559827816458/1396913440584634499
    if !column_major {
        mvp.model = mvp.model.transpose();
        mvp.view = mvp.view.transpose();
        mvp.proj = mvp.proj.transpose();
    }

    mvp
}
