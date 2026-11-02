//! Strict structural inspection of the native ASCII SU2 unstructured mesh format.
//!
//! This parser verifies declared section counts, coordinate records, element connectivity,
//! marker structure, index ranges, and exact byte digest. It does not assess mesh quality,
//! skewness, orthogonality, aspect ratio, y+, boundary-layer resolution, or CFD suitability.

use std::collections::BTreeMap;

use crate::integrity::sha256_hex;

pub const SU2_MESH_SUMMARY_SCHEMA: &str = "su2-native-mesh-summary/0.1.0";
pub const SU2_MESH_EVIDENCE_LEVEL: &str =
    "mesh_bytes_and_native_format_structure_verified_not_mesh_quality_or_solver_compatibility";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2MarkerSummary {
    pub tag: String,
    pub element_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Su2MeshSummary {
    pub schema: String,
    pub byte_length: u64,
    pub mesh_sha256: String,
    pub dimension: u8,
    pub point_count: u64,
    pub volume_element_count: u64,
    pub volume_element_type_counts: BTreeMap<u8, u64>,
    pub marker_count: u64,
    pub boundary_element_count: u64,
    pub markers: Vec<Su2MarkerSummary>,
    pub minimum_coordinates: [Option<f64>; 3],
    pub maximum_coordinates: [Option<f64>; 3],
    pub evidence_level: String,
}

impl Su2MeshSummary {
    pub fn to_csv(&self) -> String {
        let element_counts = self
            .volume_element_type_counts
            .iter()
            .map(|(kind, count)| format!("{kind}:{count}"))
            .collect::<Vec<_>>()
            .join(";");
        let markers = self
            .markers
            .iter()
            .map(|marker| format!("{}:{}", marker.tag, marker.element_count))
            .collect::<Vec<_>>()
            .join(";");
        let mut output = String::from(
            "schema,byte_length,mesh_sha256,dimension,point_count,volume_element_count,volume_element_type_counts,marker_count,boundary_element_count,markers,min_x,min_y,min_z,max_x,max_y,max_z,evidence_level\n",
        );
        let values = [
            csv_escape(&self.schema),
            self.byte_length.to_string(),
            self.mesh_sha256.clone(),
            self.dimension.to_string(),
            self.point_count.to_string(),
            self.volume_element_count.to_string(),
            csv_escape(&element_counts),
            self.marker_count.to_string(),
            self.boundary_element_count.to_string(),
            csv_escape(&markers),
            optional(self.minimum_coordinates[0]),
            optional(self.minimum_coordinates[1]),
            optional(self.minimum_coordinates[2]),
            optional(self.maximum_coordinates[0]),
            optional(self.maximum_coordinates[1]),
            optional(self.maximum_coordinates[2]),
            csv_escape(&self.evidence_level),
        ];
        output.push_str(&values.join(","));
        output.push('\n');
        output
    }
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn optional(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.17}"))
        .unwrap_or_default()
}

fn next_nonempty<'a>(
    lines: &[(usize, &'a str)],
    cursor: &mut usize,
    context: &str,
) -> Result<(usize, &'a str), String> {
    while *cursor < lines.len() {
        let (line_number, raw) = lines[*cursor];
        *cursor += 1;
        let line = raw.trim();
        if !line.is_empty() {
            return Ok((line_number, line));
        }
    }
    Err(format!("unexpected end of mesh while reading {context}"))
}

fn parse_key_count(
    line_number: usize,
    line: &str,
    key: &str,
    allow_zero: bool,
) -> Result<u64, String> {
    let value = line
        .strip_prefix(key)
        .ok_or_else(|| format!("line {line_number}: expected {key}"))?
        .trim();
    let parsed = value
        .parse::<u64>()
        .map_err(|error| format!("line {line_number}: invalid {key} count: {error}"))?;
    if parsed == 0 && !allow_zero {
        return Err(format!("line {line_number}: {key} must be positive"));
    }
    Ok(parsed)
}

fn parse_element_type(
    line_number: usize,
    line: &str,
    dimension: u8,
    boundary: bool,
    point_count: Option<u64>,
) -> Result<(u8, Vec<u64>), String> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    if fields.is_empty() {
        return Err(format!("line {line_number}: empty element connectivity"));
    }
    let kind = fields[0]
        .parse::<u8>()
        .map_err(|error| format!("line {line_number}: invalid VTK element type: {error}"))?;
    let expected_nodes = match (dimension, boundary, kind) {
        (2, false, 5) => 3,  // triangle
        (2, false, 9) => 4,  // quadrilateral
        (3, false, 10) => 4, // tetrahedron
        (3, false, 12) => 8, // hexahedron
        (3, false, 13) => 6, // prism
        (3, false, 14) => 5, // pyramid
        (2, true, 3) => 2,   // line
        (3, true, 5) => 3,   // triangle
        (3, true, 9) => 4,   // quadrilateral
        _ => {
            return Err(format!(
                "line {line_number}: VTK element type {kind} is not a supported {} element in {dimension}D",
                if boundary { "boundary" } else { "volume" }
            ));
        }
    };
    if fields.len() != expected_nodes + 1 && fields.len() != expected_nodes + 2 {
        return Err(format!(
            "line {line_number}: element type {kind} requires {expected_nodes} node indices and an optional trailing legacy index"
        ));
    }
    let mut node_indices = Vec::with_capacity(expected_nodes);
    for token in &fields[1..=expected_nodes] {
        let index = token
            .parse::<u64>()
            .map_err(|error| format!("line {line_number}: invalid node index: {error}"))?;
        if point_count.is_some_and(|count| index >= count) {
            return Err(format!(
                "line {line_number}: node index {index} is outside declared point count {}",
                point_count.unwrap_or_default()
            ));
        }
        node_indices.push(index);
    }
    if fields.len() == expected_nodes + 2 {
        fields[expected_nodes + 1].parse::<u64>().map_err(|error| {
            format!("line {line_number}: invalid optional legacy element index: {error}")
        })?;
    }
    Ok((kind, node_indices))
}

/// Parse and validate one native ASCII SU2 mesh byte sequence.
pub fn inspect_su2_mesh(bytes: &[u8]) -> Result<Su2MeshSummary, String> {
    if bytes.is_empty() {
        return Err("SU2 mesh bytes must not be empty".to_owned());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("SU2 native mesh is not valid UTF-8/ASCII text: {error}"))?;
    if !text.is_ascii() {
        return Err("SU2 native mesh must be ASCII".to_owned());
    }
    let lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim_end_matches('\r')))
        .collect::<Vec<_>>();
    let mut cursor = 0;
    let (dimension_line, dimension_text) = next_nonempty(&lines, &mut cursor, "NDIME")?;
    let dimension_text = dimension_text
        .strip_prefix("NDIME=")
        .ok_or_else(|| format!("line {dimension_line}: first record must be NDIME="))?
        .trim();
    let dimension = dimension_text
        .parse::<u8>()
        .map_err(|error| format!("line {dimension_line}: invalid NDIME: {error}"))?;
    if dimension != 2 && dimension != 3 {
        return Err(format!(
            "line {dimension_line}: only 2D or 3D SU2 meshes are supported, got {dimension}"
        ));
    }

    let mut point_count = None;
    let mut volume_element_count = None;
    let mut marker_count = None;
    let mut volume_element_type_counts = BTreeMap::<u8, u64>::new();
    let mut node_references = Vec::<(usize, u64)>::new();
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut have_points = false;
    let mut markers = Vec::<Su2MarkerSummary>::new();
    let mut total_boundary_elements = 0_u64;
    let mut seen_marker_tags = std::collections::BTreeSet::new();

    while cursor < lines.len() {
        let (line_number, line) = next_nonempty(&lines, &mut cursor, "mesh section")?;
        if line.starts_with("NPOIN=") {
            if point_count.is_some() {
                return Err(format!("line {line_number}: duplicate NPOIN section"));
            }
            let count = parse_key_count(line_number, line, "NPOIN=", false)?;
            for _ in 0..count {
                let (point_line, point_record) =
                    next_nonempty(&lines, &mut cursor, "point coordinate record")?;
                let fields = point_record.split_whitespace().collect::<Vec<_>>();
                if fields.len() != usize::from(dimension)
                    && fields.len() != usize::from(dimension) + 1
                {
                    return Err(format!(
                        "line {point_line}: expected {} coordinates and optional legacy point index",
                        dimension
                    ));
                }
                for axis in 0..usize::from(dimension) {
                    let coordinate = fields[axis].parse::<f64>().map_err(|error| {
                        format!("line {point_line}: invalid coordinate: {error}")
                    })?;
                    if !coordinate.is_finite() {
                        return Err(format!("line {point_line}: coordinate must be finite"));
                    }
                    min[axis] = min[axis].min(coordinate);
                    max[axis] = max[axis].max(coordinate);
                }
                if fields.len() == usize::from(dimension) + 1 {
                    fields[usize::from(dimension)]
                        .parse::<u64>()
                        .map_err(|error| {
                            format!("line {point_line}: invalid optional point index: {error}")
                        })?;
                }
            }
            point_count = Some(count);
            have_points = true;
        } else if line.starts_with("NELEM=") {
            if volume_element_count.is_some() {
                return Err(format!("line {line_number}: duplicate NELEM section"));
            }
            let count = parse_key_count(line_number, line, "NELEM=", false)?;
            for _ in 0..count {
                let (element_line, element_record) =
                    next_nonempty(&lines, &mut cursor, "volume element connectivity")?;
                let (kind, node_indices) = parse_element_type(
                    element_line,
                    element_record,
                    dimension,
                    false,
                    point_count,
                )?;
                node_references.extend(node_indices.into_iter().map(|index| (element_line, index)));
                *volume_element_type_counts.entry(kind).or_default() += 1;
            }
            volume_element_count = Some(count);
        } else if line.starts_with("NMARK=") {
            if marker_count.is_some() {
                return Err(format!("line {line_number}: duplicate NMARK section"));
            }
            let count = parse_key_count(line_number, line, "NMARK=", true)?;
            for _ in 0..count {
                let (tag_line, tag_record) = next_nonempty(&lines, &mut cursor, "MARKER_TAG")?;
                let tag = tag_record
                    .strip_prefix("MARKER_TAG=")
                    .ok_or_else(|| format!("line {tag_line}: expected MARKER_TAG="))?
                    .trim();
                if tag.is_empty() || !seen_marker_tags.insert(tag.to_owned()) {
                    return Err(format!(
                        "line {tag_line}: marker tag is empty or duplicated"
                    ));
                }
                let (count_line, count_record) =
                    next_nonempty(&lines, &mut cursor, "MARKER_ELEMS")?;
                let element_count =
                    parse_key_count(count_line, count_record, "MARKER_ELEMS=", false)?;
                for _ in 0..element_count {
                    let (element_line, element_record) =
                        next_nonempty(&lines, &mut cursor, "marker element connectivity")?;
                    let (_kind, node_indices) = parse_element_type(
                        element_line,
                        element_record,
                        dimension,
                        true,
                        point_count,
                    )?;
                    node_references
                        .extend(node_indices.into_iter().map(|index| (element_line, index)));
                }
                total_boundary_elements = total_boundary_elements
                    .checked_add(element_count)
                    .ok_or_else(|| "boundary element count overflowed".to_owned())?;
                markers.push(Su2MarkerSummary {
                    tag: tag.to_owned(),
                    element_count,
                });
            }
            marker_count = Some(count);
        } else {
            return Err(format!(
                "line {line_number}: unsupported SU2 mesh record; only single-zone NDIME/NELEM/NPOIN/NMARK format is accepted: {line}"
            ));
        }
    }

    let point_count = point_count.ok_or_else(|| "SU2 mesh is missing NPOIN".to_owned())?;
    let volume_element_count =
        volume_element_count.ok_or_else(|| "SU2 mesh is missing NELEM".to_owned())?;
    let marker_count = marker_count.ok_or_else(|| "SU2 mesh is missing NMARK".to_owned())?;
    if !have_points || volume_element_type_counts.values().sum::<u64>() != volume_element_count {
        return Err("SU2 mesh point/volume section counts are inconsistent".to_owned());
    }
    if marker_count != markers.len() as u64 {
        return Err("SU2 mesh marker section count is inconsistent".to_owned());
    }
    for (line, index) in node_references {
        if index >= point_count {
            return Err(format!(
                "line {line}: node index {index} is outside declared point count {point_count}"
            ));
        }
    }
    markers.sort_by(|left, right| left.tag.cmp(&right.tag));

    let mut minimum_coordinates = [None; 3];
    let mut maximum_coordinates = [None; 3];
    for axis in 0..usize::from(dimension) {
        minimum_coordinates[axis] = Some(min[axis]);
        maximum_coordinates[axis] = Some(max[axis]);
    }

    Ok(Su2MeshSummary {
        schema: SU2_MESH_SUMMARY_SCHEMA.to_owned(),
        byte_length: bytes.len() as u64,
        mesh_sha256: sha256_hex(bytes)?,
        dimension,
        point_count,
        volume_element_count,
        volume_element_type_counts,
        marker_count,
        boundary_element_count: total_boundary_elements,
        markers,
        minimum_coordinates,
        maximum_coordinates,
        evidence_level: SU2_MESH_EVIDENCE_LEVEL.to_owned(),
    })
}

/// Mesh-derived characteristic spacing for a linear 2D triangle/quadrilateral SU2 grid.
///
/// The coordinate unit is supplied by the caller and is not inferred from SU2 ASCII mesh bytes.
/// For this explicitly scoped definition, h = sqrt(total cell area / volume-cell count).
/// Unsupported dimensions and cell types fail closed; this is not an anisotropy or mesh-quality
/// metric. Cell-area coefficient of variation is retained to expose gross nonuniformity.
#[derive(Debug, Clone, PartialEq)]
pub struct Su2MeshSpacing2d {
    pub schema: &'static str,
    pub mesh_byte_length: u64,
    pub mesh_sha256: String,
    pub coordinate_unit: String,
    pub cell_count: u64,
    pub total_cell_area: f64,
    pub mean_cell_area: f64,
    pub minimum_cell_area: f64,
    pub maximum_cell_area: f64,
    pub cell_area_coefficient_of_variation: f64,
    pub characteristic_spacing: f64,
    pub evidence_level: &'static str,
}

pub const SU2_MESH_SPACING_2D_SCHEMA: &str = "su2-mesh-spacing-2d/0.1.0";
pub const SU2_MESH_SPACING_2D_EVIDENCE_LEVEL: &str =
    "derived_from_exact_2d_linear_triangle_quad_mesh_bytes_not_mesh_quality_or_cfd_validation";

/// Derive h from exact native mesh cell areas, without accepting a caller-entered spacing.
///
/// Only two-dimensional linear triangles (VTK type 5) and quadrilaterals (VTK type 9) are
/// accepted. Coordinates must form a finite, indexed point table. The coordinate unit is an
/// explicit declaration; the numerical metric itself is calculated directly from mesh bytes.
pub fn derive_su2_characteristic_spacing_2d(
    bytes: &[u8],
    coordinate_unit: &str,
) -> Result<Su2MeshSpacing2d, String> {
    if coordinate_unit.trim().is_empty() {
        return Err("mesh coordinate unit must be declared".to_owned());
    }
    let summary = inspect_su2_mesh(bytes)?;
    if summary.dimension != 2 {
        return Err(format!(
            "mesh-derived area spacing is restricted to 2D; got {}D",
            summary.dimension
        ));
    }
    if summary
        .volume_element_type_counts
        .keys()
        .any(|kind| !matches!(kind, 5 | 9))
    {
        return Err(
            "mesh-derived spacing supports only linear triangles and quadrilaterals".to_owned(),
        );
    }

    let text =
        std::str::from_utf8(bytes).map_err(|error| format!("SU2 mesh is not UTF-8: {error}"))?;
    let lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim_end_matches('\r')))
        .collect::<Vec<_>>();
    let mut cursor = 0;
    let (dimension_line, dimension_record) = next_nonempty(&lines, &mut cursor, "NDIME")?;
    let dimension = dimension_record
        .strip_prefix("NDIME=")
        .ok_or_else(|| format!("line {dimension_line}: expected NDIME="))?
        .trim()
        .parse::<u8>()
        .map_err(|error| format!("line {dimension_line}: invalid NDIME: {error}"))?;
    if dimension != 2 {
        return Err("mesh-derived characteristic spacing requires NDIME=2".to_owned());
    }

    let mut point_coordinates: Option<Vec<Option<[f64; 2]>>> = None;
    let mut volume_cells = Vec::<(usize, u8, Vec<u64>)>::new();
    let mut saw_elements = false;
    let mut saw_markers = false;
    while cursor < lines.len() {
        let (line_number, line) = next_nonempty(&lines, &mut cursor, "mesh section")?;
        if line.starts_with("NELEM=") {
            if saw_elements {
                return Err(format!("line {line_number}: duplicate NELEM section"));
            }
            saw_elements = true;
            let count = parse_key_count(line_number, line, "NELEM=", false)?;
            for _ in 0..count {
                let (element_line, record) =
                    next_nonempty(&lines, &mut cursor, "volume element connectivity")?;
                let (kind, nodes) =
                    parse_element_type(element_line, record, 2, false, Some(summary.point_count))?;
                if !matches!(kind, 5 | 9) {
                    return Err(format!(
                        "line {element_line}: spacing derivation supports only triangles/quads; got type {kind}"
                    ));
                }
                volume_cells.push((element_line, kind, nodes));
            }
        } else if line.starts_with("NPOIN=") {
            if point_coordinates.is_some() {
                return Err(format!("line {line_number}: duplicate NPOIN section"));
            }
            let count = parse_key_count(line_number, line, "NPOIN=", false)?;
            let count_usize = usize::try_from(count)
                .map_err(|error| format!("NPOIN count does not fit memory index: {error}"))?;
            let mut points = vec![None; count_usize];
            for sequential_index in 0..count_usize {
                let (point_line, record) =
                    next_nonempty(&lines, &mut cursor, "point coordinate record")?;
                let fields = record.split_whitespace().collect::<Vec<_>>();
                if fields.len() != 2 && fields.len() != 3 {
                    return Err(format!(
                        "line {point_line}: expected two coordinates and optional point index"
                    ));
                }
                let x = fields[0]
                    .parse::<f64>()
                    .map_err(|error| format!("line {point_line}: invalid x coordinate: {error}"))?;
                let y = fields[1]
                    .parse::<f64>()
                    .map_err(|error| format!("line {point_line}: invalid y coordinate: {error}"))?;
                if !x.is_finite() || !y.is_finite() {
                    return Err(format!("line {point_line}: coordinates must be finite"));
                }
                let point_index = if fields.len() == 3 {
                    fields[2].parse::<usize>().map_err(|error| {
                        format!("line {point_line}: invalid point index: {error}")
                    })?
                } else {
                    sequential_index
                };
                if point_index >= points.len() {
                    return Err(format!("line {point_line}: point index is outside NPOIN"));
                }
                if points[point_index].replace([x, y]).is_some() {
                    return Err(format!(
                        "line {point_line}: duplicate point index {point_index}"
                    ));
                }
            }
            point_coordinates = Some(points);
        } else if line.starts_with("NMARK=") {
            if saw_markers {
                return Err(format!("line {line_number}: duplicate NMARK section"));
            }
            saw_markers = true;
            let marker_count = parse_key_count(line_number, line, "NMARK=", true)?;
            for _ in 0..marker_count {
                let (tag_line, tag_record) = next_nonempty(&lines, &mut cursor, "MARKER_TAG")?;
                if !tag_record.starts_with("MARKER_TAG=") {
                    return Err(format!("line {tag_line}: expected MARKER_TAG="));
                }
                let (count_line, count_record) =
                    next_nonempty(&lines, &mut cursor, "MARKER_ELEMS")?;
                let boundary_count =
                    parse_key_count(count_line, count_record, "MARKER_ELEMS=", false)?;
                for _ in 0..boundary_count {
                    let (boundary_line, record) =
                        next_nonempty(&lines, &mut cursor, "marker element connectivity")?;
                    parse_element_type(boundary_line, record, 2, true, Some(summary.point_count))?;
                }
            }
        } else {
            return Err(format!(
                "line {line_number}: unsupported mesh record in spacing derivation: {line}"
            ));
        }
    }

    if !saw_elements {
        return Err("mesh spacing derivation did not find NELEM".to_owned());
    }
    let points =
        point_coordinates.ok_or_else(|| "mesh spacing derivation did not find NPOIN".to_owned())?;
    if points.iter().any(Option::is_none) {
        return Err("mesh point table contains an unfilled point index".to_owned());
    }
    if volume_cells.len() != summary.volume_element_count as usize {
        return Err("parsed cell count differs from native mesh summary".to_owned());
    }

    let mut areas = Vec::with_capacity(volume_cells.len());
    for (line_number, kind, nodes) in volume_cells {
        let coordinates = nodes
            .iter()
            .map(|index| {
                let index = usize::try_from(*index).map_err(|error| {
                    format!("line {line_number}: node index does not fit memory index: {error}")
                })?;
                points.get(index).and_then(|point| *point).ok_or_else(|| {
                    format!("line {line_number}: cell references missing point {index}")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let area = if kind == 5 {
            let [a, b, c] = [coordinates[0], coordinates[1], coordinates[2]];
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
        } else {
            let mut twice_area = 0.0;
            for index in 0..4 {
                let current = coordinates[index];
                let next = coordinates[(index + 1) % 4];
                twice_area += current[0] * next[1] - next[0] * current[1];
            }
            twice_area.abs() * 0.5
        };
        if !area.is_finite() || area <= 1.0e-18 {
            return Err(format!(
                "line {line_number}: cell has zero or non-finite area"
            ));
        }
        areas.push(area);
    }

    let count = areas.len() as f64;
    let mut total = 0.0;
    let mut compensation = 0.0;
    for area in &areas {
        let adjusted = *area - compensation;
        let next = total + adjusted;
        compensation = (next - total) - adjusted;
        total = next;
    }
    let mean = total / count;
    let mut squared_deviation = 0.0;
    for area in &areas {
        squared_deviation += (*area - mean).powi(2);
    }
    let standard_deviation = (squared_deviation / count).sqrt();
    let spacing = mean.sqrt();
    if !total.is_finite() || !mean.is_finite() || !spacing.is_finite() || spacing <= 0.0 {
        return Err("mesh-derived characteristic spacing is non-finite or non-positive".to_owned());
    }
    Ok(Su2MeshSpacing2d {
        schema: SU2_MESH_SPACING_2D_SCHEMA,
        mesh_byte_length: bytes.len() as u64,
        mesh_sha256: sha256_hex(bytes)?,
        coordinate_unit: coordinate_unit.to_owned(),
        cell_count: areas.len() as u64,
        total_cell_area: total,
        mean_cell_area: mean,
        minimum_cell_area: areas.iter().copied().fold(f64::INFINITY, f64::min),
        maximum_cell_area: areas.iter().copied().fold(0.0, f64::max),
        cell_area_coefficient_of_variation: standard_deviation / mean,
        characteristic_spacing: spacing,
        evidence_level: SU2_MESH_SPACING_2D_EVIDENCE_LEVEL,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE_2D: &str = "NDIME= 2\nNELEM= 2\n5 0 1 2\n5 0 2 3\nNPOIN= 4\n0 0\n1 0\n1 1\n0 1\nNMARK= 1\nMARKER_TAG= wall\nMARKER_ELEMS= 4\n3 0 1\n3 1 2\n3 2 3\n3 3 0\n";

    #[test]
    fn parses_2d_triangular_mesh_and_marker_connectivity() {
        let summary = inspect_su2_mesh(SQUARE_2D.as_bytes()).expect("valid SU2 square mesh");
        assert_eq!(summary.dimension, 2);
        assert_eq!(summary.point_count, 4);
        assert_eq!(summary.volume_element_count, 2);
        assert_eq!(summary.volume_element_type_counts.get(&5), Some(&2));
        assert_eq!(summary.marker_count, 1);
        assert_eq!(summary.boundary_element_count, 4);
        assert_eq!(summary.markers[0].tag, "wall");
        assert_eq!(summary.minimum_coordinates, [Some(0.0), Some(0.0), None]);
        assert_eq!(summary.maximum_coordinates, [Some(1.0), Some(1.0), None]);
        assert!(summary.to_csv().lines().count() == 2);
    }

    #[test]
    fn derives_characteristic_spacing_from_exact_2d_cell_areas() {
        let triangles = derive_su2_characteristic_spacing_2d(SQUARE_2D.as_bytes(), "m")
            .expect("2D triangular cell-area spacing");
        assert_eq!(triangles.cell_count, 2);
        assert!((triangles.total_cell_area - 1.0).abs() < 1.0e-14);
        assert!((triangles.mean_cell_area - 0.5).abs() < 1.0e-14);
        assert!((triangles.characteristic_spacing - 0.5_f64.sqrt()).abs() < 1.0e-14);
        assert!(triangles.cell_area_coefficient_of_variation < 1.0e-14);

        let quad = "NDIME= 2\nNELEM= 1\n9 0 1 2 3\nNPOIN= 4\n0 0\n1 0\n1 1\n0 1\nNMARK= 1\nMARKER_TAG= wall\nMARKER_ELEMS= 4\n3 0 1\n3 1 2\n3 2 3\n3 3 0\n";
        let quadrilaterals = derive_su2_characteristic_spacing_2d(quad.as_bytes(), "m")
            .expect("2D quadrilateral cell-area spacing");
        assert_eq!(quadrilaterals.cell_count, 1);
        assert!((quadrilaterals.total_cell_area - 1.0).abs() < 1.0e-14);
        assert!((quadrilaterals.characteristic_spacing - 1.0).abs() < 1.0e-14);

        let three_dimensional =
            "NDIME= 3\nNPOIN= 4\n0 0 0\n1 0 0\n0 1 0\n0 0 1\nNELEM= 1\n10 0 1 2 3\nNMARK= 0\n";
        assert!(
            derive_su2_characteristic_spacing_2d(three_dimensional.as_bytes(), "m")
                .expect_err("3D mesh must not use 2D area spacing")
                .contains("restricted to 2D")
        );
    }

    #[test]
    fn rejects_degenerate_cells_and_missing_coordinate_units() {
        let degenerate = "NDIME= 2\nNELEM= 1\n5 0 1 2\nNPOIN= 3\n0 0\n1 0\n2 0\nNMARK= 0\n";
        assert!(
            derive_su2_characteristic_spacing_2d(degenerate.as_bytes(), "m")
                .expect_err("zero-area cell rejected")
                .contains("zero or non-finite area")
        );
        assert!(derive_su2_characteristic_spacing_2d(SQUARE_2D.as_bytes(), " ").is_err());
    }

    #[test]
    fn parses_optional_legacy_point_and_element_indices() {
        let mesh = "NDIME= 2\nNPOIN= 3\n0 0 10\n1 0 11\n0 1 12\nNELEM= 1\n5 0 1 2 99\nNMARK= 0\n";
        let summary = inspect_su2_mesh(mesh.as_bytes()).expect("legacy index mesh");
        assert_eq!(summary.point_count, 3);
        assert_eq!(summary.volume_element_count, 1);
    }

    #[test]
    fn parses_3d_tetrahedral_mesh_and_triangular_marker() {
        let mesh = "NDIME= 3\nNPOIN= 4\n0 0 0\n1 0 0\n0 1 0\n0 0 1\nNELEM= 1\n10 0 1 2 3\nNMARK= 1\nMARKER_TAG= wall\nMARKER_ELEMS= 1\n5 0 1 2\n";
        let summary = inspect_su2_mesh(mesh.as_bytes()).expect("valid tetrahedral mesh");
        assert_eq!(summary.dimension, 3);
        assert_eq!(summary.point_count, 4);
        assert_eq!(summary.volume_element_type_counts.get(&10), Some(&1));
        assert_eq!(summary.boundary_element_count, 1);
        assert_eq!(
            summary.minimum_coordinates,
            [Some(0.0), Some(0.0), Some(0.0)]
        );
        assert_eq!(
            summary.maximum_coordinates,
            [Some(1.0), Some(1.0), Some(1.0)]
        );
    }

    #[test]
    fn rejects_bad_dimension_duplicate_sections_unknown_records_and_empty_bytes() {
        assert!(inspect_su2_mesh(b"").is_err());
        assert!(
            inspect_su2_mesh(b"NDIME= 4\nNPOIN= 1\n0 0 0 0\nNELEM= 1\n5 0 0 0\nNMARK= 0\n")
                .is_err()
        );
        assert!(
            inspect_su2_mesh(
                b"NDIME= 2\nNPOIN= 1\n0 0\nNPOIN= 1\n0 0\nNELEM= 1\n5 0 0 0\nNMARK= 0\n"
            )
            .is_err()
        );
        assert!(
            inspect_su2_mesh(b"NDIME= 2\nNPOIN= 1\n0 0\nNELEM= 1\n5 0 0 0\nNMARK= 0\nNZONE=2\n")
                .is_err()
        );
    }

    #[test]
    fn rejects_out_of_range_nodes_unknown_element_types_nonfinite_coordinates_and_duplicate_markers()
     {
        let out_of_range = "NDIME= 2\nNELEM= 1\n5 0 1 4\nNPOIN= 3\n0 0\n1 0\n0 1\nNMARK= 0\n";
        assert!(inspect_su2_mesh(out_of_range.as_bytes()).is_err());
        let unknown_type = "NDIME= 2\nNPOIN= 3\n0 0\n1 0\n0 1\nNELEM= 1\n77 0 1 2\nNMARK= 0\n";
        assert!(inspect_su2_mesh(unknown_type.as_bytes()).is_err());
        let nonfinite = "NDIME= 2\nNPOIN= 3\n0 0\nNaN 0\n0 1\nNELEM= 1\n5 0 1 2\nNMARK= 0\n";
        assert!(inspect_su2_mesh(nonfinite.as_bytes()).is_err());
        let duplicate_marker = "NDIME= 2\nNPOIN= 3\n0 0\n1 0\n0 1\nNELEM= 1\n5 0 1 2\nNMARK= 2\nMARKER_TAG= wall\nMARKER_ELEMS= 1\n3 0 1\nMARKER_TAG= wall\nMARKER_ELEMS= 1\n3 1 2\n";
        assert!(inspect_su2_mesh(duplicate_marker.as_bytes()).is_err());
    }
}
