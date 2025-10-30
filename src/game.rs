use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use glam::{Mat4, Vec2, Vec3};
use image::{DynamicImage, ImageReader};

use crate::renderer::{PipelineHandle, Renderer, TextureHandle, UniformBufferHandle};
use crate::shaders::atlas::{DepthTextureResources, MVPMatrices, ShaderAtlas, Vertex};
use crate::util::manifest_path;

use super::shaders::COLUMN_MAJOR;

pub mod traits;
pub use traits::{Game, WindowDescription};

// TODO
// we want a more zoomed-out api than this
//
// like, Game should say,
// 'I want these GPU resources: a uniform buffer of size x, ...', etc
//
// then the renderer returns typed indirect handles that persist through hot reload
//   so the game can keep those handles in its own struct
//   someday maybe give them a 'scene' lifetime? not worth it?
//
// then, in an 'update' callback on game, called by app
//   game calls renderer with the handles to get access to pointers for the frame
//   or renderer has higher-level methods that take a handle
//
// game makes some kind of create pipeline request to the renderer
//   gets a pipeline object handle back, or a struct w/a set of handles?
//   or the renderer has an internal 'pipeline description' type created that it uses
// can we get that whole request/description from reflection?
//   not entirely; need to be able to load textures and such
//   so a combination of the reflected shader-atlas entry and other resources
// how can we map the create request struct into the created resources struct?
//   need a macro

#[allow(unused)]
pub struct VikingRoom {
    start_time: Instant,
    aspect_ratio: f32,
    renderer: Renderer,
    pipeline: PipelineHandle,
    texture: TextureHandle,
    mvp_buffer: UniformBufferHandle<MVPMatrices>,
}

impl VikingRoom {
    // From unknownue's rust version of the vulkan tutorial
    // https://github.com/unknownue/vulkan-tutorial-rust/blob/master/src/tutorials/27_model_loading.rs
    fn load_vertices() -> anyhow::Result<(Vec<Vertex>, Vec<u32>)> {
        let file_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "models", "viking_room.obj"]
            .iter()
            .collect();

        let (mut models, _materials) = tobj::load_obj(file_path, &tobj::GPU_LOAD_OPTIONS)?;

        debug_assert!(models.len() == 1);
        let model = models.remove(0);

        let mut vertices = vec![];
        let mesh = model.mesh;
        let vertices_count = mesh.positions.len() / 3;
        for i in 0..vertices_count {
            let position = {
                let offset = i * 3;
                Vec3::new(
                    mesh.positions[offset],
                    mesh.positions[offset + 1],
                    mesh.positions[offset + 2],
                )
            };

            let tex_coord = {
                let offset = i * 2;
                let u = mesh.texcoords[offset];
                // in obj, 0 is the bottom, in vulkan, 0 is the top
                // (for texture coordinates)
                let v = 1.0 - mesh.texcoords[offset + 1];
                Vec2::new(u, v)
            };

            let vertex = Vertex {
                position,
                color: Vec3::splat(1.0),
                tex_coord,
            };

            vertices.push(vertex);
        }

        Ok((vertices, mesh.indices))
    }
}

impl Game for VikingRoom {
    fn window_title() -> &'static str {
        "Viking Room"
    }

    fn setup(mut renderer: Renderer) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (vertices, indices) = Self::load_vertices()?;

        const IMAGE_FILE_NAME: &str = "viking_room.png";
        let image = load_image(IMAGE_FILE_NAME)?;

        let shader_atlas = ShaderAtlas::init();
        let shader = shader_atlas.depth_texture;

        let texture = renderer.create_texture(IMAGE_FILE_NAME, &image)?;
        let mvp_buffer = renderer.create_uniform_buffer::<MVPMatrices>()?;
        let resources = DepthTextureResources {
            vertices,
            indices,
            texture: &texture,
            mvp_buffer: &mvp_buffer,
        };
        let pipeline_config = shader.pipeline_config(resources);
        let pipeline = renderer.create_pipeline(pipeline_config)?;

        let start_time = Instant::now();
        let window_desc = Self::window_description();
        let aspect_ratio = window_desc.width as f32 / window_desc.height as f32;

        Ok(Self {
            start_time,
            aspect_ratio,
            renderer,
            pipeline,
            texture,
            mvp_buffer,
        })
    }

    fn draw_frame(&mut self) -> anyhow::Result<()> {
        self.renderer.draw_frame(&self.pipeline, |gpu| {
            let elapsed = Instant::now() - self.start_time;
            let mvp_buffer = gpu.get_uniform_buffer_mut(&mut self.mvp_buffer);
            *mvp_buffer = make_mvp_matrices(elapsed, self.aspect_ratio, COLUMN_MAJOR);
        })
    }

    fn on_resize(&mut self) -> anyhow::Result<()> {
        self.renderer.recreate_swapchain()?;

        let (width, height) = self.renderer.current_extent();
        self.aspect_ratio = width as f32 / height as f32;

        Ok(())
    }

    fn deinit(mut self: Box<Self>) -> anyhow::Result<()> {
        self.renderer.drain_gpu()?;
        Ok(())
    }
}

#[allow(unused)]
pub struct DepthTexture {
    start_time: Instant,
    aspect_ratio: f32,
    renderer: Renderer,
    pipeline: PipelineHandle,
    texture: TextureHandle,
    mvp_buffer: UniformBufferHandle<MVPMatrices>,
}

#[allow(unused)]
impl DepthTexture {
    fn load_vertices() -> Result<(Vec<Vertex>, Vec<u32>), anyhow::Error> {
        let vertices = vec![
            Vertex {
                position: Vec3::new(-0.5, -0.5, 0.0),
                color: Vec3::new(1.0, 0.0, 0.0),
                tex_coord: Vec2::new(1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.5, -0.5, 0.0),
                color: Vec3::new(0.0, 1.0, 0.0),
                tex_coord: Vec2::new(0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.5, 0.5, 0.0),
                color: Vec3::new(0.0, 0.0, 1.0),
                tex_coord: Vec2::new(0.0, 1.0),
            },
            Vertex {
                position: Vec3::new(-0.5, 0.5, 0.0),
                color: Vec3::new(1.0, 1.0, 1.0),
                tex_coord: Vec2::new(1.0, 1.0),
            },
            Vertex {
                position: Vec3::new(-0.5, -0.5, -0.5),
                color: Vec3::new(1.0, 0.0, 0.0),
                tex_coord: Vec2::new(1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.5, -0.5, -0.5),
                color: Vec3::new(0.0, 1.0, 0.0),
                tex_coord: Vec2::new(0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.5, 0.5, -0.5),
                color: Vec3::new(0.0, 0.0, 1.0),
                tex_coord: Vec2::new(0.0, 1.0),
            },
            Vertex {
                position: Vec3::new(-0.5, 0.5, -0.5),
                color: Vec3::new(1.0, 1.0, 1.0),
                tex_coord: Vec2::new(1.0, 1.0),
            },
        ];

        #[rustfmt::skip]
        let indices = vec![
            0, 1, 2, 2, 3, 0,
            4, 5, 6, 6, 7, 4,
        ];

        Ok((vertices, indices))
    }
}

impl Game for DepthTexture {
    fn window_title() -> &'static str {
        "Depth Texture"
    }

    fn setup(mut renderer: Renderer) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (vertices, indices) = Self::load_vertices()?;

        const IMAGE_FILE_NAME: &str = "texture.jpg";
        let image = load_image(IMAGE_FILE_NAME)?;

        let shader_atlas = ShaderAtlas::init();
        let shader = shader_atlas.depth_texture;

        let texture = renderer.create_texture(IMAGE_FILE_NAME, &image)?;
        let mvp_buffer = renderer.create_uniform_buffer::<MVPMatrices>()?;
        let resources = DepthTextureResources {
            vertices,
            indices,
            texture: &texture,
            mvp_buffer: &mvp_buffer,
        };
        let pipeline_config = shader.pipeline_config(resources);
        let pipeline = renderer.create_pipeline(pipeline_config)?;

        let start_time = Instant::now();
        let window_desc = Self::window_description();
        let aspect_ratio = window_desc.width as f32 / window_desc.height as f32;

        Ok(Self {
            start_time,
            aspect_ratio,
            renderer,
            pipeline,
            texture,
            mvp_buffer,
        })
    }

    fn draw_frame(&mut self) -> anyhow::Result<()> {
        self.renderer.draw_frame(&self.pipeline, |gpu| {
            let elapsed = Instant::now() - self.start_time;
            let mvp_buffer = gpu.get_uniform_buffer_mut(&mut self.mvp_buffer);
            *mvp_buffer = make_mvp_matrices(elapsed, self.aspect_ratio, COLUMN_MAJOR);
        })
    }

    fn on_resize(&mut self) -> anyhow::Result<()> {
        self.renderer.recreate_swapchain()?;

        let (width, height) = self.renderer.current_extent();
        self.aspect_ratio = width as f32 / height as f32;

        Ok(())
    }

    fn deinit(mut self: Box<Self>) -> anyhow::Result<()> {
        self.renderer.drain_gpu()?;
        Ok(())
    }
}

fn load_image(file_name: &str) -> anyhow::Result<DynamicImage> {
    let file_path = manifest_path(["textures", file_name]);
    let image = ImageReader::open(&file_path)
        .with_context(|| format!("failed to open image: {file_path:?}"))?
        .decode()
        .with_context(|| format!("failed to decode image: {file_path:?}"))?;

    Ok(image)
}

fn make_mvp_matrices(elapsed: Duration, aspect_ratio: f32, column_major: bool) -> MVPMatrices {
    const TURN_DEGREES_PER_SECOND: f32 = 5.0;
    const STARTING_ANGLE_DEGREES: f32 = 45.0;

    let turn_radians = elapsed.as_secs_f32() * TURN_DEGREES_PER_SECOND.to_radians();

    let model = Mat4::from_rotation_z(turn_radians);
    let view = Mat4::look_at_rh(Vec3::splat(2.0), Vec3::ZERO, Vec3::Z);
    let projection =
        Mat4::perspective_rh(STARTING_ANGLE_DEGREES.to_radians(), aspect_ratio, 0.1, 10.0);

    let mut mvp = MVPMatrices {
        model,
        view,
        projection,
    };

    // "GLM was originally designed for OpenGL,
    // where the Y coordinate of the clip coordinates is inverted.
    // The easiest way to compensate for that is to flip the sign
    // on the scaling factor of the Y axis in the projection matrix.
    // If you don’t do this, then the image will be rendered upside down."
    // https://docs.vulkan.org/tutorial/latest/05_Uniform_buffers/00_Descriptor_set_layout_and_buffer.html
    mvp.projection.y_axis.y *= -1.0;

    if !column_major {
        // it's also possible to avoid this by reversing the mul() calls in shaders
        // https://discord.com/channels/1303735196696445038/1395879559827816458/1396913440584634499
        mvp.model = mvp.model.transpose();
        mvp.view = mvp.view.transpose();
        mvp.projection = mvp.projection.transpose();
    }

    mvp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_mvp_matrices_col_major() {
        let elapsed = Duration::from_secs(2);
        let aspect_ratio = 800.0 / 600.0;
        let mvp = make_mvp_matrices(elapsed, aspect_ratio, true);
        insta::assert_json_snapshot!(mvp);
    }

    #[test]
    fn make_mvp_matrices_row_major() {
        let elapsed = Duration::from_secs(2);
        let aspect_ratio = 800.0 / 600.0;
        let mvp = make_mvp_matrices(elapsed, aspect_ratio, false);
        insta::assert_json_snapshot!(mvp);
    }
}
