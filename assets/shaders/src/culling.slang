import buffers;
import mathUtils;

struct BoundingBox
{
    float3 localCenter;
    float3 worldCenter;
    float3 localExtents;
    float3 worldExtents;

    static BoundingBox calculate(in const MeshMetadata metadata, in float4x4 modelMatrix)
    {
        float3 localCenter = 0.5f * float3(metadata.minAabb[0] + metadata.maxAabb[0], metadata.minAabb[1] + metadata.maxAabb[1], metadata.minAabb[2] + metadata.maxAabb[2]);
        float3 localExtents = 0.5f * float3(metadata.maxAabb[0] - metadata.minAabb[0], metadata.maxAabb[1] - metadata.minAabb[1], metadata.maxAabb[2] - metadata.minAabb[2]);
        float3 worldCenter = mul(modelMatrix, float4(localCenter, 1.0f)).xyz;
        float3 worldExtents = abs(modelMatrix[0].xyz) * localExtents.x + abs(modelMatrix[1].xyz) * localExtents.y + abs(modelMatrix[2].xyz) * localExtents.z;

        return BoundingBox(
            localCenter,
            worldCenter,
            localExtents,
            worldExtents, );
    }
}

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
    if (sourceInstance.meshMetadataIndex == uint16_t::maxValue)
    {
        return;
    }

    // Associated mesh data
    const MeshMetadata meshMetadata = meshMetadataBuffer[sourceInstance.meshMetadataIndex];

    // Instance's model matrix
    const float4x4 modelMatrix = buildModelMatrix(modelMatrixBuffer[gID]);

    // Calculate bounding box
    const BoundingBox boundingBox = BoundingBox.calculate(meshMetadata, modelMatrix);

    // Check if mesh is visible
    if (!isVisible(boundingBox))
    {
        return;
    }

    // Calculate LOD and determine the draw call index
    uint chosenLod = selectLod(meshMetadata, boundingBox);
    if (chosenLod == uint::maxValue)
    {
        return;
    }

    uint drawIndex = sourceInstance.meshMetadataIndex * 4 + chosenLod;

    // Increase instance count for the draw call
    InterlockedAdd(indirectDrawData[drawIndex].instanceCount, 1);
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
    const MeshMetadata meshMetadata = meshMetadataBuffer[sourceInstance.meshMetadataIndex];

    // Instance's model matrix
    const float4x4 modelMatrix = buildModelMatrix(modelMatrixBuffer[gID]);

    // Calculate bounding box
    const BoundingBox boundingBox = BoundingBox.calculate(meshMetadata, modelMatrix);

    // Check if mesh is visible
    if (!isVisible(boundingBox))
    {
        return;
    }

    // Calculate LOD and determine the draw call index
    uint chosenLod = selectLod(meshMetadata, boundingBox);
    if (chosenLod == uint::maxValue)
    {
        return;
    }

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

bool isVisible(in const BoundingBox boundingBox)
{
    float4x4 viewProjMatrix = mul(ubo.proj, ubo.view);
    float4 row1 = viewProjMatrix[0];
    float4 row2 = viewProjMatrix[1];
    float4 row3 = viewProjMatrix[2];
    float4 row4 = viewProjMatrix[3];
    float4 testPlanes[6] =
        {
            // Left
            row4 + row1,
            // Right
            row4 - row1,
            // Bottom
            row4 + row2,
            // Top
            row4 - row2,
            // Near
            row3,
            // Far
            row4 - row3,
        };

    [unroll]
    for (int i = 0; i < 6; i++)
    {
        testPlanes[i] /= length(testPlanes[i].xyz);
    }

    [unroll]
    for (int i = 0; i < 6; i++)
    {
        float r = dot(boundingBox.worldExtents, abs(testPlanes[i].xyz));
        float s = dot(testPlanes[i].xyz, boundingBox.worldCenter) + testPlanes[i].w;
        if (s < -r)
        {
            return false;
        }
    }

    return true;
}

uint selectLod(in const MeshMetadata metadata, in const BoundingBox boundingBox)
{
    // Radius of the bounding sphere enclosing the local AABB
    float boundingRadius = length(boundingBox.localExtents);

    // Transform OBB center to view space
    float3 viewCenter = mul(ubo.view, float4(boundingBox.worldCenter, 1.0f)).xyz;
    float viewDepth = abs(viewCenter.z);

    // Projected diameter on screen (fraction of screen height)
    // uniformBuffer.proj[1][1] is the vertical focal length (1 / tan(FOV / 2))
    float projectedDiameter = abs(2.0f * boundingRadius * ubo.proj[1][1]) / viewDepth;

    // Convert to percentage represented as u16
    uint16_t screenPercentage = uint16_t(projectedDiameter * uint16_t::maxValue);

    if (screenPercentage < metadata.cullPercentage)
    {
        return uint::maxValue;
    }
    else if (screenPercentage < metadata.lod3Percentage)
    {
        return 3;
    }
    else if (screenPercentage < metadata.lod2Percentage)
    {
        return 2;
    }
    else if (screenPercentage < metadata.lod1Percentage)
    {
        return 1;
    }
    else
    {
        return 0;
    }
}
