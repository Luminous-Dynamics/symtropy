// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lossless byte codec for object-ID render targets.
//!
//! Raster ID zero is reserved for background. IDs are encoded little-endian
//! across RGBA8 so the GPU attachment can be read back without depending on
//! floating-point color interpretation in the evidence layer.

#[inline]
pub const fn raster_id_to_rgba8(raster_id: u32) -> [u8; 4] {
    raster_id.to_le_bytes()
}

#[inline]
pub const fn rgba8_to_raster_id(rgba: [u8; 4]) -> u32 {
    u32::from_le_bytes(rgba)
}

pub fn decode_rgba8_plane(
    width: u32,
    height: u32,
    row_stride_bytes: usize,
    bytes: &[u8],
) -> Result<Vec<u32>, ObjectIdCodecError> {
    if width == 0 || height == 0 {
        return Err(ObjectIdCodecError::InvalidDimensions);
    }
    let visible_row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or(ObjectIdCodecError::DimensionOverflow)?;
    if row_stride_bytes < visible_row_bytes || row_stride_bytes % 4 != 0 {
        return Err(ObjectIdCodecError::InvalidRowStride);
    }
    let required = row_stride_bytes
        .checked_mul(height as usize)
        .ok_or(ObjectIdCodecError::DimensionOverflow)?;
    if bytes.len() < required {
        return Err(ObjectIdCodecError::InsufficientBytes {
            required,
            actual: bytes.len(),
        });
    }

    let mut out = Vec::with_capacity((width as usize) * (height as usize));
    for y in 0..height as usize {
        let row = &bytes[y * row_stride_bytes..y * row_stride_bytes + visible_row_bytes];
        for pixel in row.chunks_exact(4) {
            out.push(rgba8_to_raster_id([pixel[0], pixel[1], pixel[2], pixel[3]]));
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectIdCodecError {
    InvalidDimensions,
    InvalidRowStride,
    DimensionOverflow,
    InsufficientBytes { required: usize, actual: usize },
}

impl std::fmt::Display for ObjectIdCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimensions => write!(f, "object-id image dimensions must be non-zero"),
            Self::InvalidRowStride => write!(f, "object-id row stride must fit whole RGBA8 pixels"),
            Self::DimensionOverflow => write!(f, "object-id image dimensions overflow usize"),
            Self::InsufficientBytes { required, actual } => write!(
                f,
                "object-id image requires {required} bytes but only {actual} were supplied"
            ),
        }
    }
}

impl std::error::Error for ObjectIdCodecError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_u32_round_trip_is_exact() {
        for id in [0, 1, 255, 256, 65_535, 16_777_215, u32::MAX] {
            assert_eq!(rgba8_to_raster_id(raster_id_to_rgba8(id)), id);
        }
    }

    #[test]
    fn decoder_ignores_row_padding() {
        let a = raster_id_to_rgba8(7);
        let b = raster_id_to_rgba8(9);
        let poison = raster_id_to_rgba8(u32::MAX);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&a);
        bytes.extend_from_slice(&b);
        bytes.extend_from_slice(&poison);
        bytes.extend_from_slice(&b);
        bytes.extend_from_slice(&a);
        bytes.extend_from_slice(&poison);
        assert_eq!(decode_rgba8_plane(2, 2, 12, &bytes).unwrap(), vec![7, 9, 9, 7]);
    }
}
