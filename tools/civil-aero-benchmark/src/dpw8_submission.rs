//! Strict parser and reconciler for DPW-8/AePW-4 force/moment V5 submissions.
//!
//! This is an ingestion component, not source authentication, a CFD solver, or aerodynamic
//! validation. It preserves dataset/zone metadata and one-based source lines. Optional -999
//! values become None; required values may not use the sentinel. The audit reconciles the
//! grid-level and angle-level views and never repairs suspicious results.

use std::collections::{BTreeMap, HashSet};

pub const V5_COLUMNS: [&str; 31] = [
    "GRID_LEVEL", "GRID_SIZE", "GRID_FAC", "MACH", "REY", "ALPHA", "CL_TOT",
    "CD_TOT", "CM_TOT", "CL_WING", "CD_WING", "CM_WING", "CD_PR", "CD_SF",
    "CL_TAIL", "CD_TAIL", "CM_TAIL", "CL_FUS", "CD_FUS", "CM_FUS", "CL_NAC",
    "CD_NAC", "CM_NAC", "CL_PY", "CD_PY", "CM_PY", "CPU_Hours", "DELTAT",
    "CTUSTART", "CTUAVG", "Q/E",
];

pub const V5_CSV_HEADER: &str = "grid_level,grid_size,alpha_deg,cl_total,cd_total,cm_total,grid_view_source_line,alpha_view_source_line,missing_optional_count,duplicate_cl_cd_at_other_alpha,adjacent_grid_discontinuity,anomaly_warning";

const REQUIRED_COLUMN_COUNT: usize = 9;
const ALPHA_ZONE_TOLERANCE: f64 = 1.0e-8;
const DUPLICATE_SIGNATURE_TOLERANCE: f64 = 1.0e-7;
const LARGE_CL_JUMP: f64 = 0.05;
const LARGE_CD_JUMP: f64 = 0.005;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dpw8V5View {
    GridLevel,
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dpw8V5ZoneKind {
    GridLevel(u32),
    Alpha(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpw8V5Zone {
    pub title: String,
    pub kind: Dpw8V5ZoneKind,
    pub header_line: usize,
    pub auxiliary: BTreeMap<String, String>,
    pub rows: Vec<Dpw8V5RawRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpw8V5Dataset {
    pub title: String,
    pub variables: Vec<String>,
    pub dataset_auxiliary: BTreeMap<String, String>,
    pub zones: Vec<Dpw8V5Zone>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpw8V5RawRow {
    pub source_line: usize,
    pub zone_title: String,
    pub view: Dpw8V5View,
    pub grid_level: u32,
    pub grid_size: u64,
    pub alpha_deg: f64,
    /// Required fields are Some. Optional -999 sentinel fields are None, never zero.
    pub values: Vec<Option<f64>>,
}

impl Dpw8V5RawRow {
    fn required(&self, index: usize) -> f64 {
        self.values[index].expect("required V5 columns were checked during parsing")
    }

    pub fn cl_total(&self) -> f64 {
        self.required(6)
    }

    pub fn cd_total(&self) -> f64 {
        self.required(7)
    }

    pub fn cm_total(&self) -> f64 {
        self.required(8)
    }

    pub fn missing_optional_count(&self) -> usize {
        self.values[REQUIRED_COLUMN_COUNT..]
            .iter()
            .filter(|value| value.is_none())
            .count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpw8V5CanonicalCell {
    pub row: Dpw8V5RawRow,
    pub grid_view_source_line: usize,
    pub alpha_view_source_line: usize,
    pub duplicate_cl_cd_at_other_alpha: bool,
    pub adjacent_grid_discontinuity: bool,
    pub anomaly_warning: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dpw8V5AuditReport {
    pub raw_record_count: usize,
    pub grid_view_record_count: usize,
    pub alpha_view_record_count: usize,
    pub canonical_cells: Vec<Dpw8V5CanonicalCell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RowKey {
    grid_level: u32,
    alpha_bits: u64,
}

/// Parse the subset of Tecplot ASCII used by the DPW-8/AePW-4 Force/Moment V5 template.
pub fn parse_v5(input: &str) -> Result<Dpw8V5Dataset, String> {
    let mut title: Option<String> = None;
    let mut variables: Option<Vec<String>> = None;
    let mut dataset_auxiliary = BTreeMap::new();
    let mut zones: Vec<Dpw8V5Zone> = Vec::new();
    let mut zone_titles = HashSet::new();
    let mut active_zone: Option<usize> = None;

    for (zero_line, raw_line) in input.lines().enumerate() {
        let line_number = zero_line + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || is_separator(line) {
            continue;
        }

        if line.starts_with("TITLE") {
            if title.is_some() || !zones.is_empty() {
                return Err(format!("line {line_number}: duplicate or misplaced TITLE"));
            }
            let (_, value) = parse_assignment(line, "TITLE", line_number)?;
            title = Some(value);
            continue;
        }

        if line.starts_with("VARIABLES") {
            if variables.is_some() || !zones.is_empty() {
                return Err(format!("line {line_number}: duplicate or misplaced VARIABLES"));
            }
            variables = Some(parse_variables(line, line_number)?);
            continue;
        }

        if line.starts_with("DATASETAUXDATA") {
            if active_zone.is_some() {
                return Err(format!(
                    "line {line_number}: DATASETAUXDATA must precede all ZONE blocks"
                ));
            }
            let (key, value) = parse_assignment(line, "DATASETAUXDATA", line_number)?;
            insert_unique(
                &mut dataset_auxiliary,
                key,
                value,
                "dataset auxiliary key",
                line_number,
            )?;
            continue;
        }

        if line.starts_with("ZONE") {
            let variable_names = variables
                .as_ref()
                .ok_or_else(|| format!("line {line_number}: ZONE appears before VARIABLES"))?;
            if variable_names.len() != V5_COLUMNS.len() {
                return Err(format!(
                    "line {line_number}: expected {} V5 columns, found {}",
                    V5_COLUMNS.len(),
                    variable_names.len()
                ));
            }
            let (key, zone_title) = parse_assignment(line, "ZONE", line_number)?;
            if key != "T" {
                return Err(format!(
                    "line {line_number}: expected ZONE T= quoted title, found key {key}"
                ));
            }
            if !zone_titles.insert(zone_title.clone()) {
                return Err(format!("line {line_number}: duplicate zone title {zone_title}"));
            }
            let kind = parse_zone_kind(&zone_title)
                .ok_or_else(|| format!("line {line_number}: unsupported V5 zone title {zone_title}"))?;
            zones.push(Dpw8V5Zone {
                title: zone_title,
                kind,
                header_line: line_number,
                auxiliary: BTreeMap::new(),
                rows: Vec::new(),
            });
            active_zone = Some(zones.len() - 1);
            continue;
        }

        if line.starts_with("AUXDATA") {
            let zone_index = active_zone
                .ok_or_else(|| format!("line {line_number}: AUXDATA appears before a ZONE"))?;
            let (key, value) = parse_assignment(line, "AUXDATA", line_number)?;
            insert_unique(
                &mut zones[zone_index].auxiliary,
                key,
                value,
                "zone auxiliary key",
                line_number,
            )?;
            continue;
        }

        let zone_index = active_zone
            .ok_or_else(|| format!("line {line_number}: numeric data appears before a ZONE"))?;
        let zone = &zones[zone_index];
        let variable_names = variables
            .as_ref()
            .ok_or_else(|| format!("line {line_number}: VARIABLES header is missing"))?;
        let row = parse_numeric_row(line, variable_names.len(), &zone.title, zone.kind, line_number)?;
        zones[zone_index].rows.push(row);
    }

    let title = title.ok_or_else(|| "V5 TITLE header is missing".to_owned())?;
    let variables = variables.ok_or_else(|| "V5 VARIABLES header is missing".to_owned())?;
    if variables
        != V5_COLUMNS
            .iter()
            .map(|column| (*column).to_owned())
            .collect::<Vec<_>>()
    {
        return Err("V5 VARIABLES do not match the 31-column Force/Moment schema".to_owned());
    }
    if zones.is_empty() {
        return Err("V5 submission contains no zones".to_owned());
    }
    for zone in &zones {
        if zone.rows.is_empty() {
            return Err(format!(
                "zone {} at line {} contains no numeric rows",
                zone.title, zone.header_line
            ));
        }
    }

    let has_grid_view = zones
        .iter()
        .any(|zone| matches!(zone.kind, Dpw8V5ZoneKind::GridLevel(_)));
    let has_alpha_view = zones
        .iter()
        .any(|zone| matches!(zone.kind, Dpw8V5ZoneKind::Alpha(_)));
    if !has_grid_view || !has_alpha_view {
        return Err("V5 submission must contain both grid-level and alpha zone views".to_owned());
    }

    Ok(Dpw8V5Dataset {
        title,
        variables,
        dataset_auxiliary,
        zones,
    })
}

/// Reconcile both V5 table views. A mismatch or duplicate in either view is fatal.
pub fn audit_v5(dataset: &Dpw8V5Dataset) -> Result<Dpw8V5AuditReport, String> {
    let mut cells: BTreeMap<RowKey, (Option<Dpw8V5RawRow>, Option<Dpw8V5RawRow>)> =
        BTreeMap::new();
    let mut raw_record_count = 0;
    let mut grid_view_record_count = 0;
    let mut alpha_view_record_count = 0;

    for zone in &dataset.zones {
        for row in &zone.rows {
            raw_record_count += 1;
            let key = RowKey {
                grid_level: row.grid_level,
                alpha_bits: row.alpha_deg.to_bits(),
            };
            let entry = cells.entry(key).or_insert((None, None));
            let target = match row.view {
                Dpw8V5View::GridLevel => {
                    grid_view_record_count += 1;
                    &mut entry.0
                }
                Dpw8V5View::Alpha => {
                    alpha_view_record_count += 1;
                    &mut entry.1
                }
            };
            if target.is_some() {
                return Err(format!(
                    "duplicate cell in {:?} view: grid_level={} alpha={} line={}",
                    row.view, row.grid_level, row.alpha_deg, row.source_line
                ));
            }
            *target = Some(row.clone());
        }
    }

    if grid_view_record_count == 0 || alpha_view_record_count == 0 {
        return Err("both grid-level and alpha views must contain records".to_owned());
    }

    let mut canonical_cells = Vec::with_capacity(cells.len());
    for ((grid_level, alpha_bits), (grid_row, alpha_row)) in cells {
        let alpha_deg = f64::from_bits(alpha_bits);
        let grid_row = grid_row.ok_or_else(|| {
            format!("cell missing from grid-level view: grid_level={grid_level} alpha={alpha_deg}")
        })?;
        let alpha_row = alpha_row.ok_or_else(|| {
            format!("cell missing from alpha view: grid_level={grid_level} alpha={alpha_deg}")
        })?;
        if grid_row.values != alpha_row.values {
            return Err(format!(
                "cross-view mismatch for grid_level={} alpha={}: grid-view line {} differs from alpha-view line {}",
                grid_row.grid_level,
                grid_row.alpha_deg,
                grid_row.source_line,
                alpha_row.source_line
            ));
        }
        canonical_cells.push(Dpw8V5CanonicalCell {
            row: grid_row.clone(),
            grid_view_source_line: grid_row.source_line,
            alpha_view_source_line: alpha_row.source_line,
            duplicate_cl_cd_at_other_alpha: false,
            adjacent_grid_discontinuity: false,
            anomaly_warning: false,
        });
    }

    canonical_cells.sort_by(|left, right| {
        left.row
            .grid_level
            .cmp(&right.row.grid_level)
            .then_with(|| left.row.alpha_deg.total_cmp(&right.row.alpha_deg))
    });

    for index in 0..canonical_cells.len() {
        let row = &canonical_cells[index].row;
        let duplicate_signature = canonical_cells.iter().any(|other| {
            other.row.grid_level == row.grid_level
                && (other.row.alpha_deg - row.alpha_deg).abs() > 0.25
                && (other.row.cl_total() - row.cl_total()).abs()
                    <= DUPLICATE_SIGNATURE_TOLERANCE
                && (other.row.cd_total() - row.cd_total()).abs()
                    <= DUPLICATE_SIGNATURE_TOLERANCE
        });
        let adjacent_jump = canonical_cells.iter().any(|other| {
            other.row.alpha_deg.to_bits() == row.alpha_deg.to_bits()
                && other.row.grid_level.abs_diff(row.grid_level) == 1
                && (other.row.cl_total() - row.cl_total()).abs() > LARGE_CL_JUMP
                && (other.row.cd_total() - row.cd_total()).abs() > LARGE_CD_JUMP
        });
        canonical_cells[index].duplicate_cl_cd_at_other_alpha = duplicate_signature;
        canonical_cells[index].adjacent_grid_discontinuity = adjacent_jump;
        canonical_cells[index].anomaly_warning = duplicate_signature && adjacent_jump;
    }

    Ok(Dpw8V5AuditReport {
        raw_record_count,
        grid_view_record_count,
        alpha_view_record_count,
        canonical_cells,
    })
}

fn is_separator(line: &str) -> bool {
    line.starts_with('+')
        && line
            .chars()
            .skip(1)
            .all(|character| character == '-' || character == '+')
}

fn parse_assignment(
    line: &str,
    prefix: &str,
    line_number: usize,
) -> Result<(String, String), String> {
    let remainder = line
        .strip_prefix(prefix)
        .ok_or_else(|| format!("line {line_number}: expected {prefix}"))?;
    let (key, raw_value) = remainder
        .split_once('=')
        .ok_or_else(|| format!("line {line_number}: {prefix} assignment is missing '='"))?;
    let value = parse_quoted(raw_value, line_number)?;
    Ok((key.trim().to_owned(), value))
}

fn parse_quoted(raw: &str, line_number: usize) -> Result<String, String> {
    let value = raw.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .ok_or_else(|| format!("line {line_number}: expected a double-quoted value"))?;
    if value.contains('"') {
        return Err(format!("line {line_number}: embedded quote is unsupported"));
    }
    Ok(value.to_owned())
}

fn parse_variables(line: &str, line_number: usize) -> Result<Vec<String>, String> {
    let remainder = line
        .strip_prefix("VARIABLES")
        .ok_or_else(|| format!("line {line_number}: malformed VARIABLES header"))?
        .trim_start();
    let mut remainder = remainder
        .strip_prefix('=')
        .ok_or_else(|| format!("line {line_number}: VARIABLES header is missing '='"))?
        .trim();
    let mut names = Vec::new();
    while !remainder.is_empty() {
        remainder = remainder.trim_start();
        let quoted = remainder
            .strip_prefix('"')
            .ok_or_else(|| format!("line {line_number}: variable names must be quoted"))?;
        let end = quoted
            .find('"')
            .ok_or_else(|| format!("line {line_number}: unterminated variable name"))?;
        let name = &quoted[..end];
        if name.is_empty() {
            return Err(format!("line {line_number}: empty variable name"));
        }
        names.push(name.to_owned());
        remainder = &quoted[end + 1..];
        if remainder.trim().is_empty() {
            break;
        }
        if !remainder.trim_start().starts_with('"') {
            return Err(format!("line {line_number}: unexpected text after VARIABLES"));
        }
    }
    let mut unique = HashSet::new();
    if names.iter().any(|name| !unique.insert(name.clone())) {
        return Err(format!("line {line_number}: duplicate variable name"));
    }
    Ok(names)
}

fn parse_zone_kind(title: &str) -> Option<Dpw8V5ZoneKind> {
    let upper = title.to_ascii_uppercase();
    if let Some(index) = upper.rfind("GRID LEVEL ") {
        let value = title[index + "GRID LEVEL ".len()..].trim().parse::<u32>().ok()?;
        return (value > 0).then_some(Dpw8V5ZoneKind::GridLevel(value));
    }
    if let Some(index) = upper.rfind("ALPHA ") {
        let value = title[index + "ALPHA ".len()..].trim().parse::<f64>().ok()?;
        return value.is_finite().then_some(Dpw8V5ZoneKind::Alpha(value));
    }
    None
}

fn parse_numeric_row(
    line: &str,
    column_count: usize,
    zone_title: &str,
    zone_kind: Dpw8V5ZoneKind,
    line_number: usize,
) -> Result<Dpw8V5RawRow, String> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    if fields.len() != column_count {
        return Err(format!(
            "line {line_number}: expected {column_count} numeric fields, found {}",
            fields.len()
        ));
    }
    let mut values = Vec::with_capacity(column_count);
    for (index, field) in fields.iter().enumerate() {
        let value = field.parse::<f64>().map_err(|error| {
            format!("line {line_number}: column {} is not numeric: {error}", index + 1)
        })?;
        if !value.is_finite() {
            return Err(format!(
                "line {line_number}: column {} is non-finite",
                index + 1
            ));
        }
        if index < REQUIRED_COLUMN_COUNT && value == -999.0 {
            return Err(format!(
                "line {line_number}: required column {} uses optional -999 sentinel",
                V5_COLUMNS[index]
            ));
        }
        values.push(if index >= REQUIRED_COLUMN_COUNT && value == -999.0 {
            None
        } else {
            Some(value)
        });
    }

    let grid_level_value = integral_value(
        values[0].expect("required field"),
        "GRID_LEVEL",
        line_number,
    )?;
    let grid_level = u32::try_from(grid_level_value)
        .map_err(|_| format!("line {line_number}: GRID_LEVEL exceeds u32"))?;
    let grid_size = integral_value(
        values[1].expect("required field"),
        "GRID_SIZE",
        line_number,
    )?;
    if values[2].expect("required field") <= 0.0 {
        return Err(format!("line {line_number}: GRID_FAC must be positive"));
    }
    if values[3].expect("required field") <= 0.0 {
        return Err(format!("line {line_number}: MACH must be positive"));
    }
    if values[4].expect("required field") <= 0.0 {
        return Err(format!("line {line_number}: REY must be positive"));
    }
    let alpha_deg = values[5].expect("required field");
    match zone_kind {
        Dpw8V5ZoneKind::GridLevel(expected) if expected != grid_level => {
            return Err(format!(
                "line {line_number}: row GRID_LEVEL {grid_level} disagrees with zone {zone_title}"
            ));
        }
        Dpw8V5ZoneKind::Alpha(expected)
            if (alpha_deg - expected).abs() > ALPHA_ZONE_TOLERANCE =>
        {
            return Err(format!(
                "line {line_number}: row ALPHA {alpha_deg} disagrees with zone {zone_title}"
            ));
        }
        _ => {}
    }

    let view = match zone_kind {
        Dpw8V5ZoneKind::GridLevel(_) => Dpw8V5View::GridLevel,
        Dpw8V5ZoneKind::Alpha(_) => Dpw8V5View::Alpha,
    };
    Ok(Dpw8V5RawRow {
        source_line: line_number,
        zone_title: zone_title.to_owned(),
        view,
        grid_level,
        grid_size,
        alpha_deg,
        values,
    })
}

fn integral_value(value: f64, name: &str, line_number: usize) -> Result<u64, String> {
    if value <= 0.0 || value.fract().abs() > 1.0e-9 || value > u64::MAX as f64 {
        return Err(format!(
            "line {line_number}: {name} must be a positive integer"
        ));
    }
    Ok(value as u64)
}

fn insert_unique(
    target: &mut BTreeMap<String, String>,
    key: String,
    value: String,
    description: &str,
    line_number: usize,
) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err(format!("line {line_number}: {description} must not be empty"));
    }
    if target.insert(key.clone(), value).is_some() {
        return Err(format!(
            "line {line_number}: duplicate {description} {key}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../data/dpw8/synthetic_two_view_fixture_v0.1.dat");

    fn replace_first(input: &str, from: &str, to: &str) -> String {
        let index = input.find(from).expect("fixture replacement target");
        format!("{}{}{}", &input[..index], to, &input[index + from.len()..])
    }

    #[test]
    fn reconciles_forty_raw_rows_to_twenty_cells_and_keeps_both_line_ids() {
        let dataset = parse_v5(FIXTURE).expect("valid fixture");
        let report = audit_v5(&dataset).expect("views reconcile");
        assert_eq!(report.raw_record_count, 40);
        assert_eq!(report.grid_view_record_count, 20);
        assert_eq!(report.alpha_view_record_count, 20);
        assert_eq!(report.canonical_cells.len(), 20);
        assert!(report.canonical_cells.iter().all(|cell| {
            cell.grid_view_source_line != cell.alpha_view_source_line
                && cell.grid_view_source_line < cell.alpha_view_source_line
        }));
        assert_eq!(
            dataset.dataset_auxiliary.get("DatasetStatus"),
            Some(&"synthetic_fixture_not_official".to_owned())
        );
        assert!(dataset.zones.iter().all(|zone| {
            zone.auxiliary.get("FixtureOnly") == Some(&"true".to_owned())
        }));
    }

    #[test]
    fn optional_minus_999_values_remain_missing_not_zero() {
        let dataset = parse_v5(FIXTURE).expect("valid fixture");
        let report = audit_v5(&dataset).expect("views reconcile");
        assert!(report.canonical_cells.iter().all(|cell| {
            cell.row.values[9..].iter().filter(|value| value.is_none()).count() == 19
                && cell.row.values[9..].iter().all(|value| *value != Some(0.0))
        }));
    }

    #[test]
    fn suspicious_cell_is_flagged_but_never_repaired_or_removed() {
        let dataset = parse_v5(FIXTURE).expect("valid fixture");
        let report = audit_v5(&dataset).expect("views reconcile");
        let anomalies = report
            .canonical_cells
            .iter()
            .filter(|cell| cell.anomaly_warning)
            .collect::<Vec<_>>();
        assert_eq!(anomalies.len(), 1);
        let anomaly = anomalies[0];
        assert_eq!(anomaly.row.grid_level, 3);
        assert!((anomaly.row.alpha_deg - 3.1).abs() < 1.0e-12);
        assert_eq!(anomaly.row.cl_total(), 0.81306463);
        assert_eq!(anomaly.row.cd_total(), 0.01460769);
        assert_eq!(anomaly.row.cm_total(), -0.13340253);
        assert_eq!(report.canonical_cells.len(), 20);
        assert!(anomaly.duplicate_cl_cd_at_other_alpha);
        assert!(anomaly.adjacent_grid_discontinuity);
        assert_ne!(
            anomaly.grid_view_source_line,
            anomaly.alpha_view_source_line
        );
    }

    #[test]
    fn one_value_cross_view_mutation_is_rejected() {
        let tampered = replace_first(
            FIXTURE,
            "0.70000000 0.01200000",
            "0.70010000 0.01200000",
        );
        let dataset = parse_v5(&tampered).expect("mutated file remains syntactically valid");
        let error = audit_v5(&dataset).expect_err("cross-view mismatch must fail");
        assert!(error.contains("cross-view mismatch"));
    }

    #[test]
    fn duplicate_cell_in_one_view_is_rejected() {
        let last_row = FIXTURE
            .lines()
            .rev()
            .find(|line| line.split_whitespace().count() == V5_COLUMNS.len())
            .expect("fixture data row");
        let tampered = format!("{FIXTURE}\n{last_row}\n");
        let dataset = parse_v5(&tampered).expect("duplicate remains parseable");
        let error = audit_v5(&dataset).expect_err("duplicate must fail");
        assert!(error.contains("duplicate cell"));
    }

    #[test]
    fn zone_label_must_agree_with_each_row() {
        let tampered = replace_first(
            FIXTURE,
            "SYNTHETIC GRID LEVEL 1",
            "SYNTHETIC GRID LEVEL 2",
        );
        let error = parse_v5(&tampered).expect_err("mislabelled zone must fail");
        assert!(error.contains("disagrees with zone"));
    }

    #[test]
    fn malformed_column_count_is_rejected() {
        let first_row = FIXTURE
            .lines()
            .find(|line| line.split_whitespace().count() == V5_COLUMNS.len())
            .expect("fixture data row");
        let shortened = first_row
            .split_whitespace()
            .take(V5_COLUMNS.len() - 1)
            .collect::<Vec<_>>()
            .join(" ");
        let tampered = replace_first(FIXTURE, first_row, &shortened);
        let error = parse_v5(&tampered).expect_err("short row must fail");
        assert!(error.contains("numeric fields"));
    }

    #[test]
    fn non_finite_required_value_is_rejected() {
        let tampered = replace_first(FIXTURE, "0.70000000", "NaN");
        let error = parse_v5(&tampered).expect_err("NaN must fail");
        assert!(error.contains("non-finite"));
    }

    #[test]
    fn unsupported_zone_title_is_rejected() {
        let tampered = FIXTURE.replace("SYNTHETIC ALPHA ", "SYNTHETIC UNKNOWN ");
        let error = parse_v5(&tampered).expect_err("unsupported zone must fail");
        assert!(error.contains("unsupported V5 zone title"));
    }
}
