use std::fs;
use std::process::Command;

use serde::Deserialize;

const SHADERS_SOURCE_DIR: &str = "./shaders/source";
const SHADERS_COMPILED_DIR: &str = "./shaders/compiled";

pub fn main() {
    if !fs::exists(SHADERS_COMPILED_DIR).unwrap() {
        fs::create_dir(SHADERS_COMPILED_DIR).unwrap();
    }

    let shaders_source_dir = fs::read_dir(SHADERS_SOURCE_DIR).unwrap();
    for entry in shaders_source_dir {
        let entry = entry.unwrap();

        let in_path = entry.path().display().to_string();
        if !in_path.ends_with(".slang") {
            panic!("non-slang file in shaders source dir: {in_path}");
        }

        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        let out_file_name = file_name.replace("slang", "spv");
        let json_file_name = file_name.replace("slang", "json");
        let out_path = format!("{SHADERS_COMPILED_DIR}/{out_file_name}");
        let json_path = format!("{SHADERS_COMPILED_DIR}/{json_file_name}");

        let output = Command::new("slangc")
            .arg(&in_path)
            .arg("-target")
            .arg("spirv")
            .arg("-o")
            .arg(&out_path)
            .arg("-reflection-json")
            .arg(&json_path)
            .output()
            .unwrap();

        if !output.status.success() {
            dbg!(&output);
            panic!("failed to compile shader");
        }
    }

    let shaders_compiled_dir = fs::read_dir(SHADERS_COMPILED_DIR).unwrap();
    for entry in shaders_compiled_dir {
        let entry = entry.unwrap();

        let in_path = entry.path().display().to_string();
        if !in_path.ends_with(".json") {
            continue;
        }

        let json = fs::read_to_string(&in_path).unwrap();
        let reflection: ReflectionJson =
            serde_json::from_str(&json).expect(&format!("failed to parse json file: {in_path}"));

        // TODO use the parsed json to generate rust structs
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReflectionJson {
    parameters: Vec<GlobalParameter>,
    entry_points: Vec<EntryPoint>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GlobalParameter {
    name: String,
    binding: Binding,
    r#type: GlobalParameterType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
enum GlobalParameterType {
    ConstantBuffer(ParameterTypeConstantBuffer),
    Resource(ParameterTypeResource),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ParameterTypeConstantBuffer {
    element_type: ElementType,
    container_var_layout: ContainerVarLayout,
    element_var_layout: ElementVarLayout,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ContainerVarLayout {
    binding: Binding,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ElementVarLayout {
    r#type: ElementType,
    binding: Binding,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
enum Binding {
    Uniform(UniformBinding),
    DescriptorTableSlot(DescriptorTableSlotBinding),
    VaryingInput(VaryingInputBinding),
    VaryingOutput(VaryingOutputBinding),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct UniformBinding {
    offset: usize,
    size: usize,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DescriptorTableSlotBinding {
    index: usize,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VaryingInputBinding {
    index: usize,
    count: Option<usize>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VaryingOutputBinding {
    index: usize,
    count: Option<usize>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ParameterTypeResource {
    base_shape: ResourceBaseShape,
    result_type: ElementType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum ResourceBaseShape {
    Texture2D,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ElementField {
    name: String,
    r#type: ElementType,
    binding: Binding,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PrimitiveFieldType {
    name: String,
    r#type: ElementType,
    semantic_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
enum ElementType {
    Struct(ElementTypeStruct),
    Matrix(ElementTypeMatrix),
    Scalar(ElementTypeScalar),
    Vector(ElementTypeVector),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ElementTypeStruct {
    name: String,
    fields: Vec<ElementField>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ElementTypeMatrix {
    row_count: usize,
    column_count: usize,
    element_type: Box<ElementType>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ElementTypeScalar {
    scalar_type: ScalarType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum ScalarType {
    Float32,
    Uint32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ElementTypeVector {
    element_count: usize,
    element_type: Box<ElementType>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EntryPoint {
    name: String,
    stage: ShaderStage,
    parameters: Vec<EntryPointParameter>,
    result: EntryPointResult,
    bindings: Vec<NamedBinding>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EntryPointResult {
    stage: ShaderStage,
    binding: Binding,
    // like ElementType, but with no bindings; unlikely to be used in codegen
    // r#type: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct NamedBinding {
    name: String,
    binding: Binding,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
enum EntryPointParameter {
    // TODO need another variant for a push constant with no hlsl semantic?
    // or make semantic_name optional
    Primitive(PrimitiveEntryPointParameter),
    Bound(BoundEntryPointParameter),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PrimitiveEntryPointParameter {
    name: String,
    semantic_name: String,
    r#type: ElementTypeScalar,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BoundEntryPointParameter {
    name: String,
    stage: ShaderStage,
    binding: Binding,
    r#type: EntryPointParameterType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
enum EntryPointParameterType {
    Struct(EntryPointParameterTypeStruct),
    Matrix(EntryPointParameterTypeMatrix),
    Scalar(EntryPointParameterTypeScalar),
    Vector(EntryPointParameterTypeVector),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EntryPointParameterTypeStruct {
    name: String,
    fields: Vec<EntryPointElementField>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
enum EntryPointElementField {
    Bound(BoundFieldType),
    Primitive(PrimitiveFieldType),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BoundFieldType {
    name: String,
    stage: ShaderStage,
    binding: Binding,
    r#type: ElementType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EntryPointParameterTypeMatrix {
    row_count: usize,
    column_count: usize,
    element_type: Box<EntryPointParameterType>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EntryPointParameterTypeScalar {
    scalar_type: ScalarType,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EntryPointParameterTypeVector {
    element_count: usize,
    element_type: Box<ElementType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTRY_POINT_PAREMETER: &str = r#"
        {
            "name": "input",
            "stage": "vertex",
            "binding": {"kind": "varyingInput", "index": 0, "count": 2},
            "type": {
                "kind": "struct",
                "name": "VSInput",
                "fields": [
                    {
                        "name": "Pos",
                        "type": {
                            "kind": "vector",
                            "elementCount": 3,
                            "elementType": {
                                "kind": "scalar",
                                "scalarType": "float32"
                            }
                        },
                        "stage": "vertex",
                        "binding": {"kind": "varyingInput", "index": 0}
                    },
                    {
                        "name": "Color",
                        "type": {
                            "kind": "vector",
                            "elementCount": 3,
                            "elementType": {
                                "kind": "scalar",
                                "scalarType": "float32"
                            }
                        },
                        "stage": "vertex",
                        "binding": {"kind": "varyingInput", "index": 1}
                    }
                ]
            }
        }
    "#;

    #[test]
    fn bound_entry_point_parameter() {
        let _parsed: BoundEntryPointParameter =
            serde_json::from_str(&ENTRY_POINT_PAREMETER).unwrap();
    }

    #[test]
    fn entry_point_parameter() {
        let _parsed: EntryPointParameter = serde_json::from_str(&ENTRY_POINT_PAREMETER).unwrap();
    }

    const POS_FIELD: &str = r#"
        {
            "name": "Pos",
            "type": {
                "kind": "vector",
                "elementCount": 3,
                "elementType": {
                    "kind": "scalar",
                    "scalarType": "float32"
                }
            },
            "stage": "vertex",
            "binding": {"kind": "varyingInput", "index": 0}
        }
    "#;

    const COLOR_FIELD: &str = r#"
        {
            "name": "Color",
            "type": {
                "kind": "vector",
                "elementCount": 3,
                "elementType": {
                    "kind": "scalar",
                    "scalarType": "float32"
                }
            },
            "stage": "vertex",
            "binding": {"kind": "varyingInput", "index": 1}
        }
    "#;

    #[test]
    fn pos_field() {
        let _parsed: EntryPointElementField = serde_json::from_str(&POS_FIELD).unwrap();
    }

    #[test]
    fn color_field() {
        let _parsed: EntryPointElementField = serde_json::from_str(&COLOR_FIELD).unwrap();
    }

    const SCALAR_PARAM: &str = r#"
        {
            "name": "vertexIndex",
            "semanticName": "SV_VERTEXID",
            "type": {
                "kind": "scalar",
                "scalarType": "uint32"
            }
        }
    "#;

    #[test]
    fn scalar_param() {
        let _parsed: PrimitiveEntryPointParameter = serde_json::from_str(&SCALAR_PARAM).unwrap();
    }
}
