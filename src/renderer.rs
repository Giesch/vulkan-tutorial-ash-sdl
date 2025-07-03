use std::collections::BTreeSet;
use std::ffi::{c_char, CStr, CString};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Instant;

use ash::vk;
use image::ImageReader;
use sdl3::sys::vulkan::SDL_Vulkan_DestroySurface;
use sdl3::video::Window;

use super::BoxError;

pub mod debug;
mod platform;

const ENABLE_VALIDATION: bool = cfg!(debug_assertions);

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    start_time: Instant,
    // fields that are created once
    #[allow(unused)]
    entry: ash::Entry,
    window: Window,
    instance: ash::Instance,
    debug_ext: vk::DebugUtilsMessengerEXT,
    surface_ext: ash::khr::surface::Instance,
    debug_loader: ash::ext::debug_utils::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: QueueFamilyIndices,
    device: ash::Device,
    graphics_queue: vk::Queue,
    presentation_queue: vk::Queue,
    swapchain_device_ext: ash::khr::swapchain::Device,

    // fields that change
    #[expect(unused)] // currently not used after init
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    image_format: vk::Format,
    image_extent: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    depth_image: vk::Image,
    depth_image_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,
    texture_image: vk::Image,
    texture_image_memory: vk::DeviceMemory,
    texture_image_view: vk::ImageView,
    #[expect(unused)] // currently not used after init
    mip_levels: u32,
    texture_sampler: vk::Sampler,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,
    uniform_buffers_mapped: Vec<*mut UniformBufferObject>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    /// image semaphores indexed by current_frame
    image_available: Vec<vk::Semaphore>,
    /// render finished semaphores indexed by image_index
    render_finished: Vec<vk::Semaphore>,
    /// frame fences indexed by current frame
    frames_in_flight: Vec<vk::Fence>,
    current_frame: usize,
}

impl Renderer {
    pub fn init(window: Window) -> Result<Self, BoxError> {
        let start_time = Instant::now();

        let entry = ash::Entry::linked();

        check_required_extensions(&entry)?;
        check_required_layers(&entry)?;

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Vulkan Tutorial")
            .engine_name(c"No Engine")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_0);

        let mut enabled_extension_names = vec![];
        let window_required_extensions: Vec<_> = window
            .vulkan_instance_extensions()?
            .into_iter()
            .map(|s| CString::new(s).unwrap())
            .collect();
        for name in &window_required_extensions {
            enabled_extension_names.push(name.as_ptr())
        }
        if ENABLE_VALIDATION {
            enabled_extension_names.push(ash::ext::debug_utils::NAME.as_ptr());
        }

        for platform_instance_ext in platform::ADDITIONAL_INSTANCE_EXTENSIONS {
            enabled_extension_names.push(platform_instance_ext.as_ptr());
        }

        let create_flags = platform::instance_create_flags();

        let mut enabled_layer_names = vec![];
        for layer_name in get_required_layers() {
            enabled_layer_names.push(layer_name.as_ptr())
        }

        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&enabled_layer_names)
            .enabled_extension_names(&enabled_extension_names)
            .flags(create_flags);
        let mut debug_create_info = debug::build_messenger_create_info();
        if ENABLE_VALIDATION {
            create_info = create_info.push_next(&mut debug_create_info);
        }

        let instance = unsafe { entry.create_instance(&create_info, None)? };
        let (debug_loader, debug_ext) =
            debug::maybe_create_debug_messager_extension(&entry, &instance, &debug_create_info);

        let surface_ext = ash::khr::surface::Instance::new(&entry, &instance);

        let surface = window.vulkan_create_surface(instance.handle())?;

        let (physical_device, queue_family_indices, physical_device_properties) =
            choose_physical_device(&instance, &surface_ext, surface)?;
        let device = create_logical_device(&instance, physical_device, &queue_family_indices)?;

        let graphics_queue = unsafe { device.get_device_queue(queue_family_indices.graphics, 0) };
        let presentation_queue =
            unsafe { device.get_device_queue(queue_family_indices.presentation, 0) };

        let swapchain_device_ext = ash::khr::swapchain::Device::new(&instance, &device);
        let CreatedSwapchain {
            swapchain,
            image_format,
            image_extent,
        } = create_swapchain(
            &window,
            &swapchain_device_ext,
            &surface_ext,
            surface,
            physical_device,
            &queue_family_indices,
        )?;

        let swapchain_images = unsafe { swapchain_device_ext.get_swapchain_images(swapchain)? };

        let swapchain_image_views =
            create_swapchain_image_views(&device, image_format, &swapchain_images)?;

        let render_pass = create_render_pass(&instance, physical_device, &device, image_format)?;

        let descriptor_set_layout = create_descriptor_set_layout(&device)?;

        let (pipeline_layout, pipeline) =
            create_graphics_pipeline(&device, render_pass, descriptor_set_layout)?;

        let command_pool = create_command_pool(&device, &queue_family_indices)?;
        let command_buffers = create_command_buffers(&device, command_pool)?;

        let (depth_image, depth_image_memory, depth_image_view) = create_depth_image(
            &instance,
            &device,
            physical_device,
            command_pool,
            graphics_queue,
            image_extent,
        )?;

        let swapchain_framebuffers = create_framebuffers(
            &device,
            render_pass,
            &swapchain_image_views,
            image_extent,
            depth_image_view,
        )?;

        let (texture_image, texture_image_memory, mip_levels) = create_texture_image(
            &instance,
            &device,
            physical_device,
            command_pool,
            graphics_queue,
        )?;
        let texture_image_view = create_image_view(
            &device,
            texture_image,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageAspectFlags::COLOR,
            mip_levels,
        )?;
        let texture_sampler = create_texture_sampler(&device, physical_device_properties)?;

        let (vertices, indices) = load_model()?;

        let (vertex_buffer, vertex_buffer_memory) = create_vertex_buffer(
            &instance,
            &device,
            physical_device,
            command_pool,
            graphics_queue,
            &vertices,
        )?;

        let (index_buffer, index_buffer_memory) = create_index_buffer(
            &instance,
            &device,
            physical_device,
            command_pool,
            graphics_queue,
            &indices,
        )?;

        let (uniform_buffers, uniform_buffers_memory, uniform_buffers_mapped) =
            create_uniform_buffers(&instance, &device, physical_device)?;

        let descriptor_pool = create_descriptor_pool(&device)?;
        let descriptor_sets = create_descriptor_sets(
            &device,
            descriptor_pool,
            descriptor_set_layout,
            &uniform_buffers,
            texture_image_view,
            texture_sampler,
        )?;

        let (image_available, render_finished, frames_in_flight) =
            create_sync_objects(&device, &swapchain_images)?;

        Ok(Self {
            start_time,
            window: window.clone(),
            entry,
            instance,
            debug_ext,
            surface_ext,
            debug_loader,
            surface,
            physical_device,
            queue_family_indices,
            device,
            graphics_queue,
            presentation_queue,
            swapchain_device_ext,
            vertices,
            indices,
            image_format,
            image_extent,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            render_pass,
            descriptor_set_layout,
            pipeline_layout,
            pipeline,
            swapchain_framebuffers,
            depth_image,
            depth_image_memory,
            depth_image_view,
            texture_image,
            texture_image_memory,
            texture_image_view,
            texture_sampler,
            mip_levels,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            uniform_buffers,
            uniform_buffers_memory,
            uniform_buffers_mapped,
            command_pool,
            command_buffers,
            descriptor_pool,
            descriptor_sets,
            image_available,
            render_finished,
            frames_in_flight,
            current_frame: 0,
        })
    }

    fn record_command_buffer(&mut self, image_index: u32) -> Result<(), BoxError> {
        let command_buffer = self.command_buffers[self.current_frame];

        let begin_info = vk::CommandBufferBeginInfo::default();
        unsafe {
            self.device
                .begin_command_buffer(command_buffer, &begin_info)?;
        }

        let framebuffer = self.swapchain_framebuffers[image_index as usize];
        let render_area = vk::Rect2D::default()
            .offset(vk::Offset2D::default())
            .extent(self.image_extent);
        let clear_color = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };
        let clear_depth_stencil = vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };
        // NOTE this must match the order of the attachments
        let clear_values = [clear_color, clear_depth_stencil];
        let render_pass_begin = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass)
            .framebuffer(framebuffer)
            .render_area(render_area)
            .clear_values(&clear_values);

        // BEGIN RENDER PASS
        unsafe {
            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );
        }

        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
        }

        let viewport = vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(self.image_extent.width as f32)
            .height(self.image_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let viewports = [viewport];
        unsafe {
            self.device.cmd_set_viewport(command_buffer, 0, &viewports);
        }

        let scissor = vk::Rect2D::default()
            .offset(vk::Offset2D::default())
            .extent(self.image_extent);
        let scissors = [scissor];
        unsafe { self.device.cmd_set_scissor(command_buffer, 0, &scissors) };

        unsafe {
            let buffers = [self.vertex_buffer];
            let offsets = [0];
            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);

            self.device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer,
                0,
                vk::IndexType::UINT32,
            );

            let descriptor_sets = [self.descriptor_sets[self.current_frame]];
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &descriptor_sets,
                &[],
            );

            self.device
                .cmd_draw_indexed(command_buffer, self.indices.len() as u32, 1, 0, 0, 0);
        }

        // END RENDER PASS
        unsafe { self.device.cmd_end_render_pass(command_buffer) };

        unsafe { self.device.end_command_buffer(command_buffer)? };

        Ok(())
    }

    pub fn draw_frame(&mut self) -> Result<(), BoxError> {
        let command_buffer = self.command_buffers[self.current_frame];

        let fences = [self.frames_in_flight[self.current_frame]];
        unsafe { self.device.wait_for_fences(&fences, true, u64::MAX)? };

        let (image_index, swapchain_suboptimal) = unsafe {
            match self.swapchain_device_ext.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available[self.current_frame],
                vk::Fence::null(),
            ) {
                Ok(tup) => tup,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    return self.recreate_swapchain();
                }
                Err(other_error) => {
                    return Err(other_error.into());
                }
            }
        };

        update_uniform_buffer(
            self.start_time,
            self.image_extent,
            self.uniform_buffers_mapped[self.current_frame],
        )?;

        // NOTE only reset fences if we're submitting work
        unsafe { self.device.reset_fences(&fences)? };

        unsafe {
            self.device
                .reset_command_buffer(command_buffer, Default::default())?;
        }
        self.record_command_buffer(image_index)?;

        let wait_semaphores = [self.image_available[self.current_frame]];
        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished[image_index as usize]];
        let submit_command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .command_buffers(&submit_command_buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            self.device.queue_submit(
                self.graphics_queue,
                &[submit_info],
                self.frames_in_flight[self.current_frame],
            )?;
        }

        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        unsafe {
            match self
                .swapchain_device_ext
                .queue_present(self.presentation_queue, &present_info)
            {
                Ok(false) => {
                    // not suboptimal, aka fine, or optimal i guess
                }
                Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    // suboptimal or out of date
                    return self.recreate_swapchain();
                }
                Err(other_error) => {
                    return Err(other_error.into());
                }
            }
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        if swapchain_suboptimal {
            return self.recreate_swapchain();
        }

        Ok(())
    }

    pub fn drain_gpu(self) -> Result<(), BoxError> {
        unsafe { self.device.device_wait_idle()? };
        Ok(())
    }

    // to be called on window resize
    pub fn recreate_swapchain(&mut self) -> Result<(), BoxError> {
        unsafe { self.device.device_wait_idle()? }

        self.cleanup_swapchain();
        unsafe {
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_image_memory, None);
        }

        let CreatedSwapchain {
            swapchain,
            image_format,
            image_extent,
        } = create_swapchain(
            &self.window,
            &self.swapchain_device_ext,
            &self.surface_ext,
            self.surface,
            self.physical_device,
            &self.queue_family_indices,
        )?;
        self.swapchain = swapchain;
        self.image_format = image_format;
        self.image_extent = image_extent;

        self.swapchain_images =
            unsafe { self.swapchain_device_ext.get_swapchain_images(swapchain)? };

        self.swapchain_image_views =
            create_swapchain_image_views(&self.device, self.image_format, &self.swapchain_images)?;

        let (depth_image, depth_image_memory, depth_image_view) = create_depth_image(
            &self.instance,
            &self.device,
            self.physical_device,
            self.command_pool,
            self.graphics_queue,
            self.image_extent,
        )?;
        self.depth_image = depth_image;
        self.depth_image_memory = depth_image_memory;
        self.depth_image_view = depth_image_view;

        self.swapchain_framebuffers = create_framebuffers(
            &self.device,
            // NOTE for some monitor changes,
            // you'd need to recreate the renderpass as well
            self.render_pass,
            &self.swapchain_image_views,
            image_extent,
            self.depth_image_view,
        )?;

        Ok(())
    }

    fn cleanup_swapchain(&mut self) {
        unsafe {
            for framebuffer in &self.swapchain_framebuffers {
                self.device.destroy_framebuffer(*framebuffer, None);
            }
            for image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }

            // NOTE this also frees the images
            self.swapchain_device_ext
                .destroy_swapchain(self.swapchain, None);
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            for fence in &self.frames_in_flight {
                self.device.destroy_fence(*fence, None);
            }
            for semaphore in &self.render_finished {
                self.device.destroy_semaphore(*semaphore, None);
            }
            for semaphore in &self.image_available {
                self.device.destroy_semaphore(*semaphore, None);
            }

            // this also destroys the sets from the pool
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);

            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);

            self.device.destroy_sampler(self.texture_sampler, None);
            self.device
                .destroy_image_view(self.texture_image_view, None);
            self.device.destroy_image(self.texture_image, None);
            self.device.free_memory(self.texture_image_memory, None);

            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_image_memory, None);

            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            self.device.destroy_render_pass(self.render_pass, None);

            self.cleanup_swapchain();

            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device.destroy_buffer(self.uniform_buffers[i], None);
                self.device
                    .free_memory(self.uniform_buffers_memory[i], None);
            }

            self.device.destroy_device(None);

            // NOTE This must be called before dropping the sdl window,
            // which means that the Renderer must be dropped before the window.
            // That should happen by default, since Renderer::init requires a window,
            // and rust drops things in reverse order.
            SDL_Vulkan_DestroySurface(self.instance.handle(), self.surface, std::ptr::null());

            if ENABLE_VALIDATION {
                self.debug_loader
                    .destroy_debug_utils_messenger(self.debug_ext, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}

fn get_required_layers() -> Vec<&'static std::ffi::CStr> {
    if ENABLE_VALIDATION {
        vec![c"VK_LAYER_KHRONOS_validation"]
    } else {
        vec![]
    }
}

fn check_required_layers(entry: &ash::Entry) -> Result<(), BoxError> {
    let required_layers = get_required_layers();
    let available_layers = unsafe { entry.enumerate_instance_layer_properties()? };

    for required_layer in required_layers {
        let mut found = false;
        for prop in &available_layers {
            let layer_name = vk_str_bytes(&prop.layer_name);
            if layer_name == required_layer.to_bytes() {
                found = true;
                break;
            }
        }

        if !found {
            let required_layer = required_layer.to_string_lossy();
            return Err(format!("missing required layer: {required_layer}").into());
        }
    }

    Ok(())
}

fn check_required_extensions(entry: &ash::Entry) -> Result<(), BoxError> {
    let mut required_extensions = vec![ash::khr::surface::NAME, platform::OS_SURFACE_EXT];

    if ENABLE_VALIDATION {
        required_extensions.push(ash::ext::debug_utils::NAME);
    }

    let available_extensions = unsafe { entry.enumerate_instance_extension_properties(None)? };

    for required_ext in &required_extensions {
        let mut found = false;
        for prop in &available_extensions {
            let ext_name: Vec<u8> = vk_str_bytes(&prop.extension_name);
            if ext_name == required_ext.to_bytes() {
                found = true;
                break;
            }
        }

        if !found {
            let required_layer = required_ext.to_string_lossy();
            return Err(format!("missing required extension: {required_layer}").into());
        }
    }

    Ok(())
}

/// trims a null-terminated c string from vulkan to only include
/// non-null bytes for comparison with CStr constants
fn vk_str_bytes(vk_str: &[c_char]) -> Vec<u8> {
    vk_str
        .iter()
        .map(|byte| *byte as u8)
        .take_while(|byte| *byte != b'\0')
        .collect()
}

struct QueueFamilyIndices {
    graphics: u32,
    presentation: u32,
}

impl QueueFamilyIndices {
    fn find(
        instance: &ash::Instance,
        surface_ext: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Option<Self>, BoxError> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut graphics = None;
        let mut presentation = None;

        for (i, family) in queue_families.iter().enumerate() {
            // NOTE this also implies vk::QueueFlags::TRANSFER
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics = Some(i as u32);
            }

            let supports_presentation = unsafe {
                surface_ext.get_physical_device_surface_support(
                    physical_device,
                    i as u32,
                    surface,
                )?
            };
            if supports_presentation {
                presentation = Some(i as u32)
            }
        }

        let indices = match (graphics, presentation) {
            (Some(graphics), Some(presentation)) => Some(Self {
                graphics,
                presentation,
            }),
            _ => None,
        };

        Ok(indices)
    }
}

const REQUIRED_DEVICE_EXTENSIONS: [&CStr; 1] = [vk::KHR_SWAPCHAIN_NAME];

fn choose_physical_device(
    instance: &ash::Instance,
    surface_ext: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<
    (
        vk::PhysicalDevice,
        QueueFamilyIndices,
        vk::PhysicalDeviceProperties,
    ),
    BoxError,
> {
    let physical_devices: Vec<vk::PhysicalDevice> =
        unsafe { instance.enumerate_physical_devices()? };

    // this corresponds to the tutorial's 'isDeviceSuitable'
    let mut devices_with_indices_and_props = vec![];
    for physical_device in physical_devices {
        let indices = QueueFamilyIndices::find(instance, surface_ext, surface, physical_device)?;
        let Some(indices) = indices else {
            continue;
        };

        let supports_extensions =
            check_device_extension_support(instance, physical_device, &REQUIRED_DEVICE_EXTENSIONS)?;
        if !supports_extensions {
            continue;
        }

        let swapchain_support =
            SwapChainSupportDetails::query(surface_ext, surface, physical_device)?;
        let swapchain_adequate =
            !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty();
        if !swapchain_adequate {
            continue;
        }

        let features = unsafe { instance.get_physical_device_features(physical_device) };
        if features.sampler_anisotropy != vk::TRUE {
            continue;
        }

        let props = unsafe { instance.get_physical_device_properties(physical_device) };

        devices_with_indices_and_props.push((physical_device, indices, props));
    }

    devices_with_indices_and_props.sort_by_key(|(_physical_device, _indices, props)| {
        match props.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => 0,
            vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
            vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
            vk::PhysicalDeviceType::CPU => 3,
            vk::PhysicalDeviceType::OTHER => 4,
            _ => 5,
        }
    });

    let Some(chosen_device) = devices_with_indices_and_props.into_iter().next() else {
        return Err("no graphics device availble".into());
    };

    Ok(chosen_device)
}

const PREFERRED_SURFACE_FORMAT: vk::SurfaceFormatKHR = vk::SurfaceFormatKHR {
    format: vk::Format::B8G8R8A8_SRGB,
    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
};

fn choose_swap_surface_format(swapchain: &SwapChainSupportDetails) -> vk::SurfaceFormatKHR {
    if swapchain.formats.contains(&PREFERRED_SURFACE_FORMAT) {
        return PREFERRED_SURFACE_FORMAT;
    }

    swapchain.fallback_format
}

fn choose_swap_present_mode(available_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    if available_modes.contains(&vk::PresentModeKHR::MAILBOX) {
        // burns battery on mobile, good otherwise
        return vk::PresentModeKHR::MAILBOX;
    }

    // aka vsync; guaranteed to be supported
    vk::PresentModeKHR::FIFO
}

fn check_device_extension_support(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    required_extensions: &[&'static CStr],
) -> Result<bool, BoxError> {
    let mut required_extensions: BTreeSet<Vec<u8>> = required_extensions
        .iter()
        .map(|&cstr| cstr.to_bytes().to_owned())
        .collect();

    let device_ext_props =
        unsafe { instance.enumerate_device_extension_properties(physical_device)? };
    for prop in device_ext_props {
        let bytes = vk_str_bytes(&prop.extension_name);
        required_extensions.remove(&bytes);
    }

    Ok(required_extensions.is_empty())
}

fn choose_swap_extent(window: &Window, capabilities: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    // u32::MAX is used as a sentinel value that means 'refer to the bounds'
    if capabilities.current_extent.width != u32::MAX {
        return capabilities.current_extent;
    }

    let (sdl_width, sdl_height) = window.size_in_pixels();

    let width = sdl_width.clamp(
        capabilities.min_image_extent.width,
        capabilities.max_image_extent.width,
    );

    let height = sdl_height.clamp(
        capabilities.min_image_extent.height,
        capabilities.max_image_extent.height,
    );

    vk::Extent2D { width, height }
}

struct CreatedSwapchain {
    swapchain: vk::SwapchainKHR,
    image_format: vk::Format,
    image_extent: vk::Extent2D,
}

fn create_swapchain(
    window: &Window,
    swapchain_device_ext: &ash::khr::swapchain::Device,
    surface_ext: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: &QueueFamilyIndices,
) -> Result<CreatedSwapchain, BoxError> {
    let swapchain_support = SwapChainSupportDetails::query(surface_ext, surface, physical_device)?;

    let surface_format = choose_swap_surface_format(&swapchain_support);
    let present_mode = choose_swap_present_mode(&swapchain_support.present_modes);
    let image_extent = choose_swap_extent(window, &swapchain_support.capabilities);

    // the number of images in the swapchain
    // going too low can result in the application blocking on the GPU
    let desired_image_count = swapchain_support.capabilities.min_image_count + 1;
    let max_image_count = swapchain_support.capabilities.max_image_count;
    // 0 is a sentinel value meaning no maximum
    let max_image_count = if max_image_count == 0 {
        u32::MAX
    } else {
        max_image_count
    };
    let image_count = desired_image_count.clamp(0, max_image_count);

    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(image_extent)
        .image_array_layers(1) // only not one for stereoscopic 3D (VR?)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT); // this would be a memory op instead, if post-processing

    let create_info_indices = [
        queue_family_indices.graphics,
        queue_family_indices.presentation,
    ];
    let create_info = if queue_family_indices.graphics != queue_family_indices.presentation {
        // different queue families; the uncommon case
        // the tutorial recommends avoiding concurrent sharing mode if possible
        // but this involves the ownership portion of the vulkan API
        create_info
            .image_sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&create_info_indices)
    } else {
        // same queue family; the common case
        create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
    };

    let create_info = create_info
        // no flip / rotation on swapchain images
        .pre_transform(swapchain_support.capabilities.current_transform)
        // for window transparency
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        // used during resizing & similar swapchain recreations
        .old_swapchain(vk::SwapchainKHR::null());

    let swapchain = unsafe { swapchain_device_ext.create_swapchain(&create_info, None)? };

    Ok(CreatedSwapchain {
        swapchain,
        image_format: surface_format.format,
        image_extent,
    })
}

fn create_logical_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    indices: &QueueFamilyIndices,
) -> Result<ash::Device, BoxError> {
    let unique_queue_families = BTreeSet::from([indices.graphics, indices.presentation]);

    let mut queue_create_infos = vec![];
    let queue_priorities = [1.0];
    for index in unique_queue_families {
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(index)
            .queue_priorities(&queue_priorities);

        queue_create_infos.push(queue_create_info);
    }

    let features = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);

    let enabled_extension_names: Vec<_> = REQUIRED_DEVICE_EXTENSIONS
        .iter()
        .map(|cstr| cstr.as_ptr())
        .collect();

    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_features(&features)
        .enabled_extension_names(&enabled_extension_names);

    let device = unsafe { instance.create_device(physical_device, &create_info, None)? };

    Ok(device)
}

struct SwapChainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    fallback_format: vk::SurfaceFormatKHR,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapChainSupportDetails {
    fn query(
        surface_ext: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, BoxError> {
        let capabilities = unsafe {
            surface_ext.get_physical_device_surface_capabilities(physical_device, surface)?
        };

        let formats =
            unsafe { surface_ext.get_physical_device_surface_formats(physical_device, surface)? };
        let fallback_format = formats
            .first()
            .copied()
            .expect("physical device had no surface formats");

        let present_modes = unsafe {
            surface_ext.get_physical_device_surface_present_modes(physical_device, surface)?
        };

        Ok(Self {
            capabilities,
            formats,
            fallback_format,
            present_modes,
        })
    }
}

fn create_swapchain_image_views(
    device: &ash::Device,
    image_format: vk::Format,
    swapchain_images: &[vk::Image],
) -> Result<Vec<vk::ImageView>, BoxError> {
    let mut swapchain_image_views = Vec::with_capacity(swapchain_images.len());
    for &image in swapchain_images {
        let image_view =
            create_image_view(device, image, image_format, vk::ImageAspectFlags::COLOR, 1)?;
        swapchain_image_views.push(image_view);
    }

    Ok(swapchain_image_views)
}

fn create_render_pass(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: &ash::Device,
    swapchain_format: vk::Format,
) -> Result<vk::RenderPass, BoxError> {
    let color_attachment = vk::AttachmentDescription::default()
        .format(swapchain_format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let color_attachment_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let depth_format = find_depth_format(instance, physical_device);
    let depth_attachment = vk::AttachmentDescription::default()
        .format(depth_format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let depth_attachment_ref = vk::AttachmentReference::default()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let color_attachment_refs = [color_attachment_ref];
    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        // NOTE the index in this array is the one referred to by
        // 'layout(location = 0) out' in the frag shader
        .color_attachments(&color_attachment_refs)
        .depth_stencil_attachment(&depth_attachment_ref);

    // NOTE an alternative to doing this would be to
    // change the wait stages of image_available to include TOP_OF_PIPE
    // https://vulkan-tutorial.com/en/Drawing_a_triangle/Drawing/Rendering_and_presentation
    let subpass_dep = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
        )
        .src_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
        .dst_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        );

    let attachments = [color_attachment, depth_attachment];
    let subpasses = [subpass];
    let dependencies = [subpass_dep];
    let render_pass_create_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    let render_pass = unsafe { device.create_render_pass(&render_pass_create_info, None)? };

    Ok(render_pass)
}

/// usage: read_shader_spv("triangle.vert.spv");
fn read_shader_spv(shader_name: &str) -> Result<Vec<u32>, BoxError> {
    let shader_path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "shaders",
        "compiled",
        shader_name,
    ]
    .iter()
    .collect();

    let mut spv_file = BufReader::new(File::open(&shader_path)?);
    let vk_bytes = ash::util::read_spv(&mut spv_file)?;

    Ok(vk_bytes)
}

fn create_graphics_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<(vk::PipelineLayout, vk::Pipeline), BoxError> {
    let vert_shader_spv = read_shader_spv("triangle.vert.spv")?;
    let frag_shader_spv = read_shader_spv("triangle.frag.spv")?;

    let vert_create_info = vk::ShaderModuleCreateInfo::default().code(&vert_shader_spv);
    let frag_create_info = vk::ShaderModuleCreateInfo::default().code(&frag_shader_spv);

    let vert_shader = unsafe { device.create_shader_module(&vert_create_info, None)? };
    let frag_shader = unsafe { device.create_shader_module(&frag_create_info, None)? };

    let vert_create_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vert_shader)
        .name(c"main");
    let frag_create_info = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(frag_shader)
        .name(c"main");
    let stages = [vert_create_info, frag_create_info];

    let dynamic_states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let binding_descriptions = [Vertex::binding_description()];
    let attribute_descriptions = Vertex::attribute_descriptions();
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&binding_descriptions)
        .vertex_attribute_descriptions(&attribute_descriptions);

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // relying on dynamic state to fill these in during draw
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    // color blend per attached framebuffer
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(false)
        .color_write_mask(vk::ColorComponentFlags::RGBA);
    let color_attachments = [color_blend_attachment];
    // global color blending
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(&color_attachments);

    // handle not struct; to be used later
    let set_layouts = [descriptor_set_layout];
    let pipeline_layout_create_info =
        vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
    let pipeline_layout =
        unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None)? };

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false);

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&color_blend_state)
        .dynamic_state(&dynamic_state)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .depth_stencil_state(&depth_stencil_state);

    let graphics_pipelines = unsafe {
        device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .map_err(|e| format!("failed to create graphics pipelines: {e:?}"))?
    };
    let graphics_pipeline = graphics_pipelines[0];

    unsafe { device.destroy_shader_module(frag_shader, None) };
    unsafe { device.destroy_shader_module(vert_shader, None) };

    Ok((pipeline_layout, graphics_pipeline))
}

fn create_framebuffers(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    swapchain_image_views: &[vk::ImageView],
    image_extent: vk::Extent2D,
    depth_image_view: vk::ImageView,
) -> Result<Vec<vk::Framebuffer>, BoxError> {
    let mut framebuffers = Vec::with_capacity(swapchain_image_views.len());

    for &image_view in swapchain_image_views {
        let attachments = [image_view, depth_image_view];

        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(image_extent.width)
            .height(image_extent.height)
            .layers(1);

        let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None)? };

        framebuffers.push(framebuffer);
    }

    Ok(framebuffers)
}

fn create_command_pool(
    device: &ash::Device,
    queue_family_indicies: &QueueFamilyIndices,
) -> Result<vk::CommandPool, BoxError> {
    let pool_info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family_indicies.graphics);

    let command_pool = unsafe { device.create_command_pool(&pool_info, None)? };

    Ok(command_pool)
}

fn create_command_buffers(
    device: &ash::Device,
    command_pool: vk::CommandPool,
) -> Result<Vec<vk::CommandBuffer>, BoxError> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);

    let buffers = unsafe { device.allocate_command_buffers(&alloc_info)? };

    Ok(buffers)
}

fn create_sync_objects(
    device: &ash::Device,
    swapchain_images: &[vk::Image],
) -> Result<(Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>), BoxError> {
    let mut image_available = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    for _frame in 0..MAX_FRAMES_IN_FLIGHT {
        let semaphore = unsafe { device.create_semaphore(&Default::default(), None)? };
        image_available.push(semaphore);
    }

    let mut render_finished = Vec::with_capacity(swapchain_images.len());
    for _image in swapchain_images {
        let semaphore = unsafe { device.create_semaphore(&Default::default(), None)? };
        render_finished.push(semaphore);
    }

    let mut frames_in_flight = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    for _frame in 0..MAX_FRAMES_IN_FLIGHT {
        let fence_create_info =
            vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let fence = unsafe { device.create_fence(&fence_create_info, None)? };
        frames_in_flight.push(fence);
    }

    Ok((image_available, render_finished, frames_in_flight))
}

#[derive(Debug)]
#[repr(C, align(16))]
struct Vertex {
    position: glam::Vec3,
    color: glam::Vec3,
    tex_coord: glam::Vec2,
}

// const VERTICES: [Vertex; 8] = [
//     Vertex {
//         position: glam::Vec3::new(-0.5, -0.5, 0.0),
//         color: glam::Vec3::new(1.0, 0.0, 0.0),
//         tex_coord: glam::Vec2::new(1.0, 0.0),
//     },
//     Vertex {
//         position: glam::Vec3::new(0.5, -0.5, 0.0),
//         color: glam::Vec3::new(0.0, 1.0, 0.0),
//         tex_coord: glam::Vec2::new(0.0, 0.0),
//     },
//     Vertex {
//         position: glam::Vec3::new(0.5, 0.5, 0.0),
//         color: glam::Vec3::new(0.0, 0.0, 1.0),
//         tex_coord: glam::Vec2::new(0.0, 1.0),
//     },
//     Vertex {
//         position: glam::Vec3::new(-0.5, 0.5, 0.0),
//         color: glam::Vec3::new(1.0, 1.0, 1.0),
//         tex_coord: glam::Vec2::new(1.0, 1.0),
//     },
//     Vertex {
//         position: glam::Vec3::new(-0.5, -0.5, -0.5),
//         color: glam::Vec3::new(1.0, 0.0, 0.0),
//         tex_coord: glam::Vec2::new(1.0, 0.0),
//     },
//     Vertex {
//         position: glam::Vec3::new(0.5, -0.5, -0.5),
//         color: glam::Vec3::new(0.0, 1.0, 0.0),
//         tex_coord: glam::Vec2::new(0.0, 0.0),
//     },
//     Vertex {
//         position: glam::Vec3::new(0.5, 0.5, -0.5),
//         color: glam::Vec3::new(0.0, 0.0, 1.0),
//         tex_coord: glam::Vec2::new(0.0, 1.0),
//     },
//     Vertex {
//         position: glam::Vec3::new(-0.5, 0.5, -0.5),
//         color: glam::Vec3::new(1.0, 1.0, 1.0),
//         tex_coord: glam::Vec2::new(1.0, 1.0),
//     },
// ];

// #[rustfmt::skip]
// const INDICES: [u32; 12] = [
//     0, 1, 2, 2, 3, 0,
//     4, 5, 6, 6, 7, 4,
// ];

impl Vertex {
    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            // position
            vk::VertexInputAttributeDescription::default()
                // the binding in glsl; matched with other vulkan structs as well
                .binding(0)
                // this is the location in glsl
                .location(0)
                // color formats are also used to define non-color vec sizes 1-4
                // (the official tutorial is mildly apologetic)
                // BUT this does matter for defaults -
                // if there aren't enough components here to fill the components shader-side,
                // the 'color' components default to 0 and 'alpha' component defaults to 1
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(std::mem::offset_of!(Vertex, position) as u32),
            // color
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(std::mem::offset_of!(Vertex, color) as u32),
            // texture coordinate
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(std::mem::offset_of!(Vertex, tex_coord) as u32),
        ]
    }
}

fn create_vertex_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    vertices: &[Vertex],
) -> Result<(vk::Buffer, vk::DeviceMemory), BoxError> {
    let buffer_size = std::mem::size_of_val(vertices) as u64;
    let (staging_buffer, staging_buffer_memory) = create_memory_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    unsafe {
        let mapped_dst =
            device.map_memory(staging_buffer_memory, 0, buffer_size, Default::default())?
                as *mut Vertex;
        std::ptr::copy_nonoverlapping(vertices.as_ptr(), mapped_dst, vertices.len());
        device.unmap_memory(staging_buffer_memory);
    };

    let (vertex_buffer, vertex_buffer_memory) = create_memory_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    copy_memory_buffer(
        device,
        command_pool,
        staging_buffer,
        vertex_buffer,
        buffer_size,
        graphics_queue,
    )?;

    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }

    Ok((vertex_buffer, vertex_buffer_memory))
}

fn create_index_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    indices: &[u32],
) -> Result<(vk::Buffer, vk::DeviceMemory), BoxError> {
    let buffer_size = std::mem::size_of_val(indices) as u64;
    let (staging_buffer, staging_buffer_memory) = create_memory_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    unsafe {
        let mapped_dst =
            device.map_memory(staging_buffer_memory, 0, buffer_size, Default::default())?
                as *mut u32;
        std::ptr::copy_nonoverlapping(indices.as_ptr(), mapped_dst, indices.len());
        device.unmap_memory(staging_buffer_memory);
    };

    let (index_buffer, index_buffer_memory) = create_memory_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    copy_memory_buffer(
        device,
        command_pool,
        staging_buffer,
        index_buffer,
        buffer_size,
        graphics_queue,
    )?;

    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }
    Ok((index_buffer, index_buffer_memory))
}

fn copy_memory_buffer(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    src_buffer: vk::Buffer,
    dst_buffer: vk::Buffer,
    size: vk::DeviceSize,
    graphics_queue: vk::Queue,
) -> Result<(), BoxError> {
    // NOTE it would be better to have a second command pool for transfers,
    // that also uses 'create transient'
    let command_buffer = begin_single_time_commands(device, command_pool)?;

    let regions = [vk::BufferCopy::default().size(size)];
    unsafe { device.cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, &regions) };

    end_single_time_commands(device, command_pool, graphics_queue, command_buffer)?;

    Ok(())
}

fn create_memory_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    buffer_size: vk::DeviceSize,
    buffer_usage: vk::BufferUsageFlags,
    memory_property_flags: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory), BoxError> {
    let buffer_create_info = vk::BufferCreateInfo::default()
        .size(buffer_size)
        .usage(buffer_usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };

    let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

    let mem_type_index = find_memory_type_index(
        instance,
        physical_device,
        memory_requirements.memory_type_bits,
        memory_property_flags,
    )?;
    let allocate_info = vk::MemoryAllocateInfo::default()
        .allocation_size(memory_requirements.size)
        .memory_type_index(mem_type_index as u32);
    let buffer_memory = unsafe { device.allocate_memory(&allocate_info, None)? };

    unsafe {
        device.bind_buffer_memory(buffer, buffer_memory, 0)?;
    };

    Ok((buffer, buffer_memory))
}

fn find_memory_type_index(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    memory_type_bits: u32,
    required_properties: vk::MemoryPropertyFlags,
) -> Result<usize, BoxError> {
    let memory_properties =
        unsafe { instance.get_physical_device_memory_properties(physical_device) };

    for (i, mem_type) in memory_properties.memory_types.iter().enumerate() {
        let matches_type_filter = (memory_type_bits & (1 << i)) != 0;
        let has_required_properties =
            (mem_type.property_flags & required_properties) == required_properties;

        if matches_type_filter && has_required_properties {
            return Ok(i);
        }
    }

    Err("failed to find suitable memory type".into())
}

#[allow(unused)] // mapped directly to gpu
struct UniformBufferObject {
    model: glam::Mat4,
    view: glam::Mat4,
    projection: glam::Mat4,
}

fn create_descriptor_set_layout(device: &ash::Device) -> Result<vk::DescriptorSetLayout, BoxError> {
    let ubo_layout_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let sampler_layout_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = [ubo_layout_binding, sampler_layout_binding];
    let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

    let descriptor_set_layout = unsafe { device.create_descriptor_set_layout(&create_info, None)? };

    Ok(descriptor_set_layout)
}

fn create_uniform_buffers(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
) -> Result<
    (
        Vec<vk::Buffer>,
        Vec<vk::DeviceMemory>,
        Vec<*mut UniformBufferObject>,
    ),
    BoxError,
> {
    let buffer_size = std::mem::size_of::<UniformBufferObject>() as vk::DeviceSize;

    let mut uniform_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    let mut uniform_buffers_memory = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    let mut uniform_buffers_mapped = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        let (buffer, memory) = create_memory_buffer(
            instance,
            device,
            physical_device,
            buffer_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        uniform_buffers.push(buffer);
        uniform_buffers_memory.push(memory);

        let mapped = unsafe {
            device.map_memory(memory, 0, buffer_size, Default::default())?
                as *mut UniformBufferObject
        };

        uniform_buffers_mapped.push(mapped);
    }

    Ok((
        uniform_buffers,
        uniform_buffers_memory,
        uniform_buffers_mapped,
    ))
}

fn update_uniform_buffer(
    start_time: Instant,
    image_extent: vk::Extent2D,
    mapped_uniform_buffer: *mut UniformBufferObject,
) -> Result<(), BoxError> {
    const DEGREES_PER_SECOND: f32 = 30.0;
    let elapsed_seconds = (Instant::now() - start_time).as_secs_f32();
    let turn_radians = elapsed_seconds * DEGREES_PER_SECOND.to_radians();

    let model = glam::Mat4::IDENTITY * glam::Mat4::from_rotation_z(turn_radians);
    let view = glam::Mat4::look_at_rh(
        glam::Vec3::splat(2.0),
        glam::Vec3::splat(0.0),
        glam::Vec3::new(0.0, 0.0, 1.0),
    );
    let aspect_ratio = image_extent.width as f32 / image_extent.height as f32;
    let projection = glam::Mat4::perspective_rh(45.0_f32.to_radians(), aspect_ratio, 0.1, 10.0);

    let mut ubo = UniformBufferObject {
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
    ubo.projection.y_axis.y *= -1.0;

    unsafe {
        std::ptr::copy_nonoverlapping(&ubo, mapped_uniform_buffer, 1);
    }

    Ok(())
}

fn create_descriptor_pool(device: &ash::Device) -> Result<vk::DescriptorPool, BoxError> {
    let ubo_pool_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32);
    let sampler_pool_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32);

    let pool_sizes = [ubo_pool_size, sampler_pool_size];
    let pool_create_info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(&pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT as u32);

    let pool = unsafe { device.create_descriptor_pool(&pool_create_info, None)? };

    Ok(pool)
}

fn create_descriptor_sets(
    device: &ash::Device,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    uniform_buffers: &[vk::Buffer],
    texture_image_view: vk::ImageView,
    texture_sampler: vk::Sampler,
) -> Result<Vec<vk::DescriptorSet>, BoxError> {
    let set_layouts = [descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
    let alloc_info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(descriptor_pool)
        .set_layouts(&set_layouts);

    let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info)? };

    for i in 0..MAX_FRAMES_IN_FLIGHT {
        let buffer = uniform_buffers[i];
        let dst_set = descriptor_sets[i];

        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(buffer)
            .offset(0)
            .range(std::mem::size_of::<UniformBufferObject>() as u64);
        let buffer_info = [buffer_info];
        let ubo_write = vk::WriteDescriptorSet::default()
            .dst_set(dst_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .buffer_info(&buffer_info);

        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(texture_image_view)
            .sampler(texture_sampler);
        let image_info = [image_info];
        let image_write = vk::WriteDescriptorSet::default()
            .dst_set(dst_set)
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .image_info(&image_info);

        let writes = [ubo_write, image_write];

        unsafe { device.update_descriptor_sets(&writes, &[]) };
    }

    Ok(descriptor_sets)
}

fn create_texture_image(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
) -> Result<(vk::Image, vk::DeviceMemory, u32), BoxError> {
    let file_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "textures", "viking_room.png"]
        .iter()
        .collect();

    let image = ImageReader::open(file_path)?.decode()?;
    let expected_size = image.width() * image.height() * 4;
    let bytes = image.to_rgba8().into_raw();

    let mip_levels = image.width().max(image.height()).ilog2() + 1;

    debug_assert!(
        bytes.len() == expected_size as usize,
        "expected rgba bytes size"
    );

    let buffer_size = bytes.len() as u64;
    let (staging_buffer, staging_buffer_memory) = create_memory_buffer(
        instance,
        device,
        physical_device,
        buffer_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    unsafe {
        let mapped_dst =
            device.map_memory(staging_buffer_memory, 0, buffer_size, Default::default())?
                as *mut u8;
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), mapped_dst, bytes.len());
        device.unmap_memory(staging_buffer_memory);
    };

    let extent = vk::Extent2D::default()
        .width(image.width())
        .height(image.height());
    let image_options = ImageOptions {
        extent,
        format: vk::Format::R8G8B8A8_SRGB,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::SAMPLED
            | vk::ImageUsageFlags::TRANSFER_SRC, // for mipmap
        memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        mip_levels,
    };
    let (vk_image, image_memory) =
        create_vk_image(instance, device, physical_device, image_options)?;

    transition_image_layout(
        device,
        command_pool,
        graphics_queue,
        vk_image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        mip_levels,
    )?;

    copy_buffer_to_image(
        device,
        command_pool,
        graphics_queue,
        staging_buffer,
        vk_image,
        extent,
    )?;

    generate_mipmaps(
        device,
        command_pool,
        graphics_queue,
        vk_image,
        (extent.width as i32, extent.height as i32),
        mip_levels,
        instance,
        physical_device,
        vk::Format::R8G8B8A8_SRGB,
    )?;

    // // TODO removed; WIP generate mipmaps
    // transition_image_layout(
    //     device,
    //     command_pool,
    //     graphics_queue,
    //     vk_image,
    //     vk::Format::R8G8B8A8_SRGB,
    //     vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    //     vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    //     mip_levels,
    // )?;

    unsafe {
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    }

    Ok((vk_image, image_memory, mip_levels))
}

struct ImageOptions {
    extent: vk::Extent2D,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    memory_properties: vk::MemoryPropertyFlags,
    mip_levels: u32,
}

fn create_vk_image(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    options: ImageOptions,
) -> Result<(vk::Image, vk::DeviceMemory), BoxError> {
    let image_create_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(options.extent.into())
        .mip_levels(options.mip_levels)
        .array_layers(1)
        .format(options.format)
        .tiling(options.tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(options.usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::TYPE_1);

    let vk_image = unsafe { device.create_image(&image_create_info, None)? };

    let memory_requirements = unsafe { device.get_image_memory_requirements(vk_image) };
    let memory_type_index = find_memory_type_index(
        instance,
        physical_device,
        memory_requirements.memory_type_bits,
        options.memory_properties,
    )?;
    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(memory_requirements.size)
        .memory_type_index(memory_type_index as u32);
    let image_memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    unsafe { device.bind_image_memory(vk_image, image_memory, 0)? };

    Ok((vk_image, image_memory))
}

fn begin_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
) -> Result<vk::CommandBuffer, BoxError> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(command_pool)
        .command_buffer_count(1);
    let command_buffers =
        unsafe { device.allocate_command_buffers(&command_buffer_allocate_info)? };
    let command_buffer = command_buffers[0];

    let begin_info =
        vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    unsafe { device.begin_command_buffer(command_buffer, &begin_info)? };

    Ok(command_buffer)
}

fn end_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    command_buffer: vk::CommandBuffer,
) -> Result<(), BoxError> {
    let command_buffers = [command_buffer];

    unsafe { device.end_command_buffer(command_buffer)? };

    let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
    let submits = [submit_info];
    unsafe {
        device.queue_submit(graphics_queue, &submits, vk::Fence::null())?;
        device.device_wait_idle()?;
    }
    unsafe { device.free_command_buffers(command_pool, &command_buffers) };

    Ok(())
}

fn transition_image_layout(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    image: vk::Image,
    format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    mip_levels: u32,
) -> Result<(), BoxError> {
    let command_buffer = begin_single_time_commands(device, command_pool)?;

    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(mip_levels)
        .base_array_layer(0)
        .layer_count(1);
    let mut barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(subresource_range);

    let src_stage_mask: vk::PipelineStageFlags;
    let dst_stage_mask: vk::PipelineStageFlags;

    match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => {
            barrier.src_access_mask = Default::default();
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;

            src_stage_mask = vk::PipelineStageFlags::TOP_OF_PIPE;
            dst_stage_mask = vk::PipelineStageFlags::TRANSFER;
        }

        (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            src_stage_mask = vk::PipelineStageFlags::TRANSFER;
            dst_stage_mask = vk::PipelineStageFlags::FRAGMENT_SHADER;
        }

        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL) => {
            barrier.subresource_range.aspect_mask = vk::ImageAspectFlags::DEPTH;

            if has_stencil_component(format) {
                barrier.subresource_range.aspect_mask |= vk::ImageAspectFlags::STENCIL;
            }

            barrier.src_access_mask = Default::default();
            barrier.dst_access_mask = vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;

            src_stage_mask = vk::PipelineStageFlags::TOP_OF_PIPE;
            dst_stage_mask = vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
        }

        transition => {
            return Err(format!("layout transition: {transition:?} not supported").into());
        }
    }

    // https://docs.vulkan.org/spec/latest/chapters/synchronization.html#synchronization-access-types-supported
    let image_barriers = [barrier];
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage_mask,
            dst_stage_mask,
            Default::default(),
            &[],
            &[],
            &image_barriers,
        )
    };

    end_single_time_commands(device, command_pool, graphics_queue, command_buffer)?;

    Ok(())
}

fn copy_buffer_to_image(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    buffer: vk::Buffer,
    image: vk::Image,
    extent: vk::Extent2D,
) -> Result<(), BoxError> {
    let command_buffer = begin_single_time_commands(device, command_pool)?;

    let image_subresource = vk::ImageSubresourceLayers::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1);

    let region = vk::BufferImageCopy::default()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(image_subresource)
        .image_offset(vk::Offset3D::default())
        .image_extent(extent.into());

    unsafe {
        let regions = [region];
        device.cmd_copy_buffer_to_image(
            command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &regions,
        )
    };

    end_single_time_commands(device, command_pool, graphics_queue, command_buffer)?;

    Ok(())
}

fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
    aspect_mask: vk::ImageAspectFlags,
    mip_levels: u32,
) -> Result<vk::ImageView, BoxError> {
    let components = vk::ComponentMapping::default()
        // NOTE these are the default
        .r(vk::ComponentSwizzle::IDENTITY)
        .g(vk::ComponentSwizzle::IDENTITY)
        .b(vk::ComponentSwizzle::IDENTITY)
        .a(vk::ComponentSwizzle::IDENTITY);

    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(aspect_mask)
        .base_mip_level(0)
        .level_count(mip_levels)
        .base_array_layer(0)
        .layer_count(1);

    let create_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .components(components)
        .subresource_range(subresource_range);

    let image_view = unsafe { device.create_image_view(&create_info, None)? };

    Ok(image_view)
}

fn create_texture_sampler(
    device: &ash::Device,
    physical_device_properties: vk::PhysicalDeviceProperties,
) -> Result<vk::Sampler, BoxError> {
    let max_anisotropy = physical_device_properties.limits.max_sampler_anisotropy;
    let create_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(true)
        .max_anisotropy(max_anisotropy)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(vk::LOD_CLAMP_NONE);

    let sampler = unsafe { device.create_sampler(&create_info, None)? };

    Ok(sampler)
}

fn create_depth_image(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    swapchain_extent: vk::Extent2D,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView), BoxError> {
    let depth_format = find_depth_format(instance, physical_device);

    let mip_levels = 1;

    let image_options = ImageOptions {
        extent: swapchain_extent,
        format: depth_format,
        tiling: vk::ImageTiling::OPTIMAL,
        usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        memory_properties: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        mip_levels,
    };

    let (depth_image, depth_image_memory) =
        create_vk_image(instance, device, physical_device, image_options)?;

    let depth_image_view = create_image_view(
        device,
        depth_image,
        depth_format,
        vk::ImageAspectFlags::DEPTH,
        mip_levels,
    )?;

    transition_image_layout(
        device,
        command_pool,
        graphics_queue,
        depth_image,
        depth_format,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        mip_levels,
    )?;

    Ok((depth_image, depth_image_memory, depth_image_view))
}

// VkFormat findSupportedFormat(const std::vector<VkFormat>& candidates,
// VkImageTiling tiling, VkFormatFeatureFlags features) {

fn find_supported_format(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    candidates: &[vk::Format],
    tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Option<vk::Format> {
    for &format in candidates {
        let format_properties =
            unsafe { instance.get_physical_device_format_properties(physical_device, format) };

        if tiling == vk::ImageTiling::LINEAR
            && (format_properties.linear_tiling_features & features) == features
        {
            return Some(format);
        }

        if tiling == vk::ImageTiling::OPTIMAL
            && (format_properties.optimal_tiling_features & features) == features
        {
            return Some(format);
        }
    }

    None
}

fn find_depth_format(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> vk::Format {
    let candidates = [
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];
    let tiling = vk::ImageTiling::OPTIMAL;
    let features = vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT;

    find_supported_format(instance, physical_device, &candidates, tiling, features)
        .expect("no supported depth format available")
}

fn has_stencil_component(format: vk::Format) -> bool {
    [
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ]
    .contains(&format)
}

// From unknownue's rust version
// https://github.com/unknownue/vulkan-tutorial-rust/blob/master/src/tutorials/27_model_loading.rs
fn load_model() -> Result<(Vec<Vertex>, Vec<u32>), BoxError> {
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

fn generate_mipmaps(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    image: vk::Image,
    tex_extent: (i32, i32),
    mip_levels: u32,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    format: vk::Format,
) -> Result<(), BoxError> {
    let format_properties =
        unsafe { instance.get_physical_device_format_properties(physical_device, format) };
    let linear_blit_support = format_properties
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR);
    if !linear_blit_support {
        return Err("no linear blitting support".into());
    }

    let command_buffer = begin_single_time_commands(device, command_pool)?;

    // base reused barrier values
    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_array_layer(0)
        .layer_count(1)
        .level_count(1);
    let mut barrier = vk::ImageMemoryBarrier::default()
        .image(image)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .subresource_range(subresource_range);

    // record blit commands
    let mut mip_width = tex_extent.0;
    let mut mip_height = tex_extent.1;
    for i in 1..mip_levels {
        barrier.subresource_range.base_mip_level = i - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

        unsafe {
            let image_memory_barriers = [barrier];
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                Default::default(),
                &[],
                &[],
                &image_memory_barriers,
            )
        };

        let src_subresource = vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(i - 1)
            .base_array_layer(0)
            .layer_count(1);
        let dst_subresource = vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(i)
            .base_array_layer(0)
            .layer_count(1);
        let blit = vk::ImageBlit::default()
            .src_offsets([
                vk::Offset3D::default(),
                vk::Offset3D::default().x(mip_width).y(mip_height).z(1),
            ])
            .src_subresource(src_subresource)
            .dst_offsets([
                vk::Offset3D::default(),
                vk::Offset3D::default()
                    .x(if mip_width > 1 { mip_width / 2 } else { 1 })
                    .y(if mip_height > 1 { mip_height / 2 } else { 1 })
                    .z(1),
            ])
            .dst_subresource(dst_subresource);

        unsafe {
            let regions = [blit];
            device.cmd_blit_image(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
                vk::Filter::LINEAR,
            )
        };

        barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
        unsafe {
            let image_memory_barriers = &[barrier];
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                Default::default(),
                &[],
                &[],
                image_memory_barriers,
            )
        };

        if mip_width > 1 {
            mip_width /= 2;
        }

        if mip_height > 1 {
            mip_height /= 2;
        }
    }

    barrier.subresource_range.base_mip_level = mip_levels - 1;
    barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
    barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
    barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
    barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

    let image_memory_barriers = [barrier];
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            Default::default(),
            &[],
            &[],
            &image_memory_barriers,
        )
    };

    end_single_time_commands(device, command_pool, graphics_queue, command_buffer)?;

    Ok(())
}
