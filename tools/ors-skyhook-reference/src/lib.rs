//! Analytical reference for the classical orbital-ring skyhook turn model.
//!
//! This crate reproduces Birch (1982), Eqs. 35, 44, 53 and 54, and the
//! rounded Table 4 values. It is not a dynamic orbital-ring, electromagnetic,
//! structural, stability, control, or flight-qualification model.

use std::collections::HashSet;
use std::f64::consts::PI;

pub const SOURCE_EARTH_RADIUS_M: f64 = 6_400_000.0;
pub const SOURCE_SURFACE_GRAVITY_M_S2: f64 = 9.81;
pub const SOURCE_RING_LINE_DENSITY_KG_M: f64 = 2.5 * PI * 1_000.0;
pub const TABLE_RELATIVE_TOLERANCE: f64 = 0.02;
pub const EXPECTED_CASE_COUNT: usize = 36;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureStatus {
    TableReproduced,
    OutsideSourceDomain,
}

impl FixtureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TableReproduced => "table_reproduced",
            Self::OutsideSourceDomain => "outside_source_domain",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fixture {
    pub case_id: String,
    pub height_m: u64,
    pub delta_h_m: u64,
    pub pi_over_alpha: u32,
    pub table_scale_n: f64,
    pub table_value: Option<f64>,
    pub expected_status: FixtureStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaseResult {
    pub case_id: String,
    pub status: FixtureStatus,
    pub orbital_speed_mps: Option<f64>,
    pub turn_angle_rad: f64,
    pub turn_angle_limit_rad: f64,
    pub predicted_force_n: Option<f64>,
    pub table_force_n: Option<f64>,
    pub relative_error: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub cases: Vec<CaseResult>,
    pub table_reproduced_count: usize,
    pub outside_source_domain_count: usize,
    pub max_relative_error: f64,
}

fn parse_number<T: std::str::FromStr>(value: &str, at: &str) -> Result<T, String> {
    value
        .parse::<T>()
        .map_err(|_| format!("{at}: invalid numeric value {value:?}"))
}

fn valid_case_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

pub fn parse_fixtures(input: &str) -> Result<Vec<Fixture>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "empty skyhook corpus".to_owned())?;
    if header != "case_id,height_m,delta_h_m,pi_over_alpha,table_scale_n,table_value,status" {
        return Err("unexpected skyhook corpus header".to_owned());
    }

    let mut fixtures = Vec::new();
    let mut ids = HashSet::new();
    let mut combinations = HashSet::new();

    for (line_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!(
                "line {}: blank rows are not admitted",
                line_index + 2
            ));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 7 {
            return Err(format!("line {}: expected seven columns", line_index + 2));
        }
        let case_id = fields[0].to_owned();
        if !valid_case_id(&case_id) || !ids.insert(case_id.clone()) {
            return Err(format!(
                "line {}: invalid or duplicate case ID",
                line_index + 2
            ));
        }
        let height_m: u64 = parse_number(fields[1], "height_m")?;
        let delta_h_m: u64 = parse_number(fields[2], "delta_h_m")?;
        let pi_over_alpha: u32 = parse_number(fields[3], "pi_over_alpha")?;
        let expected_scale_text = match delta_h_m {
            3_000 => "1000000000",
            30_000 => "10000000000",
            300_000 => "100000000000",
            _ => return Err("unsupported Table 4 DeltaH column".to_owned()),
        };
        if fields[4] != expected_scale_text {
            return Err(format!(
                "line {}: Table 4 force scale does not match the DeltaH column",
                line_index + 2
            ));
        }
        let table_scale_n: f64 = parse_number(fields[4], "table_scale_n")?;
        let table_value: Option<f64> = if fields[5].is_empty() {
            None
        } else {
            Some(parse_number(fields[5], "table_value")?)
        };
        let expected_status = match fields[6] {
            "table_reproduced" => FixtureStatus::TableReproduced,
            "outside_source_domain" => FixtureStatus::OutsideSourceDomain,
            other => {
                return Err(format!(
                    "line {}: unsupported expected status {other:?}",
                    line_index + 2
                ));
            }
        };

        if height_m == 0 || delta_h_m == 0 || !table_scale_n.is_finite() || table_scale_n <= 0.0 {
            return Err(format!(
                "line {}: non-positive or non-finite input",
                line_index + 2
            ));
        }
        if ![300_000, 600_000].contains(&height_m)
            || ![3_000, 30_000, 300_000].contains(&delta_h_m)
            || ![2, 4, 8, 16, 32, 64].contains(&pi_over_alpha)
        {
            return Err(format!(
                "line {}: fixture is outside the frozen Table 4 grid",
                line_index + 2
            ));
        }
        if !combinations.insert((height_m, delta_h_m, pi_over_alpha)) {
            return Err(format!("line {}: duplicate geometry tuple", line_index + 2));
        }
        match (expected_status, table_value) {
            (FixtureStatus::TableReproduced, Some(value)) if value.is_finite() && value > 0.0 => {}
            (FixtureStatus::OutsideSourceDomain, None) => {}
            _ => {
                return Err(format!(
                    "line {}: source table value and expected status disagree",
                    line_index + 2
                ));
            }
        }

        fixtures.push(Fixture {
            case_id,
            height_m,
            delta_h_m,
            pi_over_alpha,
            table_scale_n,
            table_value,
            expected_status,
        });
    }

    if fixtures.len() != EXPECTED_CASE_COUNT || combinations.len() != EXPECTED_CASE_COUNT {
        return Err(format!(
            "expected exactly {EXPECTED_CASE_COUNT} unique Table 4 cases; found {}",
            fixtures.len()
        ));
    }

    Ok(fixtures)
}

/// Evaluate one frozen case using Birch's Eqs. 44, 53 and 35.
pub fn evaluate_fixture(fixture: &Fixture) -> Result<CaseResult, String> {
    let height_m = fixture.height_m as f64;
    let delta_h_m = fixture.delta_h_m as f64;
    let alpha = PI / f64::from(fixture.pi_over_alpha);
    let radius_plus_height_m = SOURCE_EARTH_RADIUS_M + height_m;

    // Eq. 44: small-angle change of track at the skyhook.
    let turn_angle_rad = 4.0 * delta_h_m / (alpha * radius_plus_height_m);
    let turn_angle_limit_rad = 2.0 * alpha;

    // Eq. 49 supplies the source model's geometric limit, Δθ ≤ 2α.
    let within_turn_limit = turn_angle_rad <= turn_angle_limit_rad * (1.0 + 1.0e-12);

    // Eq. 53: orbital speed at the skyhook for the approximated conic.
    let speed_denominator_m = radius_plus_height_m + delta_h_m * (1.0 - 2.0 / alpha.powi(2));
    let within_speed_domain = speed_denominator_m.is_finite() && speed_denominator_m > 0.0;

    if !within_turn_limit || !within_speed_domain {
        if fixture.expected_status != FixtureStatus::OutsideSourceDomain {
            return Err(format!(
                "{}: source model rejects a case marked reproducible",
                fixture.case_id
            ));
        }
        return Ok(CaseResult {
            case_id: fixture.case_id.clone(),
            status: FixtureStatus::OutsideSourceDomain,
            orbital_speed_mps: None,
            turn_angle_rad,
            turn_angle_limit_rad,
            predicted_force_n: None,
            table_force_n: None,
            relative_error: None,
        });
    }

    let speed_squared_m2_s2 =
        SOURCE_SURFACE_GRAVITY_M_S2 * SOURCE_EARTH_RADIUS_M.powi(2) / speed_denominator_m;
    if !speed_squared_m2_s2.is_finite() || speed_squared_m2_s2 <= 0.0 {
        return Err(format!(
            "{}: non-finite or non-positive orbital speed",
            fixture.case_id
        ));
    }
    let orbital_speed_mps = speed_squared_m2_s2.sqrt();

    // Eq. 35: mass flow (m V0) multiplied by the velocity-vector change
    // magnitude (V0 Δθ) over the skyhook's idealized turning interval.
    let force_from_turn_n = SOURCE_RING_LINE_DENSITY_KG_M * speed_squared_m2_s2 * turn_angle_rad;

    // Eq. 54: algebraically equivalent direct expression. Keeping this
    // second path makes accidental unit or transcription changes visible.
    let force_from_eq54_n = 4.0
        * SOURCE_RING_LINE_DENSITY_KG_M
        * SOURCE_SURFACE_GRAVITY_M_S2
        * SOURCE_EARTH_RADIUS_M.powi(2)
        * delta_h_m
        / (alpha * radius_plus_height_m * speed_denominator_m);

    let algebraic_residual =
        (force_from_turn_n - force_from_eq54_n).abs() / force_from_eq54_n.abs().max(1.0);
    if !algebraic_residual.is_finite() || algebraic_residual > 1.0e-12 {
        return Err(format!(
            "{}: Eq. 35 / Eq. 54 algebraic closure failed",
            fixture.case_id
        ));
    }

    if fixture.expected_status != FixtureStatus::TableReproduced {
        return Err(format!(
            "{}: source model accepts a case marked outside its domain",
            fixture.case_id
        ));
    }
    let table_value = fixture
        .table_value
        .expect("validated fixture has Table 4 value");
    let table_force_n = table_value * fixture.table_scale_n;
    let relative_error = (force_from_turn_n - table_force_n).abs() / table_force_n.abs();
    if !relative_error.is_finite() || relative_error > TABLE_RELATIVE_TOLERANCE {
        return Err(format!(
            "{}: Eq. 35 result differs from rounded Table 4 by {:.3}% (limit {:.1}%)",
            fixture.case_id,
            relative_error * 100.0,
            TABLE_RELATIVE_TOLERANCE * 100.0
        ));
    }

    Ok(CaseResult {
        case_id: fixture.case_id.clone(),
        status: FixtureStatus::TableReproduced,
        orbital_speed_mps: Some(orbital_speed_mps),
        turn_angle_rad,
        turn_angle_limit_rad,
        predicted_force_n: Some(force_from_turn_n),
        table_force_n: Some(table_force_n),
        relative_error: Some(relative_error),
    })
}

pub fn evaluate_corpus(input: &str) -> Result<EvaluationReport, String> {
    let fixtures = parse_fixtures(input)?;
    let mut cases = Vec::with_capacity(fixtures.len());
    let mut table_reproduced_count = 0;
    let mut outside_source_domain_count = 0;
    let mut max_relative_error: f64 = 0.0;

    for fixture in &fixtures {
        let result = evaluate_fixture(fixture)?;
        match result.status {
            FixtureStatus::TableReproduced => {
                table_reproduced_count += 1;
                max_relative_error = max_relative_error.max(result.relative_error.unwrap_or(0.0));
            }
            FixtureStatus::OutsideSourceDomain => outside_source_domain_count += 1,
        }
        cases.push(result);
    }

    if table_reproduced_count != 28 || outside_source_domain_count != 8 {
        return Err(format!(
            "unexpected Table 4 disposition: {} reproduced, {} outside source domain",
            table_reproduced_count, outside_source_domain_count
        ));
    }

    Ok(EvaluationReport {
        cases,
        table_reproduced_count,
        outside_source_domain_count,
        max_relative_error,
    })
}
