#language slang 2026

module depth_texture;

import mvp;

// modified from the official tutorial example here:
// https://docs.vulkan.org/tutorial/latest/_attachments/27_shader_depth.slang

ParameterBlock<DepthTexture> depthTexture;

struct DepthTexture {
    MVPMatrices mvp;
    Sampler2D texture;
}

struct Vertex {
    float3 position;
    float3 color;
    float2 texCoord;
};

[shader("vertex")]
FragVertex vertMain(uint vertexIndex : SV_VertexID, Vertex vertex) {
    let position = depthTexture.mvp.project(vertex.position);

    if (vertexIndex == 0) {
        // this is intended as a demo/test of shader printf; it's pretty noisy
        printf(
          "first vertex position: %f, %f, %f\n",
          position.x, position.y, position.z
        );
    }

    return FragVertex(position, vertex.color, vertex.texCoord);
}

struct FragVertex {
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
