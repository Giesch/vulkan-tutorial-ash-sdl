use std::ffi::{c_char, CString};

use ash::{
    vk::{self, PhysicalDeviceType},
    Entry,
};
use sdl3::video::Window;

use super::BoxError;

mod debug;

pub struct Renderer {
    entry: ash::Entry,
    instance: ash::Instance,
    debug_ext: vk::DebugUtilsMessengerEXT,
    debug_loader: ash::ext::debug_utils::Instance,
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

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            use ash::khr;
            enabled_extension_names.push(khr::portability_enumeration::NAME.as_ptr());
            enabled_extension_names.push(khr::get_physical_device_properties2::NAME.as_ptr());
        }

        let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };

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

        let chosen_device = choose_physical_device(&instance)?;

        // let surface = window.vulkan_create_surface(instance.handle())?;

        Ok(Self {
            entry,
            instance,
            debug_ext,
            debug_loader,
        })
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if ENABLE_VALIDATION {
            unsafe {
                self.debug_loader
                    .destroy_debug_utils_messenger(self.debug_ext, None);
            }
        }

        unsafe { self.instance.destroy_instance(None) };
    }
}

const ENABLE_VALIDATION: bool = cfg!(debug_assertions);

fn get_required_layers() -> Vec<&'static std::ffi::CStr> {
    if ENABLE_VALIDATION {
        vec![c"VK_LAYER_KHRONOS_validation"]
    } else {
        vec![]
    }
}

fn check_required_layers(entry: &Entry) -> Result<(), BoxError> {
    let required_layers = get_required_layers();
    let available_layers = unsafe { entry.enumerate_instance_layer_properties()? };

    for required_layer in required_layers {
        let mut found = false;
        for prop in &available_layers {
            let layer_name = vk_str_bytes(&prop.layer_name);
            if &layer_name == required_layer.to_bytes() {
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

fn check_required_extensions(entry: &Entry) -> Result<(), BoxError> {
    let mut required_extensions = vec![ash::khr::surface::NAME];

    if cfg!(target_os = "linux") {
        required_extensions.push(ash::khr::xlib_surface::NAME);
    }
    if cfg!(target_os = "macos") {
        required_extensions.push(ash::mvk::macos_surface::NAME);
    }
    if cfg!(target_os = "windows") {
        required_extensions.push(ash::khr::win32_surface::NAME);
    }

    if cfg!(debug_assertions) {
        required_extensions.push(ash::ext::debug_utils::NAME);
    }

    let available_extensions = unsafe { entry.enumerate_instance_extension_properties(None)? };

    for required_ext in &required_extensions {
        let mut found = false;
        for prop in &available_extensions {
            let ext_name: Vec<u8> = vk_str_bytes(&prop.extension_name);
            if &ext_name == required_ext.to_bytes() {
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

fn vk_str_bytes(vk_str: &[c_char]) -> Vec<u8> {
    vk_str
        .iter()
        .map(|byte| *byte as u8)
        .take_while(|byte| *byte != b'\0')
        .collect()
}

struct QueueFamilyIndicies {
    /// the index of the first queue family that supports graphics
    graphics_family: Option<u32>,
}

impl QueueFamilyIndicies {
    fn find(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> Self {
        let mut indicies = Self {
            graphics_family: None,
        };

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        for (i, family) in queue_families.iter().enumerate() {
            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indicies.graphics_family = Some(i as u32);
                return indicies;
            }
        }

        indicies
    }
}

fn choose_physical_device(instance: &ash::Instance) -> Result<vk::PhysicalDevice, BoxError> {
    let mut physical_devices: Vec<vk::PhysicalDevice> =
        unsafe { instance.enumerate_physical_devices()? };

    physical_devices.retain(|physical_device| {
        let indicies = QueueFamilyIndicies::find(&instance, *physical_device);
        indicies.graphics_family.is_some()
    });

    physical_devices.sort_by_key(|physical_device| {
        let props: vk::PhysicalDeviceProperties =
            unsafe { instance.get_physical_device_properties(*physical_device) };

        match props.device_type {
            PhysicalDeviceType::DISCRETE_GPU => 0,
            PhysicalDeviceType::INTEGRATED_GPU => 1,
            PhysicalDeviceType::VIRTUAL_GPU => 2,
            PhysicalDeviceType::CPU => 3,
            PhysicalDeviceType::OTHER => 4,
            _ => 5,
        }
    });

    let Some(chosen_device) = physical_devices.into_iter().next() else {
        return Err("no graphics device availble".into());
    };

    Ok(chosen_device)
}
