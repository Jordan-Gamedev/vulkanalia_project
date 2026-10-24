struct UniformBuffer
{
    float4x4 view;
    float4x4 proj;
};

struct MeshMetadata
{
    float[3] min_aabb;
    float[3] max_aabb;
    uint16_t cull_percentage;
    uint16_t lod3_percentage;
    uint16_t lod1_percentage;
    uint16_t lod2_percentage;
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
    uint16_t textureIndex;
    uint16_t samplerIndex;
    uint16_t meshMetadataIndex;
    uint16_t meshAssetId;
};

[[vk::binding(2, 0)]]
ConstantBuffer<UniformBuffer> ubo;

[[vk::binding(3, 0)]]
StructuredBuffer<QuantizedModelMatrix> modelMatrixBuffer;

[[vk::binding(4, 0)]]
RWStructuredBuffer<IndirectDrawData> indirectDrawData;

[[vk::binding(5, 0)]]
StructuredBuffer<PerInstanceData> instanceData;

[[vk::binding(6, 0)]]
StructuredBuffer<MeshMetadata> meshMetadataBuffer;

[[vk::binding(7, 0)]]
RWStructuredBuffer<uint> visbuffer;

// First pass clears instance counts and instance offsets

[shader("compute")]
[numthreads(64, 1, 1)]
void pass1(uint3 threadId: SV_DispatchThreadID)
{
    if (threadId.x < indirectDrawData.getCount())
    {
        indirectDrawData[threadId.x].firstInstance = 0;
        indirectDrawData[threadId.x].instanceCount = 0;
    }
}

// Second pass calculates visible instance counts

[require(spvGroupNonUniform, sm_6_6)]
[shader("compute")]
[numthreads(64, 1, 1)]
void pass2(uint3 threadId: SV_DispatchThreadID)
{
    uint gID = threadId.x;
    if (gID >= instanceData.getCount())
    {
        return;
    }

    // Instance
    const PerInstanceData sourceInstance = instanceData[gID];
    if (sourceInstance
            .meshMetadataIndex == uint16_t::maxValue)
    {
        return;
    }

    // Associated mesh data
    const MeshMetadata sourceInstanceMesh = meshMetadataBuffer[sourceInstance.meshMetadataIndex];

    // Perform frustum/occlusion culling math here
    bool isVisible = true;
    if (!isVisible)
    {
        return;
    }

    // Calculate lod and determine the draw call index
    uint8_t chosenLod = 0;
    uint drawIndex = sourceInstance.meshMetadataIndex * 4 + chosenLod;

    // Increase instance count for the draw call
    uint prev_count;
    InterlockedAdd(indirectDrawData[drawIndex].instanceCount, 1, prev_count);
}

// Third pass calculates instance offsets

[shader("compute")]
[numthreads(1, 1, 1)]
void pass3(uint3 threadId: SV_DispatchThreadID)
{
    if (threadId.x != 0)
        return;

    uint currentOffset = 0;
    uint drawCallCount = indirectDrawData.getCount();
    for (uint i = 0; i < drawCallCount; i++)
    {
        indirectDrawData[i].firstInstance = currentOffset;
        currentOffset += indirectDrawData[i].instanceCount;
    }
}

// Fourth pass fills in visbuffer

[shader("compute")]
[numthreads(64, 1, 1)]
void pass4(uint3 threadId: SV_DispatchThreadID)
{
    uint gID = threadId.x;
    if (gID >= instanceData.getCount())
    {
        return;
    }

    // Instance
    const PerInstanceData sourceInstance = instanceData[gID];
    if (sourceInstance.meshMetadataIndex == uint16_t::maxValue)
    {
        return;
    }

    // Associated mesh data
    const MeshMetadata sourceInstanceMesh = meshMetadataBuffer[sourceInstance.meshMetadataIndex];

    // Perform frustum/occlusion culling math here (must match Pass 2)
    bool isVisible = true;
    if (!isVisible)
    {
        return;
    }

    // Calculate LOD and determine the draw call index (must match Pass 2)
    uint8_t chosenLod = 0;
    uint drawIndex = sourceInstance.meshMetadataIndex * 4 + chosenLod;

    // Caclulate visbuffer position to place instance ID
    uint destIndex;
    InterlockedAdd(indirectDrawData[drawIndex].firstInstance, 1, destIndex);

    // Store the visible instance ID in the visbuffer
    visbuffer[destIndex] = gID;
}

// Fifth pass recalculates instance offsets

[shader("compute")]
[numthreads(1, 1, 1)]
void pass5(uint3 threadId: SV_DispatchThreadID)
{
    if (threadId.x != 0)
        return;

    uint currentOffset = 0;
    uint drawCallCount = indirectDrawData.getCount();
    for (uint i = 0; i < drawCallCount; i++)
    {
        indirectDrawData[i].firstInstance = currentOffset;
        currentOffset += indirectDrawData[i].instanceCount;
    }
}

// // First pass clears instance counts and instance offsets

// [shader("compute")]
// [numthreads(64, 1, 1)]
// void pass1(uint3 threadId: SV_DispatchThreadID)
// {
//     if (threadId.x < indirectDrawData.getCount())
//     {
//         indirectDrawData[threadId.x].firstInstance = 0;
//         indirectDrawData[threadId.x].instanceCount = 0;
//     }
// }

// // Second pass fills in the visbuffer and instance counts

// [require(spvGroupNonUniform, sm_6_6)]
// [shader("compute")]
// [numthreads(64, 1, 1)]
// void pass2(uint3 threadId: SV_DispatchThreadID)
// {
//     uint gID = threadId.x;
//     bool isVisible = false;
//     uint32_t drawIndex = uint32_t::maxValue;

//     if (gID < instanceData.getCount())
//     {
//         // Get instance and associated mesh
//         const PerInstanceData sourceInstance = instanceData[gID];
//         const MeshMetadata sourceInstanceMesh = meshMetadataBuffer[sourceInstance.meshMetadataIndex];

//         // Perform frustum/occlusion culling math here
//         isVisible = sourceInstance.meshMetadataIndex != uint16_t::maxValue;

//         if (isVisible)
//         {
//             // Calculate lod and determine IndirectDrawBuffer index
//             uint8_t chosenLod = 0;
//             drawIndex = sourceInstance.meshMetadataIndex * 4 + chosenLod;
//         }
//     }

//     // FIX: Evaluate wave intrinsics UNCONDITIONALLY across all lanes in the wave
//     uint waveMatchCount = WaveActiveCountBits(isVisible);
//     uint waveMultiPrefix = WavePrefixCountBits(isVisible);

//     // 1. Compact Visible Indices to Global Visibility Buffer
//     if (isVisible)
//     {
//         uint globalBaseIndex = 0;

//         if (WaveIsFirstLane())
//         {
//             // Atomically allocate a chunk for this wave
//             InterlockedAdd(visbuffer[0], waveMatchCount, globalBaseIndex);
//         }

//         globalBaseIndex = WaveReadLaneFirst(globalBaseIndex);

//         uint flatAllocationIndex = globalBaseIndex + waveMultiPrefix + 1;
//         visbuffer[flatAllocationIndex] = gID;
//     }

//     // 2. Accumulate Multi-Draw Indirect Counts
//     while (WaveActiveAnyTrue(isVisible))
//     {
//         // Get a ballot of all visible lanes in the wave
//         uint4 ballot = WaveActiveBallot(isVisible);

//         // Find the first lane where isVisible is true across Wave32 or Wave64
//         uint firstLane;
//         if (ballot.x != 0)
//         {
//             firstLane = firstbitlow(ballot.x);
//         }
//         else
//         {
//             firstLane = firstbitlow(ballot.y) + 32;
//         }

//         // Read the draw index specifically from the first visible lane
//         uint32_t currentDrawIndex = WaveReadLaneAt(drawIndex, firstLane);

//         // Check which active lanes match this specific draw identifier
//         bool isMatch = isVisible && (drawIndex == currentDrawIndex);
//         uint matchCount = WaveActiveCountBits(isMatch);

//         // Use the first lane of the matching subset to commit the atomic add
//         if (isMatch && (WavePrefixCountBits(isMatch) == 0))
//         {
//             InterlockedAdd(indirectDrawData[currentDrawIndex].instanceCount, matchCount);
//         }

//         // Strip matched lanes out of the remaining loop evaluation
//         if (isMatch)
//         {
//             isVisible = false;
//         }
//     }
// }

// // Third pass fills in instance offsets

// [shader("compute")]
// [numthreads(1, 1, 1)]
// void pass3(uint3 threadId: SV_DispatchThreadID)
// {
//     if (threadId.x != 0)
//         return;

//     uint currentOffset = 0;
//     uint drawCallCount = indirectDrawData.getCount();
//     for (uint i = 0; i < drawCallCount; i++)
//     {
//         indirectDrawData[i].firstInstance = currentOffset;
//         currentOffset += indirectDrawData[i].instanceCount;
//     }
// }
