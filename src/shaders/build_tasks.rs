use std::path::PathBuf;

use askama::Template;
use heck::ToSnakeCase;

use crate::util::*;

use super::{ReflectedShader, json::*, prepare_reflected_shader};

pub fn write_precompiled_shaders() -> Result<(), anyhow::Error> {
    let shaders_source_dir = manifest_path(["shaders", "source"]);
    let slang_file_names: Vec<_> = std::fs::read_dir(shaders_source_dir)?
        .filter_map(|entry_res| entry_res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "slang"))
        .filter_map(|path| {
            path.file_name()
                .and_then(|os_str| os_str.to_str())
                .map(|s| s.to_string())
        })
        .collect();

    for slang_file_name in &slang_file_names {
        let ReflectedShader {
            vertex_shader,
            fragment_shader,
            reflection_json,
        } = prepare_reflected_shader(slang_file_name)?;

        let generated_files = build_generated_source_files(&reflection_json);
        for source_file in &generated_files {
            std::fs::create_dir_all(source_file.file_path.parent().unwrap())?;
            std::fs::write(&source_file.file_path, &source_file.content)?;
        }

        let source_file_name = &reflection_json.source_file_name;

        let compiled_shaders_dir = manifest_path(["shaders", "compiled"]);
        std::fs::create_dir_all(&compiled_shaders_dir)?;

        let reflection_json = serde_json::to_string_pretty(&reflection_json)?;
        let reflection_json_file_name = source_file_name.replace(".slang", ".json");
        let json_path = manifest_path(["shaders", "compiled", &reflection_json_file_name]);
        std::fs::write(json_path, reflection_json)?;

        let spv_vert_file_name = source_file_name.replace(".slang", ".vert.spv");
        let vert_path = manifest_path(["shaders", "compiled", &spv_vert_file_name]);
        std::fs::write(vert_path, vertex_shader.shader_bytecode.as_slice())?;

        let spv_frag_file_name = source_file_name.replace(".slang", ".frag.spv");
        let frag_path = manifest_path(["shaders", "compiled", &spv_frag_file_name]);
        std::fs::write(frag_path, fragment_shader.shader_bytecode.as_slice())?;
    }

    Ok(())
}

fn build_generated_source_files(reflection_json: &ReflectionJson) -> Vec<GeneratedFile> {
    let mut struct_defs = vec![];
    let mut vertex_impl_blocks = vec![];

    for vert_param in &reflection_json.vertex_entry_point.parameters {
        match vert_param {
            EntryPointParameter::Scalar(ScalarEntryPointParameter::Semantic(_)) => {}
            EntryPointParameter::Scalar(ScalarEntryPointParameter::Bound(_)) => todo!(),

            EntryPointParameter::Struct(struct_param) => {
                let mut generated_fields = vec![];
                for field in &struct_param.fields {
                    let Some(generated_field) = gather_struct_defs(field, &mut struct_defs) else {
                        continue;
                    };
                    generated_fields.push(generated_field);
                }

                let def = GeneratedStructDefinition {
                    type_name: struct_param.type_name.to_string(),
                    fields: generated_fields,
                };

                let mut attribute_descriptions = vec![];
                for (location, field) in def.fields.iter().enumerate() {
                    let format = match field.type_name.as_str() {
                        // TODO use an enum for supported glam types
                        "glam::Vec3" => "ash::vk::Format::R32G32B32_SFLOAT",
                        "glam::Vec2" => "ash::vk::Format::R32G32_SFLOAT",
                        _ => todo!(),
                    };

                    let attr = VertexAttributeDescription {
                        field_name: field.field_name.to_snake_case(),
                        format: format.to_string(),
                        location,
                    };

                    attribute_descriptions.push(attr);
                }
                let vert_block = VertexImplBlock {
                    type_name: def.type_name.clone(),
                    attribute_descriptions,
                };
                vertex_impl_blocks.push(vert_block);

                struct_defs.push(def);
            }
        }
    }

    for GlobalParameter::ParameterBlock(parameter_block) in &reflection_json.global_parameters {
        let type_name = &parameter_block.element_type.type_name;

        let mut param_block_fields = vec![];
        for field in &parameter_block.element_type.fields {
            let Some(generated_field) = gather_struct_defs(field, &mut struct_defs) else {
                continue;
            };
            param_block_fields.push(generated_field);
        }

        let param_block_struct = GeneratedStructDefinition {
            type_name: type_name.to_string(),
            fields: param_block_fields,
        };
        struct_defs.push(param_block_struct);
    }

    struct_defs.reverse();

    let shader_name = reflection_json.source_file_name.replace(".slang", "");
    let shader_names = [shader_name];
    let file_name = reflection_json.source_file_name.replace(".slang", ".rs");
    let file_path = manifest_path(["src", "generated", "shader_atlas", &file_name]);
    let shader_structs_file = GeneratedFile {
        file_path,
        content: ShaderAtlasEntryStructsModule {
            struct_defs,
            vertex_impl_blocks,
        }
        .render()
        .unwrap(),
    };

    let module_names = shader_names.iter().map(|s| s.to_string()).collect();
    let shader_atlas_module = GeneratedFile {
        file_path: manifest_path(["src", "generated", "shader_atlas.rs"]),
        content: ShaderAtlasModule { module_names }.render().unwrap(),
    };

    let top_generated_module = GeneratedFile {
        file_path: manifest_path(["src", "generated.rs"]),
        content: "pub mod shader_atlas;".to_string(),
    };

    vec![
        shader_structs_file,
        shader_atlas_module,
        top_generated_module,
    ]
}

#[derive(Template)]
#[template(path = "shader_atlas.rs.askama")]
struct ShaderAtlasModule {
    module_names: Vec<String>,
}

#[derive(Template)]
#[template(path = "shader_atlas_entry_structs.rs.askama")]
struct ShaderAtlasEntryStructsModule {
    struct_defs: Vec<GeneratedStructDefinition>,
    vertex_impl_blocks: Vec<VertexImplBlock>,
}

fn gather_struct_defs(
    field: &StructField,
    struct_defs: &mut Vec<GeneratedStructDefinition>,
) -> Option<GeneratedStructFieldDefinition> {
    match field {
        StructField::Resource(_) => None,

        StructField::Vector(VectorStructField::Semantic(_)) => None,
        StructField::Vector(VectorStructField::Bound(vector)) => {
            let VectorElementType::Scalar(element_type) = &vector.element_type;
            let field_type = match (element_type.scalar_type, vector.element_count) {
                (ScalarType::Float32, 3) => "glam::Vec3",
                (ScalarType::Float32, 2) => "glam::Vec2",
                (t, c) => panic!("vector not supported: type: {t:?}, count: {c}"),
            };

            Some(GeneratedStructFieldDefinition {
                field_name: vector.field_name.to_snake_case(),
                type_name: field_type.to_string(),
            })
        }

        StructField::Struct(struct_field) => {
            let type_name = struct_field.struct_type.type_name.to_string();
            let mut generated_sub_fields = vec![];
            for sub_field in &struct_field.struct_type.fields {
                let Some(field_def) = gather_struct_defs(sub_field, struct_defs) else {
                    continue;
                };
                generated_sub_fields.push(field_def);
            }
            let sub_struct_def = GeneratedStructDefinition {
                type_name: type_name.clone(),
                fields: generated_sub_fields,
            };
            struct_defs.push(sub_struct_def);

            Some(GeneratedStructFieldDefinition {
                field_name: struct_field.field_name.to_snake_case(),
                type_name,
            })
        }

        StructField::Matrix(matrix) => {
            let VectorElementType::Scalar(scalar) = &matrix.element_type;

            let field_type = match (scalar.scalar_type, matrix.row_count, matrix.column_count) {
                (ScalarType::Float32, 4, 4) => "glam::Mat4",
                (s, r, c) => {
                    panic!("matrix not supported: scalar_type: {s:?}, rows: {r}, cols: {c}")
                }
            };

            Some(GeneratedStructFieldDefinition {
                field_name: matrix.field_name.to_snake_case(),
                type_name: field_type.to_string(),
            })
        }
    }
}

#[derive(Debug)]
struct GeneratedStructDefinition {
    type_name: String,
    fields: Vec<GeneratedStructFieldDefinition>,
}

#[derive(Debug)]
struct GeneratedStructFieldDefinition {
    field_name: String,
    type_name: String,
}

struct GeneratedFile {
    file_path: PathBuf,
    content: String,
}

struct VertexImplBlock {
    type_name: String,
    attribute_descriptions: Vec<VertexAttributeDescription>,
}

struct VertexAttributeDescription {
    field_name: String,
    format: String,
    location: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_files() {
        let shader = prepare_reflected_shader("depth_texture.slang").unwrap();
        let mut generated_files = build_generated_source_files(&shader.reflection_json);

        for source_file in &mut generated_files {
            source_file.file_path = source_file
                .file_path
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .unwrap()
                .to_owned();

            let info = serde_json::json!({
                "file_path": &source_file.file_path
            });

            insta::with_settings!({ info => &info, omit_expression => true }, {
                insta::assert_snapshot!(source_file.content);
            });
        }
    }
}
