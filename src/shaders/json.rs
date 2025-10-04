use serde::{Deserialize, Serialize};

mod entry_points;
pub use entry_points::*;

mod pipeline_builders;
pub use pipeline_builders::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectionJson {
    pub source_file_name: String,
    pub vertex_entry_point: EntryPoint,
    pub fragment_entry_point: EntryPoint,
    pub pipeline_layout: ReflectedPipelineLayout,
}
