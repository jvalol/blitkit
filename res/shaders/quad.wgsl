struct Screen {
    // Window size in physical pixels. zw is padding.
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;

// Positions are in pixels with the origin at the top-left and y pointing down,
// the same space text is drawn in.
@vertex
fn vs_main(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
    let clip = vec2<f32>(
        position.x / screen.size.x * 2.0 - 1.0,
        1.0 - position.y / screen.size.y * 2.0,
    );
    return vec4<f32>(clip, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0);
}
