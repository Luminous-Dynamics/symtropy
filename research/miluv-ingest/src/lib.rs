//! Rust-first, loss-preserving MILUV CSV ingestion.
//!
//! This crate validates documented stream columns and basic numeric
//! invariants, while preserving every original field and source timestamp.
//! A numeric timestamp is NOT normalized to seconds, UTC, Vicon time or any
//! shared clock basis. Clock correlation and frame transforms must be supplied
//! by separately qualified benchmark metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Read;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiluvStream {
    UwbRange,
    UwbPassive,
    UwbCir,
    ImuCam,
    ImuPx4,
    Height,
    Magnetometer,
    Mocap,
}

impl MiluvStream {
    pub fn parse(value: &str) -> Result<Self, MiluvIngestError> {
        match value {
            "uwb_range" => Ok(Self::UwbRange),
            "uwb_passive" => Ok(Self::UwbPassive),
            "uwb_cir" => Ok(Self::UwbCir),
            "imu_cam" => Ok(Self::ImuCam),
            "imu_px4" => Ok(Self::ImuPx4),
            "height" => Ok(Self::Height),
            "mag" => Ok(Self::Magnetometer),
            "mocap" => Ok(Self::Mocap),
            other => Err(MiluvIngestError::UnknownStream(other.to_owned())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UwbRange => "uwb_range",
            Self::UwbPassive => "uwb_passive",
            Self::UwbCir => "uwb_cir",
            Self::ImuCam => "imu_cam",
            Self::ImuPx4 => "imu_px4",
            Self::Height => "height",
            Self::Magnetometer => "mag",
            Self::Mocap => "mocap",
        }
    }

    fn required_headers(self) -> &'static [&'static str] {
        match self {
            Self::UwbRange => &["timestamp", "range", "from_id", "to_id", "std"],
            Self::UwbPassive => {
                &["timestamp", "timestamp_n", "my_id", "from_id", "to_id", "std"]
            }
            Self::UwbCir => &["timestamp", "my_id", "from_id", "to_id", "idx", "cir"],
            Self::ImuCam | Self::ImuPx4 => &[
                "timestamp",
                "angular_velocity.x",
                "angular_velocity.y",
                "angular_velocity.z",
                "linear_acceleration.x",
                "linear_acceleration.y",
                "linear_acceleration.z",
            ],
            Self::Height => &["timestamp", "range"],
            Self::Magnetometer => &[
                "timestamp",
                "magnetic_field.x",
                "magnetic_field.y",
                "magnetic_field.z",
            ],
            Self::Mocap => &[
                "timestamp",
                "pose.position.x",
                "pose.position.y",
                "pose.position.z",
                "pose.orientation.x",
                "pose.orientation.y",
                "pose.orientation.z",
                "pose.orientation.w",
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiluvContext {
    pub dataset_id: String,
    pub release_id: String,
    pub experiment_id: String,
    pub robot_id: String,
    pub source_file_id: String,
}

impl MiluvContext {
    fn validate(&self) -> Result<(), MiluvIngestError> {
        for (name, value) in [
            ("dataset_id", &self.dataset_id),
            ("release_id", &self.release_id),
            ("experiment_id", &self.experiment_id),
            ("robot_id", &self.robot_id),
            ("source_file_id", &self.source_file_id),
        ] {
            if value.trim().is_empty() {
                return Err(MiluvIngestError::EmptyContextField(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeBasisStatus {
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCommitment {
    pub algorithm: &'static str,
    pub digest_hex: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MiluvCsvEvent {
    pub stream: MiluvStream,
    pub context: MiluvContext,
    pub row_index: u64,
    /// Original CSV field text is retained exactly as decoded by the CSV parser.
    pub source_timestamp_raw: String,
    /// Numeric validation is not a unit or time-scale conversion.
    pub source_timestamp_numeric: f64,
    pub time_basis_status: TimeBasisStatus,
    /// No coordinate frame or calibration is inferred by this parser.
    pub spatial_reference_status: &'static str,
    pub calibration_status: &'static str,
    pub fields: BTreeMap<String, String>,
    /// Event-level identity only. This is NOT the SHA-256 payload-file digest
    /// required by the separate dataset-admission manifest.
    pub event_commitment: ContentCommitment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiluvIngestSummary {
    pub stream: MiluvStream,
    pub event_count: usize,
    pub first_timestamp_raw: Option<String>,
    pub last_timestamp_raw: Option<String>,
    pub time_basis_status: TimeBasisStatus,
    pub spatial_reference_status: &'static str,
    pub calibration_status: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MiluvIngestError {
    UnknownStream(String),
    EmptyContextField(&'static str),
    Csv(String),
    MissingHeader(String),
    DuplicateHeader(String),
    EmptyHeader,
    InvalidTimestamp { row: u64, value: String },
    InvalidNumber { row: u64, field: String, value: String },
    InvalidRange { row: u64, value: f64 },
    InvalidStandardDeviation { row: u64, value: f64 },
    InvalidCirIndex { row: u64, value: String },
    InvalidQuaternion { row: u64 },
    RowIndexExhausted,
    MixedSummaryStreams,
    EventCountOverflow,
}

impl fmt::Display for MiluvIngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStream(stream) => write!(f, "unknown MILUV stream: {stream}"),
            Self::EmptyContextField(field) => write!(f, "context field must not be empty: {field}"),
            Self::Csv(error) => write!(f, "CSV parse error: {error}"),
            Self::MissingHeader(header) => write!(f, "required CSV header missing: {header}"),
            Self::DuplicateHeader(header) => write!(f, "duplicate CSV header: {header}"),
            Self::EmptyHeader => write!(f, "CSV header must not be empty"),
            Self::InvalidTimestamp { row, value } => {
                write!(f, "row {row} has invalid finite numeric timestamp: {value:?}")
            }
            Self::InvalidNumber { row, field, value } => {
                write!(f, "row {row} field {field:?} is not a finite number: {value:?}")
            }
            Self::InvalidRange { row, value } => {
                write!(f, "row {row} has negative or non-finite range: {value}")
            }
            Self::InvalidStandardDeviation { row, value } => {
                write!(f, "row {row} has non-positive or non-finite standard deviation: {value}")
            }
            Self::InvalidCirIndex { row, value } => {
                write!(f, "row {row} has invalid CIR peak index: {value:?}")
            }
            Self::InvalidQuaternion { row } => {
                write!(f, "row {row} has a zero-norm or non-finite orientation quaternion")
            }
            Self::RowIndexExhausted => write!(f, "MILUV CSV row index exhausted u64"),
            Self::MixedSummaryStreams => write!(f, "summary accumulator cannot mix MILUV streams"),
            Self::EventCountOverflow => write!(f, "MILUV event count overflowed usize"),
        }
    }
}

impl std::error::Error for MiluvIngestError {}

fn finite_number(
    fields: &BTreeMap<String, String>,
    field: &str,
    row: u64,
) -> Result<f64, MiluvIngestError> {
    let value = fields
        .get(field)
        .ok_or_else(|| MiluvIngestError::MissingHeader(field.to_owned()))?;
    let number = value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| MiluvIngestError::InvalidNumber {
            row,
            field: field.to_owned(),
            value: value.clone(),
        })?;
    Ok(number)
}

fn validate_row(
    stream: MiluvStream,
    fields: &BTreeMap<String, String>,
    row: u64,
) -> Result<f64, MiluvIngestError> {
    let timestamp_raw = fields
        .get("timestamp")
        .ok_or_else(|| MiluvIngestError::MissingHeader("timestamp".to_owned()))?;
    let timestamp = timestamp_raw
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| MiluvIngestError::InvalidTimestamp {
            row,
            value: timestamp_raw.clone(),
        })?;

    match stream {
        MiluvStream::UwbRange => {
            if fields["from_id"].trim().is_empty() || fields["to_id"].trim().is_empty() {
                return Err(MiluvIngestError::MissingHeader(
                    "nonempty from_id/to_id values".to_owned(),
                ));
            }
            let range = finite_number(fields, "range", row)?;
            if range < 0.0 {
                return Err(MiluvIngestError::InvalidRange { row, value: range });
            }
            let sigma = finite_number(fields, "std", row)?;
            if sigma <= 0.0 {
                return Err(MiluvIngestError::InvalidStandardDeviation { row, value: sigma });
            }
        }
        MiluvStream::UwbPassive => {
            if fields["my_id"].trim().is_empty()
                || fields["from_id"].trim().is_empty()
                || fields["to_id"].trim().is_empty()
            {
                return Err(MiluvIngestError::MissingHeader(
                    "nonempty my_id/from_id/to_id values".to_owned(),
                ));
            }
            let passive_timestamp = finite_number(fields, "timestamp_n", row)?;
            if !passive_timestamp.is_finite() {
                return Err(MiluvIngestError::InvalidTimestamp {
                    row,
                    value: fields["timestamp_n"].clone(),
                });
            }
            let sigma = finite_number(fields, "std", row)?;
            if sigma <= 0.0 {
                return Err(MiluvIngestError::InvalidStandardDeviation { row, value: sigma });
            }
        }
        MiluvStream::UwbCir => {
            if fields["my_id"].trim().is_empty()
                || fields["from_id"].trim().is_empty()
                || fields["to_id"].trim().is_empty()
            {
                return Err(MiluvIngestError::MissingHeader(
                    "nonempty my_id/from_id/to_id values".to_owned(),
                ));
            }
            let raw = fields.get("idx").expect("required header checked");
            if raw.parse::<u32>().is_err() {
                return Err(MiluvIngestError::InvalidCirIndex {
                    row,
                    value: raw.clone(),
                });
            }
        }
        MiluvStream::ImuCam | MiluvStream::ImuPx4 => {
            for field in [
                "angular_velocity.x",
                "angular_velocity.y",
                "angular_velocity.z",
                "linear_acceleration.x",
                "linear_acceleration.y",
                "linear_acceleration.z",
            ] {
                finite_number(fields, field, row)?;
            }
        }
        MiluvStream::Height => {
            finite_number(fields, "range", row)?;
        }
        MiluvStream::Magnetometer => {
            for field in [
                "magnetic_field.x",
                "magnetic_field.y",
                "magnetic_field.z",
            ] {
                finite_number(fields, field, row)?;
            }
        }
        MiluvStream::Mocap => {
            for field in [
                "pose.position.x",
                "pose.position.y",
                "pose.position.z",
            ] {
                finite_number(fields, field, row)?;
            }
            let q = [
                finite_number(fields, "pose.orientation.x", row)?,
                finite_number(fields, "pose.orientation.y", row)?,
                finite_number(fields, "pose.orientation.z", row)?,
                finite_number(fields, "pose.orientation.w", row)?,
            ];
            let norm_squared = q.iter().map(|value| value * value).sum::<f64>();
            if !norm_squared.is_finite() || norm_squared <= f64::EPSILON {
                return Err(MiluvIngestError::InvalidQuaternion { row });
            }
        }
    }

    Ok(timestamp)
}

fn event_commitment(
    stream: MiluvStream,
    context: &MiluvContext,
    row_index: u64,
    fields: &BTreeMap<String, String>,
) -> ContentCommitment {
    let mut hasher = blake3::Hasher::new();
    for value in [
        stream.as_str(),
        context.dataset_id.as_str(),
        context.release_id.as_str(),
        context.experiment_id.as_str(),
        context.robot_id.as_str(),
        context.source_file_id.as_str(),
    ] {
        hasher.update(value.as_bytes());
        hasher.update(&[0]);
    }
    hasher.update(&row_index.to_le_bytes());
    for (key, value) in fields {
        hasher.update(key.as_bytes());
        hasher.update(&[0]);
        hasher.update(value.as_bytes());
        hasher.update(&[0]);
    }
    ContentCommitment {
        algorithm: "blake3-event-v1",
        digest_hex: hasher.finalize().to_hex().to_string(),
    }
}

/// Bounded-memory reader for a single MILUV CSV stream.
///
/// Each call to `next_event` allocates at most one row event. The reader
/// preserves source timestamp text and every CSV field and does not convert
/// units, clock domains or coordinate frames.
pub struct MiluvCsvReader<R: Read> {
    reader: csv::Reader<R>,
    headers: csv::StringRecord,
    stream: MiluvStream,
    context: MiluvContext,
    next_row_index: u64,
}

impl<R: Read> MiluvCsvReader<R> {
    pub fn new(
        source: R,
        stream: MiluvStream,
        context: MiluvContext,
    ) -> Result<Self, MiluvIngestError> {
        context.validate()?;
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(false)
            .from_reader(source);
        let headers = reader
            .headers()
            .map_err(|error| MiluvIngestError::Csv(error.to_string()))?
            .clone();

        let mut header_set = BTreeSet::new();
        for header in headers.iter() {
            if header.trim().is_empty() {
                return Err(MiluvIngestError::EmptyHeader);
            }
            if !header_set.insert(header.to_owned()) {
                return Err(MiluvIngestError::DuplicateHeader(header.to_owned()));
            }
        }
        for required in stream.required_headers() {
            if !header_set.contains(*required) {
                return Err(MiluvIngestError::MissingHeader((*required).to_owned()));
            }
        }

        Ok(Self {
            reader,
            headers,
            stream,
            context,
            next_row_index: 0,
        })
    }

    pub fn next_event(&mut self) -> Result<Option<MiluvCsvEvent>, MiluvIngestError> {
        let mut record = csv::StringRecord::new();
        let has_record = self
            .reader
            .read_record(&mut record)
            .map_err(|error| MiluvIngestError::Csv(error.to_string()))?;
        if !has_record {
            return Ok(None);
        }

        let row_index = self.next_row_index;
        self.next_row_index = self
            .next_row_index
            .checked_add(1)
            .ok_or(MiluvIngestError::RowIndexExhausted)?;

        let mut fields = BTreeMap::new();
        for (header, value) in self.headers.iter().zip(record.iter()) {
            fields.insert(header.to_owned(), value.to_owned());
        }

        let numeric_timestamp = validate_row(self.stream, &fields, row_index)?;
        let source_timestamp_raw = fields["timestamp"].clone();
        let event_commitment =
            event_commitment(self.stream, &self.context, row_index, &fields);

        Ok(Some(MiluvCsvEvent {
            stream: self.stream,
            context: self.context.clone(),
            row_index,
            source_timestamp_raw,
            source_timestamp_numeric: numeric_timestamp,
            time_basis_status: TimeBasisStatus::Unknown,
            spatial_reference_status: "unknown",
            calibration_status: "unknown",
            fields,
            event_commitment,
        }))
    }

    pub fn stream(&self) -> MiluvStream {
        self.stream
    }

    pub fn context(&self) -> &MiluvContext {
        &self.context
    }
}

/// Streaming summary accumulator. It retains only counts and endpoint
/// timestamps, not the full event collection.
#[derive(Debug, Clone, PartialEq)]
pub struct MiluvSummaryAccumulator {
    stream: MiluvStream,
    event_count: usize,
    first_timestamp_raw: Option<String>,
    last_timestamp_raw: Option<String>,
}

impl MiluvSummaryAccumulator {
    pub fn new(stream: MiluvStream) -> Self {
        Self {
            stream,
            event_count: 0,
            first_timestamp_raw: None,
            last_timestamp_raw: None,
        }
    }

    pub fn observe(&mut self, event: &MiluvCsvEvent) -> Result<(), MiluvIngestError> {
        if event.stream != self.stream {
            return Err(MiluvIngestError::MixedSummaryStreams);
        }
        let next_count = self
            .event_count
            .checked_add(1)
            .ok_or(MiluvIngestError::EventCountOverflow)?;
        if self.first_timestamp_raw.is_none() {
            self.first_timestamp_raw = Some(event.source_timestamp_raw.clone());
        }
        self.last_timestamp_raw = Some(event.source_timestamp_raw.clone());
        self.event_count = next_count;
        Ok(())
    }

    pub fn summary(&self) -> Option<MiluvIngestSummary> {
        (self.event_count > 0).then(|| MiluvIngestSummary {
            stream: self.stream,
            event_count: self.event_count,
            first_timestamp_raw: self.first_timestamp_raw.clone(),
            last_timestamp_raw: self.last_timestamp_raw.clone(),
            time_basis_status: TimeBasisStatus::Unknown,
            spatial_reference_status: "unknown",
            calibration_status: "unknown",
        })
    }
}

/// Convenience API for small files/tests. Large dataset releases should use
/// `MiluvCsvReader` and `MiluvSummaryAccumulator` to keep memory bounded.
pub fn parse_miluv_csv<R: Read>(
    reader: R,
    stream: MiluvStream,
    context: MiluvContext,
) -> Result<Vec<MiluvCsvEvent>, MiluvIngestError> {
    let mut reader = MiluvCsvReader::new(reader, stream, context)?;
    let mut events = Vec::new();
    while let Some(event) = reader.next_event()? {
        events.push(event);
    }
    Ok(events)
}

pub fn summarize_events(events: &[MiluvCsvEvent]) -> Option<MiluvIngestSummary> {
    let first = events.first()?;
    Some(MiluvIngestSummary {
        stream: first.stream,
        event_count: events.len(),
        first_timestamp_raw: Some(first.source_timestamp_raw.clone()),
        last_timestamp_raw: events.last().map(|event| event.source_timestamp_raw.clone()),
        time_basis_status: TimeBasisStatus::Unknown,
        spatial_reference_status: "unknown",
        calibration_status: "unknown",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> MiluvContext {
        MiluvContext {
            dataset_id: "miluv-figshare-v1".to_owned(),
            release_id: "v1".to_owned(),
            experiment_id: "default_3_zigzag_0".to_owned(),
            robot_id: "ifo001".to_owned(),
            source_file_id: "ifo001/uwb_range.csv".to_owned(),
        }
    }


    #[test]
    fn streaming_reader_and_summary_use_bounded_row_state() {
        let csv = concat!(
            "timestamp,range,from_id,to_id,std\n",
            "1.000,3.5,10,4,0.1\n",
            "2.000,3.6,10,5,0.1\n",
            "3.000,3.7,10,6,0.1\n"
        );
        let mut reader =
            MiluvCsvReader::new(csv.as_bytes(), MiluvStream::UwbRange, context()).unwrap();
        let mut summary = MiluvSummaryAccumulator::new(MiluvStream::UwbRange);

        let first = reader.next_event().unwrap().unwrap();
        assert_eq!(first.row_index, 0);
        summary.observe(&first).unwrap();
        assert_eq!(summary.summary().unwrap().event_count, 1);

        let second = reader.next_event().unwrap().unwrap();
        assert_eq!(second.row_index, 1);
        summary.observe(&second).unwrap();

        let third = reader.next_event().unwrap().unwrap();
        assert_eq!(third.row_index, 2);
        summary.observe(&third).unwrap();

        assert!(reader.next_event().unwrap().is_none());
        let result = summary.summary().unwrap();
        assert_eq!(result.event_count, 3);
        assert_eq!(result.first_timestamp_raw.as_deref(), Some("1.000"));
        assert_eq!(result.last_timestamp_raw.as_deref(), Some("3.000"));
    }

    #[test]
    fn summary_accumulator_rejects_mixed_streams() {
        let range_csv = "timestamp,range,from_id,to_id,std\n1.0,3.5,10,4,0.1\n";
        let event =
            parse_miluv_csv(range_csv.as_bytes(), MiluvStream::UwbRange, context()).unwrap();
        let mut summary = MiluvSummaryAccumulator::new(MiluvStream::UwbPassive);
        assert_eq!(
            summary.observe(&event[0]),
            Err(MiluvIngestError::MixedSummaryStreams)
        );
    }

    #[test]
    fn uwb_range_parser_preserves_raw_timestamp_and_does_not_infer_time_basis() {
        let csv = "timestamp,range,from_id,to_id,std,range_raw,gt_range\n1.250,3.5,10,4,0.1,3.6,3.5\n";
        let events = parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbRange, context()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source_timestamp_raw, "1.250");
        assert_eq!(events[0].source_timestamp_numeric, 1.25);
        assert_eq!(events[0].time_basis_status, TimeBasisStatus::Unknown);
        assert_eq!(events[0].spatial_reference_status, "unknown");
        assert_eq!(events[0].fields["range_raw"], "3.6");
        assert_eq!(events[0].event_commitment.algorithm, "blake3-event-v1");
    }

    #[test]
    fn passive_uwb_stream_preserves_matched_neighbour_timestamp() {
        let csv = "timestamp,timestamp_n,my_id,from_id,to_id,rx1,rx2,rx3,std\n1.0,0.98,11,10,4,100,101,102,0.1\n";
        let events = parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbPassive, context()).unwrap();
        assert_eq!(events[0].fields["timestamp_n"], "0.98");
        assert_eq!(events[0].fields["my_id"], "11");
    }

    #[test]
    fn missing_required_column_is_rejected() {
        let csv = "timestamp,range,from_id,to_id\n1.0,3.5,10,4\n";
        assert!(matches!(
            parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbRange, context()),
            Err(MiluvIngestError::MissingHeader(field)) if field == "std"
        ));
    }

    #[test]
    fn non_finite_timestamp_and_range_are_rejected() {
        let bad_time = "timestamp,range,from_id,to_id,std\nNaN,3.5,10,4,0.1\n";
        assert!(matches!(
            parse_miluv_csv(bad_time.as_bytes(), MiluvStream::UwbRange, context()),
            Err(MiluvIngestError::InvalidTimestamp { .. })
        ));
        let bad_range = "timestamp,range,from_id,to_id,std\n1.0,-0.2,10,4,0.1\n";
        assert!(matches!(
            parse_miluv_csv(bad_range.as_bytes(), MiluvStream::UwbRange, context()),
            Err(MiluvIngestError::InvalidRange { .. })
        ));
    }

    #[test]
    fn duplicate_header_is_rejected() {
        let csv = "timestamp,range,from_id,to_id,std,range\n1.0,3.5,10,4,0.1,3.6\n";
        assert!(matches!(
            parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbRange, context()),
            Err(MiluvIngestError::DuplicateHeader(header)) if header == "range"
        ));
    }


    #[test]
    fn empty_transceiver_ids_are_rejected() {
        let csv = "timestamp,range,from_id,to_id,std\n1.0,3.5,,4,0.1\n";
        assert!(matches!(
            parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbRange, context()),
            Err(MiluvIngestError::MissingHeader(message))
                if message.contains("from_id")
        ));
    }

    #[test]
    fn zero_norm_mocap_orientation_is_rejected() {
        let csv = "timestamp,pose.position.x,pose.position.y,pose.position.z,pose.orientation.x,pose.orientation.y,pose.orientation.z,pose.orientation.w\n1.0,0,0,0,0,0,0,0\n";
        assert!(matches!(
            parse_miluv_csv(csv.as_bytes(), MiluvStream::Mocap, context()),
            Err(MiluvIngestError::InvalidQuaternion { row: 0 })
        ));
    }

    #[test]
    fn cir_index_is_validated_but_cir_payload_is_kept_raw() {
        let csv = "timestamp,my_id,from_id,to_id,idx,cir\n1.0,10,10,4,512,\"[1,2,3,4]\"\n";
        let events = parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbCir, context()).unwrap();
        assert_eq!(events[0].fields["idx"], "512");
        assert_eq!(events[0].fields["cir"], "[1,2,3,4]");
    }

    #[test]
    fn event_identity_is_stable_for_same_context_and_changes_across_experiments() {
        let csv = "timestamp,range,from_id,to_id,std\n1.0,3.5,10,4,0.1\n";
        let first = parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbRange, context()).unwrap();
        let second = parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbRange, context()).unwrap();
        assert_eq!(first[0].event_commitment, second[0].event_commitment);

        let mut other_context = context();
        other_context.experiment_id = "default_3_random_0".to_owned();
        let third =
            parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbRange, other_context).unwrap();
        assert_ne!(first[0].event_commitment, third[0].event_commitment);
    }

    #[test]
    fn imu_and_magnetometer_streams_validate_all_required_numeric_fields() {
        let imu = "timestamp,angular_velocity.x,angular_velocity.y,angular_velocity.z,linear_acceleration.x,linear_acceleration.y,linear_acceleration.z\n1,0,0,0,0,0,9.8\n";
        assert_eq!(
            parse_miluv_csv(imu.as_bytes(), MiluvStream::ImuPx4, context())
                .unwrap()
                .len(),
            1
        );
        let bad_mag = "timestamp,magnetic_field.x,magnetic_field.y,magnetic_field.z\n1,1,NaN,3\n";
        assert!(matches!(
            parse_miluv_csv(bad_mag.as_bytes(), MiluvStream::Magnetometer, context()),
            Err(MiluvIngestError::InvalidNumber { .. })
        ));
    }
}
