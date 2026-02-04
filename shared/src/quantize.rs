pub fn quantize_f32_to_u16(x: f32, min: f32, max: f32) -> u16 {
    assert!(max > min);
    assert!(min.is_finite() && max.is_finite());
    assert!(x.is_finite());

    let x = x.clamp(min, max);
    let t = (x - min) / (max - min); // [0, 1]
    let q = (t * (u16::MAX as f32)).round();

    // Defensive clamp in case of float edge cases.
    q.clamp(0.0, u16::MAX as f32) as u16
}

pub fn dequantize_u16_to_f32(code: u16, min: f32, max: f32) -> f32 {
    assert!(max > min);
    assert!(min.is_finite() && max.is_finite());

    let t = (code as f32) / (u16::MAX as f32); // [0, 1]
    min + t * (max - min)
}

pub fn quantization_step(min: f32, max: f32) -> f32 {
    assert!(max > min);
    (max - min) / (u16::MAX as f32)
}
