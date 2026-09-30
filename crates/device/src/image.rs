//! Png encoding of a rendered frame.

use crate::render::{Frame, RenderError};

/// Encodes a frame as a png.
pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, RenderError> {
    let mut bytes = Vec::new();
    write_into(&mut bytes, frame)?;
    Ok(bytes)
}

fn write_into(bytes: &mut Vec<u8>, frame: &Frame) -> Result<(), RenderError> {
    use ::png::Encoder;

    let mut encoder = Encoder::new(bytes, frame.width, frame.height);
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| RenderError::Encoding(error.to_string()))?;
    writer
        .write_image_data(&frame.rgba)
        .map_err(|error| RenderError::Encoding(error.to_string()))?;
    Ok(())
}
