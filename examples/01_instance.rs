use std::ffi::CString;
use std::time::Duration;

use ash::vk;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::sys::timer::SDL_DelayPrecise;
use sdl3::video::Window;
use sdl3::EventPump;

const WINDOW_TITLE: &str = "Basic Window";
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const FRAME_DELAY: Duration = Duration::from_millis(15);

type BoxError = Box<dyn std::error::Error>;

fn main() -> Result<(), BoxError> {
    let sdl = sdl3::init()?;
    let video_subsystem = sdl.video()?;

    let window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .resizable()
        .position_centered()
        .build()?;

    let renderer = Renderer::init(&window)?;
    let mut app = App {
        quit: false,
        renderer,
    };

    let mut event_pump = sdl.event_pump()?;
    loop {
        app.handle_events(&mut event_pump);
        if app.quit {
            break;
        }

        unsafe { SDL_DelayPrecise(FRAME_DELAY.as_nanos() as u64) };
    }

    Ok(())
}

struct App {
    quit: bool,
    renderer: Renderer,
}

impl App {
    fn handle_events(&mut self, event_pump: &mut EventPump) {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    self.quit = true;
                    return;
                }

                _ => {}
            }
        }
    }
}

struct Renderer {
    entry: ash::Entry,
    instance: ash::Instance,
}

impl Renderer {
    fn init(window: &Window) -> Result<Self, BoxError> {
        let entry = unsafe { ash::Entry::load().map_err(|e| e.to_string())? };

        // checking for available extensions:
        // let extension_properties = unsafe { entry.enumerate_instance_extension_properties(None)? };
        // dbg!(&extension_properties);

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Vulkan Tutorial")
            .engine_name(c"No Engine")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            // .api_version(vk::API_VERSION_1_3);
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

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_extension_names)
            .flags(create_flags);

        let instance = unsafe { entry.create_instance(&create_info, None)? };
        // let surface = window.vulkan_create_surface(instance.handle())?;

        Ok(Self { entry, instance })
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}
