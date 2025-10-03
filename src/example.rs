use std::ffi::c_void;
use std::time::Instant;

use super::app::Game;
use super::shaders::COLUMN_MAJOR;

#[repr(C, align(16))]
struct MVPMatrices {
    model: glam::Mat4,
    view: glam::Mat4,
    projection: glam::Mat4,
}

pub struct VikingRoom {
    start_time: Instant,
}

impl VikingRoom {
    pub fn init() -> Self {
        let start_time = Instant::now();
        Self { start_time }
    }
}

impl Game for VikingRoom {
    fn uniform_buffer_size(&self) -> usize {
        std::mem::size_of::<MVPMatrices>()
    }

    fn update_uniform_buffer(
        &self,
        aspect_ratio: f32,
        mapped_uniform_buffer: *mut c_void,
    ) -> anyhow::Result<()> {
        const TURN_DEGREES_PER_SECOND: f32 = 5.0;
        const STARTING_ANGLE_DEGREES: f32 = 45.0;

        let elapsed_seconds = (Instant::now() - self.start_time).as_secs_f32();
        let turn_radians = elapsed_seconds * TURN_DEGREES_PER_SECOND.to_radians();

        let model = glam::Mat4::from_rotation_z(turn_radians);
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::splat(2.0),
            glam::Vec3::splat(0.0),
            glam::Vec3::new(0.0, 0.0, 1.0),
        );
        let projection = glam::Mat4::perspective_rh(
            STARTING_ANGLE_DEGREES.to_radians(),
            aspect_ratio,
            0.1,
            10.0,
        );

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
            std::ptr::copy_nonoverlapping(&mvp, mapped_uniform_buffer as *mut MVPMatrices, 1);
        }

        Ok(())
    }
}
