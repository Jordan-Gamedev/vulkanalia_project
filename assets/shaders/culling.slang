struct UniformBuffer
{
    float4x4 view;
    float4x4 proj;
};

struct MeshMetadata
{
    float3 min_aabb;
    float3 max_aabb;
    uint16_t cull_percentage;
    uint16_t lod3_percentage;
    uint8_t lod1_percentage;
    uint8_t lod2_percentage;
}

struct MeshBufferLayout
{
    MeshMetadata meshes[2048];
}

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

[[vk::binding(2, 0)]]
ConstantBuffer<UniformBuffer> ubo;

[[vk::binding(3, 0)]]
ConstantBuffer<MeshBufferLayout, Std430DataLayout> meshUniformBuffer;

[[vk::binding(4, 0)]]
StructuredBuffer<QuantizedModelMatrix> staticModelMatrices;

[[vk::binding(5, 0)]]
StructuredBuffer<QuantizedModelMatrix> dynamicModelMatrices;

[[vk::binding(6, 0)]]
StructuredBuffer<PerInstanceData> sourceInstanceData;

[[vk::binding(7, 0)]]
RWStructuredBuffer<IndirectDrawData> outputIndirectDrawBuffer;

[[vk::binding(8, 0)]]
RWStructuredBuffer<PerInstanceData> outputInstanceBuffer;

// First pass updates IndirectDrawData Buffer instance counts based on the count of visible instances

[shader("compute")]
[numthreads(64, 1, 1)]
void pass1(uint3 threadId: SV_DispatchThreadID, uniform uint elementCount)
{
    uint gID = threadId.x;

    if (gID >= elementCount)
        return;

    // Get instance and associated mesh
    const PerInstanceData sourceInstance = sourceInstanceData[gID];
    MeshMetadata sourceInstanceMesh = meshUniformBuffer.meshes[sourceInstance.meshMetadataIndex];

    // Temp set isVisible to true
    bool isVisible = true;

    if (!isVisible)
        return;

    // Calculate lod and determine IndirectDrawBuffer index (temp chosen as 0)
    uint8_t chosenLod = 0;
    uint32_t drawIndex = sourceInstance.meshMetadataIndex * 4 + chosenLod;

    // Count number of visible instances for the chosen draw call
    InterlockedAdd(outputIndirectDrawBuffer[drawIndex].instanceCount, 1);
}

// Second pass updates IndirectDrawData Buffer instance offsets based on instance counts

[shader("compute")]
[numthreads(1, 1, 1)]
void pass2(uint3 threadId: SV_DispatchThreadID)
{
    uint currentOffset = 0;
    for (uint32_t i = 0; i < 8192; i++)
    {
        uint count = outputIndirectDrawBuffer[i].instanceCount;
        outputIndirectDrawBuffer[i].instanceCount = 0;
        outputIndirectDrawBuffer[i].firstInstance = currentOffset;
        currentOffset += count;
    }
}

// Third pass organizes them inside the Instance Buffer using IndirectDrawData Buffer

[shader("compute")]
[numthreads(64, 1, 1)]
void pass3(uint3 threadId: SV_DispatchThreadID, uniform uint elementCount)
{
    uint gID = threadId.x;

    if (gID >= elementCount)
        return;

    // Get instance and associated mesh
    const PerInstanceData sourceInstance = sourceInstanceData[gID];
    MeshMetadata sourceInstanceMesh = meshUniformBuffer.meshes[sourceInstance.meshMetadataIndex];

    // Temp set isVisible to true
    bool isVisible = true;

    if (!isVisible)
        return;

    // Calculate lod and determine IndirectDrawBuffer index
    uint8_t chosenLod = 0;
    uint32_t drawIndex = sourceInstance.meshMetadataIndex * 4 + chosenLod;

    // Add to the count of instances for the draw call
    uint32_t instanceIndexOffset;
    InterlockedAdd(outputIndirectDrawBuffer[drawIndex].instanceCount, 1, instanceIndexOffset);

    // Place visible instance in instance buffer
    outputInstanceBuffer[outputIndirectDrawBuffer[drawIndex].firstInstance + instanceIndexOffset] = sourceInstance;
}
