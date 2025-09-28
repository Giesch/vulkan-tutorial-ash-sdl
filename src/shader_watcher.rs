use std::sync::mpsc;

use log::*;
use notify::{Event, RecursiveMode, Watcher};

use crate::util::*;

pub struct ShaderChanges {
    #[expect(unused)]
    watcher: notify::RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<Event>>,
}

impl ShaderChanges {
    pub fn events(&mut self) -> std::result::Result<Vec<notify::Event>, BoxError> {
        let edit_events: notify::Result<Vec<notify::Event>> = self.receiver.try_iter().collect();
        let mut edit_events = edit_events?;

        edit_events.retain(|event| match event.kind {
            notify::EventKind::Create(_) => true,
            notify::EventKind::Modify(_) => true,
            notify::EventKind::Remove(_) => true,

            notify::EventKind::Access(_) => false,

            notify::EventKind::Any => {
                error!("unexpected notify event: {event:?}");
                false
            }
            notify::EventKind::Other => {
                error!("unexpected notify event: {event:?}");
                false
            }
        });

        Ok(edit_events)
    }
}

pub fn watch() -> notify::Result<ShaderChanges> {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx)?;

    let path = manifest_path(["shaders", "source"]);

    watcher.watch(&path, RecursiveMode::Recursive)?;

    // // Block forever, printing out events as they come in
    // for res in rx {
    //     match res {
    //         Ok(event) => println!("event: {:?}", event),
    //         Err(e) => println!("watch error: {:?}", e),
    //     }
    // }

    let shader_changes = ShaderChanges {
        watcher,
        receiver: rx,
    };

    Ok(shader_changes)
}
