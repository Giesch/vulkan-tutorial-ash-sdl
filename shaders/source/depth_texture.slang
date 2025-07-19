// https://docs.vulkan.org/tutorial/latest/_attachments/27_shader_depth.slang

struct VSInput {
    float3 inPosition;
    float3 inColor;
    float2 inTexCoord;
};

struct UniformBuffer {
    float4x4 model;
    float4x4 view;
    float4x4 proj;
};
ConstantBuffer<UniformBuffer> ubo;

struct VSOutput
{
    float4 pos : SV_Position;
    float3 fragColor;
    float2 fragTexCoord;
};

[shader("vertex")]
VSOutput vertMain(VSInput input) {
    VSOutput output;

    // unused in viking room example
    output.fragColor = input.inColor;

    output.pos = mul(ubo.proj, mul(ubo.view, mul(ubo.model, float4(input.inPosition, 1.0))));
    output.fragTexCoord = input.inTexCoord;

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
