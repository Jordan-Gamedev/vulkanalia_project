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

struct PushConstants {
    uint textureIndex;
};

[[vk::push_constant]]
ConstantBuffer<PushConstants> pushConstants;

[shader("vertex")]
VSOutput vertMain(VSInput input) {
    VSOutput output;
    output.pos = mul(ubo.proj, mul(ubo.view, mul(ubo.model, float4(input.inPos, 1.0))));
    output.fragColor = input.inColor;
    output.fragUv = input.inUv;
    return output;
}

// Bindless combined image samplers: runtime-sized array declared at the same binding
// as the descriptor array on the CPU side (binding 1). The engine currently
// writes textures into these array slots; shaders index into them by integer.
Sampler2D textures[];

[shader("fragment")]
float4 fragMain(VSOutput vertIn) : SV_TARGET {
    float4 texcol = textures[pushConstants.textureIndex].Sample(vertIn.fragUv);
    return float4(vertIn.fragColor * texcol.rgb, 1.0);
}