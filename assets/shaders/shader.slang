struct VSInput {
    float3 inPos;
    float3 inColor;
    float3 inNorm;
    float2 inUv;
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
    float2 fragUv;
};

[shader("vertex")]
VSOutput vertMain(VSInput input) {
    VSOutput output;
    output.pos = mul(ubo.proj, mul(ubo.view, mul(ubo.model, float4(input.inPos, 1.0))));
    output.fragColor = input.inColor;
    output.fragUv = input.inUv;
    return output;
}

Sampler2D texture;

[shader("fragment")]
float4 fragMain(VSOutput vertIn) : SV_TARGET {
    return float4(vertIn.fragColor * texture.Sample(vertIn.fragUv).rgb, 1.0);
}