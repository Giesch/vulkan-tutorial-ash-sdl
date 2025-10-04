use shader_slang as slang;

use super::json::*;

mod entry_points;
use entry_points::*;

mod pipeline_layout;
use pipeline_layout::*;

pub fn reflection_json(
    source_file_name: &str,
    program_layout: &slang::reflection::Shader,
) -> anyhow::Result<ReflectionJson> {
    let entry_points = reflect_entry_points(program_layout)?;

    let pipeline_layout = reflect_pipeline_layout(program_layout);

    let reflection_json = ReflectionJson {
        source_file_name: source_file_name.to_string(),
        vertex_entry_point: entry_points.vertex_entry_point,
        fragment_entry_point: entry_points.fragment_entry_point,
        pipeline_layout,
    };

    Ok(reflection_json)
}
