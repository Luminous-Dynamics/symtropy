#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, VecDeque};

pub const CASE_SCHEMA: &str = "luminous.orbital-gsd-kcoverage.case.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-gsd-kcoverage.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-GSD-KCOVER-VOXEL-001";

const FACE_NEIGHBOURS: [([isize; 3], [i32; 3]); 6] = [
    ([-1, 0, 0], [-1, 0, 0]),
    ([1, 0, 0], [1, 0, 0]),
    ([0, -1, 0], [0, -1, 0]),
    ([0, 1, 0], [0, 1, 0]),
    ([0, 0, -1], [0, 0, -1]),
    ([0, 0, 1], [0, 0, 1]),
];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    fn from_array(value: [f64; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
    fn array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
    fn scale(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
    fn normalized(self) -> Option<Self> {
        let norm = self.norm();
        (norm.is_finite() && norm > 1.0e-12).then(|| self.scale(1.0 / norm))
    }
    fn angle(self, other: Self) -> Option<f64> {
        let a = self.normalized()?;
        let b = other.normalized()?;
        Some(a.dot(b).clamp(-1.0, 1.0).acos())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridInput {
    pub origin_m: [f64; 3],
    pub voxel_size_m: f64,
    pub dimensions: [usize; 3],
    pub occupied_cells: Vec<[usize; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraInput {
    pub horizontal_fov_rad: f64,
    pub horizontal_pixel_count: u32,
    pub gsd_max_m_per_pixel: f64,
    pub max_oblique_angle_rad: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GsdCase {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub grid: GridInput,
    pub camera: CameraInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    Feasible,
    Infeasible,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SurfacePatch {
    pub feature_id: String,
    pub cell: [usize; 3],
    pub normal_q1e6: [i64; 3],
    pub normal_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateView {
    pub view_id: String,
    pub source_feature_id: String,
    pub source_cell: [usize; 3],
    pub normal_q1e6: [i64; 3],
    pub position_um: [i64; 3],
    pub optical_axis_q1e6: [i64; 3],
    pub visible_feature_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureCoverage {
    pub feature_id: String,
    pub candidate_view_count: usize,
    pub seen_by_view_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GsdReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub source_input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub disposition: Disposition,
    pub grid_dimensions: [usize; 3],
    pub occupied_cell_count: usize,
    pub exterior_empty_cell_count: usize,
    pub interior_empty_cell_count: usize,
    pub surface_patch_count: usize,
    pub multi_normal_patch_count: usize,
    pub horizontal_fov_microrad: i64,
    pub horizontal_pixel_count: u32,
    pub gsd_max_nanom_per_pixel: i64,
    pub max_oblique_angle_microrad: i64,
    pub slant_range_max_um: i64,
    pub perpendicular_distance_um: i64,
    pub candidate_view_count: usize,
    pub visible_relation_count: usize,
    pub surface_features: Vec<SurfacePatch>,
    pub candidate_views: Vec<CandidateView>,
    pub feature_coverage: Vec<FeatureCoverage>,
    pub authority_granted: bool,
    pub structural_acceptance_authorized: bool,
    pub flight_authorized: bool,
    pub physical_validation: String,
}

#[derive(Debug, Clone)]
struct Grid {
    origin: Vec3,
    voxel_size: f64,
    dimensions: [usize; 3],
    occupied: BTreeSet<(usize, usize, usize)>,
    exterior: BTreeSet<(usize, usize, usize)>,
    interior_empty: BTreeSet<(usize, usize, usize)>,
}

fn quantize(value: f64, scale: f64) -> Result<i64, String> {
    let scaled = value * scale;
    if !scaled.is_finite() || scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return Err("NUMERIC_QUANTIZATION_OVERFLOW".to_owned());
    }
    Ok(scaled.round() as i64)
}

fn feature_id(cell: [usize; 3]) -> String {
    format!("V-{}-{}-{}", cell[0], cell[1], cell[2])
}

fn plus(cell: [isize; 3], delta: [isize; 3]) -> [isize; 3] {
    [cell[0] + delta[0], cell[1] + delta[1], cell[2] + delta[2]]
}

impl Grid {
    fn from_input(input: &GridInput) -> Result<Self, String> {
        if !input.voxel_size_m.is_finite()
            || input.voxel_size_m <= 0.0
            || input
                .dimensions
                .iter()
                .any(|dimension| *dimension == 0 || *dimension > 128)
            || input.origin_m.iter().any(|value| !value.is_finite())
        {
            return Err("GRID_BOUNDS_INVALID".to_owned());
        }
        let mut occupied = BTreeSet::new();
        for cell in &input.occupied_cells {
            if cell
                .iter()
                .zip(input.dimensions)
                .any(|(index, dimension)| *index >= dimension)
                || !occupied.insert((cell[0], cell[1], cell[2]))
            {
                return Err("GRID_OCCUPANCY_INVALID_OR_DUPLICATE".to_owned());
            }
        }
        if occupied.is_empty() {
            return Err("GRID_EMPTY".to_owned());
        }

        // Pad one empty cell on every side and flood-fill from the padded corner.
        let padded = [
            input.dimensions[0] + 2,
            input.dimensions[1] + 2,
            input.dimensions[2] + 2,
        ];
        let mut exterior_padded = BTreeSet::new();
        let mut queue = VecDeque::new();
        exterior_padded.insert((0_usize, 0_usize, 0_usize));
        queue.push_back([0_usize, 0_usize, 0_usize]);

        while let Some(cell) = queue.pop_front() {
            for (delta, _) in FACE_NEIGHBOURS {
                let next = plus(
                    [cell[0] as isize, cell[1] as isize, cell[2] as isize],
                    delta,
                );
                if next
                    .iter()
                    .enumerate()
                    .any(|(axis, value)| *value < 0 || *value >= padded[axis] as isize)
                {
                    continue;
                }
                let original = [next[0] - 1, next[1] - 1, next[2] - 1];
                let inside_original = original
                    .iter()
                    .enumerate()
                    .all(|(axis, value)| *value >= 0 && *value < input.dimensions[axis] as isize);
                if inside_original
                    && occupied.contains(&(
                        original[0] as usize,
                        original[1] as usize,
                        original[2] as usize,
                    ))
                {
                    continue;
                }
                let key = (next[0] as usize, next[1] as usize, next[2] as usize);
                if exterior_padded.insert(key) {
                    queue.push_back([key.0, key.1, key.2]);
                }
            }
        }

        let mut exterior = BTreeSet::new();
        let mut interior_empty = BTreeSet::new();
        for x in 0..input.dimensions[0] {
            for y in 0..input.dimensions[1] {
                for z in 0..input.dimensions[2] {
                    let cell = (x, y, z);
                    if occupied.contains(&cell) {
                        continue;
                    }
                    if exterior_padded.contains(&(x + 1, y + 1, z + 1)) {
                        exterior.insert(cell);
                    } else {
                        interior_empty.insert(cell);
                    }
                }
            }
        }

        Ok(Self {
            origin: Vec3::from_array(input.origin_m),
            voxel_size: input.voxel_size_m,
            dimensions: input.dimensions,
            occupied,
            exterior,
            interior_empty,
        })
    }

    fn centre(&self, cell: [usize; 3]) -> Vec3 {
        self.origin.add(Vec3 {
            x: (cell[0] as f64 + 0.5) * self.voxel_size,
            y: (cell[1] as f64 + 0.5) * self.voxel_size,
            z: (cell[2] as f64 + 0.5) * self.voxel_size,
        })
    }

    fn is_occupied(&self, cell: [isize; 3]) -> bool {
        cell.iter()
            .enumerate()
            .all(|(axis, index)| *index >= 0 && *index < self.dimensions[axis] as isize)
            && self
                .occupied
                .contains(&(cell[0] as usize, cell[1] as usize, cell[2] as usize))
    }

    fn surface_patches(&self) -> Result<Vec<SurfacePatch>, String> {
        let mut patches = Vec::new();
        for &(x, y, z) in &self.occupied {
            let cell = [x, y, z];
            let mut normals = Vec::new();
            for (delta, normal) in FACE_NEIGHBOURS {
                let neighbour = plus([x as isize, y as isize, z as isize], delta);
                if self.is_occupied(neighbour) {
                    continue;
                }
                let inside_original = neighbour
                    .iter()
                    .enumerate()
                    .all(|(axis, value)| *value >= 0 && *value < self.dimensions[axis] as isize);
                let is_exterior = !inside_original
                    || self.exterior.contains(&(
                        neighbour[0] as usize,
                        neighbour[1] as usize,
                        neighbour[2] as usize,
                    ));
                if is_exterior {
                    normals.push(normal);
                }
            }
            if normals.is_empty() {
                continue;
            }

            let summed = normals.iter().fold(
                Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                |acc, normal| {
                    acc.add(Vec3 {
                        x: normal[0] as f64,
                        y: normal[1] as f64,
                        z: normal[2] as f64,
                    })
                },
            );

            if summed.norm() <= 1.0e-9 {
                for (normal_index, normal) in normals.iter().enumerate() {
                    let direction = Vec3 {
                        x: normal[0] as f64,
                        y: normal[1] as f64,
                        z: normal[2] as f64,
                    };
                    patches.push(SurfacePatch {
                        feature_id: format!("{}-N{}", feature_id(cell), normal_index),
                        cell,
                        normal_q1e6: [
                            quantize(direction.x, 1.0e6)?,
                            quantize(direction.y, 1.0e6)?,
                            quantize(direction.z, 1.0e6)?,
                        ],
                        normal_source: "EXTERIOR_FACE_MULTI_NORMAL".to_owned(),
                    });
                }
            } else {
                let normal = summed
                    .normalized()
                    .ok_or_else(|| "SURFACE_NORMAL_ZERO".to_owned())?;
                patches.push(SurfacePatch {
                    feature_id: feature_id(cell),
                    cell,
                    normal_q1e6: [
                        quantize(normal.x, 1.0e6)?,
                        quantize(normal.y, 1.0e6)?,
                        quantize(normal.z, 1.0e6)?,
                    ],
                    normal_source: "NORMALIZED_EXTERIOR_FACE_SUM".to_owned(),
                });
            }
        }
        patches.sort_by(|left, right| left.feature_id.cmp(&right.feature_id));
        Ok(patches)
    }

    fn ray_occluded(&self, origin: Vec3, target: Vec3, target_cell: [usize; 3]) -> bool {
        let segment = target.sub(origin);
        let distance = segment.norm();
        let Some(direction) = segment.normalized() else {
            return true;
        };
        let tolerance = self.voxel_size * 3.0_f64.sqrt() / 2.0;
        let end_t = (distance - tolerance).max(0.0);

        let origin_array = origin.array();
        let direction_array = direction.array();
        let grid_origin = self.origin.array();
        let mut cell = [
            ((origin_array[0] - grid_origin[0]) / self.voxel_size).floor() as isize,
            ((origin_array[1] - grid_origin[1]) / self.voxel_size).floor() as isize,
            ((origin_array[2] - grid_origin[2]) / self.voxel_size).floor() as isize,
        ];
        let mut step = [0_isize; 3];
        let mut t_max = [f64::INFINITY; 3];
        let mut t_delta = [f64::INFINITY; 3];

        for axis in 0..3 {
            if direction_array[axis] > 0.0 {
                step[axis] = 1;
                let boundary = grid_origin[axis] + (cell[axis] + 1) as f64 * self.voxel_size;
                t_max[axis] = (boundary - origin_array[axis]) / direction_array[axis];
                t_delta[axis] = self.voxel_size / direction_array[axis];
            } else if direction_array[axis] < 0.0 {
                step[axis] = -1;
                let boundary = grid_origin[axis] + cell[axis] as f64 * self.voxel_size;
                t_max[axis] = (boundary - origin_array[axis]) / direction_array[axis];
                t_delta[axis] = -self.voxel_size / direction_array[axis];
            }
        }

        let target = [
            target_cell[0] as isize,
            target_cell[1] as isize,
            target_cell[2] as isize,
        ];
        let mut t = 0.0;
        for _ in 0..1024 {
            if t > end_t {
                break;
            }
            if self.is_occupied(cell) && cell != target {
                return true;
            }
            let next_t = t_max.iter().copied().fold(f64::INFINITY, f64::min);
            if !next_t.is_finite() || next_t > end_t {
                break;
            }
            let tied_axes = (0..3)
                .filter(|axis| (t_max[*axis] - next_t).abs() <= 1.0e-10)
                .collect::<Vec<_>>();
            // Supercover edge/corner transitions so a grazing occupied cell is not missed.
            for mask in 1..(1_usize << tied_axes.len()) {
                let mut neighbour = cell;
                for (bit, axis) in tied_axes.iter().enumerate() {
                    if mask & (1 << bit) != 0 {
                        neighbour[*axis] += step[*axis];
                    }
                }
                if self.is_occupied(neighbour) && neighbour != target {
                    return true;
                }
            }
            for axis in tied_axes {
                cell[axis] += step[axis];
                t_max[axis] += t_delta[axis];
            }
            t = next_t;
        }
        false
    }
}

fn validate_case(case: &GsdCase) -> Result<(), String> {
    if case.schema != CASE_SCHEMA
        || case.benchmark_id != BENCHMARK_ID
        || case.evidence_class != "synthetic"
        || case.claim_ceiling != "synthetic_3d_voxel_surface_candidate_view_visibility_only"
        || case.case_id.trim().is_empty()
        || case.coordinate_frame_id.trim().is_empty()
    {
        return Err("CASE_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }
    let camera = &case.camera;
    if !camera.horizontal_fov_rad.is_finite()
        || camera.horizontal_fov_rad <= 0.0
        || camera.horizontal_fov_rad >= std::f64::consts::PI
        || camera.horizontal_pixel_count == 0
        || !camera.gsd_max_m_per_pixel.is_finite()
        || camera.gsd_max_m_per_pixel <= 0.0
        || !camera.max_oblique_angle_rad.is_finite()
        || camera.max_oblique_angle_rad < 0.0
        || camera.max_oblique_angle_rad >= std::f64::consts::FRAC_PI_2
    {
        return Err("CAMERA_MODEL_INVALID".to_owned());
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn evaluate_bytes(bytes: &[u8], source_commit: &str) -> Result<GsdReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let case: GsdCase =
        serde_json::from_slice(bytes).map_err(|error| format!("CASE_PARSE_FAILED:{error}"))?;
    validate_case(&case)?;
    let grid = Grid::from_input(&case.grid)?;
    let camera = &case.camera;
    let fov_scale = 2.0 * (camera.horizontal_fov_rad * 0.5).tan();
    let slant_range = camera.gsd_max_m_per_pixel
        * f64::from(camera.horizontal_pixel_count)
        * camera.max_oblique_angle_rad.cos()
        / fov_scale;
    let perpendicular = slant_range * camera.max_oblique_angle_rad.cos();
    if !slant_range.is_finite() || slant_range <= 0.0 {
        return Err("DERIVED_GSD_RANGE_INVALID".to_owned());
    }

    let surfaces = grid.surface_patches()?;
    let mut candidates = Vec::new();
    for surface in &surfaces {
        let normal = Vec3::from_array([
            surface.normal_q1e6[0] as f64 / 1.0e6,
            surface.normal_q1e6[1] as f64 / 1.0e6,
            surface.normal_q1e6[2] as f64 / 1.0e6,
        ]);
        let centre = grid.centre(surface.cell);
        let position = centre.add(normal.scale(perpendicular));
        let optical = normal.scale(-1.0);
        if grid.ray_occluded(position, centre, surface.cell) {
            continue;
        }

        let mut visible_features = Vec::new();
        for target in &surfaces {
            let target_normal = Vec3::from_array([
                target.normal_q1e6[0] as f64 / 1.0e6,
                target.normal_q1e6[1] as f64 / 1.0e6,
                target.normal_q1e6[2] as f64 / 1.0e6,
            ]);
            let target_centre = grid.centre(target.cell);
            let to_target = target_centre.sub(position);
            let distance = to_target.norm();
            if distance <= 1.0e-12 || distance > slant_range + 1.0e-10 {
                continue;
            }
            let Some(deviation) = normal.angle(target_normal) else {
                continue;
            };
            if deviation > camera.max_oblique_angle_rad + 1.0e-10 {
                continue;
            }
            let Some(fov_angle) = optical.angle(to_target) else {
                continue;
            };
            if fov_angle > camera.horizontal_fov_rad * 0.5 + 1.0e-10 {
                continue;
            }
            let gsd =
                distance * fov_scale / (f64::from(camera.horizontal_pixel_count) * deviation.cos());
            if gsd > camera.gsd_max_m_per_pixel + 1.0e-12 {
                continue;
            }
            if grid.ray_occluded(position, target_centre, target.cell) {
                continue;
            }
            visible_features.push(target.feature_id.clone());
        }
        visible_features.sort();
        candidates.push(CandidateView {
            view_id: format!("VIEW-{}", candidates.len()),
            source_feature_id: surface.feature_id.clone(),
            source_cell: surface.cell,
            normal_q1e6: surface.normal_q1e6,
            position_um: [
                quantize(position.x, 1.0e6)?,
                quantize(position.y, 1.0e6)?,
                quantize(position.z, 1.0e6)?,
            ],
            optical_axis_q1e6: [
                quantize(optical.x, 1.0e6)?,
                quantize(optical.y, 1.0e6)?,
                quantize(optical.z, 1.0e6)?,
            ],
            visible_feature_ids: visible_features,
        });
    }

    let visible_relation_count = candidates
        .iter()
        .map(|candidate| candidate.visible_feature_ids.len())
        .sum::<usize>();
    let feature_coverage = surfaces
        .iter()
        .map(|surface| {
            let mut seen_by_view_ids = candidates
                .iter()
                .filter(|candidate| candidate.visible_feature_ids.contains(&surface.feature_id))
                .map(|candidate| candidate.view_id.clone())
                .collect::<Vec<_>>();
            seen_by_view_ids.sort();
            FeatureCoverage {
                feature_id: surface.feature_id.clone(),
                candidate_view_count: seen_by_view_ids.len(),
                seen_by_view_ids,
            }
        })
        .collect::<Vec<_>>();

    Ok(GsdReceipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        case_id: case.case_id.clone(),
        coordinate_frame_id: case.coordinate_frame_id.clone(),
        source_input_sha256: sha256(bytes),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: case.claim_ceiling.clone(),
        disposition: if feature_coverage
            .iter()
            .all(|feature| feature.candidate_view_count > 0)
        {
            Disposition::Feasible
        } else {
            Disposition::Infeasible
        },
        grid_dimensions: grid.dimensions,
        occupied_cell_count: grid.occupied.len(),
        exterior_empty_cell_count: grid.exterior.len(),
        interior_empty_cell_count: grid.interior_empty.len(),
        surface_patch_count: surfaces.len(),
        multi_normal_patch_count: surfaces
            .iter()
            .filter(|surface| surface.normal_source == "EXTERIOR_FACE_MULTI_NORMAL")
            .count(),
        horizontal_fov_microrad: quantize(camera.horizontal_fov_rad, 1.0e6)?,
        horizontal_pixel_count: camera.horizontal_pixel_count,
        gsd_max_nanom_per_pixel: quantize(camera.gsd_max_m_per_pixel, 1.0e9)?,
        max_oblique_angle_microrad: quantize(camera.max_oblique_angle_rad, 1.0e6)?,
        slant_range_max_um: quantize(slant_range, 1.0e6)?,
        perpendicular_distance_um: quantize(perpendicular, 1.0e6)?,
        candidate_view_count: candidates.len(),
        visible_relation_count,
        surface_features: surfaces,
        candidate_views: candidates,
        feature_coverage,
        authority_granted: false,
        structural_acceptance_authorized: false,
        flight_authorized: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
    })
}

pub fn evaluate_file(path: &str, source_commit: &str) -> Result<GsdReceipt, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("CASE_READ_FAILED:{error}"))?;
    evaluate_bytes(&bytes, source_commit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> GsdCase {
        serde_json::from_str(include_str!(
            "../../../docs/research/orbital-gsd-kcoverage/fixtures/voxel-visibility-v0_1.json"
        ))
        .unwrap()
    }

    #[test]
    fn exterior_flood_fill_keeps_sealed_cavity_interior() {
        let grid = Grid::from_input(&fixture().grid).unwrap();
        assert_eq!(grid.interior_empty.len(), 1);
        assert!(grid.interior_empty.contains(&(2, 2, 2)));
    }

    #[test]
    fn thin_wall_keeps_opposing_exterior_face_normals() {
        let surfaces = Grid::from_input(&fixture().grid)
            .unwrap()
            .surface_patches()
            .unwrap();
        let first = surfaces
            .iter()
            .find(|surface| surface.feature_id == "V-5-2-2-N0")
            .unwrap();
        let second = surfaces
            .iter()
            .find(|surface| surface.feature_id == "V-5-2-2-N1")
            .unwrap();
        assert_ne!(first.normal_q1e6, second.normal_q1e6);
        assert_eq!(first.normal_source, "EXTERIOR_FACE_MULTI_NORMAL");
    }

    #[test]
    fn gsd_derived_distance_obeys_the_equation() {
        let case = fixture();
        let fov_scale = 2.0 * (case.camera.horizontal_fov_rad * 0.5).tan();
        let range = case.camera.gsd_max_m_per_pixel
            * f64::from(case.camera.horizontal_pixel_count)
            * case.camera.max_oblique_angle_rad.cos()
            / fov_scale;
        let h = range * case.camera.max_oblique_angle_rad.cos();
        let nadir = h * fov_scale / f64::from(case.camera.horizontal_pixel_count);
        let expected =
            case.camera.gsd_max_m_per_pixel * case.camera.max_oblique_angle_rad.cos().powi(2);
        assert!((nadir - expected).abs() < 1.0e-12);
    }

    #[test]
    fn generated_candidate_views_observe_their_source_feature() {
        let receipt = evaluate_bytes(
            include_bytes!(
                "../../../docs/research/orbital-gsd-kcoverage/fixtures/voxel-visibility-v0_1.json"
            ),
            &"0".repeat(40),
        )
        .unwrap();
        for candidate in &receipt.candidate_views {
            assert!(
                candidate
                    .visible_feature_ids
                    .contains(&candidate.source_feature_id)
            );
        }
        assert!(receipt.visible_relation_count >= receipt.candidate_view_count);
        assert!(!receipt.authority_granted);
        assert_eq!(receipt.physical_validation, "NOT_ASSESSED");
    }

    #[test]
    fn invalid_camera_intrinsics_are_rejected_not_defaulted() {
        let mut case = fixture();
        case.camera.horizontal_pixel_count = 0;
        let bytes = serde_json::to_vec(&case).unwrap();
        assert_eq!(
            evaluate_bytes(&bytes, &"0".repeat(40)),
            Err("CAMERA_MODEL_INVALID".to_owned())
        );
    }
}
