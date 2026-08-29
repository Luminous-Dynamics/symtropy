// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Persistent object-identity evidence for ARTIST-EYE-v1D.
//!
//! This module is renderer-neutral. A host may render an object-ID attachment
//! however it chooses, but evidence enters the artistic eye only through a
//! frozen collision-free registry that maps persistent [`crate::art_scene::ArtEntityId`]
//! values onto non-zero `u32` raster IDs. Zero is reserved for background.
//!
//! The resulting evidence is descriptive. It does not define salience,
//! aesthetic value, utility, reward, fitness, or mutation authority.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    art_capture::{ArtCaptureError, ArtCaptureReceipt, ArtRenderChannel},
    art_timeline::StudioFrame,
};

/// Frozen, collision-free mapping between stable artistic identities and the
/// integer labels carried by an object-ID raster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectIdRegistry {
    stable_to_raster: BTreeMap<String, u32>,
    raster_to_stable: BTreeMap<u32, String>,
    digest: String,
}

impl ObjectIdRegistry {
    /// Build a registry deterministically from the complete set of stable IDs
    /// that may appear in one qualification/session lineage.
    ///
    /// IDs are sorted lexicographically before assigning `1..=N`, so the same
    /// stable set always produces the same raster IDs regardless of host query
    /// order. Raster ID zero is reserved for background.
    pub fn from_stable_ids<I, S>(ids: I) -> Result<Self, ObjectIdError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut unique = BTreeSet::new();
        for id in ids {
            let id = id.into();
            if id.trim().is_empty() {
                return Err(ObjectIdError::EmptyStableId);
            }
            if !unique.insert(id.clone()) {
                return Err(ObjectIdError::DuplicateStableId(id));
            }
        }
        if unique.is_empty() {
            return Err(ObjectIdError::EmptyRegistry);
        }
        if unique.len() > (u32::MAX as usize).saturating_sub(1) {
            return Err(ObjectIdError::RegistryTooLarge);
        }

        let mut stable_to_raster = BTreeMap::new();
        let mut raster_to_stable = BTreeMap::new();
        for (index, stable_id) in unique.into_iter().enumerate() {
            let raster_id = u32::try_from(index + 1).map_err(|_| ObjectIdError::RegistryTooLarge)?;
            stable_to_raster.insert(stable_id.clone(), raster_id);
            raster_to_stable.insert(raster_id, stable_id);
        }

        let digest = registry_digest(&stable_to_raster);
        Ok(Self {
            stable_to_raster,
            raster_to_stable,
            digest,
        })
    }

    pub fn raster_id(&self, stable_id: &str) -> Option<u32> {
        self.stable_to_raster.get(stable_id).copied()
    }

    pub fn stable_id(&self, raster_id: u32) -> Option<&str> {
        self.raster_to_stable.get(&raster_id).map(String::as_str)
    }

    pub fn contains_stable_id(&self, stable_id: &str) -> bool {
        self.stable_to_raster.contains_key(stable_id)
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn len(&self) -> usize {
        self.stable_to_raster.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stable_to_raster.is_empty()
    }

    pub fn stable_ids(&self) -> impl Iterator<Item = &str> {
        self.stable_to_raster.keys().map(String::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectBoundingBox {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl ObjectBoundingBox {
    pub fn width(self) -> u32 {
        self.max_x.saturating_sub(self.min_x).saturating_add(1)
    }

    pub fn height(self) -> u32 {
        self.max_y.saturating_sub(self.min_y).saturating_add(1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectRasterEvidence {
    pub stable_id: String,
    pub raster_id: u32,
    pub visible_pixels: u64,
    pub visible_fraction: f64,
    pub centroid_x_normalized: f64,
    pub centroid_y_normalized: f64,
    pub bounding_box: ObjectBoundingBox,
    pub touches_frame_border: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectIdPlaneEvidence {
    pub width: u32,
    pub height: u32,
    pub background_fraction: f64,
    pub visible_object_count: u32,
    /// Only objects actually visible in the raster are stored here. Absence is
    /// evidence of zero visible pixels, not evidence that the semantic entity
    /// was destroyed.
    pub objects: Vec<ObjectRasterEvidence>,
}

impl ObjectIdPlaneEvidence {
    pub fn object(&self, stable_id: &str) -> Option<&ObjectRasterEvidence> {
        self.objects.iter().find(|object| object.stable_id == stable_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectIdObservation {
    pub capture_id: String,
    pub revision_id: String,
    pub frame: StudioFrame,
    pub rendered_scene_hash: String,
    pub registry_digest: String,
    pub evidence: ObjectIdPlaneEvidence,
}

impl ObjectIdObservation {
    pub fn from_capture_u32(
        receipt: &ArtCaptureReceipt,
        samples: &[u32],
        row_stride_values: usize,
        registry: &ObjectIdRegistry,
    ) -> Result<Self, ObjectIdError> {
        receipt
            .validate_alignment()
            .map_err(ObjectIdError::Capture)?;
        if !receipt.request.channels.contains(&ArtRenderChannel::ObjectId) {
            return Err(ObjectIdError::ObjectIdChannelNotDeclared);
        }
        let evidence = analyze_object_id_plane(
            receipt.request.width,
            receipt.request.height,
            row_stride_values,
            samples,
            registry,
        )?;
        Ok(Self {
            capture_id: receipt.request.capture_id.clone(),
            revision_id: receipt.observed_revision_id.clone(),
            frame: receipt.observed_frame,
            rendered_scene_hash: receipt.observed_scene_hash.clone(),
            registry_digest: registry.digest().to_owned(),
            evidence,
        })
    }
}

#[derive(Debug, Default)]
struct Accumulator {
    count: u64,
    sum_x: u128,
    sum_y: u128,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
    initialized: bool,
}

impl Accumulator {
    fn add(&mut self, x: u32, y: u32) {
        self.count = self.count.saturating_add(1);
        self.sum_x = self.sum_x.saturating_add(u128::from(x));
        self.sum_y = self.sum_y.saturating_add(u128::from(y));
        if !self.initialized {
            self.min_x = x;
            self.max_x = x;
            self.min_y = y;
            self.max_y = y;
            self.initialized = true;
        } else {
            self.min_x = self.min_x.min(x);
            self.max_x = self.max_x.max(x);
            self.min_y = self.min_y.min(y);
            self.max_y = self.max_y.max(y);
        }
    }
}

/// Analyze one object-ID plane. Row padding is ignored and unknown non-zero IDs
/// are rejected rather than silently treated as background.
pub fn analyze_object_id_plane(
    width: u32,
    height: u32,
    row_stride_values: usize,
    samples: &[u32],
    registry: &ObjectIdRegistry,
) -> Result<ObjectIdPlaneEvidence, ObjectIdError> {
    if width == 0 || height == 0 {
        return Err(ObjectIdError::InvalidDimensions);
    }
    if registry.is_empty() {
        return Err(ObjectIdError::EmptyRegistry);
    }
    let width_usize = width as usize;
    let height_usize = height as usize;
    if row_stride_values < width_usize {
        return Err(ObjectIdError::RowStrideTooSmall);
    }
    let required = row_stride_values
        .checked_mul(height_usize)
        .ok_or(ObjectIdError::DimensionOverflow)?;
    if samples.len() < required {
        return Err(ObjectIdError::InsufficientSamples {
            required,
            actual: samples.len(),
        });
    }

    let mut by_raster: BTreeMap<u32, Accumulator> = BTreeMap::new();
    let mut background_pixels = 0u64;

    for y in 0..height_usize {
        let row = &samples[y * row_stride_values..y * row_stride_values + width_usize];
        for (x, raster_id) in row.iter().copied().enumerate() {
            if raster_id == 0 {
                background_pixels = background_pixels.saturating_add(1);
                continue;
            }
            if registry.stable_id(raster_id).is_none() {
                return Err(ObjectIdError::UnknownRasterId(raster_id));
            }
            by_raster
                .entry(raster_id)
                .or_default()
                .add(x as u32, y as u32);
        }
    }

    let total_pixels = u64::from(width) * u64::from(height);
    let mut objects = Vec::with_capacity(by_raster.len());
    for (raster_id, acc) in by_raster {
        let stable_id = registry
            .stable_id(raster_id)
            .ok_or(ObjectIdError::UnknownRasterId(raster_id))?
            .to_owned();
        let bbox = ObjectBoundingBox {
            min_x: acc.min_x,
            min_y: acc.min_y,
            max_x: acc.max_x,
            max_y: acc.max_y,
        };
        let centroid_x = acc.sum_x as f64 / acc.count as f64;
        let centroid_y = acc.sum_y as f64 / acc.count as f64;
        let nx = if width <= 1 {
            0.5
        } else {
            centroid_x / f64::from(width - 1)
        };
        let ny = if height <= 1 {
            0.5
        } else {
            centroid_y / f64::from(height - 1)
        };
        let touches_frame_border = bbox.min_x == 0
            || bbox.min_y == 0
            || bbox.max_x == width - 1
            || bbox.max_y == height - 1;
        objects.push(ObjectRasterEvidence {
            stable_id,
            raster_id,
            visible_pixels: acc.count,
            visible_fraction: acc.count as f64 / total_pixels as f64,
            centroid_x_normalized: nx,
            centroid_y_normalized: ny,
            bounding_box: bbox,
            touches_frame_border,
        });
    }
    objects.sort_by(|a, b| a.stable_id.cmp(&b.stable_id));

    Ok(ObjectIdPlaneEvidence {
        width,
        height,
        background_fraction: background_pixels as f64 / total_pixels as f64,
        visible_object_count: objects.len() as u32,
        objects,
    })
}

fn registry_digest(mapping: &BTreeMap<String, u32>) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    };
    feed(b"symthaea.object-id-registry.v1\0");
    for (stable, raster) in mapping {
        feed(&(stable.len() as u64).to_le_bytes());
        feed(stable.as_bytes());
        feed(&raster.to_le_bytes());
    }
    format!("fnv1a64:{hash:016x}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectIdError {
    Capture(ArtCaptureError),
    EmptyRegistry,
    EmptyStableId,
    DuplicateStableId(String),
    RegistryTooLarge,
    ObjectIdChannelNotDeclared,
    InvalidDimensions,
    RowStrideTooSmall,
    DimensionOverflow,
    InsufficientSamples { required: usize, actual: usize },
    UnknownRasterId(u32),
}

impl std::fmt::Display for ObjectIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "object-id capture error: {error}"),
            Self::EmptyRegistry => write!(f, "object-id registry may not be empty"),
            Self::EmptyStableId => write!(f, "stable object id may not be empty"),
            Self::DuplicateStableId(id) => write!(f, "duplicate stable object id: {id}"),
            Self::RegistryTooLarge => write!(f, "object-id registry exceeds u32 label capacity"),
            Self::ObjectIdChannelNotDeclared => write!(f, "capture did not declare ObjectId channel"),
            Self::InvalidDimensions => write!(f, "object-id plane dimensions must be non-zero"),
            Self::RowStrideTooSmall => write!(f, "object-id row stride is smaller than width"),
            Self::DimensionOverflow => write!(f, "object-id plane dimensions overflow usize"),
            Self::InsufficientSamples { required, actual } => write!(
                f,
                "object-id plane needs {required} values but only {actual} were supplied"
            ),
            Self::UnknownRasterId(id) => write!(f, "object-id raster contains unknown non-zero id {id}"),
        }
    }
}

impl std::error::Error for ObjectIdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_order_independent() {
        let a = ObjectIdRegistry::from_stable_ids(["b", "a", "c"]).unwrap();
        let b = ObjectIdRegistry::from_stable_ids(["c", "b", "a"]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.raster_id("a"), Some(1));
        assert_eq!(a.raster_id("b"), Some(2));
        assert_eq!(a.raster_id("c"), Some(3));
    }

    #[test]
    fn raster_extracts_centroid_and_ignores_padding() {
        let registry = ObjectIdRegistry::from_stable_ids(["form"]).unwrap();
        // 3x2 visible plane with one padding value per row. Padding deliberately
        // contains an invalid label and must never be interpreted as a pixel.
        let samples = [0, 1, 0, 999, 0, 1, 0, 999];
        let evidence = analyze_object_id_plane(3, 2, 4, &samples, &registry).unwrap();
        let form = evidence.object("form").unwrap();
        assert_eq!(form.visible_pixels, 2);
        assert!((form.centroid_x_normalized - 0.5).abs() < 1e-12);
        assert!((form.centroid_y_normalized - 0.5).abs() < 1e-12);
    }

    #[test]
    fn unknown_nonzero_label_is_fail_closed() {
        let registry = ObjectIdRegistry::from_stable_ids(["form"]).unwrap();
        assert_eq!(
            analyze_object_id_plane(1, 1, 1, &[77], &registry),
            Err(ObjectIdError::UnknownRasterId(77))
        );
    }

    #[test]
    fn object_absence_is_not_registry_absence() {
        let registry = ObjectIdRegistry::from_stable_ids(["a", "b"]).unwrap();
        let evidence = analyze_object_id_plane(2, 1, 2, &[1, 0], &registry).unwrap();
        assert!(evidence.object("a").is_some());
        assert!(evidence.object("b").is_none());
        assert!(registry.contains_stable_id("b"));
    }
}
