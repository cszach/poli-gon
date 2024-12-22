@group(0) @binding(0) var<uniform> model_matrix: mat4x4f;
@group(0) @binding(1) var<uniform> model_view_matrix: mat4x4f;
@group(0) @binding(2) var<uniform> projection_matrix: mat4x4f;
@group(0) @binding(3) var<uniform> view_matrix: mat4x4f;
@group(0) @binding(4) var<uniform> normal_matrix: mat3x3f;
@group(0) @binding(5) var<uniform> camera_position: vec3f;

struct VertexInput {
  @location(1) position: vec3f,
  @location(2) normal: vec3f,
  @location(3) uv: vec2f,
}

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location (2) normal: vec3f,
  @location(3) uv: vec2f,
}

@vertex
fn vertexMain(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  output.position = projection_matrix * model_view_matrix * vec4f(input.position, 1);
  output.normal = input.normal;
  output.uv = input.uv;

  return output;
}

@fragment
fn fragmentMain(input: VertexOutput) -> @location(0) vec4f {
  return vec4f((input.normal + 1) / 2, 1);
}
