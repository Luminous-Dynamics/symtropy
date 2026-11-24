// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Lossless importer for the NYU continuous-collision-detection query formats.
//!
//! Rational coordinates remain numerator/denominator strings. The importer
//! deliberately does not convert them to f64 or perform geometric predicates.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryKind {
    EdgeEdge,
    VertexFace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexRole {
    EdgeAStart0,
    EdgeAStart1,
    EdgeBStart0,
    EdgeBStart1,
    EdgeAEnd0,
    EdgeAEnd1,
    EdgeBEnd0,
    EdgeBEnd1,
    VertexStart,
    Face0Start,
    Face1Start,
    Face2Start,
    VertexEnd,
    Face0End,
    Face1End,
    Face2End,
}

impl QueryKind {
    /// Row order in the eight-row source query record.
    #[must_use]
    pub fn row_roles(self) -> [VertexRole; 8] {
        match self {
            Self::EdgeEdge => [
                VertexRole::EdgeAStart0,
                VertexRole::EdgeAStart1,
                VertexRole::EdgeBStart0,
                VertexRole::EdgeBStart1,
                VertexRole::EdgeAEnd0,
                VertexRole::EdgeAEnd1,
                VertexRole::EdgeBEnd0,
                VertexRole::EdgeBEnd1,
            ],
            Self::VertexFace => [
                VertexRole::VertexStart,
                VertexRole::Face0Start,
                VertexRole::Face1Start,
                VertexRole::Face2Start,
                VertexRole::VertexEnd,
                VertexRole::Face0End,
                VertexRole::Face1End,
                VertexRole::Face2End,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportError {
    EmptyInput,
    EmptyRow,
    UnsupportedColumnCount,
    InconsistentColumnCount,
    IncompleteEightRowQuery,
    InvalidNumerator,
    InvalidDenominator,
    ZeroDenominator,
    InvalidBoolean,
    InconsistentInlineGroundTruth,
    GroundTruthCountMismatch,
    GroundTruthAlreadyPresent,
    InvalidJson,
    InvalidBoxPair,
    IntegerOverflow,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ImportError {}

/// Exact decimal-integer rational coordinate. Lexemes are preserved verbatim;
/// no fixed-width integer conversion and no floating-point rounding occurs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rational {
    numerator: String,
    denominator: String,
}

impl Rational {
    pub fn new(
        numerator: impl Into<String>,
        denominator: impl Into<String>,
    ) -> Result<Self, ImportError> {
        let numerator = numerator.into();
        let denominator = denominator.into();
        if !valid_integer(&numerator, true) {
            return Err(ImportError::InvalidNumerator);
        }
        if !valid_integer(&denominator, false) {
            return Err(ImportError::InvalidDenominator);
        }
        if denominator.bytes().all(|byte| byte == b'0') {
            return Err(ImportError::ZeroDenominator);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    #[must_use]
    pub fn numerator(&self) -> &str {
        &self.numerator
    }

    #[must_use]
    pub fn denominator(&self) -> &str {
        &self.denominator
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Point3 {
    x: Rational,
    y: Rational,
    z: Rational,
}

impl Point3 {
    pub fn new(x: Rational, y: Rational, z: Rational) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn x(&self) -> &Rational {
        &self.x
    }

    #[must_use]
    pub fn y(&self) -> &Rational {
        &self.y
    }

    #[must_use]
    pub fn z(&self) -> &Rational {
        &self.z
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CcdQuery {
    index: usize,
    kind: QueryKind,
    vertices: [Point3; 8],
    inline_ground_truth: Option<bool>,
}

impl CcdQuery {
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    #[must_use]
    pub fn kind(&self) -> QueryKind {
        self.kind
    }

    #[must_use]
    pub fn row_roles(&self) -> [VertexRole; 8] {
        self.kind.row_roles()
    }

    #[must_use]
    pub fn vertices(&self) -> &[Point3; 8] {
        &self.vertices
    }

    #[must_use]
    pub fn inline_ground_truth(&self) -> Option<bool> {
        self.inline_ground_truth
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoxPair {
    first: u64,
    second: u64,
}

impl BoxPair {
    #[must_use]
    pub fn new(first: u64, second: u64) -> Option<Self> {
        (first != second).then_some(Self {
            first: first.min(second),
            second: first.max(second),
        })
    }

    #[must_use]
    pub fn first(&self) -> u64 {
        self.first
    }

    #[must_use]
    pub fn second(&self) -> u64 {
        self.second
    }
}

/// Parse the NYU 8-row query CSV. Each query contributes eight 3D points:
/// edge-edge or vertex-face ordering is selected explicitly by QueryKind.
///
/// Six columns represent three exact rationals (numerator, denominator).
/// Seven-column legacy files add a Boolean ground-truth label to every row;
/// labels within each eight-row query must agree. Scalable-CCD six-column
/// files attach their separate JSON Boolean file using attach_ground_truth.
pub fn parse_queries_csv(input: &str, kind: QueryKind) -> Result<Vec<CcdQuery>, ImportError> {
    let mut rows = Vec::new();
    let mut column_count = None;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            return Err(ImportError::EmptyRow);
        }
        let columns: Vec<&str> = line.split(',').map(str::trim).collect();
        if columns.len() != 6 && columns.len() != 7 {
            return Err(ImportError::UnsupportedColumnCount);
        }
        match column_count {
            Some(expected) if expected != columns.len() => {
                return Err(ImportError::InconsistentColumnCount);
            }
            None => column_count = Some(columns.len()),
            _ => {}
        }

        let x = Rational::new(columns[0], columns[1])?;
        let y = Rational::new(columns[2], columns[3])?;
        let z = Rational::new(columns[4], columns[5])?;
        let ground_truth = if columns.len() == 7 {
            Some(parse_bool(columns[6])?)
        } else {
            None
        };
        rows.push((Point3::new(x, y, z), ground_truth));
    }

    if rows.is_empty() {
        return Err(ImportError::EmptyInput);
    }
    if rows.len() % 8 != 0 {
        return Err(ImportError::IncompleteEightRowQuery);
    }

    let mut queries = Vec::with_capacity(rows.len() / 8);
    for (index, chunk) in rows.chunks_exact(8).enumerate() {
        let ground_truth = chunk[0].1;
        if chunk.iter().any(|row| row.1 != ground_truth) {
            return Err(ImportError::InconsistentInlineGroundTruth);
        }
        let vertices: Vec<Point3> = chunk.iter().map(|row| row.0.clone()).collect();
        let vertices: [Point3; 8] = vertices
            .try_into()
            .map_err(|_| ImportError::IncompleteEightRowQuery)?;
        queries.push(CcdQuery {
            index,
            kind,
            vertices,
            inline_ground_truth: ground_truth,
        });
    }
    Ok(queries)
}

/// Attach the per-query Boolean vector from the Scalable-CCD mma_bool JSON.
pub fn attach_ground_truth(queries: &mut [CcdQuery], labels: &[bool]) -> Result<(), ImportError> {
    if queries.len() != labels.len() {
        return Err(ImportError::GroundTruthCountMismatch);
    }
    if queries
        .iter()
        .any(|query| query.inline_ground_truth.is_some())
    {
        return Err(ImportError::GroundTruthAlreadyPresent);
    }
    for (query, label) in queries.iter_mut().zip(labels.iter().copied()) {
        query.inline_ground_truth = Some(label);
    }
    Ok(())
}

/// Parse a JSON array of Boolean labels, such as [false, true].
pub fn parse_boolean_json(input: &str) -> Result<Vec<bool>, ImportError> {
    let mut cursor = JsonCursor::new(input);
    cursor.expect(b'[')?;
    let mut values = Vec::new();
    cursor.skip_whitespace();
    if cursor.consume_if(b']') {
        cursor.finish()?;
        return Ok(values);
    }

    loop {
        cursor.skip_whitespace();
        let token = cursor.read_ascii_token()?;
        values.push(parse_json_bool(token)?);
        cursor.skip_whitespace();
        if cursor.consume_if(b']') {
            break;
        }
        cursor.expect(b',')?;
    }
    cursor.finish()?;
    Ok(values)
}

/// Parse the Scalable-CCD boxes JSON: an array of [box_id_a, box_id_b] pairs.
pub fn parse_box_pairs_json(input: &str) -> Result<Vec<BoxPair>, ImportError> {
    let mut cursor = JsonCursor::new(input);
    cursor.expect(b'[')?;
    let mut pairs = Vec::new();
    cursor.skip_whitespace();
    if cursor.consume_if(b']') {
        cursor.finish()?;
        return Ok(pairs);
    }

    loop {
        cursor.expect(b'[')?;
        let first = cursor.read_u64()?;
        cursor.expect(b',')?;
        let second = cursor.read_u64()?;
        cursor.expect(b']')?;
        let pair = BoxPair::new(first, second).ok_or(ImportError::InvalidBoxPair)?;
        pairs.push(pair);
        cursor.skip_whitespace();
        if cursor.consume_if(b']') {
            break;
        }
        cursor.expect(b',')?;
    }
    cursor.finish()?;
    Ok(pairs)
}

fn parse_json_bool(value: &str) -> Result<bool, ImportError> {
    match value {
        "false" => Ok(false),
        "true" => Ok(true),
        _ => Err(ImportError::InvalidBoolean),
    }
}

fn parse_bool(value: &str) -> Result<bool, ImportError> {
    match value {
        "0" | "false" => Ok(false),
        "1" | "true" => Ok(true),
        _ => Err(ImportError::InvalidBoolean),
    }
}

fn valid_integer(value: &str, signed: bool) -> bool {
    let digits = if signed {
        value.strip_prefix('-').unwrap_or(value)
    } else {
        value
    };
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

struct JsonCursor<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> JsonCursor<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            index: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.index)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.index += 1;
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        self.skip_whitespace();
        if self.bytes.get(self.index) == Some(&expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), ImportError> {
        if self.consume_if(expected) {
            Ok(())
        } else {
            Err(ImportError::InvalidJson)
        }
    }

    fn read_ascii_token(&mut self) -> Result<&'a str, ImportError> {
        self.skip_whitespace();
        let start = self.index;
        while self
            .bytes
            .get(self.index)
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte.is_ascii_digit())
        {
            self.index += 1;
        }
        if self.index == start {
            return Err(ImportError::InvalidJson);
        }
        std::str::from_utf8(&self.bytes[start..self.index]).map_err(|_| ImportError::InvalidJson)
    }

    fn read_u64(&mut self) -> Result<u64, ImportError> {
        self.skip_whitespace();
        let start = self.index;
        while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
            self.index += 1;
        }
        if self.index == start {
            return Err(ImportError::InvalidJson);
        }
        let token = std::str::from_utf8(&self.bytes[start..self.index])
            .map_err(|_| ImportError::InvalidJson)?;
        token
            .parse::<u64>()
            .map_err(|_| ImportError::IntegerOverflow)
    }

    fn finish(&mut self) -> Result<(), ImportError> {
        self.skip_whitespace();
        if self.index == self.bytes.len() {
            Ok(())
        } else {
            Err(ImportError::InvalidJson)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational_row(index: usize, label: Option<bool>) -> String {
        let mut row = format!("{},2,{index},4,-5,6", index + 1);
        if let Some(label) = label {
            row.push(',');
            row.push_str(if label { "1" } else { "0" });
        }
        row
    }

    #[test]
    fn parses_eight_exact_rational_rows_per_edge_edge_query() {
        let csv = (0..8)
            .map(|index| rational_row(index, Some(true)))
            .collect::<Vec<_>>()
            .join("\n");
        let queries = parse_queries_csv(&csv, QueryKind::EdgeEdge).unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].index(), 0);
        assert_eq!(queries[0].kind(), QueryKind::EdgeEdge);
        assert_eq!(queries[0].row_roles(), QueryKind::EdgeEdge.row_roles());
        assert_eq!(queries[0].inline_ground_truth(), Some(true));
        assert_eq!(queries[0].vertices()[0].x().numerator(), "1");
        assert_eq!(queries[0].vertices()[0].x().denominator(), "2");
    }

    #[test]
    fn preserves_arbitrarily_long_integer_rationals_verbatim() {
        let numerator = "12345678901234567890123456789012345678901234567890";
        let denominator = "98765432109876543210987654321098765432109876543210";
        let row = format!("{numerator},{denominator},0,1,0,1");
        let csv = std::iter::repeat_n(row, 8).collect::<Vec<_>>().join("\n");
        let queries = parse_queries_csv(&csv, QueryKind::VertexFace).unwrap();
        assert_eq!(queries[0].vertices()[0].x().numerator(), numerator);
        assert_eq!(queries[0].vertices()[0].x().denominator(), denominator);
    }

    #[test]
    fn parses_six_column_query_file_and_attaches_separate_boolean_json() {
        let csv = (0..16)
            .map(|index| rational_row(index, None))
            .collect::<Vec<_>>()
            .join("\n");
        let mut queries = parse_queries_csv(&csv, QueryKind::VertexFace).unwrap();
        let labels = parse_boolean_json("[true, false]").unwrap();
        attach_ground_truth(&mut queries, &labels).unwrap();
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].inline_ground_truth(), Some(true));
        assert_eq!(queries[1].inline_ground_truth(), Some(false));
    }

    #[test]
    fn rejects_mismatched_boolean_denominator() {
        let csv = (0..16)
            .map(|index| rational_row(index, None))
            .collect::<Vec<_>>()
            .join("\n");
        let mut queries = parse_queries_csv(&csv, QueryKind::EdgeEdge).unwrap();
        assert_eq!(
            attach_ground_truth(&mut queries, &[true]),
            Err(ImportError::GroundTruthCountMismatch)
        );
    }

    #[test]
    fn rejects_inline_ground_truth_disagreement_within_query() {
        let csv = (0..8)
            .map(|index| rational_row(index, Some(index == 0)))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            parse_queries_csv(&csv, QueryKind::EdgeEdge),
            Err(ImportError::InconsistentInlineGroundTruth)
        );
    }

    #[test]
    fn rejects_invalid_or_zero_denominators_without_numeric_coercion() {
        assert_eq!(Rational::new("1", "0"), Err(ImportError::ZeroDenominator));
        assert_eq!(
            Rational::new("1", "-2"),
            Err(ImportError::InvalidDenominator)
        );
        assert_eq!(
            Rational::new("1.2", "3"),
            Err(ImportError::InvalidNumerator)
        );
    }

    #[test]
    fn rejects_incomplete_or_mixed_column_query_files() {
        let incomplete = (0..7)
            .map(|index| rational_row(index, None))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            parse_queries_csv(&incomplete, QueryKind::EdgeEdge),
            Err(ImportError::IncompleteEightRowQuery)
        );

        let mixed = format!("{}\n{}", rational_row(0, None), rational_row(1, Some(true)));
        assert_eq!(
            parse_queries_csv(&mixed, QueryKind::EdgeEdge),
            Err(ImportError::InconsistentColumnCount)
        );
    }

    #[test]
    fn parses_box_pair_json_and_rejects_self_pairs() {
        let pairs = parse_box_pairs_json("[[14175, 30842], [14286, 38673]]").unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].first(), 14175);
        assert_eq!(pairs[0].second(), 30842);
        assert_eq!(
            parse_box_pairs_json("[[1, 1]]"),
            Err(ImportError::InvalidBoxPair)
        );
        assert_eq!(
            parse_box_pairs_json("[[30842, 14175]]").unwrap(),
            parse_box_pairs_json("[[14175, 30842]]").unwrap()
        );
    }

    #[test]
    fn boolean_json_requires_a_strict_boolean_array() {
        assert_eq!(
            parse_boolean_json("[false, true, 0, 1]").unwrap(),
            vec![false, true, false, true]
        );
        assert_eq!(
            parse_boolean_json("[false, nope]"),
            Err(ImportError::InvalidBoolean)
        );
        assert_eq!(
            parse_boolean_json("[false, 0]"),
            Err(ImportError::InvalidBoolean)
        );
        assert_eq!(
            parse_boolean_json("[false] trailing"),
            Err(ImportError::InvalidJson)
        );
    }

    #[test]
    fn refuses_to_overwrite_inline_labels() {
        let csv = (0..8)
            .map(|index| rational_row(index, Some(true)))
            .collect::<Vec<_>>()
            .join("\n");
        let mut queries = parse_queries_csv(&csv, QueryKind::EdgeEdge).unwrap();
        assert_eq!(
            attach_ground_truth(&mut queries, &[true]),
            Err(ImportError::GroundTruthAlreadyPresent)
        );
    }
    #[test]
    fn imports_real_armadillo_rollers_3ee_sample_losslessly() {
        const CSV: &str = include_str!("../fixtures/armadillo-rollers-3ee.csv");
        const BOOLS: &str = include_str!("../fixtures/armadillo-rollers-3ee-mma-bool.json");
        const BOXES: &str = include_str!("../fixtures/armadillo-rollers-3ee-boxes.json");

        let mut queries = parse_queries_csv(CSV, QueryKind::EdgeEdge).unwrap();
        let labels = parse_boolean_json(BOOLS).unwrap();
        attach_ground_truth(&mut queries, &labels).unwrap();
        let pairs = parse_box_pairs_json(BOXES).unwrap();

        assert_eq!(queries.len(), 2);
        assert_eq!(labels, vec![false, false]);
        assert!(queries.iter().all(|query| query.inline_ground_truth() == Some(false)));
        assert_eq!(pairs.len(), 2);
        assert_eq!((pairs[0].first(), pairs[0].second()), (14175, 30842));
        assert_eq!((pairs[1].first(), pairs[1].second()), (14286, 38673));

        // The exact rational lexemes survive import without floating conversion.
        assert_eq!(
            queries[0].vertices()[0].x().numerator(),
            "6896999465383287"
        );
        assert_eq!(
            queries[0].vertices()[0].x().denominator(),
            "36028797018963968"
        );
        assert_eq!(queries[0].row_roles()[0], VertexRole::EdgeAStart0);
        assert_eq!(queries[1].row_roles()[7], VertexRole::EdgeBEnd1);
    }

}
