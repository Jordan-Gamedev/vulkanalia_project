import mathUtils;

export struct UniformBuffer
{
    float4x4 view;
    float4x4 proj;
};

export struct IndirectDrawData
{
    uint indexCount;
    uint instanceCount;
    uint firstIndex;
    int vertexOffset;
    uint firstInstance;
};

export struct PerInstanceData
{
    uint16_t textureIndex;
    uint16_t samplerIndex;
    uint16_t meshMetadataIndex;
    uint16_t meshAssetId;
};

export struct MeshMetadata
{
    float[3] minAabb;
    float[3] maxAabb;
    uint16_t lod1Percentage;
    uint16_t lod2Percentage;
    uint16_t lod3Percentage;
    uint16_t cullPercentage;
}

[[vk::binding(0, 0)]]
export Texture2D textures[];

[[vk::binding(1, 0)]]
export SamplerState samplers[];

[[vk::binding(2, 0)]]
export ConstantBuffer<UniformBuffer> ubo;

[[vk::binding(3, 0)]]
export StructuredBuffer<QuantizedModelMatrix> modelMatrixBuffer;

[[vk::binding(4, 0)]]
export RWStructuredBuffer<IndirectDrawData> indirectDrawData;

[[vk::binding(5, 0)]]
export StructuredBuffer<PerInstanceData> instanceData;

[[vk::binding(6, 0)]]
export StructuredBuffer<MeshMetadata> meshMetadataBuffer;

#if defined(__FORCE_READ_ONLY__)
[[vk::binding(7, 0)]]
export StructuredBuffer<uint> visbuffer;
#else
[[vk::binding(7, 0)]]
export RWStructuredBuffer<uint> visbuffer;
#endif
