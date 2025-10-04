use shader_slang as slang;

use crate::shaders::json::*;

pub struct VertFragEntryPoints {
    pub vertex_entry_point: EntryPoint,
    pub fragment_entry_point: EntryPoint,
}

pub fn reflect_entry_points(
    program_layout: &slang::reflection::Shader,
) -> anyhow::Result<VertFragEntryPoints> {
    let mut vertex_entry_point: Option<EntryPoint> = None;
    let mut fragment_entry_point: Option<EntryPoint> = None;

    for entry_point in program_layout.entry_points() {
        let entry_point_name = entry_point.name().to_string();

        let mut params = vec![];
        for param in entry_point.parameters() {
            let param_name = param.name().unwrap().to_string();

            let type_layout = param.type_layout();

            let entry_point_param_json = match type_layout.kind() {
                slang::TypeKind::Struct => {
                    let mut fields = vec![];
                    let struct_type_name = type_layout.name().unwrap().to_string();

                    for field in type_layout.fields() {
                        let field_name = field.name().unwrap().to_string();

                        let field_type_layout = field.type_layout();

                        let field_json = match field_type_layout.kind() {
                            slang::TypeKind::Vector => {
                                let vec_elem_count = field_type_layout.element_count().unwrap();

                                let vec_element_type_layout =
                                    field_type_layout.element_type_layout();

                                let slang_scalar_type =
                                    vec_element_type_layout.scalar_type().unwrap();
                                let scalar_type = ScalarType::from_slang(slang_scalar_type);
                                let vec_elem_type =
                                    VectorElementType::Scalar(ScalarVectorElementType {
                                        scalar_type,
                                    });

                                StructField::Vector(VectorStructField {
                                    name: field_name,
                                    element_count: vec_elem_count,
                                    element_type: vec_elem_type,
                                })
                            }

                            k => todo!("field type layout kind not handled: {k:?}"),
                        };

                        fields.push(field_json);
                    }

                    EntryPointParameter::Struct(StructEntryPointParameter {
                        parameter_name: param_name,
                        type_name: struct_type_name,
                        fields,
                    })
                }

                slang::TypeKind::Scalar => {
                    // this is only the vertexIndex system value for now

                    let semantic_name = param.semantic_name().unwrap().to_string();
                    let scalar_type = ScalarType::from_slang(type_layout.scalar_type().unwrap());

                    EntryPointParameter::Scalar(ScalarEntryPointParameter {
                        name: param_name,
                        scalar_type,
                        semantic_name,
                    })
                }

                k => todo!("type kind reflection not implemented: {k:?}"),
            };

            params.push(entry_point_param_json);
        }

        match entry_point.stage() {
            slang::Stage::Vertex => {
                vertex_entry_point = Some(EntryPoint {
                    name: entry_point_name,
                    stage: EntryPointStage::Vertex,
                    parameters: params,
                });
            }

            slang::Stage::Fragment => {
                fragment_entry_point = Some(EntryPoint {
                    name: entry_point_name,
                    stage: EntryPointStage::Fragment,
                    parameters: params,
                });
            }

            _ => todo!(),
        }
    }

    let (vertex_entry_point, fragment_entry_point) =
        match (vertex_entry_point, fragment_entry_point) {
            (Some(v), Some(f)) => (v, f),
            _ => anyhow::bail!("failed to load vertex and fragment entry points"),
        };

    Ok(VertFragEntryPoints {
        vertex_entry_point,
        fragment_entry_point,
    })
}
