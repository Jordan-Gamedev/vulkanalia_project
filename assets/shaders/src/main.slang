import buffers;
import mathUtils;

struct VSInput
{
    [[vk::location(0)]]
    float3 pos; // R32G32B32_SFLOAT (12 bytes)
    [[vk::location(1)]]
    float4 normal; // A2R10G10B10_SNORM_PACK32 (4 bytes)
    [[vk::location(2)]]
    float4 tangent; // A2R10G10B10_SNORM_PACK32 (4 bytes)
    [[vk::location(3)]]
    float2 uv; // R16G16_SFLOAT (4 bytes)
    [[vk::location(4)]]
    uint4 bone_ids; // R8G8B8A8_UINT (4 bytes)
    [[vk::location(5)]]
    float4 bone_weights; // R8G8B8A8_UNORM (4 bytes)
};

struct VSOutput
{
    float4 pos : SV_Position;
    float2 fragUv;
    uint textureIndex;
    uint samplerIndex;
};

[shader("vertex")]
VSOutput vertMain(VSInput input, uint startInstanceId: SV_StartInstanceLocation, uint instanceId: SV_InstanceID)
{
    VSOutput output;
    uint instanceIndex = visbuffer[startInstanceId + instanceId];
    PerInstanceData instance = instanceData[instanceIndex];

    QuantizedModelMatrix modelMatrix = modelMatrixBuffer[instanceIndex];
    float4x4 model = buildModelMatrix(modelMatrix);

    output.pos = mul(ubo.proj, mul(ubo.view, mul(model, float4(input.pos, 1.0))));
    output.fragUv = input.uv;
    output.textureIndex = instance.textureIndex;
    output.samplerIndex = instance.samplerIndex;
    return output;
}

[shader("fragment")]
float4 fragMain(VSOutput vertIn) : SV_TARGET
{
    float4 texcol = textures[vertIn.textureIndex].Sample(samplers[vertIn.samplerIndex], vertIn.fragUv);
    return float4(texcol.rgb, 1.0);
}
