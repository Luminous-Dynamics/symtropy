#![forbid(unsafe_code)]

//! Strict DIMACS minimum-cost-flow input parser and corpus-integrity receipt.
//!
//! This module parses the official DIMACS Network Flows 2.0 MCF ASCII format.
//! It preserves parallel arcs and signed lower/upper bounds as encoded. It
//! validates syntax, integer domains, graph references, counts and resource
//! budgets, but does not solve the MCF instance or decide its feasibility.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const DIMACS_MCF_FORMAT_VERSION: &str = "dimacs-mcf-v1";
pub const DIMACS_MCF_IMPORT_RECEIPT_SCHEMA: &str = "dimacs-mcf-import-receipt-v0.1";
pub const DIMACS_MCF_PARSER_VERSION: &str = "luminous-dimacs-mcf-import-v0.1";
pub const DIMACS_MCF_IMPORT_CLAIM_CEILING: &str =
    "DIMACS MCF syntax, raw-file identity and canonical imported-network identity only; no instance feasibility, solver optimality, external source authority, physical model validity or service claim.";
pub const DIMACS_MAX_NODES_AND_ARCS: u32 = i32::MAX as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportLimits {
    pub max_nodes: u32,
    pub max_arcs: u32,
    pub max_input_bytes: usize,
    pub max_working_bytes: u64,
}

impl Default for ImportLimits {
    fn default() -> Self {
        Self {
            // The scale programme's Stage B ceiling, not an achieved result.
            max_nodes: 1_000_000,
            max_arcs: 8_000_000,
            max_input_bytes: 1_073_741_824,
            max_working_bytes: 536_870_912,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimacsMcfImportLimitsReceipt {
    pub max_nodes: u32,
    pub max_arcs: u32,
    pub max_input_bytes: u64,
    pub max_working_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimacsMcfImportReceipt {
    pub schema_version: String,
    pub parser_version: String,
    pub format_version: String,
    pub raw_sha256: String,
    pub canonical_sha256: String,
    pub node_count: u32,
    pub declared_arc_count: u32,
    pub nonzero_supply_record_count: u32,
    pub raw_input_bytes: u64,
    pub estimated_primary_allocation_bytes: u64,
    pub limits: DimacsMcfImportLimitsReceipt,
    /// Decimal string because the exact i128 aggregate can exceed JSON's
    /// interoperable signed 64-bit integer range.
    pub total_supply_decimal: String,
    pub supply_total_is_zero: bool,
    pub negative_cost_arc_count: u32,
    pub negative_lower_bound_arc_count: u32,
    pub negative_upper_bound_arc_count: u32,
    pub nonzero_lower_bound_arc_count: u32,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimacsMcfParseError {
    pub line: usize,
    pub message: String,
}

impl DimacsMcfParseError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for DimacsMcfParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "DIMACS MCF import: {}", self.message)
        } else {
            write!(f, "DIMACS MCF import line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for DimacsMcfParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimacsMcfArc {
    /// Zero-based record ordinal. Parallel arcs retain separate ordinals.
    pub ordinal: u32,
    pub tail: u32,
    pub head: u32,
    /// DIMACS declares both bounds as signed 64-bit integers.
    pub lower_bound: i64,
    pub upper_bound: i64,
    /// DIMACS unit cost is a signed 32-bit integer.
    pub unit_cost: i32,
}

impl DimacsMcfArc {
    /// Width of the non-negative residual interval [lower_bound, upper_bound].
    ///
    /// Signed bounds are preserved; a full i64-wide interval can have a
    /// residual width of u64::MAX.
    pub fn residual_capacity(&self) -> Result<u64, String> {
        let width = i128::from(self.upper_bound) - i128::from(self.lower_bound);
        u64::try_from(width)
            .map_err(|_| "DIMACS arc residual capacity does not fit u64".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimacsMcfInstance {
    pub format_version: String,
    pub node_count: u32,
    pub declared_arc_count: u32,
    /// Index zero is unused; DIMACS node identifiers are in 1..=node_count.
    pub node_supplies: Vec<i64>,
    /// Original input record order is retained; parallel arcs are not merged.
    pub arcs: Vec<DimacsMcfArc>,
    pub nonzero_supply_record_count: u32,
    pub raw_input_bytes: usize,
    pub estimated_primary_allocation_bytes: u64,
    /// Exact parser admission ceilings used for this import.
    pub limits: ImportLimits,
    pub raw_sha256: String,
    /// Canonical semantic digest excludes comments, whitespace, node-supply
    /// record order and arc record order. Parallel arc tuples remain separate
    /// entries even when all their values are identical.
    pub canonical_sha256: String,
    /// Diagnostic only. An unbalanced total is not a parser error and no
    /// feasibility result is inferred by this importer.
    pub total_supply: i128,
    pub supply_total_is_zero: bool,
}

impl DimacsMcfInstance {
    pub fn has_negative_costs(&self) -> bool {
        self.arcs.iter().any(|arc| arc.unit_cost < 0)
    }

    pub fn has_signed_capacity_bounds(&self) -> bool {
        self.arcs
            .iter()
            .any(|arc| arc.lower_bound < 0 || arc.upper_bound < 0)
    }

    pub fn has_nonzero_lower_bounds(&self) -> bool {
        self.arcs.iter().any(|arc| arc.lower_bound != 0)
    }

    /// Create a portable receipt for the exact raw artifact and imported network.
    pub fn import_receipt(&self) -> DimacsMcfImportReceipt {
        DimacsMcfImportReceipt {
            schema_version: DIMACS_MCF_IMPORT_RECEIPT_SCHEMA.to_string(),
            parser_version: DIMACS_MCF_PARSER_VERSION.to_string(),
            format_version: self.format_version.clone(),
            raw_sha256: self.raw_sha256.clone(),
            canonical_sha256: self.canonical_sha256.clone(),
            node_count: self.node_count,
            declared_arc_count: self.declared_arc_count,
            nonzero_supply_record_count: self.nonzero_supply_record_count,
            raw_input_bytes: self.raw_input_bytes as u64,
            estimated_primary_allocation_bytes: self.estimated_primary_allocation_bytes,
            limits: DimacsMcfImportLimitsReceipt {
                max_nodes: self.limits.max_nodes,
                max_arcs: self.limits.max_arcs,
                max_input_bytes: self.limits.max_input_bytes as u64,
                max_working_bytes: self.limits.max_working_bytes,
            },
            total_supply_decimal: self.total_supply.to_string(),
            supply_total_is_zero: self.supply_total_is_zero,
            negative_cost_arc_count: self.arcs.iter().filter(|arc| arc.unit_cost < 0).count() as u32,
            negative_lower_bound_arc_count: self.arcs.iter().filter(|arc| arc.lower_bound < 0).count() as u32,
            negative_upper_bound_arc_count: self.arcs.iter().filter(|arc| arc.upper_bound < 0).count() as u32,
            nonzero_lower_bound_arc_count: self.arcs.iter().filter(|arc| arc.lower_bound != 0).count() as u32,
            claim_ceiling: DIMACS_MCF_IMPORT_CLAIM_CEILING.to_string(),
        }
    }

    pub fn canonical_digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"LUMINOUS-DIMACS-MCF-IMPORT-V1\0");
        hasher.update(self.node_count.to_be_bytes());
        hasher.update(self.declared_arc_count.to_be_bytes());

        for (node_id, supply) in self.node_supplies.iter().enumerate().skip(1) {
            hasher.update((node_id as u32).to_be_bytes());
            hasher.update(supply.to_be_bytes());
        }

        // Arc records have no intrinsic identifier in the DIMACS text
        // format. Sort by the complete semantic tuple so record order does
        // not perturb canonical identity, while retaining duplicate tuples
        // as separate entries (parallel arcs must never be aggregated).
        let mut arcs = self.arcs.iter().collect::<Vec<_>>();
        arcs.sort_by(|left, right| {
            (
                left.tail,
                left.head,
                left.lower_bound,
                left.upper_bound,
                left.unit_cost,
            )
                .cmp(&(
                    right.tail,
                    right.head,
                    right.lower_bound,
                    right.upper_bound,
                    right.unit_cost,
                ))
        });
        for (ordinal, arc) in arcs.iter().enumerate() {
            hasher.update((ordinal as u32).to_be_bytes());
            hasher.update(arc.tail.to_be_bytes());
            hasher.update(arc.head.to_be_bytes());
            hasher.update(arc.lower_bound.to_be_bytes());
            hasher.update(arc.upper_bound.to_be_bytes());
            hasher.update(arc.unit_cost.to_be_bytes());
        }

        format!("sha256:{:x}", hasher.finalize())
    }
}

fn parse_number<T>(token: &str, line: usize, field: &str) -> Result<T, DimacsMcfParseError>
where
    T: std::str::FromStr,
{
    token.parse::<T>().map_err(|_| {
        DimacsMcfParseError::new(line, format!("invalid {field} integer token {token:?}"))
    })
}

fn preflight(
    nodes: u32,
    arcs: u32,
    input_bytes: usize,
    limits: ImportLimits,
    line: usize,
) -> Result<u64, DimacsMcfParseError> {
    if nodes > DIMACS_MAX_NODES_AND_ARCS || arcs > DIMACS_MAX_NODES_AND_ARCS {
        return Err(DimacsMcfParseError::new(
            line,
            "node or arc count exceeds the DIMACS 31-bit count range",
        ));
    }
    if nodes > limits.max_nodes {
        return Err(DimacsMcfParseError::new(
            line,
            format!(
                "declared node count {nodes} exceeds configured limit {}",
                limits.max_nodes
            ),
        ));
    }
    if arcs > limits.max_arcs {
        return Err(DimacsMcfParseError::new(
            line,
            format!(
                "declared arc count {arcs} exceeds configured limit {}",
                limits.max_arcs
            ),
        ));
    }
    if input_bytes > limits.max_input_bytes {
        return Err(DimacsMcfParseError::new(
            line,
            format!(
                "input byte length {input_bytes} exceeds configured limit {}",
                limits.max_input_bytes
            ),
        ));
    }

    let node_slots = u64::from(nodes)
        .checked_add(1)
        .ok_or_else(|| DimacsMcfParseError::new(line, "node slot count overflowed u64"))?;
    let node_bytes = node_slots
        .checked_mul(std::mem::size_of::<i64>() as u64)
        .ok_or_else(|| DimacsMcfParseError::new(line, "node allocation size overflowed u64"))?;
    let arc_bytes = u64::from(arcs)
        .checked_mul(std::mem::size_of::<DimacsMcfArc>() as u64)
        .ok_or_else(|| DimacsMcfParseError::new(line, "arc allocation size overflowed u64"))?;
    // Canonical hashing sorts a scratch vector of references; include it in
    // the primary working-set estimate rather than ignoring that allocation.
    let arc_sort_bytes = u64::from(arcs)
        .checked_mul(std::mem::size_of::<&DimacsMcfArc>() as u64)
        .ok_or_else(|| DimacsMcfParseError::new(line, "arc sort allocation size overflowed u64"))?;
    let estimate = node_bytes
        .checked_add(arc_bytes)
        .and_then(|value| value.checked_add(arc_sort_bytes))
        .ok_or_else(|| DimacsMcfParseError::new(line, "allocation estimate overflowed u64"))?;

    if estimate > limits.max_working_bytes {
        return Err(DimacsMcfParseError::new(
            line,
            format!(
                "estimated primary allocation {estimate} bytes exceeds configured budget {}",
                limits.max_working_bytes
            ),
        ));
    }

    Ok(estimate)
}


/// Reparse the exact raw input with the recorded admission limits and compare
/// every import-summary field. This validates receipt/input correspondence;
/// it is not an independent MCF solver or an independent parser implementation.
pub fn verify_dimacs_mcf_import_receipt(
    input: &[u8],
    receipt: &DimacsMcfImportReceipt,
) -> Result<(), DimacsMcfParseError> {
    if receipt.schema_version != DIMACS_MCF_IMPORT_RECEIPT_SCHEMA {
        return Err(DimacsMcfParseError::new(
            0,
            "unsupported DIMACS MCF import receipt schema",
        ));
    }
    if receipt.parser_version != DIMACS_MCF_PARSER_VERSION {
        return Err(DimacsMcfParseError::new(
            0,
            "DIMACS MCF parser version mismatch",
        ));
    }
    if receipt.claim_ceiling != DIMACS_MCF_IMPORT_CLAIM_CEILING {
        return Err(DimacsMcfParseError::new(
            0,
            "DIMACS MCF import claim ceiling mismatch",
        ));
    }
    let max_input_bytes = usize::try_from(receipt.limits.max_input_bytes).map_err(|_| {
        DimacsMcfParseError::new(0, "recorded input-byte limit does not fit usize")
    })?;
    let limits = ImportLimits {
        max_nodes: receipt.limits.max_nodes,
        max_arcs: receipt.limits.max_arcs,
        max_input_bytes,
        max_working_bytes: receipt.limits.max_working_bytes,
    };
    let parsed = parse_dimacs_mcf(input, limits)?;
    if parsed.import_receipt() != *receipt {
        return Err(DimacsMcfParseError::new(
            0,
            "DIMACS MCF import receipt does not match the exact raw input",
        ));
    }
    Ok(())
}

/// Parse one DIMACS minimum-cost-flow instance.
///
/// The parser is intentionally stricter than permissive legacy readers:
/// unknown record types, misplaced records, count mismatches, duplicate
/// nonzero-supply declarations, zero-supply node records, out-of-range node
/// IDs and lower bounds above upper bounds are rejected.
pub fn parse_dimacs_mcf(
    input: &[u8],
    limits: ImportLimits,
) -> Result<DimacsMcfInstance, DimacsMcfParseError> {
    if input.len() > limits.max_input_bytes {
        return Err(DimacsMcfParseError::new(
            0,
            format!(
                "input byte length {} exceeds configured limit {}",
                input.len(),
                limits.max_input_bytes
            ),
        ));
    }
    if !input.is_ascii() {
        return Err(DimacsMcfParseError::new(
            0,
            "input is not ASCII as required by the DIMACS format",
        ));
    }
    if input
        .iter()
        .any(|byte| *byte < 0x20 && !matches!(*byte, b'\n' | b'\r' | b'\t'))
    {
        return Err(DimacsMcfParseError::new(
            0,
            "input contains an unsupported ASCII control character",
        ));
    }

    let text = std::str::from_utf8(input)
        .map_err(|_| DimacsMcfParseError::new(0, "input is not valid UTF-8/ASCII"))?;

    let mut problem: Option<(u32, u32)> = None;
    let mut supplies: Vec<i64> = Vec::new();
    let mut arcs: Vec<DimacsMcfArc> = Vec::new();
    let mut supply_records = 0_u32;
    let mut saw_arc = false;
    let mut estimated_allocation = 0_u64;

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let mut tokens = raw_line.split_whitespace();
        let Some(record_type) = tokens.next() else {
            continue;
        };

        match record_type {
            "c" => {
                // Comments may contain arbitrary text. They are skipped
                // without collecting the comment's words into an allocation.
                continue;
            }
            "p" => {
                if problem.is_some() || supply_records != 0 || saw_arc {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "problem line must appear exactly once before node and arc records",
                    ));
                }
                let (Some(format), Some(nodes_token), Some(arcs_token)) =
                    (tokens.next(), tokens.next(), tokens.next())
                else {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "problem line must have exactly the form: p min <nodes> <arcs>",
                    ));
                };
                if format != "min" || tokens.next().is_some() {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "problem line must have exactly the form: p min <nodes> <arcs>",
                    ));
                }
                let nodes: u32 = parse_number(nodes_token, line_number, "node count")?;
                let arc_count: u32 = parse_number(arcs_token, line_number, "arc count")?;
                estimated_allocation =
                    preflight(nodes, arc_count, input.len(), limits, line_number)?;
                let node_slots = usize::try_from(nodes)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| {
                        DimacsMcfParseError::new(
                            line_number,
                            "node count does not fit addressable memory",
                        )
                    })?;
                let arc_capacity = usize::try_from(arc_count).map_err(|_| {
                    DimacsMcfParseError::new(
                        line_number,
                        "arc count does not fit addressable memory",
                    )
                })?;
                supplies = vec![0_i64; node_slots];
                arcs = Vec::with_capacity(arc_capacity);
                problem = Some((nodes, arc_count));
            }
            "n" => {
                let (nodes, _) = problem.ok_or_else(|| {
                    DimacsMcfParseError::new(
                        line_number,
                        "node record appeared before the problem line",
                    )
                })?;
                if saw_arc {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "node records must precede all arc records",
                    ));
                }
                let (Some(node_token), Some(supply_token)) = (tokens.next(), tokens.next()) else {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "node record must have exactly the form: n <id> <supply>",
                    ));
                };
                if tokens.next().is_some() {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "node record must have exactly the form: n <id> <supply>",
                    ));
                }
                let node_id: u32 = parse_number(node_token, line_number, "node ID")?;
                if node_id == 0 || node_id > nodes {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        format!("node ID {node_id} is outside the required 1..={nodes} range"),
                    ));
                }
                let supply: i64 = parse_number(supply_token, line_number, "node supply")?;
                if supply == 0 {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "DIMACS node records are only for nonzero supplies",
                    ));
                }
                let slot = &mut supplies[node_id as usize];
                if *slot != 0 {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        format!("duplicate node supply record for node {node_id}"),
                    ));
                }
                *slot = supply;
                supply_records = supply_records.checked_add(1).ok_or_else(|| {
                    DimacsMcfParseError::new(line_number, "node supply record count overflowed")
                })?;
            }
            "a" => {
                let (nodes, declared_arcs) = problem.ok_or_else(|| {
                    DimacsMcfParseError::new(
                        line_number,
                        "arc record appeared before the problem line",
                    )
                })?;
                saw_arc = true;

                let (
                    Some(tail_token),
                    Some(head_token),
                    Some(lower_token),
                    Some(upper_token),
                    Some(cost_token),
                ) = (
                    tokens.next(),
                    tokens.next(),
                    tokens.next(),
                    tokens.next(),
                    tokens.next(),
                )
                else {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "arc record must have exactly the form: a <tail> <head> <lower> <upper> <cost>",
                    ));
                };
                if tokens.next().is_some() {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "arc record must have exactly the form: a <tail> <head> <lower> <upper> <cost>",
                    ));
                }

                let tail: u32 = parse_number(tail_token, line_number, "arc tail")?;
                let head: u32 = parse_number(head_token, line_number, "arc head")?;
                if tail == 0 || tail > nodes || head == 0 || head > nodes {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        format!(
                            "arc endpoints ({tail}, {head}) must both be in the 1..={nodes} range"
                        ),
                    ));
                }
                let lower_bound: i64 = parse_number(lower_token, line_number, "arc lower bound")?;
                let upper_bound: i64 = parse_number(upper_token, line_number, "arc upper bound")?;
                if lower_bound > upper_bound {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        format!("arc lower bound {lower_bound} exceeds upper bound {upper_bound}"),
                    ));
                }
                let unit_cost: i32 = parse_number(cost_token, line_number, "arc unit cost")?;
                let ordinal = u32::try_from(arcs.len()).map_err(|_| {
                    DimacsMcfParseError::new(line_number, "arc ordinal exceeds u32")
                })?;
                arcs.push(DimacsMcfArc {
                    ordinal,
                    tail,
                    head,
                    lower_bound,
                    upper_bound,
                    unit_cost,
                });
                if arcs.len() > declared_arcs as usize {
                    return Err(DimacsMcfParseError::new(
                        line_number,
                        "observed more arc records than declared in the problem line",
                    ));
                }
            }
            other => {
                return Err(DimacsMcfParseError::new(
                    line_number,
                    format!("unsupported or case-mismatched record type {other:?}"),
                ));
            }
        }
    }
    let (node_count, declared_arc_count) =
        problem.ok_or_else(|| DimacsMcfParseError::new(0, "missing DIMACS problem line"))?;
    if arcs.len() != declared_arc_count as usize {
        return Err(DimacsMcfParseError::new(
            0,
            format!(
                "declared {declared_arc_count} arcs but parsed {} arc records",
                arcs.len()
            ),
        ));
    }

    let total_supply = supplies
        .iter()
        .skip(1)
        .map(|value| i128::from(*value))
        .sum();
    let raw_sha256 = format!("sha256:{:x}", Sha256::digest(input));
    let mut instance = DimacsMcfInstance {
        format_version: DIMACS_MCF_FORMAT_VERSION.to_string(),
        node_count,
        declared_arc_count,
        node_supplies: supplies,
        arcs,
        nonzero_supply_record_count: supply_records,
        raw_input_bytes: input.len(),
        estimated_primary_allocation_bytes: estimated_allocation,
        limits,
        raw_sha256,
        canonical_sha256: String::new(),
        total_supply,
        supply_total_is_zero: total_supply == 0,
    };
    instance.canonical_sha256 = instance.canonical_digest();
    Ok(instance)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_PARALLEL: &str =
        "c parallel arcs stay separate\np min 2 2\nn 1 5\nn 2 -5\na 1 2 0 5 3\na 1 2 0 5 -1\n";

    fn parse(text: &str) -> Result<DimacsMcfInstance, DimacsMcfParseError> {
        parse_dimacs_mcf(text.as_bytes(), ImportLimits::default())
    }

    #[test]
    fn parses_parallel_arcs_without_aggregation_and_binds_both_digests() {
        let instance = parse(VALID_PARALLEL).unwrap();
        assert_eq!(instance.node_count, 2);
        assert_eq!(instance.declared_arc_count, 2);
        assert_eq!(instance.nonzero_supply_record_count, 2);
        assert_eq!(instance.node_supplies, vec![0, 5, -5]);
        assert_eq!(instance.arcs.len(), 2);
        assert_eq!(instance.arcs[0].ordinal, 0);
        assert_eq!(instance.arcs[1].ordinal, 1);
        assert_eq!(instance.arcs[0].tail, instance.arcs[1].tail);
        assert_eq!(instance.arcs[0].head, instance.arcs[1].head);
        assert_ne!(instance.arcs[0].unit_cost, instance.arcs[1].unit_cost);
        assert!(instance.supply_total_is_zero);
        assert!(instance.has_negative_costs());
        assert_eq!(instance.raw_sha256.len(), 71);
        assert_eq!(instance.canonical_sha256.len(), 71);
    }

    #[test]
    fn canonical_identity_ignores_comments_whitespace_and_node_record_order() {
        let first = "c source\np min 2 2\nn 1 5\nn 2 -5\na 1 2 0 5 3\na 1 2 0 5 -1\n";
        let second =
            "c a different comment\np   min  2 2\nn 2 -5\nn 1 5\na 1 2 0 5 3\na 1 2 0 5 -1\n";
        let a = parse(first).unwrap();
        let b = parse(second).unwrap();
        assert_ne!(a.raw_sha256, b.raw_sha256);
        assert_eq!(a.canonical_sha256, b.canonical_sha256);
    }

    #[test]
    fn canonical_identity_ignores_arc_record_order_without_merging_parallel_arcs() {
        let first = "p min 2 3\nn 1 9\nn 2 -9\na 1 2 0 4 3\na 1 2 0 4 3\na 1 2 0 5 -1\n";
        let second = "p min 2 3\nn 2 -9\nn 1 9\na 1 2 0 5 -1\na 1 2 0 4 3\na 1 2 0 4 3\n";
        let a = parse(first).unwrap();
        let b = parse(second).unwrap();
        assert_ne!(a.raw_sha256, b.raw_sha256);
        assert_eq!(a.canonical_sha256, b.canonical_sha256);
        assert_eq!(a.arcs.len(), 3);
        assert_eq!(b.arcs.len(), 3);
        assert_eq!(a.arcs.iter().filter(|arc| arc.upper_bound == 4).count(), 2);
        assert_eq!(b.arcs.iter().filter(|arc| arc.upper_bound == 4).count(), 2);
    }

    #[test]
    fn parses_signed_i64_bounds_and_i32_cost_extremes_exactly() {
        let text = format!(
            "p min 2 1\nn 1 {}\nn 2 {}\na 1 2 {} {} {}\n",
            i64::MAX,
            i64::MIN,
            i64::MIN,
            i64::MAX,
            i32::MIN
        );
        let instance = parse(&text).unwrap();
        assert_eq!(instance.arcs[0].lower_bound, i64::MIN);
        assert_eq!(instance.arcs[0].upper_bound, i64::MAX);
        assert_eq!(instance.arcs[0].unit_cost, i32::MIN);
        assert_eq!(instance.arcs[0].residual_capacity().unwrap(), u64::MAX);
        assert!(!instance.supply_total_is_zero);
        assert_eq!(instance.total_supply, -1);
        assert!(instance.has_signed_capacity_bounds());
    }

    #[test]
    fn negative_upper_bound_is_preserved_when_lower_bound_is_valid() {
        let instance = parse("p min 2 1\nn 1 1\nn 2 -1\na 1 2 -5 -1 -7\n").unwrap();
        assert_eq!(instance.arcs[0].lower_bound, -5);
        assert_eq!(instance.arcs[0].upper_bound, -1);
        assert_eq!(instance.arcs[0].residual_capacity().unwrap(), 4);
    }

    #[test]
    fn imbalance_is_reported_as_data_not_mislabeled_as_parse_failure() {
        let instance = parse("p min 2 1\nn 1 5\na 1 2 0 5 1\n").unwrap();
        assert!(!instance.supply_total_is_zero);
        assert_eq!(instance.total_supply, 5);
    }

    #[test]
    fn rejects_lower_bound_above_upper_bound() {
        let error = parse("p min 2 1\na 1 2 5 4 1\n").unwrap_err();
        assert!(error.message.contains("exceeds upper bound"));
    }

    #[test]
    fn rejects_duplicate_and_zero_supply_node_records() {
        let duplicate = parse("p min 2 1\nn 1 1\nn 1 2\na 1 2 0 1 0\n").unwrap_err();
        assert!(duplicate.message.contains("duplicate node supply"));
        let zero = parse("p min 2 0\nn 1 0\n").unwrap_err();
        assert!(zero.message.contains("nonzero supplies"));
    }

    #[test]
    fn rejects_nodes_after_first_arc_and_arc_count_mismatch() {
        let order = parse("p min 2 1\na 1 2 0 1 0\nn 1 1\n").unwrap_err();
        assert!(order.message.contains("must precede all arc records"));
        let count = parse("p min 2 2\na 1 2 0 1 0\n").unwrap_err();
        assert!(count.message.contains("but parsed 1 arc records"));
    }

    #[test]
    fn rejects_out_of_range_one_based_node_ids() {
        let error = parse("p min 2 1\na 0 2 0 1 0\n").unwrap_err();
        assert!(error.message.contains("endpoints"));
        let error = parse("p min 2 1\nn 3 1\na 1 2 0 1 0\n").unwrap_err();
        assert!(error.message.contains("node ID 3"));
    }

    #[test]
    fn rejects_non_ascii_unknown_records_and_extra_fields() {
        let non_ascii = parse_dimacs_mcf("p min 2 0\nc café\n".as_bytes(), ImportLimits::default())
            .unwrap_err();
        assert!(non_ascii.message.contains("not ASCII"));
        let unknown = parse("P min 2 0\n").unwrap_err();
        assert!(unknown.message.contains("case-mismatched"));
        let extra = parse("p min 2 1\na 1 2 0 1 0 extra\n").unwrap_err();
        assert!(extra.message.contains("exactly the form"));
    }

    #[test]
    fn preflights_node_arc_and_memory_budgets_before_allocating() {
        let nodes = parse_dimacs_mcf(
            b"p min 3 0\n",
            ImportLimits {
                max_nodes: 2,
                ..ImportLimits::default()
            },
        )
        .unwrap_err();
        assert!(nodes.message.contains("exceeds configured limit"));

        let arcs = parse_dimacs_mcf(
            b"p min 2 2\n",
            ImportLimits {
                max_arcs: 1,
                ..ImportLimits::default()
            },
        )
        .unwrap_err();
        assert!(arcs.message.contains("exceeds configured limit"));

        let memory = parse_dimacs_mcf(
            b"p min 2 1\n",
            ImportLimits {
                max_working_bytes: 1,
                ..ImportLimits::default()
            },
        )
        .unwrap_err();
        assert!(memory.message.contains("exceeds configured budget"));

        let bytes = parse_dimacs_mcf(
            b"p min 2 0\n",
            ImportLimits {
                max_input_bytes: 4,
                ..ImportLimits::default()
            },
        )
        .unwrap_err();
        assert!(bytes.message.contains("input byte length"));
    }

    #[test]
    fn portable_import_receipt_round_trips_and_rejects_unknown_fields() {
        let instance = parse(VALID_PARALLEL).unwrap();
        let receipt = instance.import_receipt();
        let bytes = serde_json::to_vec(&receipt).unwrap();
        let decoded: DimacsMcfImportReceipt = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, receipt);
        verify_dimacs_mcf_import_receipt(VALID_PARALLEL.as_bytes(), &decoded).unwrap();

        let mut unknown = serde_json::to_value(&receipt).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<DimacsMcfImportReceipt>(unknown).is_err());
    }

    #[test]
    fn import_receipt_binds_raw_bytes_and_canonical_model_separately() {
        let original = parse(VALID_PARALLEL).unwrap().import_receipt();
        let with_comment = format!("c added provenance comment\n{VALID_PARALLEL}");
        let parsed_with_comment = parse(&with_comment).unwrap();
        let comment_receipt = parsed_with_comment.import_receipt();
        assert_ne!(original.raw_sha256, comment_receipt.raw_sha256);
        assert_eq!(original.canonical_sha256, comment_receipt.canonical_sha256);
        assert!(
            verify_dimacs_mcf_import_receipt(with_comment.as_bytes(), &original)
                .unwrap_err()
                .message
                .contains("does not match")
        );
    }

    #[test]
    fn published_import_receipt_schema_is_valid_and_versioned() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/dimacs-mcf-import-receipt-v0.1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            DIMACS_MCF_IMPORT_RECEIPT_SCHEMA
        );
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn import_receipt_preserves_aggregate_supply_beyond_i64_range() {
        let text = format!(
            "p min 4 0\nn 1 {}\nn 2 {}\nn 3 {}\n",
            i64::MAX,
            i64::MAX,
            i64::MAX
        );
        let instance = parse(&text).unwrap();
        let receipt = instance.import_receipt();
        let expected = (i128::from(i64::MAX) * 3).to_string();
        assert_eq!(receipt.total_supply_decimal, expected);
        let encoded = serde_json::to_vec(&receipt).unwrap();
        let decoded: DimacsMcfImportReceipt = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, receipt);
    }

    #[test]
    fn empty_zero_node_instance_is_structurally_preserved() {
        let instance = parse("p min 0 0\n").unwrap();
        assert_eq!(instance.node_count, 0);
        assert_eq!(instance.node_supplies, vec![0]);
        assert!(instance.arcs.is_empty());
        assert!(instance.supply_total_is_zero);
    }
}
