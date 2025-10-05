use serde::{Deserialize, Serialize};

mod parameters;
pub use parameters::*;

mod pipeline_builders;
pub use pipeline_builders::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectionJson {
    pub source_file_name: String,
    pub global_parameters: Vec<GlobalParameter>,
    pub vertex_entry_point: EntryPoint,
    pub fragment_entry_point: EntryPoint,
    pub pipeline_layout: ReflectedPipelineLayout,
}
