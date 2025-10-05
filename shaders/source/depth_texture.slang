module depth_texture;

// modified from the official tutorial example here:
// https://docs.vulkan.org/tutorial/latest/_attachments/27_shader_depth.slang

ParameterBlock<DepthTexture> depthTexture;

struct DepthTexture {
    MVPMatrices mvp;
    Sampler2D texture;
}

// see game::mvp::MVPMatrices
struct MVPMatrices {
    float4x4 model;
    float4x4 view;
    float4x4 proj;
};

// see game::vertex::Vertex
struct Vertex {
    float3 position;
    float3 color;
    float2 texCoord;
};

[shader("vertex")]
FragVertex vertMain(uint vertexIndex : SV_VertexID, Vertex vertex) {
    let mvp = depthTexture.mvp;

    FragVertex output;

    let homogenousPosition = float4(vertex.position, 1.0);
    let mvpMatrix = mul(mvp.proj, mul(mvp.view, mvp.model));
    output.position = mul(mvpMatrix, homogenousPosition);

    if (vertexIndex == 0) {
        // this is intended as a demo/test of shader printf; it's pretty noisy
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
    let texture = depthTexture.texture;

    let textureColor = texture.Sample(fragVertex.texCoord);

    // this is intended as a demo/test of hot reload
    let debugColor = 1.0;
    // let debugColor = float3(1.0, 0.0, 1.0);

    let tweakedColor: float3 = debugColor * textureColor.rgb;

    return float4(tweakedColor, 1.0);
}
