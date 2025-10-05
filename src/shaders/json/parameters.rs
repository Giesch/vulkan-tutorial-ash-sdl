use serde::{Deserialize, Serialize};
use shader_slang as slang;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GlobalParameter {
    ParameterBlock(ParameterBlockGlobalParameter),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParameterBlockGlobalParameter {
    pub parameter_name: String,
    pub element_type: ParameterBlockElementType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParameterBlockElementType {
    pub type_name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntryPoint {
    pub entry_point_name: String,
    pub stage: EntryPointStage,
    pub parameters: Vec<EntryPointParameter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EntryPointStage {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EntryPointParameter {
    Struct(ReflectedStructParameter),
    Scalar(ReflectedScalarParameter),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectedStructParameter {
    pub parameter_name: String,
    pub type_name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflectedScalarParameter {
    pub parameter_name: String,
    pub scalar_type: ScalarType,
    pub semantic_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StructField {
    Vector(VectorStructField),
    Struct(StructStructField),
    Matrix(MatrixStructField),
    Resource(ResourceStructField),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorStructField {
    pub field_name: String,
    pub element_count: usize,
    pub element_type: VectorElementType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub semantic_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatrixStructField {
    pub field_name: String,
    pub row_count: u32,
    pub column_count: u32,
    pub element_type: VectorElementType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceStructField {
    pub field_name: String,
    pub resource_shape: ResourceShape,
    pub result_type: ResourceResultType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceShape {
    Texture2D,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ResourceResultType {
    Vector(VectorResultType),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorResultType {
    pub element_count: usize,
    pub element_type: VectorElementType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructStructField {
    pub field_name: String,
    pub struct_type: StructFieldType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructFieldType {
    pub type_name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VectorElementType {
    Scalar(ScalarVectorElementType),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScalarVectorElementType {
    pub scalar_type: ScalarType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScalarType {
    Float32,
    Uint32,
}

impl ScalarType {
    pub fn from_slang(scalar: slang::ScalarType) -> Self {
        match scalar {
            slang::ScalarType::Uint32 => Self::Uint32,
            slang::ScalarType::Float32 => Self::Float32,
            k => todo!("slang scalar type not handled: {k:?}"),
        }
    }
}
