export struct QuantizedModelMatrix
{
    float[3] position;
    float[3] scale;
    int16_t[4] rotation;
};

export float4 decodeQuaternion(int16_t[4] packedRotation)
{
    float4 q = float4(packedRotation[0], packedRotation[1], packedRotation[2], packedRotation[3]) / int16_t::maxValue;
    return normalize(q);
}

export float3 rotateVector(float3 vector, float4 quaternion)
{
    float3 t = 2.0 * cross(quaternion.xyz, vector);
    return vector + quaternion.w * t + cross(quaternion.xyz, t);
}

export float4x4 buildModelMatrix(QuantizedModelMatrix modelMatrix)
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
