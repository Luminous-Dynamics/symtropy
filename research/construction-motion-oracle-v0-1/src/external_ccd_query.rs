//! Lossless reader for the public CCD rational-CSV query format.
//! Eight rows encode each query. Each point coordinate is an exact integer
//! numerator/denominator pair. Lexemes are retained as strings so import never
//! narrows a large denominator or rounds it to floating point.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryKind {
    EdgeEdge,
    VertexFace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactRational {
    pub numerator: String,
    pub denominator: String,
}

impl ExactRational {
    fn parse(numerator: &str, denominator: &str) -> Result<Self, RationalQueryError> {
        if !is_integer_lexeme(numerator) || !is_integer_lexeme(denominator) {
            return Err(RationalQueryError::InvalidInteger);
        }
        if is_zero_integer(denominator) {
            return Err(RationalQueryError::ZeroDenominator);
        }
        Ok(Self {
            numerator: numerator.to_owned(),
            denominator: denominator.to_owned(),
        })
    }

    /// Convert only when an explicitly selected floating-point evaluator is used.
    pub fn to_f64(&self) -> Result<f64, RationalQueryError> {
        let numerator = self.numerator.parse::<f64>()
            .map_err(|_| RationalQueryError::FloatConversionOutOfRange)?;
        let denominator = self.denominator.parse::<f64>()
            .map_err(|_| RationalQueryError::FloatConversionOutOfRange)?;
        if !numerator.is_finite() || !denominator.is_finite() || denominator == 0.0 {
            return Err(RationalQueryError::FloatConversionOutOfRange);
        }
        let value = numerator / denominator;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(RationalQueryError::FloatConversionOutOfRange)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPoint3 {
    pub x: ExactRational,
    pub y: ExactRational,
    pub z: ExactRational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalCcdQuery {
    pub index: usize,
    pub kind: QueryKind,
    /// Eight rows in the upstream edge-edge or vertex-face ordering.
    pub vertices: [ExactPoint3; 8],
    pub ground_truth: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RationalQueryError {
    EmptyInput,
    InvalidColumnCount { line: usize, columns: usize },
    MixedColumnCount,
    InvalidInteger,
    ZeroDenominator,
    InvalidInlineGroundTruth { line: usize },
    InconsistentInlineGroundTruth { query_index: usize },
    RowCountNotMultipleOfEight { rows: usize },
    MissingGroundTruth,
    InvalidBooleanArray,
    GroundTruthCountMismatch { queries: usize, labels: usize },
    GroundTruthRepresentationMismatch { query_index: usize },
    FloatConversionOutOfRange,
}

impl fmt::Display for RationalQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RationalQueryError {}

struct ParsedRow {
    point: ExactPoint3,
    inline_truth: Option<bool>,
}

/// Parse six-column rational CSV rows, or seven-column rows with repeated
/// inline ground truth. An optional separate JSON boolean array is cross-checked.
pub fn parse_rational_queries(
    csv: &str,
    kind: QueryKind,
    separate_ground_truth_json: Option<&str>,
) -> Result<Vec<RationalCcdQuery>, RationalQueryError> {
    let mut rows = Vec::new();
    let mut expected_columns = None;

    for (line_index, raw_line) in csv.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != 6 && fields.len() != 7 {
            return Err(RationalQueryError::InvalidColumnCount {
                line: line_number,
                columns: fields.len(),
            });
        }
        match expected_columns {
            None => expected_columns = Some(fields.len()),
            Some(columns) if columns != fields.len() => {
                return Err(RationalQueryError::MixedColumnCount);
            }
            Some(_) => {}
        }

        let x = ExactRational::parse(fields[0], fields[1])?;
        let y = ExactRational::parse(fields[2], fields[3])?;
        let z = ExactRational::parse(fields[4], fields[5])?;
        let inline_truth = if fields.len() == 7 {
            Some(match fields[6] {
                "0" => false,
                "1" => true,
                _ => return Err(RationalQueryError::InvalidInlineGroundTruth {
                    line: line_number,
                }),
            })
        } else {
            None
        };
        rows.push(ParsedRow {
            point: ExactPoint3 { x, y, z },
            inline_truth,
        });
    }

    if rows.is_empty() {
        return Err(RationalQueryError::EmptyInput);
    }
    if rows.len() % 8 != 0 {
        return Err(RationalQueryError::RowCountNotMultipleOfEight { rows: rows.len() });
    }

    let query_count = rows.len() / 8;
    let external_truth = separate_ground_truth_json
        .map(parse_boolean_array)
        .transpose()?;
    if let Some(labels) = &external_truth {
        if labels.len() != query_count {
            return Err(RationalQueryError::GroundTruthCountMismatch {
                queries: query_count,
                labels: labels.len(),
            });
        }
    }

    let mut queries = Vec::with_capacity(query_count);
    for query_index in 0..query_count {
        let group = &rows[query_index * 8..query_index * 8 + 8];
        let inline_truth = group[0].inline_truth;
        if group.iter().any(|row| row.inline_truth != inline_truth) {
            return Err(RationalQueryError::InconsistentInlineGroundTruth { query_index });
        }

        let ground_truth = match (inline_truth, external_truth.as_ref()) {
            (Some(inline), Some(labels)) => {
                if inline != labels[query_index] {
                    return Err(RationalQueryError::GroundTruthRepresentationMismatch {
                        query_index,
                    });
                }
                inline
            }
            (Some(inline), None) => inline,
            (None, Some(labels)) => labels[query_index],
            (None, None) => return Err(RationalQueryError::MissingGroundTruth),
        };

        let vertices: [ExactPoint3; 8] = group
            .iter()
            .map(|row| row.point.clone())
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| RationalQueryError::RowCountNotMultipleOfEight { rows: group.len() })?;

        queries.push(RationalCcdQuery {
            index: query_index,
            kind,
            vertices,
            ground_truth,
        });
    }
    Ok(queries)
}

/// Strict JSON reader for a top-level array of booleans only.
pub fn parse_boolean_array(json: &str) -> Result<Vec<bool>, RationalQueryError> {
    let bytes = json.as_bytes();
    let mut cursor = 0;
    skip_whitespace(bytes, &mut cursor);
    expect_byte(bytes, &mut cursor, b'[')?;
    let mut values = Vec::new();
    skip_whitespace(bytes, &mut cursor);

    if peek(bytes, cursor) == Some(b']') {
        cursor += 1;
        skip_whitespace(bytes, &mut cursor);
        return if cursor == bytes.len() {
            Ok(values)
        } else {
            Err(RationalQueryError::InvalidBooleanArray)
        };
    }

    loop {
        skip_whitespace(bytes, &mut cursor);
        if consume_literal(bytes, &mut cursor, b"true") {
            values.push(true);
        } else if consume_literal(bytes, &mut cursor, b"false") {
            values.push(false);
        } else {
            return Err(RationalQueryError::InvalidBooleanArray);
        }
        skip_whitespace(bytes, &mut cursor);
        match peek(bytes, cursor) {
            Some(b',') => cursor += 1,
            Some(b']') => {
                cursor += 1;
                skip_whitespace(bytes, &mut cursor);
                return if cursor == bytes.len() {
                    Ok(values)
                } else {
                    Err(RationalQueryError::InvalidBooleanArray)
                };
            }
            _ => return Err(RationalQueryError::InvalidBooleanArray),
        }
    }
}

fn is_integer_lexeme(value: &str) -> bool {
    let digits = value
        .strip_prefix('-')
        .or_else(|| value.strip_prefix('+'))
        .unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_zero_integer(value: &str) -> bool {
    let digits = value.strip_prefix('-').or_else(|| value.strip_prefix('+')).unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte == b'0')
}

fn skip_whitespace(bytes: &[u8], cursor: &mut usize) {
    while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
        *cursor += 1;
    }
}

fn peek(bytes: &[u8], cursor: usize) -> Option<u8> {
    bytes.get(cursor).copied()
}

fn expect_byte(bytes: &[u8], cursor: &mut usize, expected: u8) -> Result<(), RationalQueryError> {
    if peek(bytes, *cursor) != Some(expected) {
        return Err(RationalQueryError::InvalidBooleanArray);
    }
    *cursor += 1;
    Ok(())
}

fn consume_literal(bytes: &[u8], cursor: &mut usize, literal: &[u8]) -> bool {
    let end = (*cursor).saturating_add(literal.len());
    if bytes.get(*cursor..end) == Some(literal) {
        *cursor = end;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csv_row(x_num: &str, x_den: &str, truth: Option<u8>) -> String {
        let suffix = truth.map(|value| format!(",{value}")).unwrap_or_default();
        format!("{x_num},{x_den},0,1,0,1{suffix}")
    }

    fn eight_rows(truth: Option<u8>) -> String {
        (0..8)
            .map(|_| csv_row("1", "2", truth))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn parses_inline_truth_and_preserves_rational_lexemes() {
        let parsed =
            parse_rational_queries(&eight_rows(Some(1)), QueryKind::EdgeEdge, None).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].kind, QueryKind::EdgeEdge);
        assert!(parsed[0].ground_truth);
        assert_eq!(parsed[0].vertices[0].x.numerator, "1");
        assert_eq!(parsed[0].vertices[0].x.denominator, "2");
    }

    #[test]
    fn preserves_denominators_larger_than_i64() {
        let denominator = "36893488147419103232";
        let csv = (0..8)
            .map(|_| csv_row("6896999465383287", denominator, Some(0)))
            .collect::<Vec<_>>()
            .join("\n");
        let parsed = parse_rational_queries(&csv, QueryKind::VertexFace, None).unwrap();
        assert_eq!(parsed[0].vertices[0].x.denominator, denominator);
        assert!(parsed[0].vertices[0].x.to_f64().unwrap().is_finite());
    }

    #[test]
    fn parses_separate_boolean_json() {
        let parsed =
            parse_rational_queries(&eight_rows(None), QueryKind::VertexFace, Some("[ true ]"))
                .unwrap();
        assert!(!parsed[0].ground_truth);
    }

    #[test]
    fn rejects_mismatched_inline_and_external_truth() {
        assert_eq!(
            parse_rational_queries(&eight_rows(Some(1)), QueryKind::EdgeEdge, Some("[false]")),
            Err(RationalQueryError::GroundTruthRepresentationMismatch { query_index: 0 })
        );
    }

    #[test]
    fn rejects_inconsistent_truth_repeated_across_eight_rows() {
        let rows = (0..8)
            .map(|index| csv_row("1", "2", Some(u8::from(index != 7))))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            parse_rational_queries(&rows, QueryKind::EdgeEdge, None),
            Err(RationalQueryError::InconsistentInlineGroundTruth { query_index: 0 })
        );
    }

    #[test]
    fn requires_a_ground_truth_source() {
        assert_eq!(
            parse_rational_queries(&eight_rows(None), QueryKind::EdgeEdge, None),
            Err(RationalQueryError::MissingGroundTruth)
        );
    }

    #[test]
    fn rejects_zero_denominators_and_invalid_integer_lexemes() {
        assert_eq!(
            parse_rational_queries(
                &eight_rows(Some(0)).replace("1,2", "1,0"),
                QueryKind::EdgeEdge,
                None
            ),
            Err(RationalQueryError::ZeroDenominator)
        );
        assert_eq!(
            parse_rational_queries(
                &eight_rows(Some(0)).replace("1,2", "one,2"),
                QueryKind::EdgeEdge,
                None
            ),
            Err(RationalQueryError::InvalidInteger)
        );
    }

    #[test]
    fn rejects_wrong_row_and_column_counts() {
        assert_eq!(
            parse_rational_queries("1,2,0,1,0,1,0", QueryKind::EdgeEdge, None),
            Err(RationalQueryError::RowCountNotMultipleOfEight { rows: 1 })
        );
        let mixed = format!("{}\n1,2,0,1,0", eight_rows(Some(0)));
        assert_eq!(
            parse_rational_queries(&mixed, QueryKind::EdgeEdge, None),
            Err(RationalQueryError::InvalidColumnCount {
                line: 9,
                columns: 5
            })
        );
    }

    #[test]
    fn rejects_mixed_six_and_seven_column_layouts() {
        let mixed = format!("{}\n{}", eight_rows(Some(0)), eight_rows(None));
        assert_eq!(
            parse_rational_queries(&mixed, QueryKind::EdgeEdge, None),
            Err(RationalQueryError::MixedColumnCount)
        );
    }

    #[test]
    fn validates_json_booleans_and_query_count() {
        for invalid in [
            "[0]",
            "[null]",
            "[true false]",
            "[true,]",
            "true",
            "[true] trailing",
        ] {
            assert_eq!(
                parse_boolean_array(invalid),
                Err(RationalQueryError::InvalidBooleanArray)
            );
        }
        assert_eq!(
            parse_rational_queries(&eight_rows(None), QueryKind::EdgeEdge, Some("[]")),
            Err(RationalQueryError::GroundTruthCountMismatch {
                queries: 1,
                labels: 0
            })
        );
    }

    #[test]
    fn parses_multiple_query_groups_independently() {
        let csv = format!("{}\n{}", eight_rows(Some(0)), eight_rows(Some(1)));
        let parsed = parse_rational_queries(&csv, QueryKind::EdgeEdge, None).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(!parsed[0].ground_truth);
        assert!(parsed[1].ground_truth);
        assert_eq!(parsed[1].index, 1);
    }

    #[test]
    fn rejects_empty_input_and_non_finite_float_conversion() {
        assert_eq!(
            parse_rational_queries("", QueryKind::EdgeEdge, None),
            Err(RationalQueryError::EmptyInput)
        );
        let huge = ExactRational {
            numerator: "9".repeat(400),
            denominator: "1".into(),
        };
        assert_eq!(
            huge.to_f64(),
            Err(RationalQueryError::FloatConversionOutOfRange)
        );
    }
}
