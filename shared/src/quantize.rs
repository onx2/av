// pub fn quantize_f32_to_u16(x: f32, min: f32, max: f32) -> u16 {
//     assert!(max > min);
//     assert!(min.is_finite() && max.is_finite());
//     assert!(x.is_finite());

//     let x = x.clamp(min, max);
//     let t = (x - min) / (max - min); // [0, 1]
//     let q = (t * (u16::MAX as f32)).round();

//     // Defensive clamp in case of float edge cases.
//     q.clamp(0.0, u16::MAX as f32) as u16
// }

// pub fn dequantize_u16_to_f32(code: u16, min: f32, max: f32) -> f32 {
//     assert!(max > min);
//     assert!(min.is_finite() && max.is_finite());

//     let t = (code as f32) / (u16::MAX as f32); // [0, 1]
//     min + t * (max - min)
// }

// pub fn quantization_step(min: f32, max: f32) -> f32 {
//     assert!(max > min);
//     (max - min) / (u16::MAX as f32)
// }

// pub fn yaw_to_u8(yaw_radians: f32) -> u8 {
//     const SCALE: f32 = 256.0 / TAU;

//     // 1. Multiply to get range approx [-128.0, 128.0]
//     // 2. Cast to i32 to handle the negative sign
//     // 3. Cast to u8 to truncate to the 0..255 range
//     (yaw_radians * SCALE) as i32 as u8
// }

// /// Dequantize `u8` yaw back into radians in [0, 2π).
// pub fn yaw_from_u8(code: u8) -> f32 {
//     (code as f32) * (TAU / 256.0)
// }

// /// Quantize radians into a u16 [0, 65535].
// pub fn yaw_to_u16(yaw_radians: f32) -> u16 {
//     const SCALE: f32 = 65536.0 / TAU;

//     // 1. Multiply to get range approx [-32768.0, 32768.0] (if input is -PI to PI)
//     // 2. Cast to i32 to handle negative signs via bit wrapping
//     // 3. Cast to u16 to truncate to the 0..65535 range
//     (yaw_radians * SCALE) as i32 as u16
// }

// /// Dequantize `u16` yaw back into radians in [0, 2π).
// pub fn yaw_from_u16(code: u16) -> f32 {
//     const REV_SCALE: f32 = TAU / 65536.0;

//     (code as f32) * REV_SCALE
// }
//
use crate::VERTICAL_VELOCITY_Q_MPS;

pub fn quantize_vertical_velocity(vel: f32) -> i8 {
    let vq = (vel / VERTICAL_VELOCITY_Q_MPS).round();
    vq.clamp(i8::MIN as f32, i8::MAX as f32) as i8
}

pub fn dequantize_vertical_velocity(v_q: i8) -> f32 {
    v_q as f32 * VERTICAL_VELOCITY_Q_MPS
}
