use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::EventPump;

use crate::Renderer;

pub struct App {
    pub quit: bool,
    pub renderer: Renderer,
}

impl App {
    pub fn handle_events(&mut self, event_pump: &mut EventPump) {
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
