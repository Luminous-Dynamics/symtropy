use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockKind {
    Displacements,
    ExternalForces,
}

impl fmt::Display for BlockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Displacements => f.write_str("displacements"),
            Self::ExternalForces => f.write_str("external forces"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodalField {
    pub output_time: f64,
    pub values: BTreeMap<usize, Vec3>,
}

impl NodalField {
    pub fn get(&self, node_id: usize) -> Option<Vec3> {
        self.values.get(&node_id).copied()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedDat {
    pub displacements: NodalField,
    /// The RF key outputs external forces; this is not universally reaction-only.
    pub external_forces: NodalField,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    EmptyExpectedNodeSet,
    EmptyExpectedSetName,
    InvalidExpectedNodeId(usize),
    DuplicateExpectedNodeId(usize),
    InvalidTargetTime,
    InvalidTimeTolerance,
    MalformedHeader { line: usize, detail: &'static str },
    MalformedRow { line: usize, text: String },
    NonFiniteValue { line: usize, node_id: usize },
    DuplicateNode { kind: BlockKind, node_id: usize },
    MissingBlock { kind: BlockKind, set_name: String, time: f64 },
    DuplicateBlock { kind: BlockKind, set_name: String, time: f64 },
    TimeNotFound { kind: BlockKind, set_name: String, time: f64, observed: Vec<f64> },
    NodeInventoryMismatch { kind: BlockKind, missing: Vec<usize>, unexpected: Vec<usize> },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyExpectedNodeSet => f.write_str("expected node set is empty"),
            Self::EmptyExpectedSetName => f.write_str("expected set name is empty"),
            Self::InvalidExpectedNodeId(id) => write!(f, "invalid expected node ID {id}"),
            Self::DuplicateExpectedNodeId(id) => write!(f, "duplicate expected node ID {id}"),
            Self::InvalidTargetTime => f.write_str("target time must be finite"),
            Self::InvalidTimeTolerance => {
                f.write_str("time tolerance must be finite and nonnegative")
            }
            Self::MalformedHeader { line, detail } => {
                write!(f, "malformed section header at line {line}: {detail}")
            }
            Self::MalformedRow { line, text } => {
                write!(f, "malformed numeric row at line {line}: {text}")
            }
            Self::NonFiniteValue { line, node_id } => {
                write!(f, "non-finite value at line {line}, node {node_id}")
            }
            Self::DuplicateNode { kind, node_id } => {
                write!(f, "duplicate node {node_id} in {kind} block")
            }
            Self::MissingBlock { kind, set_name, time } => {
                write!(f, "missing {kind} block for set {set_name} at time {time}")
            }
            Self::DuplicateBlock { kind, set_name, time } => {
                write!(f, "duplicate {kind} blocks for set {set_name} at time {time}")
            }
            Self::TimeNotFound {
                kind,
                set_name,
                time,
                observed,
            } => write!(
                f,
                "no {kind} block for set {set_name} at time {time}; observed {observed:?}"
            ),
            Self::NodeInventoryMismatch {
                kind,
                missing,
                unexpected,
            } => write!(
                f,
                "{kind} node inventory mismatch; missing {missing:?}, unexpected {unexpected:?}"
            ),
        }
    }
}

impl Error for ParseError {}

struct Section {
    kind: BlockKind,
    set_name: String,
    time: f64,
    values: BTreeMap<usize, Vec3>,
}

pub fn parse_dat(
    text: &str,
    expected_set: &str,
    expected_node_ids: &[usize],
    target_time: f64,
    time_tolerance: f64,
) -> Result<ParsedDat, ParseError> {
    if expected_node_ids.is_empty() {
        return Err(ParseError::EmptyExpectedNodeSet);
    }
    if expected_set.trim().is_empty() {
        return Err(ParseError::EmptyExpectedSetName);
    }
    if !target_time.is_finite() {
        return Err(ParseError::InvalidTargetTime);
    }
    if !time_tolerance.is_finite() || time_tolerance < 0.0 {
        return Err(ParseError::InvalidTimeTolerance);
    }
    let mut expected = BTreeSet::new();
    for id in expected_node_ids {
        if *id == 0 {
            return Err(ParseError::InvalidExpectedNodeId(*id));
        }
        if !expected.insert(*id) {
            return Err(ParseError::DuplicateExpectedNodeId(*id));
        }
    }

    let sections = parse_sections(text)?;
    let displacements = select_section(
        &sections,
        BlockKind::Displacements,
        expected_set,
        &expected,
        target_time,
        time_tolerance,
    )?;
    let external_forces = select_section(
        &sections,
        BlockKind::ExternalForces,
        expected_set,
        &expected,
        target_time,
        time_tolerance,
    )?;
    Ok(ParsedDat { displacements, external_forces })
}

fn classify_heading(line: &str) -> Option<BlockKind> {
    let lower = line.trim().to_ascii_lowercase();
    if lower.starts_with("displacements (") {
        Some(BlockKind::Displacements)
    } else if lower.starts_with("forces (") {
        Some(BlockKind::ExternalForces)
    } else {
        None
    }
}

fn parse_sections(text: &str) -> Result<Vec<Section>, ParseError> {
    let lines: Vec<&str> = text.lines().collect();
    let mut sections = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let header = lines[index].trim();
        let Some(kind) = classify_heading(header) else {
            index += 1;
            continue;
        };
        let (set_name, time) = parse_header(header, index + 1)?;
        let mut values = BTreeMap::new();
        let mut row_index = index + 1;
        while row_index < lines.len() {
            let row = lines[row_index].trim();
            if row.is_empty() {
                if values.is_empty() {
                    row_index += 1;
                    continue;
                }
                break;
            }
            if classify_heading(row).is_some() {
                break;
            }
            match parse_row(row, row_index + 1)? {
                Some((node_id, value)) => {
                    match values.entry(node_id) {
                        Entry::Vacant(entry) => {
                            entry.insert(value);
                        }
                        Entry::Occupied(_) => {
                            return Err(ParseError::DuplicateNode { kind, node_id });
                        }
                    }
                }
                None => break,
            }
            row_index += 1;
        }
        sections.push(Section { kind, set_name, time, values });
        index = row_index;
    }
    Ok(sections)
}

fn parse_header(header: &str, line: usize) -> Result<(String, f64), ParseError> {
    let tokens: Vec<&str> = header.split_whitespace().collect();
    let lower: Vec<String> = tokens
        .iter()
        .map(|token| token.to_ascii_lowercase())
        .collect();
    let set_pos = lower
        .iter()
        .position(|token| token == "set")
        .ok_or(ParseError::MalformedHeader {
            line,
            detail: "missing set name",
        })?;
    let set_name = tokens
        .get(set_pos + 1)
        .ok_or(ParseError::MalformedHeader {
            line,
            detail: "missing set name",
        })?
        .trim_end_matches(',')
        .to_owned();
    let time_pos = lower
        .iter()
        .position(|token| token == "time")
        .ok_or(ParseError::MalformedHeader {
            line,
            detail: "missing time",
        })?;
    let time_str = tokens
        .get(time_pos + 1)
        .ok_or(ParseError::MalformedHeader {
            line,
            detail: "missing time value",
        })?;
    let time = parse_number(time_str).ok_or(ParseError::MalformedHeader {
        line,
        detail: "invalid time value",
    })?;
    if !time.is_finite() {
        return Err(ParseError::MalformedHeader {
            line,
            detail: "non-finite time",
        });
    }
    Ok((set_name, time))
}

fn parse_row(row: &str, line: usize) -> Result<Option<(usize, Vec3)>, ParseError> {
    let fields: Vec<&str> = row.split_whitespace().collect();
    let Some(first) = fields.first() else {
        return Ok(None);
    };
    let Ok(node_id) = first.parse::<usize>() else {
        return Ok(None);
    };
    if node_id == 0 || fields.len() != 4 {
        return Err(ParseError::MalformedRow {
            line,
            text: row.to_owned(),
        });
    }
    let (Some(x), Some(y), Some(z)) = (
        parse_number(fields[1]),
        parse_number(fields[2]),
        parse_number(fields[3]),
    ) else {
        return Err(ParseError::MalformedRow {
            line,
            text: row.to_owned(),
        });
    };
    if !(x.is_finite() && y.is_finite() && z.is_finite()) {
        return Err(ParseError::NonFiniteValue { line, node_id });
    }
    Ok(Some((node_id, Vec3 { x, y, z })))
}

fn parse_number(field: &str) -> Option<f64> {
    field.replace('D', "E").replace('d', "E").parse::<f64>().ok()
}

fn select_section(
    sections: &[Section],
    kind: BlockKind,
    expected_set: &str,
    expected_ids: &BTreeSet<usize>,
    target_time: f64,
    time_tolerance: f64,
) -> Result<NodalField, ParseError> {
    let matching_set: Vec<&Section> = sections
        .iter()
        .filter(|section| {
            section.kind == kind && section.set_name.eq_ignore_ascii_case(expected_set)
        })
        .collect();
    let matching_time: Vec<&Section> = matching_set
        .iter()
        .copied()
        .filter(|section| (section.time - target_time).abs() <= time_tolerance)
        .collect();
    if matching_time.is_empty() {
        if matching_set.is_empty() {
            return Err(ParseError::MissingBlock {
                kind,
                set_name: expected_set.to_owned(),
                time: target_time,
            });
        }
        return Err(ParseError::TimeNotFound {
            kind, set_name: expected_set.to_owned(), time: target_time,
            observed: matching_set.iter().map(|s| s.time).collect(),
        });
    }
    if matching_time.len() != 1 {
        return Err(ParseError::DuplicateBlock {
            kind,
            set_name: expected_set.to_owned(),
            time: target_time,
        });
    }
    let section = matching_time[0];
    let actual: BTreeSet<usize> = section.values.keys().copied().collect();
    let missing = expected_ids.difference(&actual).copied().collect::<Vec<_>>();
    let unexpected = actual.difference(expected_ids).copied().collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        return Err(ParseError::NodeInventoryMismatch { kind, missing, unexpected });
    }
    Ok(NodalField { output_time: section.time, values: section.values.clone() })
}


#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/ccx-2.22-synthetic-nall.dat");
    const NODES: [usize; 4] = [1, 2, 3, 4];

    #[test]
    fn parses_displacements_and_rf_external_forces_separately() {
        let parsed = parse_dat(FIXTURE, "NALL", &NODES, 1.0, 1.0e-8).unwrap();
        assert_eq!(parsed.displacements.values.len(), 4);
        assert_eq!(parsed.external_forces.values.len(), 4);
        assert_eq!(parsed.displacements.get(2).unwrap().x, 1.0e-3);
        assert_eq!(parsed.external_forces.get(3).unwrap().y, 6.0);
        assert_eq!(parsed.displacements.output_time, 1.0);
        assert_eq!(parsed.external_forces.output_time, 1.0);
    }

    #[test]
    fn reaction_only_block_is_not_accepted_as_rf_external_forces() {
        let only_reaction = FIXTURE
            .lines()
            .filter(|line| !line.trim().starts_with("forces ("))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(matches!(
            parse_dat(&only_reaction, "NALL", &NODES, 1.0, 1.0e-8),
            Err(ParseError::MissingBlock {
                kind: BlockKind::ExternalForces,
                ..
            })
        ));
    }

    #[test]
    fn rejects_missing_node_and_duplicate_node() {
        let missing = FIXTURE.replace(
            "             4  0.000000E+00  2.000000E-03  0.000000E+00\n",
            "",
        );
        assert!(matches!(
            parse_dat(&missing, "NALL", &NODES, 1.0, 1.0e-8),
            Err(ParseError::NodeInventoryMismatch { .. })
        ));

        let duplicate = FIXTURE.replace(
            "             4  0.000000E+00  2.000000E-03  0.000000E+00\n",
            concat!(
                "             3  0.000000E+00  2.000000E-03  0.000000E+00\n",
                "             4  0.000000E+00  2.000000E-03  0.000000E+00\n",
            ),
        );
        assert!(matches!(
            parse_dat(&duplicate, "NALL", &NODES, 1.0, 1.0e-8),
            Err(ParseError::DuplicateNode { .. })
        ));
    }

    #[test]
    fn rejects_nonfinite_values_and_malformed_numeric_rows() {
        let infinite = FIXTURE.replace("1.000000E-03", "inf");
        assert!(matches!(
            parse_dat(&infinite, "NALL", &NODES, 1.0, 1.0e-8),
            Err(ParseError::NonFiniteValue { .. })
        ));
        let malformed = FIXTURE.replace(
            "             2  1.000000E-03  0.000000E+00  0.000000E+00\n",
            "             2  1.000000E-03  0.000000E+00\n",
        );
        assert!(matches!(
            parse_dat(&malformed, "NALL", &NODES, 1.0, 1.0e-8),
            Err(ParseError::MalformedRow { .. })
        ));
    }

    #[test]
    fn rejects_wrong_time_set_and_duplicate_target_section() {
        assert!(matches!(
            parse_dat(FIXTURE, "NALL", &NODES, 2.0, 1.0e-8),
            Err(ParseError::TimeNotFound { .. })
        ));
        assert!(matches!(
            parse_dat(FIXTURE, "UNKNOWN_SET", &NODES, 1.0, 1.0e-8),
            Err(ParseError::MissingBlock { .. })
        ));
        let duplicate_section = format!(
            "{}\n{}",
            FIXTURE,
            concat!(
                "forces (fx,fy,fz) for set NALL and time 0.1000000E+01\n",
                "1 0 0 0\n",
                "2 0 0 0\n",
                "3 0 0 0\n",
                "4 0 0 0\n",
            )
        );
        assert!(matches!(
            parse_dat(&duplicate_section, "NALL", &NODES, 1.0, 1.0e-8),
            Err(ParseError::DuplicateBlock {
                kind: BlockKind::ExternalForces,
                ..
            })
        ));
    }

    #[test]
    fn rejects_invalid_requests_and_supports_d_exponents() {
        assert!(matches!(
            parse_dat(FIXTURE, "NALL", &[], 1.0, 1.0e-8),
            Err(ParseError::EmptyExpectedNodeSet)
        ));
        assert!(matches!(
            parse_dat(FIXTURE, "NALL", &[1, 1], 1.0, 1.0e-8),
            Err(ParseError::DuplicateExpectedNodeId(1))
        ));
        assert!(matches!(
            parse_dat(FIXTURE, "", &NODES, 1.0, 1.0e-8),
            Err(ParseError::EmptyExpectedSetName)
        ));
        let d_exponents = FIXTURE.replace("1.000000E-03", "1.000000D-03");
        assert!(parse_dat(&d_exponents, "NALL", &NODES, 1.0, 1.0e-8).is_ok());
    }
}
