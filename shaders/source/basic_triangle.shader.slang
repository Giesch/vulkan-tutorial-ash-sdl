#language slang 2026

module basic_triangle;

import mvp;

ParameterBlock<MVPMatrices> mvp;

struct BasicTriangle {
    MVPMatrices mvp;
}

struct Vertex {
    float3 position;
    float3 color;
};

struct FragVertex {
    float4 position : SV_POSITION;
    float3 color;
};

[shader("vertex")]
FragVertex vertexMain(Vertex vertex) {
    let position = mvp.project(vertex.position);
    return FragVertex(position, vertex.color);
}

[shader("fragment")]
float4 fragmentMain(FragVertex fragVertex) {
    return float4(fragVertex.color, 1.0);
}
