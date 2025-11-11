#language slang 2026

module mvp;

public struct MVPMatrices {
    float4x4 model;
    float4x4 view;
    float4x4 proj;

    [ForceInline]
    public func project(position: float3) -> float4 {
        let homogenousPosition = float4(position, 1.0);
        let mvpMatrix = mul(this.proj, mul(this.view, this.model));

        return mul(mvpMatrix, homogenousPosition);
    }
};
