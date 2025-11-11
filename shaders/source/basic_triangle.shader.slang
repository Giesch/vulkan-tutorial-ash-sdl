#language slang 2026

module basic_triangle;

ParameterBlock<BasicTriangle> basicTriangle;

struct BasicTriangle {
    MVPMatrices mvp;
}

struct MVPMatrices {
    float4x4 model;
    float4x4 view;
    float4x4 proj;
};

struct Vertex {
    float3 position;
    float3 color;
};

struct FragVertex
{
	float4 position : SV_POSITION;
	float3 color;
};

[shader("vertex")]
FragVertex vertexMain(Vertex vertex)
{
    let mvp = basicTriangle.mvp;

	FragVertex fragVertex;

    let homogenousPosition = float4(vertex.position, 1.0);
    let mvpMatrix = mul(mvp.proj, mul(mvp.view, mvp.model));
    fragVertex.position = mul(mvpMatrix, homogenousPosition);

    fragVertex.color = vertex.color;

	return fragVertex;
}

[shader("fragment")]
float4 fragmentMain(FragVertex fragVertex)
{
    return float4(fragVertex.color, 1.0);
}
