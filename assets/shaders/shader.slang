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
    float3 position;
    float3 scale;
    uint4 rotation;
};

[[vk::binding(2, 0)]]
StructuredBuffer<QuantizedModelMatrix> staticModelMatrices;

[[vk::binding(3, 0)]]
StructuredBuffer<QuantizedModelMatrix> dynamicModelMatrices;
ConstantBuffer<UniformBuffer> ubo;

struct VSOutput
{
    float4 pos : SV_Position;
    float3 fragColor;
    float2 fragUv;
};

struct PushConstants {
    uint modelMatrixInfo;
    uint textureIndex;
};

[[vk::push_constant]]
ConstantBuffer<PushConstants> pushConstants;

float4 decodeQuaternion(uint4 packedRotation) {
    float4 q = (float4(packedRotation) / 65535.0) * 2.0 - 1.0;
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
        float4(right * modelMatrix.scale.x, 0.0),
        float4(up * modelMatrix.scale.y, 0.0),
        float4(forward * modelMatrix.scale.z, 0.0),
        float4(modelMatrix.position, 1.0)
    );
}

[shader("vertex")]
VSOutput vertMain(VSInput input) {
    VSOutput output;

    bool isStatic = (pushConstants.modelMatrixInfo & 0x80000000u) != 0;
    uint modelMatrixIndex = pushConstants.modelMatrixInfo & 0x7FFFFFFFu;

    QuantizedModelMatrix modelMatrix = isStatic ? staticModelMatrices[modelMatrixIndex] : dynamicModelMatrices[modelMatrixIndex];
    float4x4 model = buildModelMatrix(modelMatrix);

    output.pos = mul(ubo.proj, mul(ubo.view, mul(model, float4(input.inPos, 1.0))));
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