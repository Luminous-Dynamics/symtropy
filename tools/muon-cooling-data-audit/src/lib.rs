// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Value, json};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha256;
use std::fs;
use std::path::Path;

const COLUMN_COUNT: usize = 7;
const MOMENT_NAMES: [&str; COLUMN_COUNT] = ["x", "xp", "y", "yp", "z", "Ek", "t"];
const MOMENT_UNITS: [&str; COLUMN_COUNT] = ["mm", "rad", "mm", "rad", "mm", "MeV", "ns"];

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnStats {
    pub name: String,
    pub unit: String,
    pub mean: f64,
    pub standard_deviation: f64,
    pub minimum: f64,
    pub maximum: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeEmittance {
    pub value_mm_rad: f64,
    pub covariance: [[f64; 2]; 2],
    pub correlation: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserfileStats {
    pub record_count: usize,
    pub columns: Vec<ColumnStats>,
    pub horizontal_native_emittance: NativeEmittance,
    pub vertical_native_emittance: NativeEmittance,
}

pub fn audit_files(
    manifest_path: &Path,
    input_path: &Path,
) -> Result<Value, Vec<String>> {
    let manifest_bytes = fs::read(manifest_path).map_err(|error| {
        vec![format!(
            "cannot read source manifest {}: {error}",
            manifest_path.display()
        )]
    })?;
    let input_bytes = fs::read(input_path).map_err(|error| {
        vec![format!(
            "cannot read BDSIM userfile {}: {error}",
            input_path.display()
        )]
    })?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| vec![format!("source manifest is not valid JSON: {error}")])?;

    audit_bytes(&manifest, &input_bytes)
}

pub fn audit_bytes(manifest: &Value, input_bytes: &[u8]) -> Result<Value, Vec<String>> {
    validate_manifest(manifest)?;

    let expected_length = manifest
        .pointer("/input/byte_length")
        .and_then(Value::as_u64)
        .ok_or_else(|| vec!["input.byte_length must be an unsigned integer".to_owned()])?;

    if input_bytes.len() as u64 != expected_length {
        return Err(vec![format!(
            "input byte-length mismatch: manifest expects {expected_length}, received {}",
            input_bytes.len()
        )]);
    }

    let expected_blob = manifest
        .pointer("/input/git_blob_sha1")
        .and_then(Value::as_str)
        .ok_or_else(|| vec!["input.git_blob_sha1 must be a string".to_owned()])?;
    let actual_blob = git_blob_sha1(input_bytes);

    if actual_blob != expected_blob {
        return Err(vec![format!(
            "Git blob SHA-1 mismatch: manifest expects {expected_blob}, computed {actual_blob}"
        )]);
    }

    let input_text = std::str::from_utf8(input_bytes)
        .map_err(|error| vec![format!("BDSIM userfile is not valid UTF-8: {error}")])?;
    if input_text.contains('\0') {
        return Err(vec!["BDSIM userfile contains a NUL byte".to_owned()]);
    }

    let header_lines = manifest
        .pointer("/input/ignored_header_lines")
        .and_then(Value::as_u64)
        .ok_or_else(|| vec!["input.ignored_header_lines must be an unsigned integer".to_owned()])?;
    if header_lines != 1 {
        return Err(vec![
            "this adapter version supports exactly one ignored header line".to_owned(),
        ]);
    }

    let expected_header = manifest
        .pointer("/input/expected_header")
        .and_then(Value::as_str)
        .ok_or_else(|| vec!["input.expected_header must be a string".to_owned()])?;
    let mut lines = input_text.lines();
    let observed_header = lines
        .next()
        .ok_or_else(|| vec!["BDSIM userfile is empty".to_owned()])?;

    if observed_header.trim_end_matches('\r') != expected_header {
        return Err(vec![format!(
            "BDSIM header mismatch: expected {expected_header:?}, found {observed_header:?}"
        )]);
    }

    let stats = parse_records(lines, manifest)?;
    let expected_count = manifest
        .pointer("/input/expected_record_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| vec!["input.expected_record_count must be an unsigned integer".to_owned()])?;

    if stats.record_count as u64 != expected_count {
        return Err(vec![format!(
            "BDSIM particle record count mismatch: manifest expects {expected_count}, parsed {}",
            stats.record_count
        )]);
    }

    let input_sha256 = sha256_hex(input_bytes);
    let stats_value = stats_to_value(&stats);

    let report = json!({
        "audit_version": env!("CARGO_PKG_VERSION"),
        "audit_id": manifest["audit_id"],
        "status": "passed",
        "evidence_class": "upstream_native_input_distribution",
        "source_identity": {
            "repository": manifest["upstream"]["repository"],
            "ref": manifest["upstream"]["ref"],
            "commit": manifest["upstream"]["commit"],
            "path": manifest["input"]["path"],
            "git_blob_sha1": actual_blob,
            "sha256": input_sha256,
            "byte_length": input_bytes.len()
        },
        "beam_declaration": {
            "particle": manifest["bdsim_declaration"]["particle"],
            "energy": manifest["bdsim_declaration"]["energy"],
            "distribution_type": manifest["bdsim_declaration"]["distr_type"],
            "native_format": manifest["bdsim_declaration"]["distr_file_format"],
            "header_lines_ignored": manifest["bdsim_declaration"]["nlines_ignore"]
        },
        "native_distribution_summary": stats_value,
        "model_context": {
            "bdsim_lattice": manifest["bdsim_lattice"],
            "g4bl_baseline": manifest["g4bl_baseline"],
            "g4bl_run_log": manifest["g4bl_run_log"],
            "cross_code_equivalence": "not_established"
        },
        "claim_status": {
            "input_data_valid": true,
            "source_identity_valid": true,
            "native_coordinates_only": true,
            "canonical_momenta_emitted": false,
            "normalized_emittance_emitted": false,
            "explicit_particle_ids_available": false,
            "explicit_weights_available": false,
            "ionization_cooling_qualification": "not_evaluated",
            "rf_acceleration_qualification": "not_evaluated",
            "muon_survival_qualification": "not_evaluated",
            "claim_ceiling": manifest["claim_ceiling"]
        }
    });

    Ok(report)
}

fn validate_manifest(manifest: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if manifest.get("schema_version").and_then(Value::as_str) != Some("0.1.0") {
        errors.push("unsupported source manifest schema_version".to_owned());
    }
    if manifest.get("evidence_class").and_then(Value::as_str)
        != Some("upstream_native_input_distribution")
    {
        errors.push("source manifest must declare upstream_native_input_distribution".to_owned());
    }
    if manifest.pointer("/upstream/repository").and_then(Value::as_str)
        != Some("MuonCollider-WG4/cooling_demonstrator")
    {
        errors.push("unexpected upstream repository".to_owned());
    }
    if manifest.pointer("/upstream/ref").and_then(Value::as_str)
        != Some("2026-11-09-prerelease")
    {
        errors.push("unexpected upstream ref; pin the 2026-11-09 prerelease snapshot".to_owned());
    }
    if manifest.pointer("/upstream/commit").and_then(Value::as_str)
        != Some("d453b0ce0efd018ffb8d32927ac74cb597a02d26")
    {
        errors.push("unexpected upstream commit; update the manifest only through a reviewed source refresh".to_owned());
    }
    if manifest.pointer("/input/path").and_then(Value::as_str)
        != Some("cooling/bdsim/lowemittancebeam.dat")
    {
        errors.push("unexpected native input path".to_owned());
    }
    if manifest
        .pointer("/input/expected_record_count")
        .and_then(Value::as_u64)
        .is_none_or(|count| count == 0)
    {
        errors.push("input.expected_record_count must be a positive integer".to_owned());
    }
    if manifest.pointer("/input/expected_header").and_then(Value::as_str)
        != Some("\"x [mm]\" \"xp [rad]\" \"y [mm]\" \"yp [rad]\" \"z [mm]\" \"Ek [MeV]\" \"t [ns]\"")
    {
        errors.push("input.expected_header does not match the pinned BDSIM userfile header".to_owned());
    }
    if manifest.pointer("/bdsim_declaration/distr_type").and_then(Value::as_str)
        != Some("userfile")
        || manifest.pointer("/bdsim_declaration/distr_file").and_then(Value::as_str)
            != Some("lowemittancebeam.dat")
        || manifest.pointer("/bdsim_declaration/distr_file_format").and_then(Value::as_str)
            != Some("x[mm]:xp[rad]:y[mm]:yp[rad]:z[mm]:Ek[MeV]:t[ns]")
    {
        errors.push("BDSIM declaration must preserve the pinned userfile, file name and native units".to_owned());
    }
    if manifest.pointer("/bdsim_declaration/particle").and_then(Value::as_str) != Some("mu-") {
        errors.push("the BDSIM beam declaration must identify mu-".to_owned());
    }
    if manifest.pointer("/bdsim_declaration/nlines_ignore").and_then(Value::as_u64) != Some(1) {
        errors.push("the BDSIM userfile declaration must ignore exactly one header line".to_owned());
    }
    if manifest.pointer("/bdsim_declaration/path").and_then(Value::as_str)
        != Some("cooling/bdsim/lowemittancebeam.gmad")
        || manifest.pointer("/bdsim_declaration/git_blob_sha1").and_then(Value::as_str)
            != Some("f5ff777ee61ec8985a03f5439b272903917c8e8e")
    {
        errors.push("BDSIM userfile declaration identity does not match the pinned upstream object".to_owned());
    }

    let columns = manifest
        .pointer("/input/columns")
        .and_then(Value::as_array);
    match columns {
        Some(columns) if columns.len() == COLUMN_COUNT => {
            for (index, column) in columns.iter().enumerate() {
                let name = column.get("name").and_then(Value::as_str);
                let unit = column.get("unit").and_then(Value::as_str);
                if name != Some(MOMENT_NAMES[index]) {
                    errors.push(format!(
                        "column {index} must be {} in the pinned BDSIM coordinate order",
                        MOMENT_NAMES[index]
                    ));
                }
                if unit != Some(MOMENT_UNITS[index]) {
                    errors.push(format!(
                        "column {} must use native unit {}",
                        MOMENT_NAMES[index],
                        MOMENT_UNITS[index]
                    ));
                }
            }
        }
        Some(columns) => errors.push(format!(
            "input.columns must contain exactly {COLUMN_COUNT} declared columns; found {}",
            columns.len()
        )),
        None => errors.push("input.columns must be an array".to_owned()),
    }

    if manifest.pointer("/input/explicit_particle_ids").and_then(Value::as_bool) != Some(false) {
        errors.push("the pinned native userfile does not contain explicit particle IDs".to_owned());
    }
    if manifest.pointer("/input/explicit_weights").and_then(Value::as_bool) != Some(false) {
        errors.push("the pinned native userfile does not contain an explicit weight column".to_owned());
    }

    let bdsim_lattice = manifest.get("bdsim_lattice");
    let has_no_cooling_hardware = bdsim_lattice.is_some_and(|model| {
        model.get("coil_count").and_then(Value::as_u64) == Some(102)
            && model.get("absorber_count").and_then(Value::as_u64) == Some(0)
            && model.get("rf_cavity_count").and_then(Value::as_u64) == Some(0)
            && model.get("ionization_cooling_claim_admissible").and_then(Value::as_bool)
                == Some(false)
            && model.get("rf_acceleration_claim_admissible").and_then(Value::as_bool)
                == Some(false)
    });
    if !has_no_cooling_hardware {
        errors.push(
            "BDSIM model metadata must preserve 102 coils, zero absorbers, zero RF cavities and non-admissible cooling/RF claims"
                .to_owned(),
        );
    }
    if manifest.pointer("/bdsim_lattice/path").and_then(Value::as_str)
        != Some("cooling/bdsim/solenoidsheetdemonstrator102.gmad")
        || manifest.pointer("/bdsim_lattice/git_blob_sha1").and_then(Value::as_str)
            != Some("1f564a733fcc63d46d0eff6ece5c89d2b93519c9")
    {
        errors.push("BDSIM lattice identity does not match the pinned upstream object".to_owned());
    }
    if manifest.pointer("/g4bl_baseline/path").and_then(Value::as_str)
        != Some("cooling/g4bl/cooling.g4bl")
        || manifest.pointer("/g4bl_baseline/git_blob_sha1").and_then(Value::as_str)
            != Some("dc0cb955ef657cdc1862b4b5c30e2fe415d4e377")
        || manifest.pointer("/g4bl_baseline/cell_count").and_then(Value::as_u64) != Some(60)
    {
        errors.push("G4Beamline baseline identity or source cell count does not match the pinned upstream object".to_owned());
    }

    if manifest.pointer("/g4bl_baseline/equivalent_to_bdsim_model").and_then(Value::as_bool)
        != Some(false)
    {
        errors.push("G4BL and BDSIM models must remain distinct until equivalence is established".to_owned());
    }
    if manifest.pointer("/g4bl_run_log/path").and_then(Value::as_str)
        != Some("cooling/g4bl/g4bl.log")
        || manifest.pointer("/g4bl_run_log/git_blob_sha1").and_then(Value::as_str)
            != Some("a69acc28ea77d9055685c05936615460f87059d9")
        || manifest.pointer("/g4bl_run_log/decay_process_disabled").and_then(Value::as_bool)
            != Some(true)
        || manifest.pointer("/g4bl_run_log/muon_survival_status").and_then(Value::as_str)
            != Some("not_evaluated")
    {
        errors.push("the G4BL source log must preserve decay-disabled and survival-not-evaluated semantics".to_owned());
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(())
}

fn parse_records<'a>(
    lines: impl Iterator<Item = &'a str>,
    manifest: &Value,
) -> Result<UserfileStats, Vec<String>> {
    let expected_count = manifest
        .pointer("/input/expected_record_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| vec!["input.expected_record_count must be an unsigned integer".to_owned()])?;

    let mut count = 0_usize;
    let mut minimum = [f64::INFINITY; COLUMN_COUNT];
    let mut maximum = [f64::NEG_INFINITY; COLUMN_COUNT];
    let mut sums = [0.0_f64; COLUMN_COUNT];
    let mut points: Vec<[f64; COLUMN_COUNT]> = Vec::with_capacity(expected_count as usize);

    for (line_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            return Err(vec![format!(
                "unexpected blank record at data line {}",
                line_index + 1
            )]);
        }
        if line.trim_start().starts_with('#') {
            return Err(vec![format!(
                "unexpected comment line at data line {}; comments are not declared by the pinned format",
                line_index + 1
            )]);
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != COLUMN_COUNT {
            return Err(vec![format!(
                "data line {} must contain exactly {COLUMN_COUNT} whitespace-separated values, found {}",
                line_index + 1,
                tokens.len()
            )]);
        }

        let mut row = [0.0_f64; COLUMN_COUNT];
        for column_index in 0..COLUMN_COUNT {
            let value = tokens[column_index].parse::<f64>().map_err(|error| {
                vec![format!(
                    "data line {} column {} is not numeric: {error}",
                    line_index + 1,
                    MOMENT_NAMES[column_index]
                )]
            })?;

            if !value.is_finite() {
                return Err(vec![format!(
                    "data line {} column {} is not finite",
                    line_index + 1,
                    MOMENT_NAMES[column_index]
                )]);
            }

            row[column_index] = value;
            minimum[column_index] = minimum[column_index].min(value);
            maximum[column_index] = maximum[column_index].max(value);
            sums[column_index] += value;
        }

        points.push(row);
        count += 1;
        if count as u64 > expected_count {
            return Err(vec![
                "input contains more records than the pinned manifest permits".to_owned(),
            ]);
        }
    }

    if count == 0 {
        return Err(vec!["BDSIM userfile contains no particle records".to_owned()]);
    }

    let mut means = [0.0_f64; COLUMN_COUNT];
    for index in 0..COLUMN_COUNT {
        means[index] = sums[index] / count as f64;
    }

    let mut covariance = [[0.0_f64; COLUMN_COUNT]; COLUMN_COUNT];
    for row in &points {
        for i in 0..COLUMN_COUNT {
            for j in 0..COLUMN_COUNT {
                covariance[i][j] += (row[i] - means[i]) * (row[j] - means[j]);
            }
        }
    }
    for row in &mut covariance {
        for value in row {
            *value /= count as f64;
        }
    }

    let mut columns = Vec::with_capacity(COLUMN_COUNT);
    for index in 0..COLUMN_COUNT {
        columns.push(ColumnStats {
            name: MOMENT_NAMES[index].to_owned(),
            unit: MOMENT_UNITS[index].to_owned(),
            mean: means[index],
            standard_deviation: covariance[index][index].sqrt(),
            minimum: minimum[index],
            maximum: maximum[index],
        });
    }

    let horizontal_native_emittance = native_emittance(
        covariance[0][0],
        covariance[0][1],
        covariance[1][1],
    )?;
    let vertical_native_emittance = native_emittance(
        covariance[2][2],
        covariance[2][3],
        covariance[3][3],
    )?;

    Ok(UserfileStats {
        record_count: count,
        columns,
        horizontal_native_emittance,
        vertical_native_emittance,
    })
}

fn native_emittance(
    position_variance: f64,
    position_angle_covariance: f64,
    angle_variance: f64,
) -> Result<NativeEmittance, Vec<String>> {
    let determinant = position_variance * angle_variance
        - position_angle_covariance * position_angle_covariance;
    if !determinant.is_finite() || determinant < -1.0e-12 {
        return Err(vec![
            "native 2D covariance determinant is negative or not finite".to_owned(),
        ]);
    }

    let position_std = position_variance.sqrt();
    let angle_std = angle_variance.sqrt();
    let correlation = if position_std > 0.0 && angle_std > 0.0 {
        position_angle_covariance / (position_std * angle_std)
    } else {
        0.0
    };

    Ok(NativeEmittance {
        value_mm_rad: determinant.max(0.0).sqrt(),
        covariance: [
            [position_variance, position_angle_covariance],
            [position_angle_covariance, angle_variance],
        ],
        correlation,
    })
}

fn stats_to_value(stats: &UserfileStats) -> Value {
    json!({
        "record_count": stats.record_count,
        "coordinate_representation": "native_bdsim_userfile",
        "columns": stats.columns.iter().map(|column| json!({
            "name": column.name,
            "unit": column.unit,
            "mean": column.mean,
            "population_standard_deviation": column.standard_deviation,
            "minimum": column.minimum,
            "maximum": column.maximum
        })).collect::<Vec<_>>(),
        "native_rms_covariance_determinants": {
            "horizontal_x_xp": {
                "value": stats.horizontal_native_emittance.value_mm_rad,
                "unit": "mm*rad",
                "covariance": stats.horizontal_native_emittance.covariance,
                "correlation": stats.horizontal_native_emittance.correlation,
                "label": "native coordinate RMS covariance determinant; not normalized emittance"
            },
            "vertical_y_yp": {
                "value": stats.vertical_native_emittance.value_mm_rad,
                "unit": "mm*rad",
                "covariance": stats.vertical_native_emittance.covariance,
                "correlation": stats.vertical_native_emittance.correlation,
                "label": "native coordinate RMS covariance determinant; not normalized emittance"
            }
        },
        "canonical_momenta_computed": false,
        "normalized_emittance_computed": false,
        "weights_inferred": false,
        "particle_ids_inferred": false
    })
}

fn git_blob_sha1(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", bytes.len()).as_bytes());
    hasher.update(bytes);
    hex_string(&hasher.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_string(&Sha256::digest(bytes))
}

fn hex_string(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn synthetic_manifest() -> Value {
        json!({
            "schema_version": "0.1.0",
            "audit_id": "synthetic-test",
            "evidence_class": "upstream_native_input_distribution",
            "upstream": {
                "repository": "MuonCollider-WG4/cooling_demonstrator",
                "ref": "2026-11-09-prerelease",
                "commit": "d453b0ce0efd018ffb8d32927ac74cb597a02d26"
            },
            "input": {
                "path": "cooling/bdsim/lowemittancebeam.dat",
                "git_blob_sha1": "REPLACED",
                "byte_length": 0,
                "ignored_header_lines": 1,
                "expected_record_count": 4,
                "expected_header": "\"x [mm]\" \"xp [rad]\" \"y [mm]\" \"yp [rad]\" \"z [mm]\" \"Ek [MeV]\" \"t [ns]\"",
                "columns": [
                    {"name":"x","unit":"mm"},
                    {"name":"xp","unit":"rad"},
                    {"name":"y","unit":"mm"},
                    {"name":"yp","unit":"rad"},
                    {"name":"z","unit":"mm"},
                    {"name":"Ek","unit":"MeV"},
                    {"name":"t","unit":"ns"}
                ],
                "explicit_particle_ids": false,
                "explicit_weights": false
            },
            "bdsim_declaration": {
                "particle":"mu-",
                "nlines_ignore":1
            },
            "bdsim_lattice": {
                "coil_count":102,
                "absorber_count":0,
                "rf_cavity_count":0,
                "ionization_cooling_claim_admissible":false,
                "rf_acceleration_claim_admissible":false
            },
            "g4bl_baseline": {"equivalent_to_bdsim_model":false},
            "g4bl_run_log": {
                "decay_process_disabled":true,
                "muon_survival_status":"not_evaluated"
            },
            "claim_ceiling":"native input audit only"
        })
    }

    fn fixture() -> Vec<u8> {
        b"\"x [mm]\" \"xp [rad]\" \"y [mm]\" \"yp [rad]\" \"z [mm]\" \"Ek [MeV]\" \"t [ns]\"\n\
          -1.0 -0.1 -2.0 -0.2 0.0 200.0 -1.0\n\
          1.0 0.1 -2.0 -0.2 0.0 200.0 -0.5\n\
          -1.0 -0.1 2.0 0.2 0.0 200.0 0.5\n\
          1.0 0.1 2.0 0.2 0.0 200.0 1.0\n"
            .to_vec()
    }

    fn manifest_for(bytes: &[u8]) -> Value {
        let mut manifest = synthetic_manifest();
        manifest["input"]["byte_length"] = json!(bytes.len());
        manifest["input"]["git_blob_sha1"] = json!(git_blob_sha1(bytes));
        manifest
    }

    #[test]
    fn valid_synthetic_userfile_parses_in_native_coordinates() {
        let bytes = fixture();
        let report = audit_bytes(&manifest_for(&bytes), &bytes).expect("valid fixture");
        assert_eq!(report["status"], "passed");
        assert_eq!(report["native_distribution_summary"]["record_count"], 4);
        assert_eq!(
            report["native_distribution_summary"]["coordinate_representation"],
            "native_bdsim_userfile"
        );
        assert_eq!(
            report["claim_status"]["canonical_momenta_emitted"],
            false
        );
        assert_eq!(
            report["claim_status"]["normalized_emittance_emitted"],
            false
        );
    }

    #[test]
    fn native_rms_emittance_oracle_is_recomputed() {
        let bytes = fixture();
        let report = audit_bytes(&manifest_for(&bytes), &bytes).expect("valid fixture");
        let horizontal = report["native_distribution_summary"]
            ["native_rms_covariance_determinants"]["horizontal_x_xp"]["value"]
            .as_f64()
            .expect("numeric horizontal emittance");
        assert!((horizontal - 0.1).abs() < 1.0e-12);
    }

    #[test]
    fn negative_control_changed_bytes_fail_blob_identity() {
        let bytes = fixture();
        let manifest = manifest_for(&bytes);
        let mut changed = bytes.clone();
        changed.push(b'\n');
        assert!(audit_bytes(&manifest, &changed).is_err());
    }

    #[test]
    fn negative_control_wrong_header_is_rejected() {
        let bytes = fixture();
        let mut changed = bytes.clone();
        let original = b"\"x [mm]\"";
        let replacement = b"\"xp [rad]\"";
        let content = String::from_utf8(changed.clone()).expect("fixture UTF-8");
        let updated = content.replacen(
            std::str::from_utf8(original).unwrap(),
            std::str::from_utf8(replacement).unwrap(),
            1,
        );
        changed = updated.into_bytes();
        assert!(audit_bytes(&manifest_for(&bytes), &changed).is_err());
    }

    #[test]
    fn negative_control_wrong_column_count_is_rejected() {
        let bytes = fixture();
        let changed = String::from_utf8(bytes.clone())
            .expect("fixture UTF-8")
            .replace("-1.0 -0.1 -2.0 -0.2 0.0 200.0 -1.0", "-1.0 -0.1 -2.0 -0.2 0.0 200.0");
        assert!(audit_bytes(&manifest_for(changed.as_bytes()), changed.as_bytes()).is_err());
    }

    #[test]
    fn negative_control_non_finite_data_is_rejected() {
        let bytes = fixture();
        let changed = String::from_utf8(bytes.clone())
            .expect("fixture UTF-8")
            .replace("-1.0 -0.1 -2.0", "NaN -0.1 -2.0");
        assert!(audit_bytes(&manifest_for(changed.as_bytes()), changed.as_bytes()).is_err());
    }

    #[test]
    fn negative_control_reordered_or_relabelled_units_are_rejected() {
        let bytes = fixture();
        let mut manifest = manifest_for(&bytes);
        manifest["input"]["columns"][1]["unit"] = json!("mrad");
        assert!(audit_bytes(&manifest, &bytes).is_err());
        manifest = manifest_for(&bytes);
        manifest["input"]["columns"][1]["name"] = json!("py");
        assert!(audit_bytes(&manifest, &bytes).is_err());
    }

    #[test]
    fn negative_control_bdsim_cooling_claim_cannot_be_promoted() {
        let bytes = fixture();
        let mut manifest = manifest_for(&bytes);
        manifest["bdsim_lattice"]["absorber_count"] = json!(1);
        assert!(audit_bytes(&manifest, &bytes).is_err());
        manifest = manifest_for(&bytes);
        manifest["bdsim_lattice"]["ionization_cooling_claim_admissible"] = json!(true);
        assert!(audit_bytes(&manifest, &bytes).is_err());
    }

    #[test]
    fn negative_control_cross_code_equivalence_cannot_be_assumed() {
        let bytes = fixture();
        let mut manifest = manifest_for(&bytes);
        manifest["g4bl_baseline"]["equivalent_to_bdsim_model"] = json!(true);
        assert!(audit_bytes(&manifest, &bytes).is_err());
    }

    #[test]
    fn negative_control_decay_disabled_log_cannot_qualify_survival() {
        let bytes = fixture();
        let mut manifest = manifest_for(&bytes);
        manifest["g4bl_run_log"]["muon_survival_status"] = json!("qualified");
        assert!(audit_bytes(&manifest, &bytes).is_err());
    }
}
