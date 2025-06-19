use ash::vk;

use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;

use ash::ext::debug_utils;

// based on the logging callback here:
// https://github.com/unknownue/vulkan-tutorial-rust/blob/master/src/utility/debug.rs#L8

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let message_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };

    let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };
    let message = message.to_string_lossy();

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            tracing::debug!("{message_type}, {message}");
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            tracing::warn!("{message_type}, {message}");
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            tracing::error!("{message_type}, {message}");
        }
        _info_or_other => {
            tracing::info!("{message_type} {message}");
        }
    };

    vk::FALSE
}

pub fn maybe_create_debug_messager_extension(
    entry: &ash::Entry,
    vk_instance: &ash::Instance,
    messenger_create_info: &vk::DebugUtilsMessengerCreateInfoEXT<'static>,
) -> (debug_utils::Instance, vk::DebugUtilsMessengerEXT) {
    let loader = debug_utils::Instance::new(entry, vk_instance);
    if !super::ENABLE_VALIDATION {
        return (loader, vk::DebugUtilsMessengerEXT::null());
    }

    let extension = unsafe {
        loader
            .create_debug_utils_messenger(messenger_create_info, None)
            .expect("failed to create Debug Utils extension messenger")
    };

    (loader, extension)
}

pub fn build_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        p_next: ptr::null(),
        flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
            // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
            // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        pfn_user_callback: Some(vulkan_debug_utils_callback),
        p_user_data: ptr::null_mut(),
        _marker: PhantomData,
    }
}
