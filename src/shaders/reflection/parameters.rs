use shader_slang as slang;

use crate::shaders::json::*;

pub struct Parameters {
    pub global_parameters: Vec<GlobalParameter>,
    pub entry_points: VertFragEntryPoints,
}

pub struct VertFragEntryPoints {
    pub vertex_entry_point: EntryPoint,
    pub fragment_entry_point: EntryPoint,
}

pub fn reflect_entry_points(
    program_layout: &slang::reflection::Shader,
) -> anyhow::Result<Parameters> {
    let mut vertex_entry_point: Option<EntryPoint> = None;
    let mut fragment_entry_point: Option<EntryPoint> = None;

    let mut global_parameters: Vec<GlobalParameter> = vec![];
    for global_param in program_layout.parameters() {
        let parameter_name = global_param.name().unwrap().to_string();

        let global_param_type_layout = global_param.type_layout();
        assert!(global_param_type_layout.kind() == slang::TypeKind::ParameterBlock);

        let element_type_layout = global_param.type_layout().element_type_layout();

        let element_type = match element_type_layout.kind() {
            shader_slang::TypeKind::Struct => {
                let element_type_name = element_type_layout.name().unwrap().to_string();
                let fields = reflect_struct_fields(element_type_layout);

                ParameterBlockElementType {
                    type_name: element_type_name,
                    fields,
                }
            }
            k => todo!("type kind reflection not implemented: {k:?}"),
        };

        let parameter_block = ParameterBlockGlobalParameter {
            parameter_name,
            element_type,
        };
        let global_parameter = GlobalParameter::ParameterBlock(parameter_block);

        global_parameters.push(global_parameter);
    }

    for entry_point in program_layout.entry_points() {
        let entry_point_name = entry_point.name().to_string();

        let mut params = vec![];
        for param in entry_point.parameters() {
            let param_name = param.name().unwrap().to_string();

            let type_layout = param.type_layout();

            let entry_point_param_json = match type_layout.kind() {
                slang::TypeKind::Struct => {
                    let struct_param = reflect_struct_parameter(type_layout, param_name);
                    EntryPointParameter::Struct(struct_param)
                }

                slang::TypeKind::Scalar => {
                    // this is only the vertexIndex system value for now

                    let semantic_name = param.semantic_name().unwrap().to_string();
                    let scalar_type = ScalarType::from_slang(type_layout.scalar_type().unwrap());

                    EntryPointParameter::Scalar(ReflectedScalarParameter {
                        parameter_name: param_name,
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
                    entry_point_name,
                    stage: EntryPointStage::Vertex,
                    parameters: params,
                });
            }

            slang::Stage::Fragment => {
                fragment_entry_point = Some(EntryPoint {
                    entry_point_name,
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

    let entry_points = VertFragEntryPoints {
        vertex_entry_point,
        fragment_entry_point,
    };

    let parameters = Parameters {
        global_parameters,
        entry_points,
    };

    Ok(parameters)
}

fn reflect_struct_parameter(
    type_layout: &slang::reflection::TypeLayout,
    param_name: String,
) -> ReflectedStructParameter {
    let fields = reflect_struct_fields(type_layout);
    let struct_type_name = type_layout.name().unwrap().to_string();

    ReflectedStructParameter {
        parameter_name: param_name,
        type_name: struct_type_name,
        fields,
    }
}

fn reflect_struct_fields(struct_type_layout: &slang::reflection::TypeLayout) -> Vec<StructField> {
    let mut fields = vec![];

    for field in struct_type_layout.fields() {
        let field_name = field.name().unwrap().to_string();
        let field_semantic_name = field.semantic_name().map(str::to_string);
        let field_type_layout = field.type_layout();

        let field_json = match field_type_layout.kind() {
            slang::TypeKind::Vector => {
                let vec_elem_count = field_type_layout.element_count().unwrap();

                let vec_element_type_layout = field_type_layout.element_type_layout();

                let slang_scalar_type = vec_element_type_layout.scalar_type().unwrap();
                let scalar_type = ScalarType::from_slang(slang_scalar_type);
                let vec_elem_type =
                    VectorElementType::Scalar(ScalarVectorElementType { scalar_type });

                StructField::Vector(VectorStructField {
                    field_name,
                    element_count: vec_elem_count,
                    element_type: vec_elem_type,
                    semantic_name: field_semantic_name,
                })
            }

            slang::TypeKind::Matrix => {
                let row_count = field_type_layout.row_count().unwrap();
                let column_count = field_type_layout.column_count().unwrap();

                let mat_element_type_layout = field_type_layout.element_type_layout();

                let scalar_type =
                    ScalarType::from_slang(mat_element_type_layout.scalar_type().unwrap());
                let element_type =
                    VectorElementType::Scalar(ScalarVectorElementType { scalar_type });

                StructField::Matrix(MatrixStructField {
                    field_name,
                    row_count,
                    column_count,
                    element_type,
                })
            }

            slang::TypeKind::Struct => {
                let field_fields = reflect_struct_fields(field_type_layout);
                let field_type_name = field_type_layout.name().unwrap().to_string();

                StructField::Struct(StructStructField {
                    field_name,
                    struct_type: StructFieldType {
                        type_name: field_type_name,
                        fields: field_fields,
                    },
                })
            }

            slang::TypeKind::Resource => {
                let resource_shape = match field_type_layout.resource_shape().unwrap() {
                    shader_slang::ResourceShape::SlangTexture2d => ResourceShape::Texture2D,
                    s => todo!("resource shape not handled: {s:?}"),
                };

                let result_type = field_type_layout.resource_result_type().unwrap();
                let result_type = match result_type.kind() {
                    shader_slang::TypeKind::Vector => {
                        let element_count = result_type.element_count();

                        let scalar_type = ScalarType::from_slang(result_type.scalar_type());
                        let element_type =
                            VectorElementType::Scalar(ScalarVectorElementType { scalar_type });

                        ResourceResultType::Vector(VectorResultType {
                            element_count,
                            element_type,
                        })
                    }
                    k => todo!("result type kind not handled: {k:?}"),
                };

                StructField::Resource(ResourceStructField {
                    field_name,
                    resource_shape,
                    result_type,
                })
            }

            k => todo!("field type layout kind not handled: {k:?}"),
        };

        fields.push(field_json);
    }

    fields
}
