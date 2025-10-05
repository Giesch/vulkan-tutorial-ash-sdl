use serde::{Deserialize, Serialize};
use shader_slang as slang;

#[derive(Debug, Serialize, Deserialize)]
pub struct EntryPoint {
    pub name: String,
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
    Struct(StructEntryPointParameter),
    Scalar(ScalarEntryPointParameter),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructEntryPointParameter {
    pub parameter_name: String,
    pub type_name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScalarEntryPointParameter {
    pub name: String,
    pub scalar_type: ScalarType,
    pub semantic_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StructField {
    Vector(VectorStructField),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VectorStructField {
    pub name: String,
    pub element_count: usize,
    pub element_type: VectorElementType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub semantic_name: Option<String>,
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
