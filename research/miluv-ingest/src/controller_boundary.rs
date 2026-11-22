//! Explicit controller/evaluator partition for loss-preserving MILUV ingestion.
//!
//! Raw MiluvCsvEvent records retain every source CSV field for archival and
//! audit. They must not be sent directly to a controller because some UWB
//! streams contain ground-truth range and derived bias columns. This module
//! creates an allowlisted controller view, a separate evaluator-only truth
//! view, and a quarantine for fields that have not been classified.

use crate::{ContentCommitment, MiluvContext, MiluvCsvEvent, MiluvStream};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct MiluvControllerView {
    pub stream: MiluvStream,
    pub context: MiluvContext,
    pub row_index: u64,
    pub source_timestamp_raw: String,
    pub source_timestamp_numeric: f64,
    pub time_basis_status: crate::TimeBasisStatus,
    pub spatial_reference_status: &'static str,
    pub calibration_status: &'static str,
    pub fields: BTreeMap<String, String>,
    /// Commitment is computed over controller-visible fields only.
    pub controller_commitment: ContentCommitment,
}

/// Evaluator-only labels and reference truth. Never pass this type to a controller.
#[derive(Debug, Clone, PartialEq)]
pub struct MiluvEvaluatorTruthView {
    pub stream: MiluvStream,
    pub context: MiluvContext,
    pub row_index: u64,
    pub source_timestamp_raw: String,
    pub fields: BTreeMap<String, String>,
    /// Full original event commitment binds these labels to their source row.
    pub source_event_commitment: ContentCommitment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MiluvPartitionedEvent {
    /// None for evaluator-only streams such as motion-capture ground truth.
    pub controller: Option<MiluvControllerView>,
    pub evaluator_truth: Option<MiluvEvaluatorTruthView>,
    /// Unknown columns are quarantined rather than auto-promoted.
    pub quarantined_fields: BTreeMap<String, String>,
}

fn controller_allowlist(stream: MiluvStream) -> BTreeSet<&'static str> {
    let fields: &[&str] = match stream {
        MiluvStream::UwbRange => &[
            "timestamp", "range", "from_id", "to_id", "std", "range_raw",
            "tx1", "tx2", "tx3", "rx1", "rx2", "rx3",
            "fpp1", "fpp2", "fpp3", "skew1", "skew2", "skew3",
            "tx1_raw", "tx2_raw", "tx3_raw", "rx1_raw", "rx2_raw", "rx3_raw",
        ],
        MiluvStream::UwbPassive => &[
            "timestamp", "timestamp_n", "my_id", "from_id", "to_id", "std",
            "rx1", "rx2", "rx3", "fpp1", "fpp2", "fpp3", "skew1", "skew2", "skew3",
            "fpp1_n", "fpp2_n", "skew1_n", "skew2_n",
            "rx1_raw", "rx2_raw", "rx3_raw", "tx1_raw_n", "tx2_raw_n",
            "rx1_raw_n", "rx2_raw_n", "tx1_n", "tx2_n", "rx1_n", "rx2_n",
            "range_raw_n", "range_n", "std_n",
        ],
        MiluvStream::UwbCir => &["timestamp", "my_id", "from_id", "to_id", "idx", "cir"],
        MiluvStream::ImuCam | MiluvStream::ImuPx4 => &[
            "timestamp", "angular_velocity.x", "angular_velocity.y", "angular_velocity.z",
            "linear_acceleration.x", "linear_acceleration.y", "linear_acceleration.z",
        ],
        MiluvStream::Height => &["timestamp", "range"],
        MiluvStream::Magnetometer => &[
            "timestamp", "magnetic_field.x", "magnetic_field.y", "magnetic_field.z",
        ],
        MiluvStream::Mocap => &[],
    };
    fields.iter().copied().collect()
}

fn evaluator_truth_allowlist(stream: MiluvStream) -> BTreeSet<&'static str> {
    let fields: &[&str] = match stream {
        MiluvStream::UwbRange => &["gt_range", "bias", "bias_raw"],
        MiluvStream::UwbPassive => &["gt_range_n", "bias_n", "bias_raw_n", "gt_range", "bias", "bias_raw"],
        MiluvStream::Mocap => &[
            "timestamp", "pose.position.x", "pose.position.y", "pose.position.z",
            "pose.orientation.x", "pose.orientation.y", "pose.orientation.z", "pose.orientation.w",
        ],
        _ => &[],
    };
    fields.iter().copied().collect()
}

fn controller_commitment(
    stream: MiluvStream,
    context: &MiluvContext,
    row_index: u64,
    fields: &BTreeMap<String, String>,
) -> ContentCommitment {
    let mut hasher = blake3::Hasher::new();
    for value in [
        stream.as_str(), context.dataset_id.as_str(), context.release_id.as_str(),
        context.experiment_id.as_str(), context.robot_id.as_str(), context.source_file_id.as_str(),
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
        algorithm: "blake3-controller-view-v1",
        digest_hex: hasher.finalize().to_hex().to_string(),
    }
}

/// Partition one archival MILUV event into a controller allowlist,
/// evaluator-only labels and an explicit quarantine.
///
/// Controller code must consume only the returned controller view. The
/// archival MiluvCsvEvent.fields map is loss-preserving and may contain
/// ground truth; this partition is the required controller boundary.
pub fn partition_miluv_event(event: &MiluvCsvEvent) -> MiluvPartitionedEvent {
    let controller_allowed = controller_allowlist(event.stream);
    let evaluator_allowed = evaluator_truth_allowlist(event.stream);
    let mut controller_fields = BTreeMap::new();
    let mut evaluator_fields = BTreeMap::new();
    let mut quarantined_fields = BTreeMap::new();

    for (key, value) in &event.fields {
        if evaluator_allowed.contains(key.as_str()) {
            evaluator_fields.insert(key.clone(), value.clone());
        } else if controller_allowed.contains(key.as_str()) {
            controller_fields.insert(key.clone(), value.clone());
        } else {
            quarantined_fields.insert(key.clone(), value.clone());
        }
    }

    let controller = (!controller_fields.is_empty()).then(|| MiluvControllerView {
        stream: event.stream,
        context: event.context.clone(),
        row_index: event.row_index,
        source_timestamp_raw: event.source_timestamp_raw.clone(),
        source_timestamp_numeric: event.source_timestamp_numeric,
        time_basis_status: event.time_basis_status,
        spatial_reference_status: event.spatial_reference_status,
        calibration_status: event.calibration_status,
        controller_commitment: controller_commitment(
            event.stream, &event.context, event.row_index, &controller_fields,
        ),
        fields: controller_fields,
    });

    let evaluator_truth = (!evaluator_fields.is_empty()).then(|| MiluvEvaluatorTruthView {
        stream: event.stream,
        context: event.context.clone(),
        row_index: event.row_index,
        source_timestamp_raw: event.source_timestamp_raw.clone(),
        fields: evaluator_fields,
        source_event_commitment: event.event_commitment.clone(),
    });

    MiluvPartitionedEvent { controller, evaluator_truth, quarantined_fields }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_miluv_csv, MiluvContext};

    fn context(file: &str) -> MiluvContext {
        MiluvContext {
            dataset_id: "miluv-figshare-v1".to_owned(),
            release_id: "v1".to_owned(),
            experiment_id: "default_3_zigzag_0".to_owned(),
            robot_id: "ifo001".to_owned(),
            source_file_id: file.to_owned(),
        }
    }

    #[test]
    fn evaluator_truth_change_does_not_change_controller_view_commitment() {
        let a = "timestamp,range,from_id,to_id,std,range_raw,gt_range,bias,bias_raw\n1.250,3.5,10,4,0.1,3.6,3.5,0.0,0.1\n";
        let b = "timestamp,range,from_id,to_id,std,range_raw,gt_range,bias,bias_raw\n1.250,3.5,10,4,0.1,3.6,9.0,5.5,5.6\n";
        let event_a = parse_miluv_csv(a.as_bytes(), MiluvStream::UwbRange, context("uwb_range.csv")).unwrap();
        let event_b = parse_miluv_csv(b.as_bytes(), MiluvStream::UwbRange, context("uwb_range.csv")).unwrap();
        let part_a = partition_miluv_event(&event_a[0]);
        let part_b = partition_miluv_event(&event_b[0]);
        let input_a = part_a.controller.unwrap();
        let input_b = part_b.controller.unwrap();
        assert!(!input_a.fields.contains_key("gt_range"));
        assert!(!input_a.fields.contains_key("bias"));
        assert!(!input_a.fields.contains_key("bias_raw"));
        assert_eq!(input_a.fields["range"], "3.5");
        assert_eq!(input_a.controller_commitment.digest_hex, input_b.controller_commitment.digest_hex);
        let truth_a = part_a.evaluator_truth.unwrap();
        let truth_b = part_b.evaluator_truth.unwrap();
        assert_eq!(truth_a.fields["gt_range"], "3.5");
        assert_eq!(truth_b.fields["gt_range"], "9.0");
        assert_ne!(truth_a.source_event_commitment.digest_hex, truth_b.source_event_commitment.digest_hex);
    }

    #[test]
    fn mocap_is_evaluator_only_and_unknown_columns_are_quarantined() {
        let mocap = "timestamp,pose.position.x,pose.position.y,pose.position.z,pose.orientation.x,pose.orientation.y,pose.orientation.z,pose.orientation.w\n1.0,1,2,3,0,0,0,1\n";
        let events = parse_miluv_csv(mocap.as_bytes(), MiluvStream::Mocap, context("mocap.csv")).unwrap();
        let part = partition_miluv_event(&events[0]);
        assert!(part.controller.is_none());
        assert_eq!(part.evaluator_truth.unwrap().fields["pose.position.x"], "1");

        let range = "timestamp,range,from_id,to_id,std,debug_future_feature\n1.0,3.5,10,4,0.1,do-not-auto-promote\n";
        let events = parse_miluv_csv(range.as_bytes(), MiluvStream::UwbRange, context("uwb_range.csv")).unwrap();
        let part = partition_miluv_event(&events[0]);
        assert!(!part.controller.as_ref().unwrap().fields.contains_key("debug_future_feature"));
        assert_eq!(part.quarantined_fields["debug_future_feature"], "do-not-auto-promote");
    }

    #[test]
    fn passive_listener_truth_labels_are_separate_from_reception_fields() {
        let csv = "timestamp,timestamp_n,my_id,from_id,to_id,rx1,rx2,rx3,std,gt_range_n,bias_n\n1.0,0.98,11,10,4,100,101,102,0.1,3.5,0.2\n";
        let events = parse_miluv_csv(csv.as_bytes(), MiluvStream::UwbPassive, context("uwb_passive.csv")).unwrap();
        let part = partition_miluv_event(&events[0]);
        let input = part.controller.unwrap();
        assert!(input.fields.contains_key("timestamp_n"));
        assert!(input.fields.contains_key("rx1"));
        assert!(!input.fields.contains_key("gt_range_n"));
        assert!(!input.fields.contains_key("bias_n"));
        let truth = part.evaluator_truth.unwrap();
        assert_eq!(truth.fields["gt_range_n"], "3.5");
        assert_eq!(truth.fields["bias_n"], "0.2");
    }
}
