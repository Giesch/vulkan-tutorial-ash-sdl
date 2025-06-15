use std::collections::BTreeSet;
use std::ffi::{c_char, CStr, CString};
use std::fs::File;
use std::path::PathBuf;

use ash::vk;
use sdl3::sys::vulkan::SDL_Vulkan_DestroySurface;
use sdl3::video::Window;

use super::BoxError;

mod debug;
mod platform;

// TODO replace this with env var
// https://github.com/ash-rs/ash/issues/190#issuecomment-758269723
// use separate config option for logging; maybe tracing env vars
const ENABLE_VALIDATION: bool = cfg!(debug_assertions);

// TODO load all extensions / device extensions at the start

pub struct Renderer {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_ext: vk::DebugUtilsMessengerEXT,
    debug_loader: ash::ext::debug_utils::Instance,
    surface: vk::SurfaceKHR,
    device: ash::Device,
    graphics_queue: vk::Queue,
    presentation_queue: vk::Queue,
    swapchain_device_ext: ash::khr::swapchain::Device,
    image_format: vk::Format,
    image_extent: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    pipeline_layout: vk::PipelineLayout,
}

impl Renderer {
    pub fn init(window: &Window) -> Result<Self, BoxError> {
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
        let (debug_loader, debug_ext) = debug::maybe_create_debug_messager_extension(
            ENABLE_VALIDATION,
            &entry,
            &instance,
            &debug_create_info,
        );

        let surface_ext = ash::khr::surface::Instance::new(&entry, &instance);

        let surface = window.vulkan_create_surface(instance.handle())?;

        let (physical_device, queue_family_indices) =
            choose_physical_device(&instance, &surface_ext, surface)?;
        let device = create_logical_device(&instance, physical_device, &queue_family_indices)?;

        let graphics_queue = unsafe { device.get_device_queue(queue_family_indices.graphics, 0) };
        let presentation_queue =
            unsafe { device.get_device_queue(queue_family_indices.presentation, 0) };

        let CreatedSwapchain {
            swapchain_device_ext,
            swapchain,
            image_format,
            image_extent,
        } = create_swapchain(
            &instance,
            &device,
            window,
            &surface_ext,
            surface,
            physical_device,
        )?;

        let swapchain_images = unsafe { swapchain_device_ext.get_swapchain_images(swapchain)? };

        let mut swapchain_image_views = Vec::with_capacity(swapchain_images.len());
        for image in &swapchain_images {
            let components = vk::ComponentMapping::default()
                .r(vk::ComponentSwizzle::IDENTITY)
                .g(vk::ComponentSwizzle::IDENTITY)
                .b(vk::ComponentSwizzle::IDENTITY)
                .a(vk::ComponentSwizzle::IDENTITY);

            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let create_info = vk::ImageViewCreateInfo::default()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(image_format)
                .components(components)
                .subresource_range(subresource_range);

            let image_view = unsafe { device.create_image_view(&create_info, None)? };
            swapchain_image_views.push(image_view);
        }

        // TODO create graphics pipeline

        let pipeline_layout = create_graphics_pipeline(&device)?;

        Ok(Self {
            entry,
            instance,
            debug_ext,
            debug_loader,
            surface,
            device,
            graphics_queue,
            presentation_queue,
            swapchain_device_ext,
            image_format,
            image_extent,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            pipeline_layout,
        })
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);

            for image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }

            // NOTE this also frees the images
            self.swapchain_device_ext
                .destroy_swapchain(self.swapchain, None);

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

/// converts a null-terminated c string from vulkan
/// to non-nul bytes for comparison with CStr constants
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
) -> Result<(vk::PhysicalDevice, QueueFamilyIndices), BoxError> {
    let physical_devices: Vec<vk::PhysicalDevice> =
        unsafe { instance.enumerate_physical_devices()? };

    // this corresponds to the tutorial's 'isDeviceSuitable'
    let mut devices_with_indices = vec![];
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

        devices_with_indices.push((physical_device, indices));
    }

    devices_with_indices.sort_by_key(|(physical_device, _indices)| {
        let props: vk::PhysicalDeviceProperties =
            unsafe { instance.get_physical_device_properties(*physical_device) };

        match props.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => 0,
            vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
            vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
            vk::PhysicalDeviceType::CPU => 3,
            vk::PhysicalDeviceType::OTHER => 4,
            _ => 5,
        }
    });

    let Some(chosen_device) = devices_with_indices.into_iter().next() else {
        return Err("no graphics device availble".into());
    };

    Ok(chosen_device)
}

const PREFERRED_SURFACE_FORMAT: vk::SurfaceFormatKHR = vk::SurfaceFormatKHR {
    format: vk::Format::B8G8R8A8_SRGB,
    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
};

fn choose_swap_surface_format(
    available_formats: &[vk::SurfaceFormatKHR],
) -> Option<vk::SurfaceFormatKHR> {
    if available_formats.contains(&PREFERRED_SURFACE_FORMAT) {
        return Some(PREFERRED_SURFACE_FORMAT);
    }

    available_formats.first().copied()
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
    swapchain_device_ext: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    image_format: vk::Format,
    image_extent: vk::Extent2D,
}

fn create_swapchain(
    instance: &ash::Instance,
    device: &ash::Device,
    window: &Window,
    surface_ext: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
) -> Result<CreatedSwapchain, BoxError> {
    let swapchain_support = SwapChainSupportDetails::query(surface_ext, surface, physical_device)?;

    // TODO avoid this unwrap; get this during the empty check
    let surface_format = choose_swap_surface_format(&swapchain_support.formats).unwrap();
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

    // TODO avoid this unwrap; pass it in
    let indices =
        QueueFamilyIndices::find(instance, surface_ext, surface, physical_device)?.unwrap();
    let queue_family_indices = [indices.graphics, indices.presentation];

    let create_info = if indices.graphics != indices.presentation {
        // different queue families; the uncommon case
        // the tutorial recommends avoiding concurrent sharing mode if possible
        // but this involves the ownership portion of the vulkan API
        create_info
            .image_sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&queue_family_indices)
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

    let swapchain_device_ext = ash::khr::swapchain::Device::new(instance, device);

    let swapchain = unsafe { swapchain_device_ext.create_swapchain(&create_info, None)? };

    Ok(CreatedSwapchain {
        swapchain_device_ext,
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

    let features = vk::PhysicalDeviceFeatures::default();

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

        let present_modes = unsafe {
            surface_ext.get_physical_device_surface_present_modes(physical_device, surface)?
        };

        Ok(Self {
            capabilities,
            formats,
            present_modes,
        })
    }
}

fn create_graphics_pipeline(device: &ash::Device) -> Result<vk::PipelineLayout, BoxError> {
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
    let shader_stages = [vert_create_info, frag_create_info];

    let dynamic_states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state_create_info =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // relying on dynamic state to fill these in during draw
    let mut viewport_state = vk::PipelineViewportStateCreateInfo::default();
    viewport_state.viewport_count = 1;
    viewport_state.scissor_count = 1;

    let rasterizer_state = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    // color blend per attached framebuffer
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(false)
        .color_write_mask(vk::ColorComponentFlags::RGBA);
    // global color blending
    let color_blend_create_info = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(&[color_blend_attachment]);

    let pipeline_layout = vk::PipelineLayout::default();

    unsafe { device.destroy_shader_module(frag_shader, None) };
    unsafe { device.destroy_shader_module(vert_shader, None) };

    Ok(pipeline_layout)
}

/// usage: read_shader_spv("triangle.vert.spv");
fn read_shader_spv(shader_name: &str) -> Result<Vec<u32>, BoxError> {
    let shader_path: PathBuf = [
        std::env!("CARGO_MANIFEST_DIR"),
        "shaders",
        "compiled",
        shader_name,
    ]
    .iter()
    .collect();

    let mut spv_file = File::open(&shader_path)?;
    let vk_bytes = ash::util::read_spv(&mut spv_file)?;

    Ok(vk_bytes)
}
