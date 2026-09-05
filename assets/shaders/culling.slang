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
StructuredBuffer<QuantizedModelMatrix> modelMatrixBuffer;

[[vk::binding(4, 0)]]
RWStructuredBuffer<IndirectDrawData> indirectDrawData;

[[vk::binding(5, 0)]]
StructuredBuffer<PerInstanceData> instanceData;

[[vk::binding(6, 0)]]
ConstantBuffer<MeshBufferLayout, Std430DataLayout> meshUniformBuffer;

[[vk::binding(7, 0)]]
RWStructuredBuffer<uint> visbuffer;

// First pass fills in the visbuffer

[require(spvGroupNonUniform, sm_6_6)]
[shader("compute")]
[numthreads(64, 1, 1)]
void pass1(uint3 threadId: SV_DispatchThreadID, uniform uint elementCount)
{
    uint gID = threadId.x;
    bool isVisible = false;
    uint32_t drawIndex = uint32_t::maxValue;

    if (gID < elementCount)
    {
        // Get instance and associated mesh
        const PerInstanceData sourceInstance = instanceData[gID];
        MeshMetadata sourceInstanceMesh = meshUniformBuffer.meshes[sourceInstance.meshMetadataIndex];

        // Temp set isVisible to trued (place cull arithmetic here)
        isVisible = true;

        if (isVisible)
        {
            // Calculate lod and determine IndirectDrawBuffer index (temp chosen as 0)
            uint8_t chosenLod = 0;
            drawIndex = sourceInstance.meshMetadataIndex * 4 + chosenLod;
        }
    }

    if (isVisible)
    {
        // Get number of threads returning isVisible as true
        uint waveMatchCount = WaveActiveCountBits(isVisible);

        // Get this shader program's active thread index
        uint waveMultiPrefix = WavePrefixCountBits(isVisible);

        uint globalBaseIndex = 0;

        // Make only one lane access Vram
        if (WaveIsFirstLane())
        {
            // Reserve a chunk of memory for the whole wave
            InterlockedAdd(visbuffer[0], waveMatchCount, globalBaseIndex);
        }

        // Broadcast base index to all other lanes in the wave
        globalBaseIndex = WaveReadLaneFirst(globalBaseIndex);
        uint flatAllocationIndex = globalBaseIndex + waveMultiPrefix + 1;

        // Write to visbuffer
        visbuffer[flatAllocationIndex] = gID;
    }

    while (WaveActiveAnyTrue(isVisible))
    {
        uint32_t currentDrawIndex = WaveReadLaneFirst(drawIndex);
        bool isMatch = isVisible && drawIndex == currentDrawIndex;
        uint matchCount = WaveActiveCountBits(isMatch);

        if (isMatch && WaveIsFirstLane())
        {
            // Track how many instances this draw call has
            InterlockedAdd(indirectDrawData[drawIndex].instanceCount, matchCount);
        }

        // Mark matching threads as processed
        if (isMatch)
        {
            isVisible = false;
        }
    }
}

// TODO: Verify if this is good code since this pass was written entirely with AI

// Allocate scratch space for inter-wave communication
// (256 threads / 32 lane wave size = max 8 waves)
groupshared uint sharedScratch[8];

[shader("compute")]
[numthreads(256, 1, 1)]
void pass2(uint3 threadId: SV_GroupThreadID)
{
    uint tid = threadId.x;

    // 8,192 total draws / 256 threads = exactly 32 draws assigned per thread
    const uint DRAWS_PER_THREAD = 32;
    uint startIndex = tid * DRAWS_PER_THREAD;

    // Step 1: Read the scattered instance counts from Pass 1 into local registers
    // Accumulate a local running sum for this thread's 32-draw chunk
    uint localSum = 0;
    uint threadCounts[DRAWS_PER_THREAD];

    [unroll]
    for (uint i = 0; i < DRAWS_PER_THREAD; ++i)
    {
        threadCounts[i] = indirectDrawData[startIndex + i].instanceCount;
        localSum += threadCounts[i];
    }

    // Step 2: Intra-Wave Scan
    // Calculate the running sum prefix across threads inside the same hardware wave
    uint waveSum = WaveActiveSum(localSum);
    uint wavePrefix = WavePrefixSum(localSum);

    // Step 3: Inter-Wave Communication
    uint laneId = tid % WaveGetLaneCount();
    uint waveId = tid / WaveGetLaneCount();

    // The first lane of each wave publishes that wave's total sum to group memory
    if (laneId == 0)
    {
        sharedScratch[waveId] = waveSum;
    }

    // Sync to guarantee all waves have successfully checked in
    GroupMemoryBarrierWithGroupSync();

    // The first wave handles the prefix sum of the wave totals
    if (waveId == 0 && laneId < 8)
    {
        uint s = sharedScratch[laneId];
        uint p = WavePrefixSum(s);
        sharedScratch[laneId] = p; // Store the global base offset for each wave
    }

    // Sync again so all threads can safely read the finalized wave base offsets
    GroupMemoryBarrierWithGroupSync();

    // Step 4: Combine the wave base and lane prefix to find this thread's global chunk starting point
    uint globalChunkOffset = sharedScratch[waveId] + wavePrefix;

    // Step 5: Iteratively unpack and write the sequential prefix sum back to firstInstance
    uint runningOffset = globalChunkOffset;

    for (uint j = 0; j < DRAWS_PER_THREAD; ++j)
    {
        indirectDrawData[startIndex + j].firstInstance = runningOffset;
        runningOffset += threadCounts[j];
    }
}
