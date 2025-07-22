// https://docs.vulkan.org/tutorial/latest/_attachments/27_shader_depth.slang

struct VSInput {
    float3 inPosition;
    float3 inColor;
    float2 inTexCoord;
};

// see renderer::UniformBufferObject
struct UniformBuffer {
    float4x4 model;
    float4x4 view;
    float4x4 proj;
};
ConstantBuffer<UniformBuffer> ubo;

// see renderer::Vertex
struct VSOutput
{
    float4 pos : SV_Position;
    float3 fragColor;
    float2 fragTexCoord;
};

[shader("vertex")]
VSOutput vertMain(uint vertexIndex : SV_VertexID, VSInput input) {
    VSOutput output;

    let homogenousPosition = float4(input.inPosition, 1.0);
    let mvpMatrix = mul(ubo.proj, mul(ubo.view, ubo.model));
    output.pos = mul(mvpMatrix, homogenousPosition);

    if (vertexIndex == 0) {
        printf("first vertex pos: %u, %u, %u\n", output.pos.x, output.pos.y, output.pos.z);
    }

    output.fragTexCoord = input.inTexCoord;

    // unused in viking room example
    output.fragColor = input.inColor;

    return output;
}

Sampler2D texture;

[shader("fragment")]
float4 fragMain(VSOutput vertIn) : SV_TARGET {
    let textureColor = texture.Sample(vertIn.fragTexCoord);

    let debugColor = 1.0;
    // let debugColor = float3(1.0, 0.0, 0.0);

    let tweakedColor: float3 = debugColor * textureColor.rgb;

    return float4(tweakedColor, 1.0);
}
