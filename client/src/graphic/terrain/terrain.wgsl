
// Vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
    origin: vec3<i32>,
};

@group(0) @binding(0) //group is define in the Pipeline Layout, binding is defined in the Camera layout
var<uniform> camera: CameraUniform;


struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_coord: vec2<f32>,
    @location(2) texture_index: u32,
    @location(3) chunk_pos: vec3<i32>, //chunk_pos in the world, can be seen as a dynamic origin
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coord: vec2<f32>,
    @location(1) texture_index: u32,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let displacement = vec3<f32>(model.chunk_pos - camera.origin) * 16.0;
    out.clip_position = camera.view_proj * vec4<f32>(model.position + displacement, 1.0);
    out.texture_coord = model.texture_coord;
    out.texture_index = model.texture_index;
    return out;
}

// Fragment shader

@group(1) @binding(0) //group is define in the Pipeline Layout, binding is defined in the Texture layout
var texture: texture_2d_array<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(texture, texture_sampler, in.texture_coord, in.texture_index);
}



