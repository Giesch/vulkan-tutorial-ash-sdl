#language slang 2026

module pixel_rick;

// TODO avoid requiring this?
ParameterBlock<PixelRick> pixelRick;

struct PixelRick {
    Circle circle;
}

struct Circle {
    float2 position;
    float radius;
}

// TODO avoid needing a distinct name
struct PixelRickVertex {
    float3 position;
    float3 color;
    float2 texCoord;
};

[shader("vertex")]
FragVertex vertMain(uint vertexIndex : SV_VertexID, PixelRickVertex vertex) {
    FragVertex output;

    output.position = float4(vertex.position, 1.0);
    output.color = vertex.color;
    output.texCoord = vertex.texCoord;

    return output;
}

struct FragVertex
{
    float4 position : SV_Position;
    float3 color;
    float2 texCoord;
};

[shader("fragment")]
float4 fragMain(FragVertex fragVertex) : SV_TARGET {
    let debugColor = 1.0;
    // let debugColor = float3(1.0, 0.0, 1.0);

    let tweakedColor: float3 = debugColor * fragVertex.color;

    return float4(tweakedColor, 1.0);
}
