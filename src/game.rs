use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Context;
use image::{DynamicImage, ImageReader};

use crate::renderer::{PipelineHandle, Renderer, RendererConfig, RendererVertex, TextureHandle};
use crate::shaders::atlas::MVPMatrices;
use crate::shaders::atlas::ShaderAtlas;
use crate::util::manifest_path;

use super::shaders::COLUMN_MAJOR;

mod vertex;
pub use vertex::*;

pub mod traits;
pub use traits::{Game, WindowDescription};

const INITIAL_WINDOW_WIDTH: u32 = 800;
const INITIAL_WINDOW_HEIGHT: u32 = 600;
const FRAME_DELAY: Duration = Duration::from_millis(15);

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
    pipeline_handle: PipelineHandle,
    texture_handle: TextureHandle,
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
                glam::Vec3::new(
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
                glam::Vec2::new(u, v)
            };

            let vertex = Vertex {
                position,
                color: glam::Vec3::splat(1.0),
                tex_coord,
            };

            vertices.push(vertex);
        }

        Ok((vertices, mesh.indices))
    }
}

impl Game for VikingRoom {
    fn window_description() -> WindowDescription {
        WindowDescription {
            title: "Viking Room",
            width: INITIAL_WINDOW_WIDTH,
            height: INITIAL_WINDOW_HEIGHT,
        }
    }

    fn frame_delay(&self) -> Duration {
        FRAME_DELAY
    }

    fn setup(window: sdl3::video::Window) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (vertices, indices) = Self::load_vertices()?;

        let image = load_image("viking_room.png")?;

        let vertex_binding_descriptions = Vertex::binding_descriptions();
        let vertex_attribute_descriptions = Vertex::attribute_descriptions();

        let shader_atlas = ShaderAtlas::init();
        let shader = shader_atlas.depth_texture;

        let renderer_config = RendererConfig {
            vertices,
            indices,
            vertex_binding_descriptions,
            vertex_attribute_descriptions,
            shader,
        };

        let mut renderer = Renderer::init(window, renderer_config)?;
        let texture_handle = renderer.create_texture(&image)?;
        let pipeline_handle = renderer.create_pipeline(&texture_handle)?;

        let start_time = Instant::now();
        let window_desc = Self::window_description();
        let aspect_ratio = window_desc.width as f32 / window_desc.height as f32;

        Ok(Self {
            start_time,
            aspect_ratio,
            renderer,
            pipeline_handle,
            texture_handle,
        })
    }

    fn draw_frame(&mut self) -> anyhow::Result<()> {
        self.renderer
            .draw_frame(&self.pipeline_handle, |mapped_uniform_buffer| {
                update_mvp_uniform_buffer(
                    self.start_time,
                    self.aspect_ratio,
                    mapped_uniform_buffer as *mut MVPMatrices,
                )
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
        self.renderer.drop_texture(self.texture_handle);
        self.renderer.drop_pipeline(self.pipeline_handle);

        Ok(())
    }
}
#[allow(unused)]
pub struct DepthTexture {
    start_time: Instant,
    aspect_ratio: f32,
    renderer: Renderer,
    pipeline_handle: PipelineHandle,
    texture_handle: TextureHandle,
}

#[allow(unused)]
impl DepthTexture {
    fn load_vertices() -> Result<(Vec<Vertex>, Vec<u32>), anyhow::Error> {
        let vertices = vec![
            Vertex {
                position: glam::Vec3::new(-0.5, -0.5, 0.0),
                color: glam::Vec3::new(1.0, 0.0, 0.0),
                tex_coord: glam::Vec2::new(1.0, 0.0),
            },
            Vertex {
                position: glam::Vec3::new(0.5, -0.5, 0.0),
                color: glam::Vec3::new(0.0, 1.0, 0.0),
                tex_coord: glam::Vec2::new(0.0, 0.0),
            },
            Vertex {
                position: glam::Vec3::new(0.5, 0.5, 0.0),
                color: glam::Vec3::new(0.0, 0.0, 1.0),
                tex_coord: glam::Vec2::new(0.0, 1.0),
            },
            Vertex {
                position: glam::Vec3::new(-0.5, 0.5, 0.0),
                color: glam::Vec3::new(1.0, 1.0, 1.0),
                tex_coord: glam::Vec2::new(1.0, 1.0),
            },
            Vertex {
                position: glam::Vec3::new(-0.5, -0.5, -0.5),
                color: glam::Vec3::new(1.0, 0.0, 0.0),
                tex_coord: glam::Vec2::new(1.0, 0.0),
            },
            Vertex {
                position: glam::Vec3::new(0.5, -0.5, -0.5),
                color: glam::Vec3::new(0.0, 1.0, 0.0),
                tex_coord: glam::Vec2::new(0.0, 0.0),
            },
            Vertex {
                position: glam::Vec3::new(0.5, 0.5, -0.5),
                color: glam::Vec3::new(0.0, 0.0, 1.0),
                tex_coord: glam::Vec2::new(0.0, 1.0),
            },
            Vertex {
                position: glam::Vec3::new(-0.5, 0.5, -0.5),
                color: glam::Vec3::new(1.0, 1.0, 1.0),
                tex_coord: glam::Vec2::new(1.0, 1.0),
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
    fn window_description() -> WindowDescription {
        WindowDescription {
            title: "Depth Texture",
            width: INITIAL_WINDOW_WIDTH,
            height: INITIAL_WINDOW_HEIGHT,
        }
    }

    fn frame_delay(&self) -> Duration {
        FRAME_DELAY
    }

    fn setup(window: sdl3::video::Window) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (vertices, indices) = Self::load_vertices()?;

        let image = load_image("texture.jpg")?;

        let vertex_binding_descriptions = Vertex::binding_descriptions();
        let vertex_attribute_descriptions = Vertex::attribute_descriptions();

        let shader_atlas = ShaderAtlas::init();
        let shader = shader_atlas.depth_texture;

        let renderer_config = RendererConfig {
            vertices,
            indices,
            vertex_binding_descriptions,
            vertex_attribute_descriptions,
            shader,
        };

        let mut renderer = Renderer::init(window, renderer_config)?;
        let texture_handle = renderer.create_texture(&image)?;
        let pipeline_handle = renderer.create_pipeline(&texture_handle)?;

        let start_time = Instant::now();
        let window_desc = Self::window_description();
        let aspect_ratio = window_desc.width as f32 / window_desc.height as f32;

        Ok(Self {
            start_time,
            aspect_ratio,
            renderer,
            pipeline_handle,
            texture_handle,
        })
    }

    fn draw_frame(&mut self) -> anyhow::Result<()> {
        self.renderer
            .draw_frame(&self.pipeline_handle, |mapped_uniform_buffer| {
                update_mvp_uniform_buffer(
                    self.start_time,
                    self.aspect_ratio,
                    mapped_uniform_buffer as *mut MVPMatrices,
                )
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
        self.renderer.drop_texture(self.texture_handle);
        self.renderer.drop_pipeline(self.pipeline_handle);

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

fn update_mvp_uniform_buffer(
    start_time: Instant,
    aspect_ratio: f32,
    mapped_uniform_buffer: *mut MVPMatrices,
) -> Result<(), anyhow::Error> {
    const TURN_DEGREES_PER_SECOND: f32 = 5.0;
    const STARTING_ANGLE_DEGREES: f32 = 45.0;

    let elapsed_seconds = (Instant::now() - start_time).as_secs_f32();
    let turn_radians = elapsed_seconds * TURN_DEGREES_PER_SECOND.to_radians();

    let model = glam::Mat4::from_rotation_z(turn_radians);
    let view = glam::Mat4::look_at_rh(
        glam::Vec3::splat(2.0),
        glam::Vec3::splat(0.0),
        glam::Vec3::new(0.0, 0.0, 1.0),
    );
    let projection =
        glam::Mat4::perspective_rh(STARTING_ANGLE_DEGREES.to_radians(), aspect_ratio, 0.1, 10.0);

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

    if !COLUMN_MAJOR {
        // it's also possible to avoid this by reversing the mul() calls in shaders
        // https://discord.com/channels/1303735196696445038/1395879559827816458/1396913440584634499
        mvp.model = mvp.model.transpose();
        mvp.view = mvp.view.transpose();
        mvp.projection = mvp.projection.transpose();
    }

    unsafe {
        std::ptr::copy_nonoverlapping(&mvp, mapped_uniform_buffer, 1);
    }

    Ok(())
}
