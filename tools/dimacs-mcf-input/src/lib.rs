#![forbid(unsafe_code)]

//! Bounded parser and canonical serializer for DIMACS minimum-cost-flow input.
//!
//! This crate preserves logical arc identity by input ordinal. Parallel arcs
//! are never merged. It parses mathematical input only: it does not solve the
//! instance, certify feasibility/optimality, or reinterpret multi-commodity or
//! process-coupled problems.

use std::collections::BTreeSet;
use std::fmt;

pub mod certificate;
pub use certificate::{
    McfArcFlow, McfCertificateError, McfOptimalityCertificate,
    MCF_OPTIMALITY_CERTIFICATE_SCHEMA, canonical_instance_sha256, verify_mcf_optimality,
};

const DIMACS_INTEGER_MAX: u32 = i32::MAX as u32;

/// Resource ceilings applied before allocating node/arc storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McfParseLimits {
    pub max_input_bytes: usize,
    pub max_line_bytes: usize,
    pub max_nodes: u32,
    pub max_arcs: u32,
}

impl Default for McfParseLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 256 * 1024 * 1024,
            max_line_bytes: 16 * 1024,
            max_nodes: 100_000,
            max_arcs: 1_000_000,
        }
    }
}

/// One DIMACS arc, with its original input-order identity preserved.
#[derive(Debug, Clone)]
pub struct McfArc {
    /// One-based ordinal among arc records; distinct parallel arcs stay distinct.
    pub ordinal: u32,
    /// DIMACS first endpoint: tail / source vertex.
    pub tail: u32,
    /// DIMACS second endpoint: head / destination vertex.
    pub head: u32,
    pub lower_bound: i64,
    pub upper_bound: i64,
    pub unit_cost: i32,
    /// One-based source line, diagnostic metadata and not serialized.
    pub source_line: usize,
}

impl PartialEq for McfArc {
    fn eq(&self, other: &Self) -> bool {
        self.ordinal == other.ordinal
            && self.tail == other.tail
            && self.head == other.head
            && self.lower_bound == other.lower_bound
            && self.upper_bound == other.upper_bound
            && self.unit_cost == other.unit_cost
    }
}

impl Eq for McfArc {}

/// Parsed DIMACS minimum-cost-flow instance.
///
/// node_supplies has length node_count + 1; index zero is an unused sentinel
/// and node_supplies[v] is the signed supply of one-based vertex v. Positive
/// values are supply and negative values are demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McfInstance {
    pub node_count: u32,
    pub arc_count: u32,
    pub node_supplies: Vec<i64>,
    /// Arc records in original file order, including parallel arcs.
    pub arcs: Vec<McfArc>,
}

impl McfInstance {
    /// Sum supplies using i128 so valid i64 records cannot overflow balance reporting.
    pub fn supply_balance(&self) -> i128 {
        self.node_supplies
            .iter()
            .skip(1)
            .map(|&supply| i128::from(supply))
            .sum()
    }

    pub fn is_supply_balanced(&self) -> bool {
        self.supply_balance() == 0
    }

    /// Serialize deterministically: nonzero supplies in ascending vertex order,
    /// then arcs in original order. Parallel arcs are never coalesced.
    pub fn to_dimacs(&self) -> String {
        let mut output = format!("p min {} {}\n", self.node_count, self.arc_count);
        for vertex in 1..=self.node_count {
            let supply = self.node_supplies[vertex as usize];
            if supply != 0 {
                output.push_str(&format!("n {vertex} {supply}\n"));
            }
        }
        for arc in &self.arcs {
            output.push_str(&format!(
                "a {} {} {} {} {}\n",
                arc.tail, arc.head, arc.lower_bound, arc.upper_bound, arc.unit_cost
            ));
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McfParseError {
    pub line: Option<usize>,
    pub message: String,
}

impl McfParseError {
    fn at(line: usize, message: impl Into<String>) -> Self {
        Self {
            line: Some(line),
            message: message.into(),
        }
    }

    fn global(message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
        }
    }
}

impl fmt::Display for McfParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "DIMACS MCF line {line}: {}", self.message),
            None => write!(formatter, "DIMACS MCF: {}", self.message),
        }
    }
}

impl std::error::Error for McfParseError {}

fn parse_u32(token: &str, label: &str, line: usize) -> Result<u32, McfParseError> {
    token
        .parse::<u32>()
        .map_err(|_| McfParseError::at(line, format!("{label} must be an unsigned 32-bit integer")))
}

fn parse_i64(token: &str, label: &str, line: usize) -> Result<i64, McfParseError> {
    token
        .parse::<i64>()
        .map_err(|_| McfParseError::at(line, format!("{label} must be a signed 64-bit integer")))
}

fn parse_i32(token: &str, label: &str, line: usize) -> Result<i32, McfParseError> {
    token
        .parse::<i32>()
        .map_err(|_| McfParseError::at(line, format!("{label} must be a signed 32-bit integer")))
}

/// Parse a DIMACS minimum-cost-flow instance using conservative default limits.
pub fn parse_dimacs_mcf(input: &str) -> Result<McfInstance, McfParseError> {
    parse_dimacs_mcf_with_limits(input, McfParseLimits::default())
}

/// Parse a DIMACS MCF instance with explicit byte, line, node and arc budgets.
///
/// Syntax and numeric ranges follow the DIMACS Network Flows 2.0 MCF format:
/// one p min n m header; optional nonzero n vertex supply records before all
/// arcs; and exactly m a tail head lower upper cost records. Supplies and
/// bounds are signed 64-bit fields, arc costs signed 32-bit. Flow bounds
/// describe nonnegative quantities, so negative lower/upper bounds are
/// rejected. A valid but unbalanced supply vector is preserved and reported;
/// it is not a syntax error and is not silently repaired.
pub fn parse_dimacs_mcf_with_limits(
    input: &str,
    limits: McfParseLimits,
) -> Result<McfInstance, McfParseError> {
    if input.len() > limits.max_input_bytes {
        return Err(McfParseError::global(format!(
            "input is {} bytes; configured limit is {}",
            input.len(),
            limits.max_input_bytes
        )));
    }

    let mut header: Option<(u32, u32)> = None;
    let mut node_supplies: Vec<i64> = Vec::new();
    let mut seen_node_records = BTreeSet::new();
    let mut arcs: Vec<McfArc> = Vec::new();
    let mut arc_records_started = false;

    for (line_index, raw_line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        if raw_line.len() > limits.max_line_bytes {
            return Err(McfParseError::at(
                line_number,
                format!(
                    "line is {} bytes; configured limit is {}",
                    raw_line.len(),
                    limits.max_line_bytes
                ),
            ));
        }

        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let record = fields.next().expect("nonempty line has a first token");
        if record == "c" {
            continue;
        }

        match record {
            "p" => {
                if header.is_some() {
                    return Err(McfParseError::at(line_number, "duplicate problem header"));
                }
                let problem_type = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing problem type"))?;
                if problem_type != "min" {
                    return Err(McfParseError::at(
                        line_number,
                        "only p min n m minimum-cost-flow problems are supported",
                    ));
                }
                let node_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing node count"))?;
                let arc_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing arc count"))?;
                if fields.next().is_some() {
                    return Err(McfParseError::at(
                        line_number,
                        "problem header has extra fields",
                    ));
                }
                let node_count = parse_u32(node_token, "node count", line_number)?;
                let arc_count = parse_u32(arc_token, "arc count", line_number)?;
                if node_count > DIMACS_INTEGER_MAX || arc_count > DIMACS_INTEGER_MAX {
                    return Err(McfParseError::at(
                        line_number,
                        "node and arc counts must not exceed 2^31-1",
                    ));
                }
                if node_count > limits.max_nodes {
                    return Err(McfParseError::at(
                        line_number,
                        format!(
                            "declared node count {node_count} exceeds configured limit {}",
                            limits.max_nodes
                        ),
                    ));
                }
                if arc_count > limits.max_arcs {
                    return Err(McfParseError::at(
                        line_number,
                        format!(
                            "declared arc count {arc_count} exceeds configured limit {}",
                            limits.max_arcs
                        ),
                    ));
                }

                let supply_len = usize::try_from(node_count)
                    .ok()
                    .and_then(|count| count.checked_add(1))
                    .ok_or_else(|| {
                        McfParseError::at(line_number, "node supply vector size overflow")
                    })?;
                node_supplies
                    .try_reserve_exact(supply_len)
                    .map_err(|error| {
                        McfParseError::at(
                            line_number,
                            format!("cannot reserve bounded node supply vector: {error}"),
                        )
                    })?;
                node_supplies.resize(supply_len, 0);

                let arc_capacity = usize::try_from(arc_count).map_err(|_| {
                    McfParseError::at(line_number, "arc count does not fit addressable memory")
                })?;
                arcs.try_reserve_exact(arc_capacity).map_err(|error| {
                    McfParseError::at(
                        line_number,
                        format!("cannot reserve bounded arc vector: {error}"),
                    )
                })?;
                header = Some((node_count, arc_count));
            }
            "n" => {
                let (node_count, _) = header.ok_or_else(|| {
                    McfParseError::at(line_number, "node supply record precedes problem header")
                })?;
                if arc_records_started {
                    return Err(McfParseError::at(
                        line_number,
                        "node supply records must precede all arc records",
                    ));
                }
                let vertex_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing supply vertex"))?;
                let supply_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing node supply"))?;
                if fields.next().is_some() {
                    return Err(McfParseError::at(
                        line_number,
                        "node supply record has extra fields",
                    ));
                }
                let vertex = parse_u32(vertex_token, "supply vertex", line_number)?;
                if vertex == 0 || vertex > node_count {
                    return Err(McfParseError::at(
                        line_number,
                        format!("supply vertex {vertex} is outside 1..={node_count}"),
                    ));
                }
                if !seen_node_records.insert(vertex) {
                    return Err(McfParseError::at(
                        line_number,
                        format!("duplicate supply record for vertex {vertex}"),
                    ));
                }
                let supply = parse_i64(supply_token, "node supply", line_number)?;
                if supply == 0 {
                    return Err(McfParseError::at(
                        line_number,
                        "zero-supply node records are noncanonical; omit the record",
                    ));
                }
                node_supplies[vertex as usize] = supply;
            }
            "a" => {
                let (node_count, arc_count) = header.ok_or_else(|| {
                    McfParseError::at(line_number, "arc record precedes problem header")
                })?;
                arc_records_started = true;
                if arcs.len() >= arc_count as usize {
                    return Err(McfParseError::at(
                        line_number,
                        format!("more than the declared {arc_count} arc records"),
                    ));
                }

                let tail_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing arc tail"))?;
                let head_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing arc head"))?;
                let lower_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing lower bound"))?;
                let upper_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing upper bound"))?;
                let cost_token = fields
                    .next()
                    .ok_or_else(|| McfParseError::at(line_number, "missing unit cost"))?;
                if fields.next().is_some() {
                    return Err(McfParseError::at(
                        line_number,
                        "arc record has extra fields",
                    ));
                }

                let tail = parse_u32(tail_token, "arc tail", line_number)?;
                let head = parse_u32(head_token, "arc head", line_number)?;
                for (endpoint, label) in [(tail, "tail"), (head, "head")] {
                    if endpoint == 0 || endpoint > node_count {
                        return Err(McfParseError::at(
                            line_number,
                            format!("arc {label} {endpoint} is outside 1..={node_count}"),
                        ));
                    }
                }
                let lower_bound = parse_i64(lower_token, "arc lower bound", line_number)?;
                let upper_bound = parse_i64(upper_token, "arc upper bound", line_number)?;
                let unit_cost = parse_i32(cost_token, "arc unit cost", line_number)?;
                if lower_bound < 0 {
                    return Err(McfParseError::at(
                        line_number,
                        "arc lower bound must be nonnegative",
                    ));
                }
                if upper_bound < 0 || upper_bound < lower_bound {
                    return Err(McfParseError::at(
                        line_number,
                        "arc upper bound must be nonnegative and at least its lower bound",
                    ));
                }

                let ordinal = u32::try_from(arcs.len() + 1)
                    .map_err(|_| McfParseError::at(line_number, "arc ordinal overflow"))?;
                arcs.push(McfArc {
                    ordinal,
                    tail,
                    head,
                    lower_bound,
                    upper_bound,
                    unit_cost,
                    source_line: line_number,
                });
            }
            other => {
                return Err(McfParseError::at(
                    line_number,
                    format!("unsupported record type {other}"),
                ));
            }
        }
    }

    let (node_count, arc_count) =
        header.ok_or_else(|| McfParseError::global("missing problem header p min n m"))?;
    if arcs.len() != arc_count as usize {
        return Err(McfParseError::global(format!(
            "declared {arc_count} arcs but parsed {}",
            arcs.len()
        )));
    }

    Ok(McfInstance {
        node_count,
        arc_count,
        node_supplies,
        arcs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "c small balanced instance\n\
p min 4 5\n\
n 1 4\n\
n 4 -4\n\
a 1 2 0 3 2\n\
a 1 3 0 2 1\n\
a 2 4 0 3 1\n\
a 3 4 0 2 3\n\
a 1 2 1 2 -1";

    #[test]
    fn parses_balanced_instance_and_preserves_tail_head_order() {
        let instance = parse_dimacs_mcf(SAMPLE).unwrap();
        assert_eq!(instance.node_count, 4);
        assert_eq!(instance.arc_count, 5);
        assert_eq!(instance.node_supplies, vec![0, 4, 0, 0, -4]);
        assert_eq!(instance.supply_balance(), 0);
        assert!(instance.is_supply_balanced());
        assert_eq!((instance.arcs[0].tail, instance.arcs[0].head), (1, 2));
        assert_eq!(instance.arcs[0].ordinal, 1);
        assert_eq!(instance.arcs[4].ordinal, 5);
        assert_eq!(instance.arcs[4].unit_cost, -1);
        assert_eq!(instance.arcs[4].source_line, 9);
    }

    #[test]
    fn parallel_arcs_are_preserved_with_distinct_ordinals_and_costs() {
        let instance =
            parse_dimacs_mcf("p min 2 3\nn 1 7\nn 2 -7\na 1 2 0 3 1\na 1 2 1 4 -9\na 1 2 0 2 7")
                .unwrap();
        assert_eq!(instance.arcs.len(), 3);
        assert_eq!(instance.arcs[0].ordinal, 1);
        assert_eq!(instance.arcs[1].ordinal, 2);
        assert_eq!(instance.arcs[2].ordinal, 3);
        assert!(
            instance
                .arcs
                .iter()
                .all(|arc| (arc.tail, arc.head) == (1, 2))
        );
        assert_eq!(
            instance
                .arcs
                .iter()
                .map(|arc| arc.unit_cost)
                .collect::<Vec<_>>(),
            vec![1, -9, 7]
        );
    }

    #[test]
    fn parses_signed_numeric_boundaries_without_narrowing() {
        let text = format!(
            "p min 2 1\nn 1 {}\nn 2 {}\na 1 2 {} {} {}",
            i64::MAX,
            i64::MIN,
            0,
            i64::MAX,
            i32::MIN
        );
        let instance = parse_dimacs_mcf(&text).unwrap();
        assert_eq!(instance.node_supplies[1], i64::MAX);
        assert_eq!(instance.node_supplies[2], i64::MIN);
        assert_eq!(instance.arcs[0].upper_bound, i64::MAX);
        assert_eq!(instance.arcs[0].unit_cost, i32::MIN);
    }

    #[test]
    fn preserves_unbalanced_supply_as_instance_data() {
        let instance = parse_dimacs_mcf("p min 2 1\nn 1 9\na 1 2 0 4 0").unwrap();
        assert_eq!(instance.supply_balance(), 9);
        assert!(!instance.is_supply_balanced());
    }

    #[test]
    fn comments_blank_lines_crlf_and_missing_final_newline_are_supported() {
        let instance =
            parse_dimacs_mcf("c header\r\n\r\np min 2 1\r\nn 1 1\r\nn 2 -1\r\na 1 2 0 1 0")
                .unwrap();
        assert_eq!(instance.arcs.len(), 1);
        assert!(instance.is_supply_balanced());
    }

    #[test]
    fn canonical_serializer_is_deterministic_and_preserves_arc_order() {
        let instance = parse_dimacs_mcf(SAMPLE).unwrap();
        let text = instance.to_dimacs();
        let reparsed = parse_dimacs_mcf(&text).unwrap();
        assert_eq!(reparsed, instance);
        assert!(text.find("a 1 2 0 3 2").unwrap() < text.find("a 1 2 1 2 -1").unwrap());
    }

    #[test]
    fn rejects_cost_outside_signed_32_bit_range() {
        let error = parse_dimacs_mcf("p min 2 1\na 1 2 0 1 2147483648").unwrap_err();
        assert!(error.to_string().contains("signed 32-bit"));
    }

    #[test]
    fn rejects_negative_flow_bounds_and_inverted_bounds() {
        for input in [
            "p min 2 1\na 1 2 -1 1 0",
            "p min 2 1\na 1 2 2 1 0",
            "p min 2 1\na 1 2 0 -1 0",
        ] {
            assert!(parse_dimacs_mcf(input).is_err());
        }
    }

    #[test]
    fn rejects_duplicate_header_and_duplicate_node_supply() {
        assert!(
            parse_dimacs_mcf("p min 1 0\np min 1 0")
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
        assert!(
            parse_dimacs_mcf("p min 1 0\nn 1 1\nn 1 -1")
                .unwrap_err()
                .to_string()
                .contains("duplicate supply")
        );
    }

    #[test]
    fn rejects_node_records_after_first_arc() {
        let error = parse_dimacs_mcf("p min 2 1\na 1 2 0 1 0\nn 1 1").unwrap_err();
        assert!(error.to_string().contains("must precede all arc"));
    }

    #[test]
    fn rejects_missing_header_unknown_record_and_wrong_problem_type() {
        assert!(
            parse_dimacs_mcf("n 1 1")
                .unwrap_err()
                .to_string()
                .contains("precedes")
        );
        assert!(
            parse_dimacs_mcf("p max 2 0")
                .unwrap_err()
                .to_string()
                .contains("only")
        );
        assert!(
            parse_dimacs_mcf("p min 2 0\nx 1 2")
                .unwrap_err()
                .to_string()
                .contains("unsupported")
        );
    }

    #[test]
    fn rejects_out_of_range_node_ids_and_bad_record_counts() {
        assert!(
            parse_dimacs_mcf("p min 2 1\na 0 2 0 1 0")
                .unwrap_err()
                .to_string()
                .contains("outside")
        );
        assert!(
            parse_dimacs_mcf("p min 2 1\na 1 3 0 1 0")
                .unwrap_err()
                .to_string()
                .contains("outside")
        );
        assert!(
            parse_dimacs_mcf("p min 2 2\na 1 2 0 1 0")
                .unwrap_err()
                .to_string()
                .contains("parsed 1")
        );
        assert!(
            parse_dimacs_mcf("p min 2 0\na 1 2 0 1 0")
                .unwrap_err()
                .to_string()
                .contains("more than")
        );
    }

    #[test]
    fn rejects_duplicate_supply_record_and_zero_supply_record() {
        assert!(
            parse_dimacs_mcf("p min 2 0\nn 1 1\nn 1 2")
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
        assert!(
            parse_dimacs_mcf("p min 2 0\nn 1 0")
                .unwrap_err()
                .to_string()
                .contains("zero-supply")
        );
    }

    #[test]
    fn enforces_header_limits_before_allocating_declared_arrays() {
        let limits = McfParseLimits {
            max_input_bytes: 1024,
            max_line_bytes: 128,
            max_nodes: 2,
            max_arcs: 1,
        };
        assert!(
            parse_dimacs_mcf_with_limits("p min 3 0", limits)
                .unwrap_err()
                .to_string()
                .contains("node count 3")
        );
        assert!(
            parse_dimacs_mcf_with_limits("p min 2 2", limits)
                .unwrap_err()
                .to_string()
                .contains("arc count 2")
        );
    }

    #[test]
    fn enforces_byte_and_line_limits() {
        let limits = McfParseLimits {
            max_input_bytes: 5,
            max_line_bytes: 3,
            max_nodes: 10,
            max_arcs: 10,
        };
        assert!(
            parse_dimacs_mcf_with_limits("p min 1 0", limits)
                .unwrap_err()
                .to_string()
                .contains("bytes")
        );
        let limits = McfParseLimits {
            max_input_bytes: 100,
            max_line_bytes: 4,
            ..limits
        };
        assert!(
            parse_dimacs_mcf_with_limits("p min 1 0", limits)
                .unwrap_err()
                .to_string()
                .contains("line is")
        );
    }

    #[test]
    fn requires_exact_field_counts() {
        for input in [
            "p min 2 0 extra",
            "p min 2 0\nn 1 1 extra",
            "p min 2 1\na 1 2 0 1 0 extra",
        ] {
            assert!(parse_dimacs_mcf(input).is_err());
        }
    }
}
