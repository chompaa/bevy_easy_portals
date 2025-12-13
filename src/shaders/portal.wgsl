#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view, 
    utils::coords_to_viewport_uv,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_color_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let viewport_uv = coords_to_viewport_uv(mesh.position.xy, view.viewport);
    return textureSample(base_color_texture, base_color_sampler, viewport_uv);
}
