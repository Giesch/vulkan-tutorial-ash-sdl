use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GlobalParameter {
    ParameterBlock(ParameterBlockGlobalParameter),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterBlockGlobalParameter {
    pub parameter_name: String,
    pub element_type: ParameterBlockElementType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterBlockElementType {
    pub type_name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPoint {
    pub entry_point_name: String,
    pub stage: EntryPointStage,
    pub parameters: Vec<EntryPointParameter>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct ReflectedStructParameter {
    pub parameter_name: String,
    pub type_name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StructFieldBinding {
    Uniform(OffsetSizeBinding),
    DescriptorTableSlot(OffsetSizeBinding),
    VaryingInput(OffsetSizeBinding),
    ConstantBuffer(OffsetSizeBinding),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OffsetSizeBinding {
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum VectorStructField {
    Bound(BoundVectorStructField),
    Semantic(SemanticVectorStructField),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticVectorStructField {
    pub field_name: String,
    pub semantic_name: String,
    pub element_count: usize,
    pub element_type: VectorElementType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundVectorStructField {
    pub field_name: String,
    pub binding: StructFieldBinding,
    pub element_count: usize,
    pub element_type: VectorElementType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixStructField {
    pub field_name: String,
    pub binding: StructFieldBinding,
    pub row_count: u32,
    pub column_count: u32,
    pub element_type: VectorElementType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStructField {
    pub field_name: String,
    pub binding: StructFieldBinding,
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
#[serde(rename_all = "camelCase")]
pub struct VectorResultType {
    pub element_count: usize,
    pub element_type: VectorElementType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructStructField {
    pub field_name: String,
    pub binding: StructFieldBinding,
    pub struct_type: StructFieldType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct ScalarVectorElementType {
    pub scalar_type: ScalarType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScalarType {
    Float32,
    Uint32,
}
