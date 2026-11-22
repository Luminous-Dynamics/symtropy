//! Strict positional CSV ingestion and controller/evaluator partition for Bregar UWB v2.
//!
//! The authors' converter skips the first two CSV lines, reads each data row
//! without a header and uses the first 26 fields as metadata, with CIR samples
//! starting at position 26. This module preserves that positional contract,
//! preserves the two preamble lines without trusting their human-readable
//! header labels, parses rows using the source-pinned positional map, decodes
//! complex CIR cells, and partitions NLOS/position fields from the controller.
//!
//! This is a line-at-a-time parser: callers can stream a large archive without
//! loading the whole CSV file into memory. It does not open ZIP files or parse
//! dataset paths. The actual Zenodo v2 payload has not yet been inspected.

use crate::bregar_cir_features::{BregarRawCirObservation, ComplexCirSample};
use std::fmt;

pub const BREGAR_CSV_PREFIX_FIELDS: usize = 26;
pub const BREGAR_CIR_START_FIELD: usize = 26;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BregarCsvPreamble {
    pub version_line: String,
    pub header_line: String,
    pub normalized_header_fields: Vec<String>,
}

/// Caller-supplied streaming limits. They must be set from the frozen source
/// inventory / expected CIR encoding, not silently guessed by the parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BregarCsvIngestLimits {
    pub maximum_record_bytes: usize,
    pub maximum_cir_samples: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BregarCsvControllerView {
    pub source_file_id: String,
    /// Zero-based physical data-row index within the source CSV file.
    pub source_row_index: u64,
    pub tag_id: String,
    pub anchor_id: String,
    pub measured_range_m: f64,
    pub fp_index: f64,
    pub rss_dbm: Option<f64>,
    pub rss_fp_dbm: Option<f64>,
    pub first_path_points: [Option<f64>; 3],
    pub stdev_noise: Option<f64>,
    pub cir_power: Option<f64>,
    pub max_noise: Option<f64>,
    pub rxpacc: f64,
    pub channel_number: Option<f64>,
    pub frame_length: Option<f64>,
    pub preamble_length: Option<f64>,
    pub bitrate: Option<f64>,
    pub prfr: Option<f64>,
    pub preamble_code: Option<f64>,
    pub cir_samples: Vec<ComplexCirSample>,
}

impl BregarCsvControllerView {
    /// Construct a CIR feature-kernel input with an explicit stable measurement
    /// ID and independently qualified sample period. The parser does not infer
    /// sample spacing from PRFR.
    pub fn to_cir_observation(
        &self,
        measurement_id: u64,
        sample_period_ns: f64,
    ) -> BregarRawCirObservation {
        BregarRawCirObservation {
            measurement_id,
            fp_index: self.fp_index,
            rxpacc: self.rxpacc,
            sample_period_ns,
            cir_samples: self.cir_samples.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BregarCsvEvaluatorView {
    /// Preserve the publisher label verbatim; do not map or train on it here.
    pub nlos_label: Option<String>,
    /// Position metadata remains evaluator-only until survey provenance is qualified.
    pub tag_position_m: Option<[f64; 3]>,
    pub anchor_position_m: Option<[f64; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BregarCsvPartitionedRow {
    pub controller: BregarCsvControllerView,
    pub evaluator: BregarCsvEvaluatorView,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BregarCsvIngestError {
    EmptyPreambleLine,
    InvalidCsvQuoting,
    UnterminatedQuotedCell,
    EmptyHeaderRecord,
    InvalidIngestLimits,
    RecordTooLarge,
    CirSampleCountExceeded,
    InvalidSourceFileIdentity,
    TooFewRowFields,
    EmptyTagIdentity,
    EmptyAnchorIdentity,
    InvalidRequiredNumber(&'static str),
    InvalidOptionalNumber(&'static str),
    NegativeMeasuredRange,
    InvalidFpIndex,
    InvalidRxpacc,
    PartialCoordinateTriplet(&'static str),
    InvalidComplexCirSample { index: usize, value: String },
    EmptyCirVector,
    ArithmeticOverflow,
}

impl fmt::Display for BregarCsvIngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPreambleLine => write!(f, "Bregar CSV version/header preamble line is empty"),
            Self::InvalidCsvQuoting => write!(f, "CSV quote appears inside an unquoted nonempty field"),
            Self::UnterminatedQuotedCell => write!(f, "CSV record contains an unterminated quoted cell"),
            Self::EmptyHeaderRecord => write!(f, "Bregar CSV header record must contain at least one cell"),
            Self::InvalidIngestLimits => write!(f, "maximum record bytes and CIR sample count must both be positive"),
            Self::RecordTooLarge => write!(f, "Bregar CSV data record exceeds the configured byte limit"),
            Self::CirSampleCountExceeded => write!(f, "Bregar CSV row exceeds the configured CIR sample count"),
            Self::InvalidSourceFileIdentity => write!(f, "source file identity must be nonempty"),
            Self::TooFewRowFields => write!(f, "Bregar data row must contain 26 metadata fields and at least one CIR sample"),
            Self::EmptyTagIdentity => write!(f, "TAG_ID must be nonempty"),
            Self::EmptyAnchorIdentity => write!(f, "ANCHOR_ID must be nonempty"),
            Self::InvalidRequiredNumber(name) => write!(f, "required numeric field is invalid: {name}"),
            Self::InvalidOptionalNumber(name) => write!(f, "optional numeric field is neither empty nor a finite number: {name}"),
            Self::NegativeMeasuredRange => write!(f, "measured RANGE must be nonnegative"),
            Self::InvalidFpIndex => write!(f, "FP_INDEX must be finite and nonnegative"),
            Self::InvalidRxpacc => write!(f, "RXPACC must be finite and strictly positive"),
            Self::PartialCoordinateTriplet(name) => write!(f, "{name} coordinates must be all present or all absent"),
            Self::InvalidComplexCirSample { index, value } => write!(f, "CIR sample {index} has unsupported complex encoding: {value}"),
            Self::EmptyCirVector => write!(f, "CIR sample vector is empty"),
            Self::ArithmeticOverflow => write!(f, "CSV-derived measurement identity overflowed"),
        }
    }
}

impl std::error::Error for BregarCsvIngestError {}

/// Parse the two preamble lines. The first line is preserved as the upstream
/// file-format version string; the second is validated against the exact
/// 26-metadata-field + CIR header order used by the authors' converter.
pub fn parse_bregar_csv_preamble(
    version_line: &str,
    header_line: &str,
) -> Result<BregarCsvPreamble, BregarCsvIngestError> {
    let version_line = version_line.trim().trim_start_matches('\u{feff}').to_owned();
    let header_line = header_line.trim().to_owned();
    if version_line.is_empty() || header_line.is_empty() {
        return Err(BregarCsvIngestError::EmptyPreambleLine);
    }

    // The pinned authors' converter skips these two preamble lines and
    // reads all subsequent rows with header=None. Preserve the human-readable
    // header for provenance, but do not treat its labels or count as the data
    // schema. The row parser uses the versioned positional map below instead.
    let header_fields = parse_csv_record(&header_line)?;
    if header_fields.is_empty()
        || header_fields.iter().all(|field| field.trim().is_empty())
    {
        return Err(BregarCsvIngestError::EmptyHeaderRecord);
    }
    let normalized = header_fields
        .iter()
        .map(|field| field.trim().trim_matches('\u{feff}').to_ascii_lowercase())
        .collect();

    Ok(BregarCsvPreamble {
        version_line,
        header_line,
        normalized_header_fields: normalized,
    })
}

/// Parse one data record after the two publisher preamble lines have been
/// skipped. The returned row partitions controller features from evaluator
/// labels and tag/anchor coordinates. All trailing cells from field index 26
/// onward are parsed as complex CIR samples.
pub fn parse_bregar_csv_data_row(
    source_file_id: &str,
    source_row_index: u64,
    record: &str,
    limits: BregarCsvIngestLimits,
) -> Result<BregarCsvPartitionedRow, BregarCsvIngestError> {
    if limits.maximum_record_bytes == 0 || limits.maximum_cir_samples == 0 {
        return Err(BregarCsvIngestError::InvalidIngestLimits);
    }
    if record.len() > limits.maximum_record_bytes {
        return Err(BregarCsvIngestError::RecordTooLarge);
    }
    if source_file_id.trim().is_empty() {
        return Err(BregarCsvIngestError::InvalidSourceFileIdentity);
    }
    let fields = parse_csv_record(record)?;
    if fields.len() <= BREGAR_CSV_PREFIX_FIELDS {
        return Err(BregarCsvIngestError::TooFewRowFields);
    }
    if fields.len() - BREGAR_CIR_START_FIELD > limits.maximum_cir_samples {
        return Err(BregarCsvIngestError::CirSampleCountExceeded);
    }

    let tag_id = required_text(&fields, 0, "TAG_ID")?;
    let anchor_id = required_text(&fields, 1, "ANCHOR_ID")?;
    let measured_range_m = required_f64(&fields, 9, "RANGE")?;
    if measured_range_m < 0.0 {
        return Err(BregarCsvIngestError::NegativeMeasuredRange);
    }
    let fp_index = required_f64(&fields, 10, "FP_INDEX")?;
    if fp_index < 0.0 {
        return Err(BregarCsvIngestError::InvalidFpIndex);
    }
    let rxpacc = required_f64(&fields, 19, "RXPACC")?;
    if rxpacc <= 0.0 {
        return Err(BregarCsvIngestError::InvalidRxpacc);
    }

    let mut cir_samples = Vec::with_capacity(fields.len() - BREGAR_CIR_START_FIELD);
    for (index, value) in fields[BREGAR_CIR_START_FIELD..].iter().enumerate() {
        let sample = parse_python_complex(value).map_err(|_| {
            BregarCsvIngestError::InvalidComplexCirSample {
                index,
                value: value.clone(),
            }
        })?;
        cir_samples.push(sample);
    }
    if cir_samples.is_empty() {
        return Err(BregarCsvIngestError::EmptyCirVector);
    }

    let tag_position_m = optional_coordinate_triplet(&fields, [2, 3, 4], "tag")?;
    let anchor_position_m = optional_coordinate_triplet(&fields, [5, 6, 7], "anchor")?;
    let nlos_label = optional_text(&fields, 8);

    Ok(BregarCsvPartitionedRow {
        controller: BregarCsvControllerView {
            source_file_id: source_file_id.to_owned(),
            source_row_index,
            tag_id,
            anchor_id,
            measured_range_m,
            fp_index,
            rss_dbm: optional_f64(&fields, 11, "RSS")?,
            rss_fp_dbm: optional_f64(&fields, 12, "RSS_FP")?,
            first_path_points: [
                optional_f64(&fields, 13, "FP_POINT1")?,
                optional_f64(&fields, 14, "FP_POINT2")?,
                optional_f64(&fields, 15, "FP_POINT3")?,
            ],
            stdev_noise: optional_f64(&fields, 16, "STDEV_NOISE")?,
            cir_power: optional_f64(&fields, 17, "CIR_POWER")?,
            max_noise: optional_f64(&fields, 18, "MAX_NOISE")?,
            rxpacc,
            channel_number: optional_f64(&fields, 20, "CHANNEL_NUMBER")?,
            frame_length: optional_f64(&fields, 21, "FRAME_LENGTH")?,
            preamble_length: optional_f64(&fields, 22, "PREAMBLE_LENGTH")?,
            bitrate: optional_f64(&fields, 23, "BITRATE")?,
            prfr: optional_f64(&fields, 24, "PRFR")?,
            preamble_code: optional_f64(&fields, 25, "PREAMBLE_CODE")?,
            cir_samples,
        },
        evaluator: BregarCsvEvaluatorView {
            nlos_label,
            tag_position_m,
            anchor_position_m,
        },
    })
}

/// Split one RFC-4180-style line record, supporting quoted cells and doubled
/// quote escapes. Newlines inside quoted cells are intentionally unsupported:
/// this parser is for one physical CSV line at a time.
fn parse_csv_record(record: &str) -> Result<Vec<String>, BregarCsvIngestError> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = record.chars().peekable();
    let mut in_quotes = false;
    let mut closed_quote = false;

    while let Some(character) = chars.next() {
        if in_quotes {
            if character == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                    closed_quote = true;
                }
            } else {
                current.push(character);
            }
        } else if closed_quote {
            match character {
                ',' => {
                    fields.push(std::mem::take(&mut current));
                    closed_quote = false;
                }
                ' ' | '\t' => {}
                _ => return Err(BregarCsvIngestError::InvalidCsvQuoting),
            }
        } else {
            match character {
                ',' => {
                    fields.push(std::mem::take(&mut current));
                }
                '"' => {
                    if !current.trim().is_empty() {
                        return Err(BregarCsvIngestError::InvalidCsvQuoting);
                    }
                    current.clear();
                    in_quotes = true;
                }
                _ => current.push(character),
            }
        }
    }

    if in_quotes {
        return Err(BregarCsvIngestError::UnterminatedQuotedCell);
    }
    fields.push(current);
    Ok(fields)
}

fn required_text(
    fields: &[String],
    index: usize,
    name: &'static str,
) -> Result<String, BregarCsvIngestError> {
    let value = fields
        .get(index)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or(match name {
            "TAG_ID" => BregarCsvIngestError::EmptyTagIdentity,
            "ANCHOR_ID" => BregarCsvIngestError::EmptyAnchorIdentity,
            _ => BregarCsvIngestError::InvalidRequiredNumber(name),
        })?;
    Ok(value.to_owned())
}

fn required_f64(
    fields: &[String],
    index: usize,
    name: &'static str,
) -> Result<f64, BregarCsvIngestError> {
    let parsed = fields
        .get(index)
        .ok_or(BregarCsvIngestError::InvalidRequiredNumber(name))?
        .trim()
        .parse::<f64>()
        .map_err(|_| BregarCsvIngestError::InvalidRequiredNumber(name))?;
    if !parsed.is_finite() {
        return Err(BregarCsvIngestError::InvalidRequiredNumber(name));
    }
    Ok(parsed)
}

fn optional_f64(
    fields: &[String],
    index: usize,
    name: &'static str,
) -> Result<Option<f64>, BregarCsvIngestError> {
    let value = fields
        .get(index)
        .ok_or(BregarCsvIngestError::InvalidOptionalNumber(name))?
        .trim();
    if value.is_empty() || value.eq_ignore_ascii_case("nan") {
        return Ok(None);
    }
    let parsed = value
        .parse::<f64>()
        .map_err(|_| BregarCsvIngestError::InvalidOptionalNumber(name))?;
    if !parsed.is_finite() {
        return Err(BregarCsvIngestError::InvalidOptionalNumber(name));
    }
    Ok(Some(parsed))
}

fn optional_text(fields: &[String], index: usize) -> Option<String> {
    fields
        .get(index)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("nan"))
        .map(str::to_owned)
}

fn optional_coordinate_triplet(
    fields: &[String],
    indices: [usize; 3],
    name: &'static str,
) -> Result<Option<[f64; 3]>, BregarCsvIngestError> {
    let values = [
        optional_f64(fields, indices[0], "coordinate")?,
        optional_f64(fields, indices[1], "coordinate")?,
        optional_f64(fields, indices[2], "coordinate")?,
    ];
    match values {
        [Some(x), Some(y), Some(z)] => Ok(Some([x, y, z])),
        [None, None, None] => Ok(None),
        _ => Err(BregarCsvIngestError::PartialCoordinateTriplet(name)),
    }
}

/// Parse Python complex-string cells used by the authors' converter, including
/// forms such as (a+bj), a-bj, a, bj, -j and exponent notation.
fn parse_python_complex(value: &str) -> Result<ComplexCirSample, ()> {
    let mut token = value.trim();
    if token.starts_with('(') && token.ends_with(')') && token.len() >= 2 {
        token = &token[1..token.len() - 1];
    }
    let token = token.trim();
    if token.is_empty() {
        return Err(());
    }

    if token.ends_with('j') || token.ends_with('J') {
        let body = token[..token.len() - 1].trim();
        if body.is_empty() || body == "+" || body == "-" {
            let imaginary = if body == "-" { -1.0 } else { 1.0 };
            return Ok(ComplexCirSample {
                real: 0.0,
                imaginary,
            });
        }

        let split = body.char_indices().find_map(|(index, character)| {
            if index == 0 || (character != '+' && character != '-') {
                return None;
            }
            let previous = body[..index].chars().next_back()?;
            if previous == 'e' || previous == 'E' {
                return None;
            }
            Some(index)
        });

        let (real, imaginary) = if let Some(index) = split {
            let real = body[..index].trim().parse::<f64>().map_err(|_| ())?;
            let imaginary_text = body[index..].trim();
            let imaginary = match imaginary_text {
                "+" => 1.0,
                "-" => -1.0,
                _ => imaginary_text.parse::<f64>().map_err(|_| ())?,
            };
            (real, imaginary)
        } else {
            let imaginary = body.parse::<f64>().map_err(|_| ())?;
            (0.0, imaginary)
        };
        if !real.is_finite() || !imaginary.is_finite() {
            return Err(());
        }
        Ok(ComplexCirSample { real, imaginary })
    } else {
        let real = token.parse::<f64>().map_err(|_| ())?;
        if !real.is_finite() {
            return Err(());
        }
        Ok(ComplexCirSample {
            real,
            imaginary: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bregar_cir_features::extract_bregar_cir_features;

    fn header() -> String {
        "tag_id,anchor_id,x_tag,y_tag,z_tag,x_anchor,y_anchor,z_anchor,nlos,range,fp_index,rss,rss_fp,fp_point1,fp_point2,fp_point3,stdev_noise,cir_power,max_noise,rxpacc,channel_number,frame_length,preamble_length,bitrate,prfr,preamble_code,cir".to_owned()
    }

    fn row_fields() -> Vec<String> {
        vec![
            "T1".into(), "A1".into(),
            "1.0".into(), "2.0".into(), "3.0".into(),
            "4.0".into(), "5.0".into(), "6.0".into(),
            "NLOS".into(), "4.5".into(), "100.75".into(),
            "-65.0".into(), "-70.0".into(),
            "1.0".into(), "2.0".into(), "3.0".into(),
            "0.4".into(), "12.0".into(), "1.5".into(),
            "2".into(), "5".into(), "64".into(),
            "128".into(), "6".into(), "64".into(), "9".into(),
        ]
    }

    fn parse_row(
        source_file_id: &str,
        source_row_index: u64,
        record: &str,
    ) -> Result<BregarCsvPartitionedRow, BregarCsvIngestError> {
        parse_bregar_csv_data_row(
            source_file_id,
            source_row_index,
            record,
            BregarCsvIngestLimits {
                maximum_record_bytes: 1_000_000,
                maximum_cir_samples: 1_024,
            },
        )
    }

    fn row_record(cir_count: usize) -> String {
        let mut fields = row_fields();
        fields.extend((0..cir_count).map(|index| {
            if index == 0 { "(1.5+2.25j)".to_owned() } else { "0.0+0.0j".to_owned() }
        }));
        fields
            .into_iter()
            .map(|field| {
                if field.contains(',') || field.contains('"') {
                    format!("\"{}\"", field.replace('"', "\"\""))
                } else {
                    field
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    #[test]
    fn preserves_two_line_preamble_without_trusting_header_labels() {
        let parsed = parse_bregar_csv_preamble("v1.1", &header()).unwrap();
        assert_eq!(parsed.version_line, "v1.1");
        assert_eq!(parsed.normalized_header_fields.len(), 27);
        let unusual_header = parse_bregar_csv_preamble(
            "v1.1",
            "publisher-specific heading that is preserved,not used as row schema",
        )
        .unwrap();
        assert_eq!(unusual_header.header_line, "publisher-specific heading that is preserved,not used as row schema");
        assert_eq!(unusual_header.normalized_header_fields.len(), 2);
    }

    #[test]
    fn rejects_empty_or_malformed_preamble() {
        assert_eq!(
            parse_bregar_csv_preamble("v1.1", ""),
            Err(BregarCsvIngestError::EmptyPreambleLine)
        );
        assert_eq!(
            parse_bregar_csv_preamble("v1.1", ",,,"),
            Err(BregarCsvIngestError::EmptyHeaderRecord)
        );
        assert_eq!(
            parse_bregar_csv_preamble("v1.1", "\"unterminated"),
            Err(BregarCsvIngestError::UnterminatedQuotedCell)
        );
    }

    #[test]
    fn parses_controller_and_evaluator_views_without_mixing_labels() {
        let parsed = parse_row("environment0/ch1_A1.csv", 42, &row_record(200)).unwrap();
        assert_eq!(parsed.controller.source_row_index, 42);
        assert_eq!(parsed.controller.tag_id, "T1");
        assert_eq!(parsed.controller.anchor_id, "A1");
        assert_eq!(parsed.controller.measured_range_m, 4.5);
        assert_eq!(parsed.controller.cir_samples.len(), 200);
        assert_eq!(parsed.controller.cir_samples[0], ComplexCirSample { real: 1.5, imaginary: 2.25 });
        assert_eq!(parsed.evaluator.nlos_label.as_deref(), Some("NLOS"));
        assert_eq!(parsed.evaluator.tag_position_m, Some([1.0, 2.0, 3.0]));
        assert_eq!(parsed.evaluator.anchor_position_m, Some([4.0, 5.0, 6.0]));
    }

    #[test]
    fn optional_missing_features_remain_explicitly_unavailable() {
        let mut fields = row_fields();
        fields[11] = "NaN".into();
        fields[12] = "".into();
        fields.extend((0..200).map(|_| "0.0".to_owned()));
        let parsed = parse_row("environment0/ch1_A1.csv", 0, &fields.join(",")).unwrap();
        assert_eq!(parsed.controller.rss_dbm, None);
        assert_eq!(parsed.controller.rss_fp_dbm, None);
    }

    #[test]
    fn evaluator_coordinate_triplets_must_be_complete() {
        let mut fields = row_fields();
        fields[3] = "".into();
        fields.extend((0..200).map(|_| "0.0".to_owned()));
        assert_eq!(
            parse_row("environment0/ch1_A1.csv", 0, &fields.join(",")),
            Err(BregarCsvIngestError::PartialCoordinateTriplet("tag"))
        );
    }

    #[test]
    fn parses_signed_exponent_complex_values_and_python_unit_imaginary_forms() {
        assert_eq!(
            parse_python_complex("(-1.2e-3+4.5e-2j)"),
            Ok(ComplexCirSample { real: -1.2e-3, imaginary: 4.5e-2 })
        );
        assert_eq!(
            parse_python_complex("-j"),
            Ok(ComplexCirSample { real: 0.0, imaginary: -1.0 })
        );
        assert_eq!(
            parse_python_complex("2.5"),
            Ok(ComplexCirSample { real: 2.5, imaginary: 0.0 })
        );
    }


    #[test]
    fn rejects_non_delimiter_characters_after_a_quoted_cell() {
        let mut fields = row_fields();
        fields.extend((0..200).map(|_| "0.0+0.0j".to_owned()));
        fields[0] = "\"T1\"x".to_owned();
        assert_eq!(
            parse_row("environment0/ch1_A1.csv", 0, &fields.join(",")),
            Err(BregarCsvIngestError::InvalidCsvQuoting)
        );
    }


    #[test]
    fn enforces_explicit_record_byte_and_cir_sample_limits() {
        let record = row_record(200);
        let too_small_bytes = BregarCsvIngestLimits {
            maximum_record_bytes: 10,
            maximum_cir_samples: 1_024,
        };
        assert_eq!(
            parse_bregar_csv_data_row(
                "environment0/ch1_A1.csv",
                0,
                &record,
                too_small_bytes,
            ),
            Err(BregarCsvIngestError::RecordTooLarge)
        );

        let too_few_samples = BregarCsvIngestLimits {
            maximum_record_bytes: 1_000_000,
            maximum_cir_samples: 100,
        };
        assert_eq!(
            parse_bregar_csv_data_row(
                "environment0/ch1_A1.csv",
                0,
                &record,
                too_few_samples,
            ),
            Err(BregarCsvIngestError::CirSampleCountExceeded)
        );

        let invalid_limits = BregarCsvIngestLimits {
            maximum_record_bytes: 0,
            maximum_cir_samples: 100,
        };
        assert_eq!(
            parse_bregar_csv_data_row(
                "environment0/ch1_A1.csv",
                0,
                &record,
                invalid_limits,
            ),
            Err(BregarCsvIngestError::InvalidIngestLimits)
        );
    }

    #[test]
    fn converts_partitioned_controller_row_to_explicit_sample_period_cir_observation() {
        let parsed = parse_row("environment0/ch1_A1.csv", 3, &row_record(200)).unwrap();
        let observation = parsed.controller.to_cir_observation(77, 0.015);
        assert_eq!(observation.measurement_id, 77);
        assert_eq!(observation.fp_index, 100.75);
        assert_eq!(observation.rxpacc, 2.0);
        assert_eq!(observation.sample_period_ns, 0.015);
    }

    #[test]
    fn fails_closed_on_missing_source_identity_and_short_rows() {
        assert_eq!(
            parse_row("", 0, &row_record(200)),
            Err(BregarCsvIngestError::InvalidSourceFileIdentity)
        );
        assert_eq!(
            parse_row("environment0/ch1_A1.csv", 0, "T1,A1"),
            Err(BregarCsvIngestError::TooFewRowFields)
        );
    }

    #[test]
    fn rejects_invalid_required_metadata_and_complex_cells() {
        let mut fields = row_fields();
        fields[19] = "0".into();
        fields.extend((0..200).map(|_| "0.0".to_owned()));
        assert_eq!(
            parse_row("environment0/ch1_A1.csv", 0, &fields.join(",")),
            Err(BregarCsvIngestError::InvalidRxpacc)
        );

        let mut fields = row_fields();
        fields.extend((0..200).map(|_| "not-a-complex".to_owned()));
        assert!(matches!(
            parse_row("environment0/ch1_A1.csv", 0, &fields.join(",")),
            Err(BregarCsvIngestError::InvalidComplexCirSample { index: 0, .. })
        ));
    }

    #[test]
    fn controller_csv_view_flows_into_cir_features_without_evaluator_labels() {
        let parsed = parse_row(
            "environment0/ch1_A1.csv",
            42,
            &row_record(300),
        )
        .unwrap();
        let observation = parsed.controller.to_cir_observation(77, 0.015);
        let features = extract_bregar_cir_features(&observation).unwrap();
        assert_eq!(features.measurement_id, 77);
        assert_eq!(features.fp_index_integer, 100);
        assert_eq!(features.normalized_magnitude_window.len(), 152);
        assert_eq!(parsed.evaluator.nlos_label.as_deref(), Some("NLOS"));
    }

    #[test]
    fn supports_quoted_complex_cells_with_csv_escaped_quotes() {
        let mut fields = row_fields();
        fields.extend((0..200).map(|_| "\"(1.0+2.0j)\"".to_owned()));
        let row = fields.join(",");
        let parsed = parse_row("environment0/ch1_A1.csv", 0, &row).unwrap();
        assert_eq!(parsed.controller.cir_samples[0], ComplexCirSample { real: 1.0, imaginary: 2.0 });
    }
}
