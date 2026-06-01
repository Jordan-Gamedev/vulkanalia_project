struct VSInput {
    float3 inPos;
    float3 inColor;
    float3 inNorm;
    float2 inUv;
};

struct UniformBuffer {
    float4x4 view;
    float4x4 proj;
};

struct QuantizedModelMatrix {
    float[3] position;
    float[3] scale;
    int16_t[4] rotation;
};

struct IndirectDrawData {
    uint indexCount;
    uint instanceCount;
    uint firstIndex;
    int vertexOffset;
    uint firstInstance;
    uint modelMatrixInfo;
    uint textureIndex;
    uint padding;
};

[[vk::binding(2, 0)]]
StructuredBuffer<QuantizedModelMatrix> staticModelMatrices;

[[vk::binding(3, 0)]]
StructuredBuffer<QuantizedModelMatrix> dynamicModelMatrices;

[[vk::binding(4, 0)]]
StructuredBuffer<IndirectDrawData> indirectDrawData;
 
struct PerInstanceData {
    uint modelMatrixInfo;
    uint textureIndex;
    uint samplerIndex;
    uint padding;
};

[[vk::binding(5, 0)]]
StructuredBuffer<PerInstanceData> instanceData;
ConstantBuffer<UniformBuffer> ubo;

struct VSOutput
{
    float4 pos : SV_Position;
    float3 fragColor;
    float2 fragUv;
    uint textureIndex;
    uint samplerIndex;
};

float4 decodeQuaternion(int16_t[4] packedRotation) {
    float4 q = float4(packedRotation[0], packedRotation[1], packedRotation[2], packedRotation[3]) / int16_t::maxValue;
    return normalize(q);
}

float3 rotateVector(float3 vector, float4 quaternion) {
    float3 t = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * t + cross(quaternion.xyz, t);
}

float4x4 buildModelMatrix(QuantizedModelMatrix modelMatrix) {
    float4 quaternion = decodeQuaternion(modelMatrix.rotation);

    float3 right = rotateVector(float3(1.0, 0.0, 0.0), quaternion);
    float3 up = rotateVector(float3(0.0, 1.0, 0.0), quaternion);
    float3 forward = rotateVector(float3(0.0, 0.0, 1.0), quaternion);

    return float4x4(
        float4(right.x * modelMatrix.scale[0], up.x * modelMatrix.scale[1], forward.x * modelMatrix.scale[2], modelMatrix.position[0]),
        float4(right.y * modelMatrix.scale[0], up.y * modelMatrix.scale[1], forward.y * modelMatrix.scale[2], modelMatrix.position[1]),
        float4(right.z * modelMatrix.scale[0], up.z * modelMatrix.scale[1], forward.z * modelMatrix.scale[2], modelMatrix.position[2]),
        float4(0.0, 0.0, 0.0, 1.0)
    );
}

[shader("vertex")]
VSOutput vertMain(VSInput input, uint drawId : SV_DrawIndex, uint instanceId : SV_InstanceID) {
    VSOutput output;

    IndirectDrawData drawData = indirectDrawData[drawId];
    uint instanceIndex = drawData.firstInstance + instanceId;
    PerInstanceData pi = instanceData[instanceIndex];

    bool isStatic = (pi.modelMatrixInfo & 0x80000000u) != 0;
    uint modelMatrixIndex = pi.modelMatrixInfo & 0x7FFFFFFFu;

    QuantizedModelMatrix modelMatrix = isStatic ? staticModelMatrices[modelMatrixIndex] : dynamicModelMatrices[modelMatrixIndex];
    float4x4 model = buildModelMatrix(modelMatrix);

    output.pos = mul(ubo.proj, mul(ubo.view, mul(model, float4(input.inPos, 1.0))));
    output.fragColor = input.inColor;
    output.fragUv = input.inUv;
    output.textureIndex = pi.textureIndex;
    output.samplerIndex = pi.samplerIndex;
    return output;
}

// Bindless texture and sampler arrays are separate: binding 1 is sampled images,
// binding 6 is samplers. The engine writes both arrays independently.
Texture2D textures[];
SamplerState samplers[];

[shader("fragment")]
float4 fragMain(VSOutput vertIn) : SV_TARGET {
    float4 texcol = textures[vertIn.textureIndex].Sample(samplers[vertIn.samplerIndex], vertIn.fragUv);
    return float4(vertIn.fragColor * texcol.rgb, 1.0);
}