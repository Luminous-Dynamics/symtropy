#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-translation-geometry-audit.receipt.v0.1";
pub const MAX_MASK_PIXELS: usize = 16_384;
pub const MILLI_PIXELS_PER_PIXEL: i64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnitKind {
    Target,
    Session,
    Sequence,
}

impl UnitKind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "TARGET" => Ok(Self::Target),
            "SESSION" => Ok(Self::Session),
            "SEQUENCE" => Ok(Self::Sequence),
            _ => Err(format!("unsupported unit kind: {value}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Target => "TARGET",
            Self::Session => "SESSION",
            Self::Sequence => "SEQUENCE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Occluded,
    OutOfFrame,
}

impl Visibility {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "VISIBLE" => Ok(Self::Visible),
            "OCCLUDED" => Ok(Self::Occluded),
            "OUT_OF_FRAME" => Ok(Self::OutOfFrame),
            _ => Err(format!("unsupported visibility state: {value}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Visible => "VISIBLE",
            Self::Occluded => "OCCLUDED",
            Self::OutOfFrame => "OUT_OF_FRAME",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropState {
    Full,
    Cropped,
    Truncated,
}

impl CropState {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "FULL" => Ok(Self::Full),
            "CROPPED" => Ok(Self::Cropped),
            "TRUNCATED" => Ok(Self::Truncated),
            _ => Err(format!("unsupported crop state: {value}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "FULL",
            Self::Cropped => "CROPPED",
            Self::Truncated => "TRUNCATED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopeKey {
    pub domain_id: String,
    pub condition_id: String,
    pub crop_stratum: String,
    pub modality: String,
    pub ood_group: String,
    pub calibration_revision: String,
    pub translator_sha256: String,
    pub image_width_px: u32,
    pub image_height_px: u32,
    pub unit_kind: UnitKind,
    pub unit_frame_budget: u32,
    pub landmark_tolerance_mpx: u64,
    pub temporal_motion_tolerance_mpx: u64,
    pub default_mask_iou_floor_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMetadata {
    pub sample_id: String,
    pub target_id: String,
    pub session_id: String,
    pub sequence_id: String,
    pub unit_id: String,
    pub unit_kind: UnitKind,
    pub partition: String,
    pub frame_index: u32,
    pub scope: ScopeKey,
    pub source_image_sha256: String,
    pub translated_image_sha256: String,
    pub source_annotation_sha256: String,
    pub translated_annotation_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandmarkPair {
    pub landmark_id: String,
    pub source_x_mpx: i64,
    pub source_y_mpx: i64,
    pub translated_x_mpx: i64,
    pub translated_y_mpx: i64,
    pub source_visibility: Visibility,
    pub translated_visibility: Visibility,
    pub source_crop_state: CropState,
    pub translated_crop_state: CropState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskPair {
    pub region_id: String,
    /// Canonical packed mask: one hexadecimal nibble per four pixels; pixel zero is bit 0.
    pub source_mask_hex: String,
    pub translated_mask_hex: String,
    pub iou_floor_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameAnnotations {
    pub metadata: FrameMetadata,
    pub landmarks: Vec<LandmarkPair>,
    pub masks: Vec<MaskPair>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskScore {
    pub region_id: String,
    pub intersection_pixels: u64,
    pub union_pixels: u64,
    pub iou_bps: u16,
    pub source_component_count: u32,
    pub translated_component_count: u32,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameScore {
    pub sample_id: String,
    pub frame_index: u32,
    pub disposition: String,
    pub reason_codes: Vec<String>,
    pub compared_landmark_count: u32,
    pub max_landmark_displacement_mpx: Option<u64>,
    pub p95_landmark_displacement_mpx: Option<u64>,
    pub mask_scores: Vec<MaskScore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitScore {
    pub unit_id: String,
    pub unit_kind: String,
    pub frame_count: u32,
    pub passed_frame_count: u32,
    pub blocked_frame_count: u32,
    pub max_landmark_displacement_mpx: Option<u64>,
    pub p95_landmark_displacement_mpx: Option<u64>,
    pub max_temporal_motion_residual_mpx: Option<u64>,
    pub disposition: String,
    pub reason_codes: Vec<String>,
    pub frames: Vec<FrameScore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReceipt {
    pub schema: String,
    pub unit_count: u64,
    pub frame_count: u64,
    pub passed_unit_count: u64,
    pub blocked_unit_count: u64,
    pub results: Vec<UnitScore>,
    pub image_pixels_inspected_here: bool,
    pub annotation_source_bytes_authenticated: bool,
    pub independent_metrology_verified_here: bool,
    pub physical_truth_established: bool,
    pub calibration_exchangeability_proven: bool,
    pub flight_qualification: bool,
    pub maneuver_authority_granted: bool,
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value != "NONE"
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn unit_identity(metadata: &FrameMetadata) -> &str {
    match metadata.unit_kind {
        UnitKind::Target => &metadata.target_id,
        UnitKind::Session => &metadata.session_id,
        UnitKind::Sequence => &metadata.sequence_id,
    }
}

fn ceil_sqrt(value: u128) -> Option<u64> {
    if value == 0 {
        return Some(0);
    }
    let mut low = 0_u128;
    let mut high = 1_u128 << 64;
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle;
        }
    }
    let square = low.checked_mul(low)?;
    let result = if square == value { low } else { low.checked_add(1)? };
    u64::try_from(result).ok()
}

fn displacement_mpx(dx: i64, dy: i64) -> Option<u64> {
    let x = i128::from(dx).abs() as u128;
    let y = i128::from(dy).abs() as u128;
    let squared = x.checked_mul(x)?.checked_add(y.checked_mul(y)?)?;
    ceil_sqrt(squared)
}

fn percentile_95(values: &mut [u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let rank = (values.len() * 95).div_ceil(100);
    values.get(rank.saturating_sub(1)).copied()
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn decode_mask(hex: &str, pixel_count: usize) -> Result<Vec<bool>, String> {
    if hex.len() != pixel_count.div_ceil(4) {
        return Err(format!(
            "mask hex length {} does not match {} pixels",
            hex.len(),
            pixel_count
        ));
    }
    let mut bits = Vec::with_capacity(pixel_count);
    for byte in hex.bytes() {
        let nibble = hex_nibble(byte).ok_or_else(|| "mask has non-hex character".to_owned())?;
        for bit in 0..4 {
            if bits.len() < pixel_count {
                bits.push((nibble & (1 << bit)) != 0);
            }
        }
    }
    Ok(bits)
}

fn component_count(mask: &[bool], width: usize, height: usize) -> u32 {
    let mut visited = vec![false; mask.len()];
    let mut components = 0_u32;
    let mut queue = VecDeque::<usize>::new();

    for index in 0..mask.len() {
        if !mask[index] || visited[index] {
            continue;
        }
        components += 1;
        visited[index] = true;
        queue.push_back(index);

        while let Some(current) = queue.pop_front() {
            let x = current % width;
            let y = current / width;
            let neighbors = [
                if x > 0 { Some(current - 1) } else { None },
                if x + 1 < width { Some(current + 1) } else { None },
                if y > 0 { Some(current - width) } else { None },
                if y + 1 < height { Some(current + width) } else { None },
            ];
            for neighbor in neighbors.into_iter().flatten() {
                if mask[neighbor] && !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
    }
    components
}

fn score_mask(pair: &MaskPair, width: usize, height: usize) -> Result<MaskScore, String> {
    if pair.iou_floor_bps > 10_000 {
        return Err(format!("mask {} IoU floor exceeds 10000 bps", pair.region_id));
    }
    let pixel_count = width
        .checked_mul(height)
        .filter(|count| *count > 0 && *count <= MAX_MASK_PIXELS)
        .ok_or_else(|| "mask dimensions exceed bounded v0.1 size".to_owned())?;
    let source = decode_mask(&pair.source_mask_hex, pixel_count)?;
    let translated = decode_mask(&pair.translated_mask_hex, pixel_count)?;

    let mut intersection = 0_u64;
    let mut union = 0_u64;
    for (&left, &right) in source.iter().zip(translated.iter()) {
        if left && right {
            intersection += 1;
        }
        if left || right {
            union += 1;
        }
    }
    if union == 0 {
        return Err(format!("mask {} has an empty source/translated union", pair.region_id));
    }
    let iou = u128::from(intersection)
        .checked_mul(10_000)
        .and_then(|value| value.checked_div(u128::from(union)))
        .ok_or_else(|| "IoU arithmetic failed".to_owned())?;
    let iou_bps = u16::try_from(iou).map_err(|_| "IoU basis point conversion failed")?;
    let source_components = component_count(&source, width, height);
    let translated_components = component_count(&translated, width, height);
    let passed = iou_bps >= pair.iou_floor_bps && source_components == translated_components;

    Ok(MaskScore {
        region_id: pair.region_id.clone(),
        intersection_pixels: intersection,
        union_pixels: union,
        iou_bps,
        source_component_count: source_components,
        translated_component_count: translated_components,
        passed,
    })
}

fn validate_frame(frame: &FrameAnnotations) -> Result<(), String> {
    let metadata = &frame.metadata;
    for (name, value) in [
        ("sample_id", metadata.sample_id.as_str()),
        ("target_id", metadata.target_id.as_str()),
        ("session_id", metadata.session_id.as_str()),
        ("sequence_id", metadata.sequence_id.as_str()),
        ("unit_id", metadata.unit_id.as_str()),
        ("domain_id", metadata.scope.domain_id.as_str()),
        ("condition_id", metadata.scope.condition_id.as_str()),
        ("crop_stratum", metadata.scope.crop_stratum.as_str()),
        ("modality", metadata.scope.modality.as_str()),
        ("ood_group", metadata.scope.ood_group.as_str()),
        ("calibration_revision", metadata.scope.calibration_revision.as_str()),
    ] {
        if !valid_text(value) {
            return Err(format!("{name} is empty or reserved"));
        }
    }
    if metadata.partition != "CALIBRATION" {
        return Err(format!(
            "geometry-preservation audit accepts CALIBRATION rows only, got {}",
            metadata.partition
        ));
    }
    if metadata.unit_id != unit_identity(metadata) {
        return Err(format!(
            "unit_id does not match declared {} identity",
            metadata.unit_kind.as_str()
        ));
    }
    if metadata.frame_index >= metadata.scope.unit_frame_budget
        || metadata.scope.unit_frame_budget == 0
        || metadata.scope.image_width_px == 0
        || metadata.scope.image_height_px == 0
    {
        return Err("invalid frame index, budget, or image dimensions".to_owned());
    }
    if metadata.scope.image_width_px
        .checked_mul(metadata.scope.image_height_px)
        .is_none_or(|count| count as usize > MAX_MASK_PIXELS)
    {
        return Err("image dimensions exceed bounded v0.1 mask size".to_owned());
    }
    if metadata.scope.default_mask_iou_floor_bps > 10_000 {
        return Err("default mask IoU floor exceeds 10000 bps".to_owned());
    }
    if !valid_sha256(&metadata.scope.translator_sha256)
        || !valid_sha256(&metadata.source_image_sha256)
        || !valid_sha256(&metadata.translated_image_sha256)
        || !valid_sha256(&metadata.source_annotation_sha256)
        || !valid_sha256(&metadata.translated_annotation_sha256)
    {
        return Err("invalid translator/image/annotation SHA-256 digest".to_owned());
    }
    if frame.landmarks.is_empty() || frame.masks.is_empty() {
        return Err("every frame requires at least one landmark and one mask".to_owned());
    }
    if metadata.unit_kind != metadata.scope.unit_kind {
        return Err("frame unit_kind differs from scope unit_kind".to_owned());
    }

    let mut landmark_ids = BTreeSet::new();
    for landmark in &frame.landmarks {
        if !valid_text(&landmark.landmark_id) || !landmark_ids.insert(landmark.landmark_id.as_str()) {
            return Err("empty or duplicate landmark identity".to_owned());
        }
    }
    let mut mask_ids = BTreeSet::new();
    for mask in &frame.masks {
        if !valid_text(&mask.region_id) || !mask_ids.insert(mask.region_id.as_str()) {
            return Err("empty or duplicate mask identity".to_owned());
        }
        if mask.iou_floor_bps < metadata.scope.default_mask_iou_floor_bps {
            return Err("per-region IoU floor is below the declared scope floor".to_owned());
        }
    }
    Ok(())
}

fn score_frame(frame: &FrameAnnotations) -> Result<FrameScore, String> {
    let mut reasons = BTreeSet::<String>::new();
    let width_mpx = i64::from(frame.metadata.scope.image_width_px) * MILLI_PIXELS_PER_PIXEL;
    let height_mpx = i64::from(frame.metadata.scope.image_height_px) * MILLI_PIXELS_PER_PIXEL;
    let mut displacements = Vec::new();

    for landmark in &frame.landmarks {
        if landmark.source_visibility != landmark.translated_visibility {
            reasons.insert(format!("VISIBILITY_CHANGED:{}", landmark.landmark_id));
            continue;
        }
        if landmark.source_crop_state != landmark.translated_crop_state {
            reasons.insert(format!("CROP_STATE_CHANGED:{}", landmark.landmark_id));
            continue;
        }
        if landmark.source_visibility == Visibility::Visible {
            let source_in_bounds = landmark.source_x_mpx >= 0
                && landmark.source_x_mpx < width_mpx
                && landmark.source_y_mpx >= 0
                && landmark.source_y_mpx < height_mpx;
            let translated_in_bounds = landmark.translated_x_mpx >= 0
                && landmark.translated_x_mpx < width_mpx
                && landmark.translated_y_mpx >= 0
                && landmark.translated_y_mpx < height_mpx;
            if !source_in_bounds || !translated_in_bounds {
                reasons.insert(format!("VISIBLE_LANDMARK_OUT_OF_BOUNDS:{}", landmark.landmark_id));
                continue;
            }
            let dx = landmark.translated_x_mpx - landmark.source_x_mpx;
            let dy = landmark.translated_y_mpx - landmark.source_y_mpx;
            let displacement = displacement_mpx(dx, dy)
                .ok_or_else(|| "landmark displacement overflow".to_owned())?;
            if displacement > frame.metadata.scope.landmark_tolerance_mpx {
                reasons.insert(format!("LANDMARK_DISPLACEMENT_EXCEEDED:{}", landmark.landmark_id));
            }
            displacements.push(displacement);
        }
    }

    let mut mask_scores = Vec::with_capacity(frame.masks.len());
    for mask in &frame.masks {
        let score = score_mask(
            mask,
            frame.metadata.scope.image_width_px as usize,
            frame.metadata.scope.image_height_px as usize,
        )?;
        if !score.passed {
            reasons.insert(format!("MASK_PRESERVATION_FAILED:{}", mask.region_id));
        }
        mask_scores.push(score);
    }

    let mut sorted = displacements.clone();
    let max_displacement = sorted.iter().copied().max();
    let p95_displacement = percentile_95(&mut sorted);

    Ok(FrameScore {
        sample_id: frame.metadata.sample_id.clone(),
        frame_index: frame.metadata.frame_index,
        disposition: if reasons.is_empty() {
            "PASS_GEOMETRY_PRESERVATION".to_owned()
        } else {
            "BLOCKED_GEOMETRY_PRESERVATION".to_owned()
        },
        reason_codes: reasons.into_iter().collect(),
        compared_landmark_count: u32::try_from(displacements.len()).unwrap_or(u32::MAX),
        max_landmark_displacement_mpx: max_displacement,
        p95_landmark_displacement_mpx: p95_displacement,
        mask_scores,
    })
}

fn validate_unit_frames(frames: &[FrameAnnotations]) -> Result<(), String> {
    if frames.is_empty() {
        return Err("unit has no frames".to_owned());
    }
    let first = &frames[0].metadata;
    let budget = first.scope.unit_frame_budget;
    if frames.len() != budget as usize {
        return Err(format!(
            "unit {} has {} frames but budget is {}",
            first.unit_id,
            frames.len(),
            budget
        ));
    }
    let mut indices = BTreeSet::new();
    let expected_landmarks: BTreeSet<&str> =
        frames[0].landmarks.iter().map(|item| item.landmark_id.as_str()).collect();
    let expected_masks: BTreeSet<&str> =
        frames[0].masks.iter().map(|item| item.region_id.as_str()).collect();

    for frame in frames {
        let metadata = &frame.metadata;
        if metadata.unit_id != first.unit_id
            || metadata.unit_kind != first.unit_kind
            || metadata.scope != first.scope
            || metadata.target_id != first.target_id
        {
            return Err(format!("unit {} changes scope or target identity", first.unit_id));
        }
        if !indices.insert(metadata.frame_index) {
            return Err(format!("unit {} has duplicate frame index", first.unit_id));
        }
        let landmark_ids: BTreeSet<&str> =
            frame.landmarks.iter().map(|item| item.landmark_id.as_str()).collect();
        let mask_ids: BTreeSet<&str> =
            frame.masks.iter().map(|item| item.region_id.as_str()).collect();
        if landmark_ids != expected_landmarks || mask_ids != expected_masks {
            return Err(format!("unit {} changes landmark or mask identity set", first.unit_id));
        }
    }
    if indices.iter().copied().ne(0..budget) {
        return Err(format!("unit {} has missing frame indices", first.unit_id));
    }
    Ok(())
}

fn temporal_motion_residual(frames: &[FrameAnnotations]) -> Result<Option<u64>, String> {
    let mut maximum = None::<u64>;
    let mut by_frame = frames.to_vec();
    by_frame.sort_by_key(|frame| frame.metadata.frame_index);

    for pair in by_frame.windows(2) {
        let previous = &pair[0];
        let next = &pair[1];
        let previous_landmarks: BTreeMap<&str, &LandmarkPair> = previous
            .landmarks
            .iter()
            .map(|item| (item.landmark_id.as_str(), item))
            .collect();
        let next_landmarks: BTreeMap<&str, &LandmarkPair> = next
            .landmarks
            .iter()
            .map(|item| (item.landmark_id.as_str(), item))
            .collect();

        for (landmark_id, left) in previous_landmarks {
            let right = next_landmarks
                .get(landmark_id)
                .ok_or_else(|| format!("temporal landmark missing: {landmark_id}"))?;
            if left.source_visibility != Visibility::Visible
                || right.source_visibility != Visibility::Visible
                || left.translated_visibility != Visibility::Visible
                || right.translated_visibility != Visibility::Visible
            {
                continue;
            }

            let source_dx = right.source_x_mpx - left.source_x_mpx;
            let source_dy = right.source_y_mpx - left.source_y_mpx;
            let translated_dx = right.translated_x_mpx - left.translated_x_mpx;
            let translated_dy = right.translated_y_mpx - left.translated_y_mpx;
            let residual = displacement_mpx(
                translated_dx - source_dx,
                translated_dy - source_dy,
            )
            .ok_or_else(|| "temporal motion residual overflow".to_owned())?;
            maximum = Some(maximum.map_or(residual, |current| current.max(residual)));
        }
    }
    Ok(maximum)
}

pub fn audit_manifest(frames: &[FrameAnnotations]) -> Result<AuditReceipt, String> {
    if frames.is_empty() {
        return Err("manifest has no frames".to_owned());
    }

    let mut sample_ids = BTreeSet::new();
    let unit_kind = frames[0].metadata.unit_kind;
    let mut units = BTreeMap::<String, Vec<FrameAnnotations>>::new();
    let mut scopes = BTreeMap::<String, ScopeKey>::new();

    for frame in frames {
        validate_frame(frame)?;
        if frame.metadata.unit_kind != unit_kind {
            return Err("unit kind changes within manifest".to_owned());
        }
        if !sample_ids.insert(frame.metadata.sample_id.as_str()) {
            return Err(format!("duplicate sample id: {}", frame.metadata.sample_id));
        }
        if let Some(existing) = scopes.insert(
            frame.metadata.unit_id.clone(),
            frame.metadata.scope.clone(),
        ) {
            if existing != frame.metadata.scope {
                return Err(format!("unit {} crosses geometry audit scopes", frame.metadata.unit_id));
            }
        }
        units
            .entry(frame.metadata.unit_id.clone())
            .or_default()
            .push(frame.clone());
    }

    let mut results = Vec::with_capacity(units.len());
    let mut passed_units = 0_u64;
    let mut blocked_units = 0_u64;
    let mut frame_count = 0_u64;

    for (unit_id, mut unit_frames) in units {
        validate_unit_frames(&unit_frames)?;
        unit_frames.sort_by_key(|frame| frame.metadata.frame_index);
        let mut frame_scores = unit_frames
            .iter()
            .map(score_frame)
            .collect::<Result<Vec<_>, _>>()?;
        let max_landmark = frame_scores
            .iter()
            .filter_map(|frame| frame.max_landmark_displacement_mpx)
            .max();
        let mut all_displacements = frame_scores
            .iter()
            .filter_map(|frame| frame.max_landmark_displacement_mpx)
            .collect::<Vec<_>>();
        let p95_landmark = percentile_95(&mut all_displacements);
        let temporal = temporal_motion_residual(&unit_frames)?;
        let temporal_limit = unit_frames[0].metadata.scope.temporal_motion_tolerance_mpx;
        let mut unit_reasons = BTreeSet::new();
        if temporal.is_some_and(|value| value > temporal_limit) {
            unit_reasons.insert("TEMPORAL_MOTION_RESIDUAL_EXCEEDED".to_owned());
        } else if unit_frames[0].metadata.scope.unit_frame_budget > 1 && temporal.is_none() {
            unit_reasons.insert("NO_VISIBLE_LANDMARK_TRAJECTORY_FOR_TEMPORAL_AUDIT".to_owned());
        }
        if frame_scores.iter().any(|score| score.disposition != "PASS_GEOMETRY_PRESERVATION") {
            unit_reasons.insert("ONE_OR_MORE_FRAMES_FAILED_GEOMETRY_PRESERVATION".to_owned());
        }

        if unit_reasons.is_empty() {
            passed_units += 1;
        } else {
            blocked_units += 1;
        }
        frame_count += frame_scores.len() as u64;

        let frame_passed = frame_scores
            .iter()
            .filter(|score| score.disposition == "PASS_GEOMETRY_PRESERVATION")
            .count() as u32;
        let frame_blocked = frame_scores.len() as u32 - frame_passed;
        let first = &unit_frames[0].metadata;
        results.push(UnitScore {
            unit_id,
            unit_kind: first.unit_kind.as_str().to_owned(),
            frame_count: frame_scores.len() as u32,
            passed_frame_count: frame_passed,
            blocked_frame_count: frame_blocked,
            max_landmark_displacement_mpx: max_landmark,
            p95_landmark_displacement_mpx: p95_landmark,
            max_temporal_motion_residual_mpx: temporal,
            disposition: if unit_reasons.is_empty() {
                "PASS_GEOMETRY_PRESERVATION".to_owned()
            } else {
                "BLOCKED_GEOMETRY_PRESERVATION".to_owned()
            },
            reason_codes: unit_reasons.into_iter().collect(),
            frames: std::mem::take(&mut frame_scores),
        });
    }

    Ok(AuditReceipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        unit_count: u64::try_from(results.len()).unwrap_or(u64::MAX),
        frame_count,
        passed_unit_count: passed_units,
        blocked_unit_count: blocked_units,
        results,
        image_pixels_inspected_here: false,
        annotation_source_bytes_authenticated: false,
        independent_metrology_verified_here: false,
        physical_truth_established: false,
        calibration_exchangeability_proven: false,
        flight_qualification: false,
        maneuver_authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CropState, FrameAnnotations, FrameMetadata, LandmarkPair, MaskPair,
        ScopeKey, UnitKind, Visibility, audit_manifest,
    };

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn mask_32x32() -> String {
        let mut nibbles = vec![0_u8; 256];
        for y in 8..24 {
            for x in 8..24 {
                let index = y * 32 + x;
                nibbles[index / 4] |= 1 << (index % 4);
            }
        }
        nibbles.iter().map(|nibble| char::from_digit(u32::from(*nibble), 16).unwrap()).collect()
    }

    fn frame(unit: &str, index: u32, x: i64, translated_x: i64) -> FrameAnnotations {
        let mask = mask_32x32();
        let metadata = FrameMetadata {
            sample_id: format!("{unit}-frame-{index}"),
            target_id: "TANGO".to_owned(),
            session_id: format!("SESSION-{unit}"),
            sequence_id: unit.to_owned(),
            unit_id: unit.to_owned(),
            unit_kind: UnitKind::Sequence,
            partition: "CALIBRATION".to_owned(),
            frame_index: index,
            scope: ScopeKey {
                domain_id: "SYNTHETIC_TO_HIL".to_owned(),
                condition_id: "SUNLAMP".to_owned(),
                crop_stratum: "FULL".to_owned(),
                modality: "RGB".to_owned(),
                ood_group: "IN_DOMAIN".to_owned(),
                calibration_revision: "cal-r1".to_owned(),
                translator_sha256: digest('a'),
                image_width_px: 32,
                image_height_px: 32,
                unit_kind: UnitKind::Sequence,
                unit_frame_budget: 3,
                landmark_tolerance_mpx: 1_200,
                temporal_motion_tolerance_mpx: 500,
                default_mask_iou_floor_bps: 9_900,
            },
            source_image_sha256: digest(char::from_digit((index + 1) % 10, 10).unwrap()),
            translated_image_sha256: digest(char::from_digit((index + 4) % 10, 10).unwrap()),
            source_annotation_sha256: digest('b'),
            translated_annotation_sha256: digest('c'),
        };
        FrameAnnotations {
            metadata,
            landmarks: vec![
                LandmarkPair {
                    landmark_id: "solar_hinge".to_owned(),
                    source_x_mpx: x,
                    source_y_mpx: 12_000,
                    translated_x_mpx: translated_x,
                    translated_y_mpx: 12_000,
                    source_visibility: Visibility::Visible,
                    translated_visibility: Visibility::Visible,
                    source_crop_state: CropState::Full,
                    translated_crop_state: CropState::Full,
                },
                LandmarkPair {
                    landmark_id: "antenna_tip".to_owned(),
                    source_x_mpx: 20_000,
                    source_y_mpx: 20_000,
                    translated_x_mpx: 20_000,
                    translated_y_mpx: 20_000,
                    source_visibility: Visibility::Visible,
                    translated_visibility: Visibility::Visible,
                    source_crop_state: CropState::Full,
                    translated_crop_state: CropState::Full,
                },
            ],
            masks: vec![
                MaskPair {
                    region_id: "SILHOUETTE".to_owned(),
                    source_mask_hex: mask.clone(),
                    translated_mask_hex: mask.clone(),
                    iou_floor_bps: 9_900,
                },
                MaskPair {
                    region_id: "SOLAR_ARRAY".to_owned(),
                    source_mask_hex: mask.clone(),
                    translated_mask_hex: mask,
                    iou_floor_bps: 9_500,
                },
            ],
        }
    }

    fn clean_unit(unit: &str) -> Vec<FrameAnnotations> {
        vec![
            frame(unit, 0, 10_000, 10_000),
            frame(unit, 1, 11_000, 11_000),
            frame(unit, 2, 12_000, 12_000),
        ]
    }

    #[test]
    fn photometric_only_positive_control_passes_all_frames_and_sequence() {
        let receipt = audit_manifest(&clean_unit("SEQ-001")).expect("valid annotation manifest");
        assert_eq!(receipt.unit_count, 1);
        assert_eq!(receipt.frame_count, 3);
        assert_eq!(receipt.passed_unit_count, 1);
        assert_eq!(receipt.blocked_unit_count, 0);
        assert_eq!(receipt.results[0].disposition, "PASS_GEOMETRY_PRESERVATION");
        assert_eq!(receipt.results[0].max_temporal_motion_residual_mpx, Some(0));
        assert!(!receipt.image_pixels_inspected_here);
        assert!(!receipt.independent_metrology_verified_here);
        assert!(!receipt.physical_truth_established);
    }

    #[test]
    fn keypoint_shift_over_tolerance_blocks_unit() {
        let mut frames = clean_unit("SEQ-002");
        frames[1].landmarks[0].translated_x_mpx += 2_000;
        let receipt = audit_manifest(&frames).expect("well-formed annotation manifest");
        assert_eq!(receipt.blocked_unit_count, 1);
        assert!(receipt.results[0].reason_codes.iter().any(|reason| reason.contains("ONE_OR_MORE_FRAMES")));
    }

    #[test]
    fn visibility_change_blocks_even_when_coordinates_match() {
        let mut frames = clean_unit("SEQ-003");
        frames[1].landmarks[0].translated_visibility = Visibility::Occluded;
        let receipt = audit_manifest(&frames).expect("well-formed annotation manifest");
        assert_eq!(receipt.results[0].frames[1].disposition, "BLOCKED_GEOMETRY_PRESERVATION");
    }

    #[test]
    fn crop_change_blocks_even_when_landmark_is_close() {
        let mut frames = clean_unit("SEQ-004");
        frames[1].landmarks[0].translated_crop_state = CropState::Truncated;
        let receipt = audit_manifest(&frames).expect("well-formed annotation manifest");
        assert!(receipt.results[0].frames[1].reason_codes.iter().any(|reason| reason.contains("CROP_STATE_CHANGED")));
    }

    #[test]
    fn mask_iou_below_floor_blocks_frame() {
        let mut frames = clean_unit("SEQ-005");
        frames[1].masks[1].translated_mask_hex = {
            let bits = vec![0_u8; 256];
            bits.iter().map(|nibble| char::from_digit(u32::from(*nibble), 16).unwrap()).collect()
        };
        let receipt = audit_manifest(&frames).expect("well-formed mask annotations");
        assert_eq!(receipt.results[0].frames[1].disposition, "BLOCKED_GEOMETRY_PRESERVATION");
    }

    #[test]
    fn topology_change_blocks_even_when_iou_might_be_high() {
        let mut frames = clean_unit("SEQ-006");
        let mut split = mask_32x32().into_bytes();
        split[0] = b'1';
        frames[1].masks[0].translated_mask_hex = String::from_utf8(split).unwrap();
        let receipt = audit_manifest(&frames).expect("valid mask encoding");
        let mask = receipt.results[0].frames[1]
            .mask_scores
            .iter()
            .find(|score| score.region_id == "SILHOUETTE")
            .unwrap();
        assert!(!mask.passed);
    }

    #[test]
    fn temporal_jitter_fails_even_when_each_frame_is_within_static_tolerance() {
        let mut frames = clean_unit("SEQ-007");
        frames[1].landmarks[0].translated_x_mpx += 1_000;
        frames[2].landmarks[0].translated_x_mpx += 1_000;
        let receipt = audit_manifest(&frames).expect("valid frame annotations");
        assert!(receipt.results[0].frames.iter().all(|score| score.disposition == "PASS_GEOMETRY_PRESERVATION"));
        assert_eq!(receipt.results[0].max_temporal_motion_residual_mpx, Some(1_000));
        assert_eq!(receipt.results[0].disposition, "BLOCKED_GEOMETRY_PRESERVATION");
        assert!(receipt.results[0].reason_codes.contains(&"TEMPORAL_MOTION_RESIDUAL_EXCEEDED".to_owned()));
    }

    #[test]
    fn missing_frame_rejects_incomplete_horizon() {
        let mut frames = clean_unit("SEQ-008");
        frames.pop();
        assert!(audit_manifest(&frames).is_err());
    }

    #[test]
    fn duplicate_frame_index_rejects_manifest() {
        let mut frames = clean_unit("SEQ-009");
        frames[2].metadata.frame_index = 1;
        assert!(audit_manifest(&frames).is_err());
    }

    #[test]
    fn missing_landmark_identity_across_frames_rejects_unit() {
        let mut frames = clean_unit("SEQ-010");
        frames[1].landmarks.pop();
        assert!(audit_manifest(&frames).is_err());
    }

    #[test]
    fn duplicate_sample_id_rejects_manifest() {
        let mut frames = clean_unit("SEQ-011");
        frames[1].metadata.sample_id = frames[0].metadata.sample_id.clone();
        assert!(audit_manifest(&frames).is_err());
    }

    #[test]
    fn malformed_mask_hex_rejects_manifest() {
        let mut frames = clean_unit("SEQ-012");
        frames[1].masks[0].translated_mask_hex = "not-hex".to_owned();
        assert!(audit_manifest(&frames).is_err());
    }

    #[test]
    fn test_partition_is_never_accepted_as_translation_calibration() {
        let mut frames = clean_unit("SEQ-013");
        frames[0].metadata.partition = "TEST".to_owned();
        assert!(audit_manifest(&frames).is_err());
    }

    #[test]
    fn translator_or_image_digest_must_be_valid() {
        let mut frames = clean_unit("SEQ-014");
        frames[0].metadata.translated_image_sha256 = "bad".to_owned();
        assert!(audit_manifest(&frames).is_err());
    }

    #[test]
    fn mismatched_source_and_translated_crop_metadata_blocks() {
        let mut frames = clean_unit("SEQ-015");
        frames[2].landmarks[1].source_crop_state = CropState::Cropped;
        frames[2].landmarks[1].translated_crop_state = CropState::Full;
        assert!(audit_manifest(&frames).unwrap().blocked_unit_count == 1);
    }
}
