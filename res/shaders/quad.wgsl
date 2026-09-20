struct Screen {
    // Window size in physical pixels. zw is padding.
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// Positions are in pixels with the origin at the top-left and y pointing down,
// the same space text is drawn in.
@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    let clip = vec2<f32>(
        position.x / screen.size.x * 2.0 - 1.0,
        1.0 - position.y / screen.size.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(clip, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
