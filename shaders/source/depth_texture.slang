// https://docs.vulkan.org/tutorial/latest/_attachments/27_shader_depth.slang

ConstantBuffer<MVP> mvp;

Sampler2D texture;

// see renderer::UniformBufferObject
struct MVP {
    float4x4 model;
    float4x4 view;
    float4x4 proj;
};

// see renderer::Vertex
struct Vertex {
    float3 position;
    float3 color;
    float2 texCoord;
};

// ParameterBlock<Sampler2D> texture;
// ParameterBlock<MVP> mvp;

[shader("vertex")]
FragVertex vertMain(
    uint vertexIndex : SV_VertexID,
    Vertex vertex
    // ParameterBlock<Vertex> vertex
) {
    FragVertex output;

    let homogenousPosition = float4(vertex.position, 1.0);
    let mvpMatrix = mul(mvp.proj, mul(mvp.view, mvp.model));
    output.position = mul(mvpMatrix, homogenousPosition);

    if (vertexIndex == 0) {
        printf("first vertex position: %u, %u, %u\n", output.position.x, output.position.y, output.position.z);
    }

    output.texCoord = vertex.texCoord;
    output.color = vertex.color; // unused in viking room example

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
    let textureColor = texture.Sample(fragVertex.texCoord);

    let debugColor = 1.0;
    // let debugColor = float3(1.0, 0.0, 0.0);

    let tweakedColor: float3 = debugColor * textureColor.rgb;

    return float4(tweakedColor, 1.0);
}
