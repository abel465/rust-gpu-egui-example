#![cfg_attr(target_arch = "spirv", no_std)]

use complex::Complex;
use shared::*;
use spirv_std::glam::{vec2, vec4, Vec2, Vec4};
use spirv_std::spirv;

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(frag_coord)] frag_coord: Vec4,
    #[spirv(push_constant)] constants: &ShaderConstants,
    output: &mut Vec4,
) {
    let coord = Complex::new(
        frag_coord.x + constants.translate_x + (constants.drag_start_x - constants.drag_end_x),
        frag_coord.y + constants.translate_y + (constants.drag_start_y - constants.drag_end_y),
    );

    let uv = constants.zoom
        * (coord - 0.5 * Complex::new(constants.width as f32, constants.height as f32))
        / constants.height as f32;

    let mut z = Complex::ZERO;
    let mut n = 35;
    while z.length() < 2.0 && n > 0 {
        z = z * z + uv;
        n -= 1;
    }

    let c = n as f32 / 35.0;
    *output = vec4(c, c, c, 1.0);
}

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(vertex_index)] vert_id: i32,
    #[spirv(position, invariant)] out_pos: &mut Vec4,
) {
    let uv = vec2(((vert_id << 1) & 2) as f32, (vert_id & 2) as f32);
    let pos = 2.0 * uv - Vec2::ONE;

    *out_pos = pos.extend(0.0).extend(1.0);
}
