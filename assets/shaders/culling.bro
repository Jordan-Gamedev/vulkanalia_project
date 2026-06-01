struct PerInstanceData {
    uint modelMatrixInfo;
    uint textureIndex;
    uint padding0;
    uint padding1;
};

struct QuantizedModelMatrix {
    float[3] position;
    float[3] scale;
    int16_t[4] rotation;
};

[[vk::binding(2, 0)]]
StructuredBuffer<QuantizedModelMatrix> staticModelMatrices;

[[vk::binding(3, 0)]]
StructuredBuffer<QuantizedModelMatrix> dynamicModelMatrices;

[[vk::binding(5, 0)]]
StructuredBuffer<PerInstanceData> instanceData;

[shader("compute")]
[numthreads(256, 1, 1)]
void compMain(uint3 threadId: SV_DispatchThreadID) {
    uint gID = threadId.x;
    
    
}