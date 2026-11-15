// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dependency-free normalizer for the public ScheduleOpt FJSPLib text formats.
//!
//! Supported inputs:
//! - FJSP: per-job operation chains; machine identifiers are 1-based.
//! - FAJSP: explicit precedence graph; machine identifiers are 0-based.
//!
//! This crate parses and validates benchmark data. It is not a scheduler.

use std::collections::{BTreeSet, VecDeque};
use std::fmt;

pub const PARSER_VERSION: &str = "fjsplib-text-adapter/0.1";
pub const FJSPLIB_SOURCE: &str = "https://scheduleopt.github.io/benchmarks/fjsplib";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Fjsp,
    Fajsp,
}

impl SourceFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fjsp => "fjsp",
            Self::Fajsp => "fajsp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineAlternative {
    pub machine_id: usize,
    pub processing_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlexibleOperation {
    pub id: usize,
    pub job_id: Option<usize>,
    pub position_in_job: Option<usize>,
    pub alternatives: Vec<MachineAlternative>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Precedence {
    pub before: usize,
    pub after: usize,
    pub job_id: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlexibleInstance {
    pub parser_version: &'static str,
    pub source: &'static str,
    pub source_format: SourceFormat,
    pub instance_id: String,
    pub source_family: Option<String>,
    pub machine_count: usize,
    pub declared_job_count: Option<usize>,
    pub average_flexibility: Option<f64>,
    pub operations: Vec<FlexibleOperation>,
    pub precedences: Vec<Precedence>,
    pub canonical_fingerprint_fnv1a64: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    EmptyInstanceId,
    EmptyInput,
    MissingHeader,
    InvalidHeader,
    ZeroJobs,
    ZeroMachines,
    ZeroOperations,
    ZeroOptions,
    MissingToken,
    InvalidInteger,
    InvalidFloat,
    NonFiniteAverageFlexibility,
    NegativeAverageFlexibility,
    RecordTokenCountMismatch,
    TrailingToken,
    MachineOutOfRange,
    ZeroProcessingTime,
    DuplicateMachineAlternative,
    PrecedenceEndpointOutOfRange,
    SelfPrecedence,
    DuplicatePrecedence,
    PrecedenceCycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: Option<usize>,
    pub kind: ParseErrorKind,
    pub detail: String,
}

impl ParseError {
    fn new(line: Option<usize>, kind: ParseErrorKind, detail: impl Into<String>) -> Self {
        Self { line, kind, detail: detail.into() }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "line {}: {:?}: {}", line, self.kind, self.detail),
            None => write!(formatter, "{:?}: {}", self.kind, self.detail),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Copy)]
struct DataLine<'a> {
    number: usize,
    content: &'a str,
}

fn data_lines(input: &str) -> Vec<DataLine<'_>> {
    input.lines().enumerate().filter_map(|(index, line)| {
        let content = line.split('#').next().unwrap_or("").trim();
        (!content.is_empty()).then_some(DataLine {
            number: index + 1,
            content,
        })
    }).collect()
}

fn parse_usize(token: Option<&&str>, line: usize, field: &str) -> Result<usize, ParseError> {
    let value = token.ok_or_else(|| ParseError::new(
        Some(line), ParseErrorKind::MissingToken, format!("missing {field}")
    ))?;
    value.parse::<usize>().map_err(|_| ParseError::new(
        Some(line), ParseErrorKind::InvalidInteger, format!("invalid {field}: {value}")
    ))
}

fn parse_u64(token: Option<&&str>, line: usize, field: &str) -> Result<u64, ParseError> {
    let value = token.ok_or_else(|| ParseError::new(
        Some(line), ParseErrorKind::MissingToken, format!("missing {field}")
    ))?;
    value.parse::<u64>().map_err(|_| ParseError::new(
        Some(line), ParseErrorKind::InvalidInteger, format!("invalid {field}: {value}")
    ))
}

fn parse_f64(token: Option<&&str>, line: usize, field: &str) -> Result<f64, ParseError> {
    let value = token.ok_or_else(|| ParseError::new(
        Some(line), ParseErrorKind::MissingToken, format!("missing {field}")
    ))?;
    value.parse::<f64>().map_err(|_| ParseError::new(
        Some(line), ParseErrorKind::InvalidFloat, format!("invalid {field}: {value}")
    ))
}

fn parse_options(
    tokens: &[&str],
    mut cursor: usize,
    line: usize,
    machine_count: usize,
    one_based_machine_ids: bool,
) -> Result<(Vec<MachineAlternative>, usize), ParseError> {
    let option_count = parse_usize(tokens.get(cursor), line, "alternative-machine count")?;
    cursor += 1;
    if option_count == 0 {
        return Err(ParseError::new(Some(line), ParseErrorKind::ZeroOptions, "an operation must have at least one eligible machine"));
    }

    let required = option_count.checked_mul(2).and_then(|count| cursor.checked_add(count))
        .ok_or_else(|| ParseError::new(Some(line), ParseErrorKind::RecordTokenCountMismatch, "alternative count overflow"))?;
    if required > tokens.len() {
        return Err(ParseError::new(Some(line), ParseErrorKind::MissingToken, "machine/duration pair is incomplete"));
    }

    let mut alternatives = Vec::with_capacity(option_count);
    let mut seen_machines = BTreeSet::new();
    for _ in 0..option_count {
        let raw_machine = parse_usize(tokens.get(cursor), line, "machine identifier")?;
        let processing_ticks = parse_u64(tokens.get(cursor + 1), line, "processing duration")?;
        cursor += 2;

        let machine_id = if one_based_machine_ids {
            raw_machine.checked_sub(1).ok_or_else(|| ParseError::new(
                Some(line), ParseErrorKind::MachineOutOfRange, "FJSP machine identifiers start at 1"
            ))?
        } else {
            raw_machine
        };
        if machine_id >= machine_count {
            return Err(ParseError::new(
                Some(line),
                ParseErrorKind::MachineOutOfRange,
                format!("machine {raw_machine} is outside the declared machine range"),
            ));
        }
        if processing_ticks == 0 {
            return Err(ParseError::new(Some(line), ParseErrorKind::ZeroProcessingTime, "processing duration must be positive"));
        }
        if !seen_machines.insert(machine_id) {
            return Err(ParseError::new(
                Some(line),
                ParseErrorKind::DuplicateMachineAlternative,
                format!("machine {machine_id} appears more than once for an operation"),
            ));
        }
        alternatives.push(MachineAlternative { machine_id, processing_ticks });
    }
    Ok((alternatives, cursor))
}

fn validate_precedence_graph(
    operation_count: usize,
    precedences: &[Precedence],
) -> Result<(), ParseError> {
    let mut indegree = vec![0usize; operation_count];
    let mut outgoing = vec![Vec::<usize>::new(); operation_count];
    let mut seen = BTreeSet::new();

    for edge in precedences {
        if edge.before >= operation_count || edge.after >= operation_count {
            return Err(ParseError::new(
                None,
                ParseErrorKind::PrecedenceEndpointOutOfRange,
                format!("edge {} -> {} references an absent operation", edge.before, edge.after),
            ));
        }
        if edge.before == edge.after {
            return Err(ParseError::new(None, ParseErrorKind::SelfPrecedence, format!("operation {} precedes itself", edge.before)));
        }
        if !seen.insert((edge.before, edge.after, edge.job_id)) {
            return Err(ParseError::new(
                None,
                ParseErrorKind::DuplicatePrecedence,
                format!("duplicate precedence edge {} -> {}", edge.before, edge.after),
            ));
        }
        outgoing[edge.before].push(edge.after);
        indegree[edge.after] += 1;
    }

    let mut ready = VecDeque::new();
    for (operation, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(operation);
        }
    }
    let mut visited = 0usize;
    while let Some(operation) = ready.pop_front() {
        visited += 1;
        for successor in &outgoing[operation] {
            indegree[*successor] -= 1;
            if indegree[*successor] == 0 {
                ready.push_back(*successor);
            }
        }
    }
    if visited != operation_count {
        return Err(ParseError::new(None, ParseErrorKind::PrecedenceCycle, "precedence graph contains a directed cycle"));
    }
    Ok(())
}

fn canonical_fingerprint(
    format: SourceFormat,
    instance_id: &str,
    source_family: Option<&str>,
    machine_count: usize,
    declared_job_count: Option<usize>,
    average_flexibility: Option<f64>,
    operations: &[FlexibleOperation],
    precedences: &[Precedence],
) -> String {
    let mut canonical = format!(
        "{}|{}|{}|{}|{}|{}|",
        format.as_str(),
        instance_id,
        source_family.unwrap_or(""),
        machine_count,
        declared_job_count.map_or_else(|| "-".to_owned(), |value| value.to_string()),
        average_flexibility.map_or_else(|| "-".to_owned(), |value| format!("{value:.12}")),
    );
    for operation in operations {
        canonical.push_str(&format!(
            "O{}:{}:{}|",
            operation.id,
            operation.job_id.map_or_else(|| "-".to_owned(), |value| value.to_string()),
            operation.position_in_job.map_or_else(|| "-".to_owned(), |value| value.to_string())
        ));
        for alternative in &operation.alternatives {
            canonical.push_str(&format!("{}:{};", alternative.machine_id, alternative.processing_ticks));
        }
        canonical.push('|');
    }
    for edge in precedences {
        canonical.push_str(&format!(
            "P{}:{}:{}|",
            edge.before,
            edge.after,
            edge.job_id.map_or_else(|| "-".to_owned(), |value| value.to_string())
        ));
    }

    // FNV-1a is a stable non-cryptographic fingerprint, not a signature.
    let mut hash = 0xcbf29ce484222325u64;
    for byte in canonical.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn build_instance(
    format: SourceFormat,
    instance_id: &str,
    source_family: Option<&str>,
    machine_count: usize,
    declared_job_count: Option<usize>,
    average_flexibility: Option<f64>,
    operations: Vec<FlexibleOperation>,
    precedences: Vec<Precedence>,
) -> Result<FlexibleInstance, ParseError> {
    if instance_id.trim().is_empty() {
        return Err(ParseError::new(None, ParseErrorKind::EmptyInstanceId, "instance identifier is empty"));
    }
    if machine_count == 0 {
        return Err(ParseError::new(None, ParseErrorKind::ZeroMachines, "machine count must be positive"));
    }
    if operations.is_empty() {
        return Err(ParseError::new(None, ParseErrorKind::ZeroOperations, "operation count must be positive"));
    }
    validate_precedence_graph(operations.len(), &precedences)?;
    let fingerprint = canonical_fingerprint(
        format,
        instance_id,
        source_family,
        machine_count,
        declared_job_count,
        average_flexibility,
        &operations,
        &precedences,
    );
    Ok(FlexibleInstance {
        parser_version: PARSER_VERSION,
        source: FJSPLIB_SOURCE,
        source_format: format,
        instance_id: instance_id.to_owned(),
        source_family: source_family.map(str::to_owned),
        machine_count,
        declared_job_count,
        average_flexibility,
        operations,
        precedences,
        canonical_fingerprint_fnv1a64: fingerprint,
    })
}

/// Parse FJSPLib text FJSP: one ordered operation chain per job,
/// with 1-based machine identifiers.
pub fn parse_fjsp(
    instance_id: &str,
    source_family: Option<&str>,
    input: &str,
) -> Result<FlexibleInstance, ParseError> {
    let lines = data_lines(input);
    let header = lines.first().ok_or_else(|| ParseError::new(None, ParseErrorKind::EmptyInput, "no data lines"))?;
    let header_tokens: Vec<&str> = header.content.split_whitespace().collect();
    if header_tokens.len() != 3 {
        return Err(ParseError::new(Some(header.number), ParseErrorKind::InvalidHeader, "FJSP header must contain jobs, machines, average flexibility"));
    }

    let job_count = parse_usize(header_tokens.get(0), header.number, "job count")?;
    let machine_count = parse_usize(header_tokens.get(1), header.number, "machine count")?;
    let average_flexibility = parse_f64(header_tokens.get(2), header.number, "average flexibility")?;
    if job_count == 0 {
        return Err(ParseError::new(Some(header.number), ParseErrorKind::ZeroJobs, "job count must be positive"));
    }
    if machine_count == 0 {
        return Err(ParseError::new(Some(header.number), ParseErrorKind::ZeroMachines, "machine count must be positive"));
    }
    if !average_flexibility.is_finite() {
        return Err(ParseError::new(Some(header.number), ParseErrorKind::NonFiniteAverageFlexibility, "average flexibility must be finite"));
    }
    if average_flexibility < 0.0 {
        return Err(ParseError::new(Some(header.number), ParseErrorKind::NegativeAverageFlexibility, "average flexibility must be non-negative"));
    }
    if lines.len() != job_count + 1 {
        return Err(ParseError::new(None, ParseErrorKind::RecordTokenCountMismatch,
            format!("header declares {job_count} jobs but {} job records were supplied", lines.len().saturating_sub(1))));
    }

    let mut operations = Vec::new();
    let mut precedences = Vec::new();
    for (job_id, line) in lines.iter().skip(1).enumerate() {
        let tokens: Vec<&str> = line.content.split_whitespace().collect();
        let operation_count = parse_usize(tokens.get(0), line.number, "operation count")?;
        if operation_count == 0 {
            return Err(ParseError::new(Some(line.number), ParseErrorKind::ZeroOperations, "job must contain at least one operation"));
        }
        let mut cursor = 1usize;
        let mut job_operation_ids = Vec::with_capacity(operation_count);
        for position in 0..operation_count {
            let operation_id = operations.len();
            let (alternatives, next) = parse_options(&tokens, cursor, line.number, machine_count, true)?;
            cursor = next;
            operations.push(FlexibleOperation {
                id: operation_id,
                job_id: Some(job_id),
                position_in_job: Some(position),
                alternatives,
            });
            job_operation_ids.push(operation_id);
        }
        if cursor != tokens.len() {
            return Err(ParseError::new(Some(line.number), ParseErrorKind::TrailingToken, "unexpected token after job route"));
        }
        for pair in job_operation_ids.windows(2) {
            precedences.push(Precedence {
                before: pair[0],
                after: pair[1],
                job_id: Some(job_id),
            });
        }
    }

    build_instance(
        SourceFormat::Fjsp,
        instance_id,
        source_family,
        machine_count,
        Some(job_count),
        Some(average_flexibility),
        operations,
        precedences,
    )
}

/// Parse FJSPLib text FAJSP: operations plus explicit precedence edges,
/// with 0-based machine identifiers.
pub fn parse_fajsp(
    instance_id: &str,
    source_family: Option<&str>,
    input: &str,
) -> Result<FlexibleInstance, ParseError> {
    let lines = data_lines(input);
    let header = lines.first().ok_or_else(|| ParseError::new(None, ParseErrorKind::EmptyInput, "no data lines"))?;
    let header_tokens: Vec<&str> = header.content.split_whitespace().collect();
    if header_tokens.len() != 3 {
        return Err(ParseError::new(Some(header.number), ParseErrorKind::InvalidHeader, "FAJSP header must contain operations, precedence edges, machines"));
    }

    let operation_count = parse_usize(header_tokens.get(0), header.number, "operation count")?;
    let precedence_count = parse_usize(header_tokens.get(1), header.number, "precedence count")?;
    let machine_count = parse_usize(header_tokens.get(2), header.number, "machine count")?;
    if operation_count == 0 {
        return Err(ParseError::new(Some(header.number), ParseErrorKind::ZeroOperations, "operation count must be positive"));
    }
    if machine_count == 0 {
        return Err(ParseError::new(Some(header.number), ParseErrorKind::ZeroMachines, "machine count must be positive"));
    }
    if lines.len() != 1 + precedence_count + operation_count {
        return Err(ParseError::new(None, ParseErrorKind::RecordTokenCountMismatch,
            format!("header declares {precedence_count} precedence and {operation_count} operation records but {} records were supplied", lines.len().saturating_sub(1))));
    }

    let mut precedences = Vec::with_capacity(precedence_count);
    let mut job_ids = BTreeSet::new();
    for line in lines.iter().skip(1).take(precedence_count) {
        let tokens: Vec<&str> = line.content.split_whitespace().collect();
        if tokens.len() != 3 {
            return Err(ParseError::new(Some(line.number), ParseErrorKind::RecordTokenCountMismatch, "FAJSP precedence record must contain before, after, job"));
        }
        let before = parse_usize(tokens.get(0), line.number, "precedence before-operation")?;
        let after = parse_usize(tokens.get(1), line.number, "precedence after-operation")?;
        let job_id = parse_usize(tokens.get(2), line.number, "precedence job identifier")?;
        job_ids.insert(job_id);
        precedences.push(Precedence { before, after, job_id: Some(job_id) });
    }

    let mut operations = Vec::with_capacity(operation_count);
    for line in lines.iter().skip(1 + precedence_count) {
        let tokens: Vec<&str> = line.content.split_whitespace().collect();
        let (alternatives, cursor) = parse_options(&tokens, 0, line.number, machine_count, false)?;
        if cursor != tokens.len() {
            return Err(ParseError::new(Some(line.number), ParseErrorKind::TrailingToken, "unexpected token after operation alternatives"));
        }
        operations.push(FlexibleOperation {
            id: operations.len(),
            job_id: None,
            position_in_job: None,
            alternatives,
        });
    }

    build_instance(
        SourceFormat::Fajsp,
        instance_id,
        source_family,
        machine_count,
        Some(job_ids.len()),
        None,
        operations,
        precedences,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FJSP_FATTAHI1: &str = "2 2 2
2 2 1 25 2 37 2 1 32 2 24
2 2 1 45 2 65 2 1 21 2 65
";

    const FAJSP_FATTAHI1: &str = "4 2 2
0 1 0
2 3 1
2 0 25 1 37
2 0 32 1 24
2 0 45 1 65
2 0 21 1 65
";

    #[test]
    fn fjsp_fattahi_example_normalizes_one_based_machine_ids() {
        let instance = parse_fjsp("fattahi1", Some("fattahi"), FJSP_FATTAHI1).unwrap();
        assert_eq!(instance.source_format, SourceFormat::Fjsp);
        assert_eq!(instance.declared_job_count, Some(2));
        assert_eq!(instance.machine_count, 2);
        assert_eq!(instance.operations.len(), 4);
        assert_eq!(instance.precedences.len(), 2);
        assert_eq!(instance.operations[0].alternatives, vec![
            MachineAlternative { machine_id: 0, processing_ticks: 25 },
            MachineAlternative { machine_id: 1, processing_ticks: 37 },
        ]);
        assert_eq!(instance.precedences[0], Precedence { before: 0, after: 1, job_id: Some(0) });
    }

    #[test]
    fn fajsp_fattahi_example_preserves_explicit_precedence_graph() {
        let instance = parse_fajsp("fattahi1", Some("fattahi"), FAJSP_FATTAHI1).unwrap();
        assert_eq!(instance.source_format, SourceFormat::Fajsp);
        assert_eq!(instance.declared_job_count, Some(2));
        assert_eq!(instance.machine_count, 2);
        assert_eq!(instance.operations.len(), 4);
        assert_eq!(instance.precedences, vec![
            Precedence { before: 0, after: 1, job_id: Some(0) },
            Precedence { before: 2, after: 3, job_id: Some(1) },
        ]);
        assert_eq!(instance.operations[0].alternatives[0], MachineAlternative { machine_id: 0, processing_ticks: 25 });
    }

    #[test]
    fn same_semantics_import_to_deterministic_normalized_records() {
        let fjsp = parse_fjsp("fattahi1", Some("fattahi"), FJSP_FATTAHI1).unwrap();
        let fajsp = parse_fajsp("fattahi1", Some("fattahi"), FAJSP_FATTAHI1).unwrap();
        assert_eq!(fjsp.operations, fajsp.operations.iter().map(|operation| FlexibleOperation {
            id: operation.id,
            job_id: Some(if operation.id < 2 { 0 } else { 1 }),
            position_in_job: Some(operation.id % 2),
            alternatives: operation.alternatives.clone(),
        }).collect::<Vec<_>>());
        assert_ne!(fjsp.canonical_fingerprint_fnv1a64, fajsp.canonical_fingerprint_fnv1a64);
    }

    #[test]
    fn duplicate_alternative_machine_is_rejected() {
        let input = "1 2 2
1 2 1 5 1 7
";
        let error = parse_fjsp("bad", None, input).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::DuplicateMachineAlternative);
    }

    #[test]
    fn out_of_range_machine_is_rejected() {
        let input = "1 2 1
1 1 3 5
";
        let error = parse_fjsp("bad", None, input).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::MachineOutOfRange);
    }

    #[test]
    fn zero_duration_is_rejected() {
        let input = "1 2 1
1 1 1 0
";
        let error = parse_fjsp("bad", None, input).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::ZeroProcessingTime);
    }

    #[test]
    fn incomplete_record_is_rejected() {
        let input = "1 2 1
1 2 1 5
";
        let error = parse_fjsp("bad", None, input).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::MissingToken);
    }

    #[test]
    fn fajsp_precedence_cycle_is_rejected() {
        let input = "2 2 2
0 1 0
1 0 0
1 0 5
1 1 5
";
        let error = parse_fajsp("cycle", None, input).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::PrecedenceCycle);
    }

    #[test]
    fn fajsp_unknown_endpoint_is_rejected() {
        let input = "2 1 2
0 2 0
1 0 5
1 1 5
";
        let error = parse_fajsp("bad-edge", None, input).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::PrecedenceEndpointOutOfRange);
    }

    #[test]
    fn comment_and_blank_lines_are_ignored() {
        let input = "# FJSPLib fixture
1 2 1

1 1 1 5 # one operation
";
        let instance = parse_fjsp("comments", None, input).unwrap();
        assert_eq!(instance.operations.len(), 1);
    }

    #[test]
    fn empty_instance_id_is_rejected() {
        let error = parse_fjsp("", None, FJSP_FATTAHI1).unwrap_err();
        assert_eq!(error.kind, ParseErrorKind::EmptyInstanceId);
    }

    #[test]
    fn average_flexibility_must_be_finite_and_nonnegative() {
        let nan = "1 1 NaN
1 1 1 5
";
        let neg = "1 1 -1
1 1 1 5
";
        assert_eq!(parse_fjsp("nan", None, nan).unwrap_err().kind, ParseErrorKind::NonFiniteAverageFlexibility);
        assert_eq!(parse_fjsp("negative", None, neg).unwrap_err().kind, ParseErrorKind::NegativeAverageFlexibility);
    }

    #[test]
    fn fingerprint_is_reproducible_for_same_source_and_metadata() {
        let a = parse_fjsp("fattahi1", Some("fattahi"), FJSP_FATTAHI1).unwrap();
        let b = parse_fjsp("fattahi1", Some("fattahi"), FJSP_FATTAHI1).unwrap();
        assert_eq!(a.canonical_fingerprint_fnv1a64, b.canonical_fingerprint_fnv1a64);
    }
}
