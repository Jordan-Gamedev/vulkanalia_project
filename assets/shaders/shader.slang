struct VSInput
{
    float3 inPos;
    float3 inColor;
    float3 inNorm;
    float2 inUv;
};

struct VSOutput
{
    float4 pos : SV_Position;
    float3 fragColor;
    float2 fragUv;
    uint textureIndex;
    uint samplerIndex;
};

struct UniformBuffer
{
    float4x4 view;
    float4x4 proj;
};

struct QuantizedModelMatrix
{
    float[3] position;
    float[3] scale;
    int16_t[4] rotation;
};

struct IndirectDrawData
{
    uint indexCount;
    uint instanceCount;
    uint firstIndex;
    int vertexOffset;
    uint firstInstance;
};

struct PerInstanceData
{
    uint modelMatrixInfo;
    uint16_t textureIndex;
    uint16_t samplerIndex;
    uint16_t meshMetadataIndex;
    uint16_t meshAssetId;
};

[[vk::binding(0, 0)]]
Texture2D textures[];

[[vk::binding(1, 0)]]
SamplerState samplers[];

[[vk::binding(2, 0)]]
ConstantBuffer<UniformBuffer> ubo;

[[vk::binding(3, 0)]]
StructuredBuffer<QuantizedModelMatrix> staticModelMatrices;

[[vk::binding(4, 0)]]
StructuredBuffer<QuantizedModelMatrix> dynamicModelMatrices;

[[vk::binding(5, 0)]]
StructuredBuffer<IndirectDrawData> indirectDrawData;

[[vk::binding(6, 0)]]
StructuredBuffer<PerInstanceData> instanceData;

[[vk::binding(8, 0)]]
StructuredBuffer<uint> visbuffer;

float4 decodeQuaternion(int16_t[4] packedRotation)
{
    float4 q = float4(packedRotation[0], packedRotation[1], packedRotation[2], packedRotation[3]) / int16_t::maxValue;
    return normalize(q);
}

float3 rotateVector(float3 vector, float4 quaternion)
{
    float3 t = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * t + cross(quaternion.xyz, t);
}

float4x4 buildModelMatrix(QuantizedModelMatrix modelMatrix)
{
    float4 quaternion = decodeQuaternion(modelMatrix.rotation);

    float3 right = rotateVector(float3(1.0, 0.0, 0.0), quaternion);
    float3 up = rotateVector(float3(0.0, 1.0, 0.0), quaternion);
    float3 forward = rotateVector(float3(0.0, 0.0, 1.0), quaternion);

    return float4x4(
        float4(right.x * modelMatrix.scale[0], up.x * modelMatrix.scale[1], forward.x * modelMatrix.scale[2], modelMatrix.position[0]),
        float4(right.y * modelMatrix.scale[0], up.y * modelMatrix.scale[1], forward.y * modelMatrix.scale[2], modelMatrix.position[1]),
        float4(right.z * modelMatrix.scale[0], up.z * modelMatrix.scale[1], forward.z * modelMatrix.scale[2], modelMatrix.position[2]),
        float4(0.0, 0.0, 0.0, 1.0));
}

[shader("vertex")]
VSOutput vertMain(VSInput input, uint startInstanceId: SV_StartInstanceLocation, uint instanceId: SV_InstanceID)
{
    VSOutput output;
    uint instanceIndex = visbuffer[1 + startInstanceId + instanceId];
    PerInstanceData instance = instanceData[instanceIndex];

    bool isStatic = (instance.modelMatrixInfo & 0x80000000u) != 0;
    uint modelMatrixIndex = instance.modelMatrixInfo & 0x7FFFFFFFu;

    QuantizedModelMatrix modelMatrix = isStatic ? staticModelMatrices[modelMatrixIndex] : dynamicModelMatrices[modelMatrixIndex];
    float4x4 model = buildModelMatrix(modelMatrix);

    output.pos = mul(ubo.proj, mul(ubo.view, mul(model, float4(input.inPos, 1.0))));
    output.fragColor = input.inColor;
    output.fragUv = input.inUv;
    output.textureIndex = instance.textureIndex;
    output.samplerIndex = instance.samplerIndex;
    return output;
}

[shader("fragment")]
float4 fragMain(VSOutput vertIn) : SV_TARGET
{
    float4 texcol = textures[vertIn.textureIndex].Sample(samplers[vertIn.samplerIndex], vertIn.fragUv);
    return float4(vertIn.fragColor * texcol.rgb, 1.0);
}
