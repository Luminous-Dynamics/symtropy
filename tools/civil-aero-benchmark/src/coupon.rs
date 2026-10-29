//! Geometry generation for a non-flight-critical, extruded NACA 0012 coupon.
//!
//! The profile follows NASA TMR's published corrected scaled sharp-trailing-edge
//! polynomial. It is not copied from the TMR grid files, and the resulting coupon is not
//! a flight structure, validated wind-tunnel model, or authorization to manufacture/operate.

use std::collections::BTreeMap;
use std::f64::consts::PI;

const PROFILE_REFERENCE: &str = "https://tmbwg.github.io/turbmodels/naca0012_val.html";
const PROFILE_REVISION: &str = "naca0012-tmr-corrected-scaled-sharp-te-polynomial-v0.1";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CouponSpec {
    pub chord_mm: f64,
    pub span_mm: f64,
    pub chordwise_panels: usize,
    pub spanwise_panels: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriangleMesh {
    pub vertices_mm: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeshAudit {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub unique_edge_count: usize,
    pub signed_volume_mm3: f64,
    pub watertight_two_manifold: bool,
    pub orientation_consistent: bool,
}

impl CouponSpec {
    pub fn validate(self) -> Result<(), String> {
        if !self.chord_mm.is_finite() || self.chord_mm <= 0.0 {
            return Err("chord_mm must be finite and greater than zero".to_owned());
        }
        if !self.span_mm.is_finite() || self.span_mm <= 0.0 {
            return Err("span_mm must be finite and greater than zero".to_owned());
        }
        if !(16..=400).contains(&self.chordwise_panels) {
            return Err("chordwise_panels must be in 16..=400".to_owned());
        }
        if !(1..=60).contains(&self.spanwise_panels) {
            return Err("spanwise_panels must be in 1..=60".to_owned());
        }
        Ok(())
    }
}

pub fn generate_coupon(spec: CouponSpec) -> Result<TriangleMesh, String> {
    spec.validate()?;

    let n = spec.chordwise_panels;
    let m = spec.spanwise_panels;
    let row_width = m + 1;
    let upper_count = (n + 1) * row_width;
    let mut vertices_mm = Vec::with_capacity(upper_count + (n - 1) * row_width);

    // Cosine spacing clusters stations near the leading/trailing edges.
    for i in 0..=n {
        let theta = PI * i as f64 / n as f64;
        let x_fraction = 0.5 * (1.0 - theta.cos());
        let half_thickness = tmr_naca0012_half_thickness(x_fraction) * spec.chord_mm;
        for j in 0..=m {
            let y_mm = spec.span_mm * j as f64 / m as f64;
            vertices_mm.push([x_fraction * spec.chord_mm, y_mm, half_thickness]);
        }
    }

    // The leading and trailing edge have zero thickness, so upper and lower surfaces
    // share indices there. Interior lower-surface vertices have their own indices.
    let mut lower_ids = vec![0usize; upper_count];
    for i in 0..=n {
        for j in 0..=m {
            let slot = grid_index(i, j, row_width);
            if i == 0 || i == n {
                lower_ids[slot] = slot;
            } else {
                let x_fraction = 0.5 * (1.0 - (PI * i as f64 / n as f64).cos());
                let half_thickness = tmr_naca0012_half_thickness(x_fraction) * spec.chord_mm;
                let y_mm = spec.span_mm * j as f64 / m as f64;
                lower_ids[slot] = vertices_mm.len();
                vertices_mm.push([x_fraction * spec.chord_mm, y_mm, -half_thickness]);
            }
        }
    }

    let top = |i: usize, j: usize| grid_index(i, j, row_width);
    let bottom = |i: usize, j: usize| lower_ids[grid_index(i, j, row_width)];
    let estimated_faces = 4 * n * m + 4 * n + 4 * m;
    let mut mesh = TriangleMesh {
        vertices_mm,
        triangles: Vec::with_capacity(estimated_faces),
    };

    // Upper skin: +Z outward normal.
    for i in 0..n {
        for j in 0..m {
            let a = top(i, j);
            let b = top(i + 1, j);
            let c = top(i + 1, j + 1);
            let d = top(i, j + 1);
            push_non_degenerate(&mut mesh, [a, b, c]);
            push_non_degenerate(&mut mesh, [a, c, d]);

            // Lower skin: reverse each triangle so its outward normal is -Z.
            let la = bottom(i, j);
            let lb = bottom(i + 1, j);
            let lc = bottom(i + 1, j + 1);
            let ld = bottom(i, j + 1);
            push_non_degenerate(&mut mesh, [la, lc, lb]);
            push_non_degenerate(&mut mesh, [la, ld, lc]);
        }
    }

    // Root end cap: outward normal -Y.
    for i in 0..n {
        let a = top(i, 0);
        let b = top(i + 1, 0);
        let c = bottom(i + 1, 0);
        let d = bottom(i, 0);
        push_non_degenerate(&mut mesh, [a, d, c]);
        push_non_degenerate(&mut mesh, [a, c, b]);
    }

    // Tip end cap: outward normal +Y.
    for i in 0..n {
        let a = top(i, m);
        let b = top(i + 1, m);
        let c = bottom(i + 1, m);
        let d = bottom(i, m);
        push_non_degenerate(&mut mesh, [a, b, c]);
        push_non_degenerate(&mut mesh, [a, c, d]);
    }

    let audit = mesh.audit()?;
    if !audit.watertight_two_manifold {
        return Err("generated coupon mesh failed closed two-manifold edge audit".to_owned());
    }
    if audit.signed_volume_mm3 <= 0.0 {
        return Err("generated coupon mesh has non-positive signed volume".to_owned());
    }
    Ok(mesh)
}

/// Half-thickness ratio from NASA TMR's corrected, scaled sharp-trailing-edge profile.
/// x_fraction is chord-normalized and must lie in [0, 1].
pub fn tmr_naca0012_half_thickness(x_fraction: f64) -> f64 {
    let x = x_fraction.clamp(0.0, 1.0);
    0.594_689_181
        * (0.298_222_773 * x.sqrt()
            - 0.127_125_232 * x
            - 0.357_907_906 * x.powi(2)
            + 0.291_984_971 * x.powi(3)
            - 0.105_174_606 * x.powi(4))
}

impl TriangleMesh {
    pub fn audit(&self) -> Result<MeshAudit, String> {
        if self.vertices_mm.iter().flatten().any(|v| !v.is_finite()) {
            return Err("mesh contains non-finite coordinates".to_owned());
        }
        if self.triangles.is_empty() {
            return Err("mesh contains no triangles".to_owned());
        }

        let mut edge_uses: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut edge_orientation: BTreeMap<(usize, usize), i32> = BTreeMap::new();
        let mut vertex_used = vec![false; self.vertices_mm.len()];
        for triangle in &self.triangles {
            let [a, b, c] = *triangle;
            if a >= self.vertices_mm.len()
                || b >= self.vertices_mm.len()
                || c >= self.vertices_mm.len()
            {
                return Err("triangle index is outside the vertex array".to_owned());
            }
            if a == b
                || b == c
                || c == a
                || triangle_area_mm2(self.vertices_mm, *triangle) <= 1e-12
            {
                return Err("mesh contains a degenerate triangle".to_owned());
            }
            vertex_used[a] = true;
            vertex_used[b] = true;
            vertex_used[c] = true;
            for (u, v) in [(a, b), (b, c), (c, a)] {
                let key = if u < v { (u, v) } else { (v, u) };
                *edge_uses.entry(key).or_insert(0) += 1;
                *edge_orientation.entry(key).or_insert(0) += if u < v { 1 } else { -1 };
            }
        }
        if vertex_used.iter().any(|used| !used) {
            return Err("mesh contains an unreferenced vertex".to_owned());
        }

        let edge_two_manifold = edge_uses.values().all(|uses| *uses == 2);
        let orientation_consistent = edge_orientation.values().all(|balance| *balance == 0);
        let watertight_two_manifold = edge_two_manifold && orientation_consistent;
        let signed_volume_mm3 = self.signed_volume_mm3();
        Ok(MeshAudit {
            vertex_count: self.vertices_mm.len(),
            triangle_count: self.triangles.len(),
            unique_edge_count: edge_uses.len(),
            signed_volume_mm3,
            watertight_two_manifold,
            orientation_consistent,
        })
    }

    fn signed_volume_mm3(&self) -> f64 {
        self.triangles
            .iter()
            .map(|triangle| {
                let a = self.vertices_mm[triangle[0]];
                let b = self.vertices_mm[triangle[1]];
                let c = self.vertices_mm[triangle[2]];
                dot(a, cross(b, c)) / 6.0
            })
            .sum()
    }

    pub fn to_ascii_stl(&self, solid_name: &str) -> Result<String, String> {
        if solid_name.is_empty()
            || !solid_name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err("STL solid name must use ASCII letters, digits, '_' or '-'".to_owned());
        }
        let audit = self.audit()?;
        if !audit.watertight_two_manifold || audit.signed_volume_mm3 <= 0.0 {
            return Err("refusing to export a mesh that fails the closed-volume audit".to_owned());
        }

        let mut stl = String::with_capacity(self.triangles.len() * 180);
        stl.push_str(&format!("solid {solid_name}\n"));
        for triangle in &self.triangles {
            let a = self.vertices_mm[triangle[0]];
            let b = self.vertices_mm[triangle[1]];
            let c = self.vertices_mm[triangle[2]];
            let n = normalized(cross(subtract(b, a), subtract(c, a)))?;
            stl.push_str(&format!(
                "  facet normal {:.9e} {:.9e} {:.9e}\n    outer loop\n",
                n[0], n[1], n[2]
            ));
            for point in [a, b, c] {
                stl.push_str(&format!(
                    "      vertex {:.9e} {:.9e} {:.9e}\n",
                    point[0], point[1], point[2]
                ));
            }
            stl.push_str("    endloop\n  endfacet\n");
        }
        stl.push_str(&format!("endsolid {solid_name}\n"));
        Ok(stl)
    }

    pub fn json_manifest(&self, spec: CouponSpec, output_file: &str) -> Result<String, String> {
        let audit = self.audit()?;
        let escaped_output = json_escape(output_file);
        Ok(format!(
            concat!(
                "{{\n",
                "  \"manifest_schema\": \"civil-aero-coupon/0.1.0\",\n",
                "  \"artifact_scope\": \"non_flight_critical_ground_test_coupon\",\n",
                "  \"profile_revision\": \"{}\",\n",
                "  \"profile_reference\": \"{}\",\n",
                "  \"geometry_convention\": \"published corrected scaled sharp-trailing-edge NACA 0012 polynomial; not copied from NASA grid point bytes\",\n",
                "  \"units\": \"millimetres; STL itself does not encode units\",\n",
                "  \"chord_mm\": {:.9},\n",
                "  \"span_mm\": {:.9},\n",
                "  \"chordwise_panels\": {},\n",
                "  \"spanwise_panels\": {},\n",
                "  \"vertex_count\": {},\n",
                "  \"triangle_count\": {},\n",
                "  \"unique_edge_count\": {},\n",
                "  \"watertight_two_manifold\": {},\n",
                "  \"orientation_consistent\": {},\n",
                "  \"signed_volume_mm3\": {:.9},\n",
                "  \"output_file\": \"{}\",\n",
                "  \"output_sha256\": null,\n",
                "  \"output_sha256_status\": \"must be computed over the exact exported bytes after writing\",\n",
                "  \"manufacturing_release\": \"not_authorized\",\n",
                "  \"flight_use\": \"prohibited\",\n",
                "  \"wind_tunnel_suitability\": \"not_assessed; verify facility envelope, blockage, support, Reynolds regime and instrumentation\",\n",
                "  \"airworthiness_or_structural_claim\": false\n",
                "}}\n"
            ),
            PROFILE_REVISION,
            PROFILE_REFERENCE,
            spec.chord_mm,
            spec.span_mm,
            spec.chordwise_panels,
            spec.spanwise_panels,
            audit.vertex_count,
            audit.triangle_count,
            audit.unique_edge_count,
            audit.watertight_two_manifold,
            audit.orientation_consistent,
            audit.signed_volume_mm3,
            escaped_output
        ))
    }
}

fn grid_index(i: usize, j: usize, row_width: usize) -> usize {
    i * row_width + j
}

fn push_non_degenerate(mesh: &mut TriangleMesh, triangle: [usize; 3]) {
    if triangle_area_mm2(&mesh.vertices_mm, triangle) > 1e-12 {
        mesh.triangles.push(triangle);
    }
}

fn triangle_area_mm2(vertices: &[[f64; 3]], triangle: [usize; 3]) -> f64 {
    let a = vertices[triangle[0]];
    let b = vertices[triangle[1]];
    let c = vertices[triangle[2]];
    let normal = cross(subtract(b, a), subtract(c, a));
    dot(normal, normal).sqrt() * 0.5
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalized(value: [f64; 3]) -> Result<[f64; 3], String> {
    let length = dot(value, value).sqrt();
    if !length.is_finite() || length <= 0.0 {
        return Err("cannot calculate normal for a degenerate facet".to_owned());
    }
    Ok([value[0] / length, value[1] / length, value[2] / length])
}

fn json_escape(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> CouponSpec {
        CouponSpec {
            chord_mm: 100.0,
            span_mm: 80.0,
            chordwise_panels: 32,
            spanwise_panels: 4,
        }
    }

    #[test]
    fn generated_mesh_is_closed_manifold_with_positive_volume() {
        let mesh = generate_coupon(spec()).expect("coupon mesh");
        let audit = mesh.audit().expect("mesh audit");
        assert!(audit.watertight_two_manifold);
        assert!(audit.orientation_consistent);
        assert!(audit.signed_volume_mm3 > 0.0);
        assert!(audit.triangle_count > 100);
        assert!(audit.unique_edge_count > 100);
    }

    #[test]
    fn profile_has_closed_sharp_edges_and_positive_mid_chord_thickness() {
        assert!(tmr_naca0012_half_thickness(0.0).abs() < 1e-12);
        assert!(tmr_naca0012_half_thickness(1.0).abs() < 1e-12);
        assert!(tmr_naca0012_half_thickness(0.3) > 0.05);
        assert!((tmr_naca0012_half_thickness(0.29717) - 0.05948603830292508).abs() < 1e-10);
    }

    #[test]
    fn rejects_inconsistent_face_winding() {
        let mut mesh = generate_coupon(spec()).expect("coupon mesh");
        mesh.triangles[0].swap(1, 2);
        let audit = mesh.audit().expect("topology can still be inspected");
        assert!(!audit.orientation_consistent);
        assert!(!audit.watertight_two_manifold);
        assert!(mesh.to_ascii_stl("bad_winding").is_err());
    }

    #[test]
    fn rejects_bad_dimensions_and_resolution() {
        let mut bad = spec();
        bad.chord_mm = f64::NAN;
        assert!(generate_coupon(bad).is_err());
        let mut bad_resolution = spec();
        bad_resolution.chordwise_panels = 8;
        assert!(generate_coupon(bad_resolution).is_err());
    }

    #[test]
    fn exports_units_and_non_flight_boundary_in_manifest() {
        let spec = spec();
        let mesh = generate_coupon(spec).expect("coupon mesh");
        let stl = mesh.to_ascii_stl("civil_aero_nonflight_coupon").expect("STL");
        assert!(stl.starts_with("solid civil_aero_nonflight_coupon\n"));
        let manifest = mesh
            .json_manifest(spec, "coupon.stl")
            .expect("manifest");
        assert!(manifest.contains("\"units\": \"millimetres; STL itself does not encode units\""));
        assert!(manifest.contains("\"orientation_consistent\": true"));
        assert!(manifest.contains("\"manufacturing_release\": \"not_authorized\""));
        assert!(manifest.contains("\"output_sha256\": null"));
    }
}
