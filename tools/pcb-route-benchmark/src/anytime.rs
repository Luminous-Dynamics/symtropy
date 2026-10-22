use crate::kicad_drc::{evaluate_kicad_drc_report_bytes, KicadDrcEvidenceReceipt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    fs,
    path::{Component, Path},
};

pub const ANYTIME_ARCHIVE_SCHEMA: &str = "pcb-route-verified-incumbent-archive-v0.1";
pub const CHECKPOINT_SCHEMA: &str = "pcb-route-checkpoint-evaluation-v0.1";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchiveIdentity {
    pub board_id: String,
    pub input_board_sha256: String,
    pub dataset_manifest_sha256: String,
    pub drc_policy_sha256: String,
    pub kicad_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CheckpointMetadata {
    pub checkpoint_id: String,
    pub board_id: String,
    pub input_board_sha256: String,
    pub dataset_manifest_sha256: String,
    pub required_nets: u32,
    pub connected_nets: u32,
    pub dangling_track_count: u32,
    pub dangling_via_count: u32,
    pub elapsed_ms: u64,
    pub wirelength_mm: f64,
    pub via_count: u32,
    pub output_board_relative_path: String,
    pub drc_report_relative_path: String,
    pub board_filename: String,
    pub kicad_version: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CheckpointEvaluation {
    pub schema: String,
    pub checkpoint: CheckpointMetadata,
    pub output_board_sha256: String,
    pub raw_drc_report_sha256: String,
    pub drc_policy_sha256: String,
    pub drc_evidence: KicadDrcEvidenceReceipt,
    pub eligible_incumbent: bool,
    pub rejection_reasons: Vec<String>,
    pub board_drc_clean: bool,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VerifiedIncumbentArchive {
    pub schema: String,
    pub evaluator_git_head: String,
    pub identity: ArchiveIdentity,
    pub candidate_count: usize,
    pub eligible_checkpoint_count: usize,
    pub rejected_checkpoint_count: usize,
    pub candidates: Vec<CheckpointEvaluation>,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BudgetSelection {
    pub budget_ms: u64,
    pub attempts_seen_by_budget: usize,
    pub eligible_checkpoints_by_budget: usize,
    pub selected_checkpoint: Option<CheckpointEvaluation>,
    pub pareto_front: Vec<CheckpointEvaluation>,
    pub authority: String,
    pub claim_ceiling: String,
}

/// Verify a saved native board and its raw KiCad DRC report before marking it
/// as an incumbent. The caller's "success" label is intentionally not an input.
/// This checks geometric routing under a pinned policy, not electrical or
/// manufacturing qualification.
pub fn evaluate_checkpoint(
    metadata: CheckpointMetadata,
    manifest_bytes: &[u8],
    board_bytes: &[u8],
    drc_report_bytes: &[u8],
    policy_bytes: &[u8],
) -> Result<CheckpointEvaluation, String> {
    validate_metadata(&metadata)?;
    validate_relative_path(&metadata.output_board_relative_path, "output board")?;
    validate_relative_path(&metadata.drc_report_relative_path, "DRC report")?;
    if metadata.output_board_relative_path == metadata.drc_report_relative_path {
        return Err("output board and DRC report paths must be distinct".into());
    }
    if sha256_hex(manifest_bytes) != metadata.dataset_manifest_sha256 {
        return Err("checkpoint dataset manifest digest does not match supplied manifest bytes".into());
    }
    let manifest: crate::DatasetManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("checkpoint dataset manifest JSON invalid: {error}"))?;
    crate::validate_manifest(&manifest)?;
    let board_pin = manifest
        .boards
        .iter()
        .find(|board| board.board_id == metadata.board_id)
        .ok_or_else(|| format!("checkpoint board {} is not in the pinned dataset manifest", metadata.board_id))?;
    if board_pin.input_board_sha256 != metadata.input_board_sha256 {
        return Err("checkpoint source board digest does not match pinned dataset entry".into());
    }
    if board_pin.required_nets != metadata.required_nets {
        return Err("checkpoint required-net count does not match pinned dataset entry".into());
    }
    if board_bytes.is_empty() {
        return Err("checkpoint output board bytes must not be empty".into());
    }
    if drc_report_bytes.is_empty() {
        return Err("checkpoint raw DRC report bytes must not be empty".into());
    }
    let policy_sha256 = sha256_hex(policy_bytes);
    let evidence = evaluate_kicad_drc_report_bytes(
        drc_report_bytes,
        policy_bytes,
        &metadata.board_filename,
        &metadata.kicad_version,
    )?;

    let mut rejection_reasons = Vec::new();
    if metadata.connected_nets != metadata.required_nets {
        rejection_reasons.push("REQUIRED_NETS_NOT_ALL_CONNECTED".to_string());
    }
    if metadata.dangling_track_count > 0 {
        rejection_reasons.push("DANGLING_TRACKS_REMAIN".to_string());
    }
    if metadata.dangling_via_count > 0 {
        rejection_reasons.push("DANGLING_VIAS_REMAIN".to_string());
    }
    if !evidence.route_drc_pass {
        rejection_reasons.push("RAW_KICAD_ROUTING_DRC_FAILED".to_string());
    }
    if evidence.unconnected_item_count > 0 {
        rejection_reasons.push("RAW_KICAD_UNCONNECTED_ITEMS_REMAIN".to_string());
    }
    if evidence.schematic_parity_count > 0 {
        rejection_reasons.push("RAW_KICAD_SCHEMATIC_PARITY_FAILED".to_string());
    }
    rejection_reasons.sort();
    rejection_reasons.dedup();

    Ok(CheckpointEvaluation {
        schema: CHECKPOINT_SCHEMA.to_string(),
        checkpoint: metadata,
        output_board_sha256: sha256_hex(board_bytes),
        raw_drc_report_sha256: sha256_hex(drc_report_bytes),
        drc_policy_sha256: policy_sha256,
        board_drc_clean: evidence.board_drc_clean,
        eligible_incumbent: rejection_reasons.is_empty(),
        drc_evidence: evidence,
        rejection_reasons,
        authority: "native_kicad_drc_verified_route_checkpoint_only".to_string(),
        claim_ceiling: "Eligibility means this exact saved board had all declared required nets connected, no dangling route items, and zero blocking route findings under the supplied policy bytes. It does not prove schematic design intent beyond the report, SI/PI, manufacturability, physical quality, safety or production release.".to_string(),
    })
}


/// Preferred external path: resolve the declared artifact paths under a canonical root,
/// read the current file bytes, and only then derive checkpoint evidence. Symlink escapes,
/// path traversal, missing files, and aliases are rejected fail-closed.
pub fn evaluate_checkpoint_from_root(
    metadata: CheckpointMetadata,
    manifest_bytes: &[u8],
    artifact_root: &Path,
    policy_bytes: &[u8],
) -> Result<CheckpointEvaluation, String> {
    validate_metadata(&metadata)?;
    let root = artifact_root
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize checkpoint artifact root: {error}"))?;
    if !root.is_dir() {
        return Err("checkpoint artifact root is not a directory".into());
    }
    let board_path = root
        .join(&metadata.output_board_relative_path)
        .canonicalize()
        .map_err(|error| format!("cannot resolve output board: {error}"))?;
    let drc_path = root
        .join(&metadata.drc_report_relative_path)
        .canonicalize()
        .map_err(|error| format!("cannot resolve DRC report: {error}"))?;
    if !board_path.starts_with(&root) || !board_path.is_file() {
        return Err("output board is not a regular file contained by the checkpoint root".into());
    }
    if !drc_path.starts_with(&root) || !drc_path.is_file() {
        return Err("DRC report is not a regular file contained by the checkpoint root".into());
    }
    if board_path == drc_path {
        return Err("output board and DRC report resolve to the same file".into());
    }
    let board_bytes = fs::read(&board_path)
        .map_err(|error| format!("cannot read output board bytes: {error}"))?;
    let drc_bytes = fs::read(&drc_path)
        .map_err(|error| format!("cannot read raw DRC report bytes: {error}"))?;
    evaluate_checkpoint(metadata, manifest_bytes, &board_bytes, &drc_bytes, policy_bytes)
}

fn validate_relative_path(raw: &str, label: &str) -> Result<(), String> {
    if raw.trim().is_empty() || raw.contains('\\') || raw.contains(':') {
        return Err(format!("{label} path is empty or unsafe"));
    }
    let path = Path::new(raw);
    if path.is_absolute() {
        return Err(format!("{label} path must be relative"));
    }
    let mut has_normal_component = false;
    for component in path.components() {
        match component {
            Component::Normal(_) => has_normal_component = true,
            Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("{label} path contains traversal or prefix components"));
            }
        }
    }
    if !has_normal_component {
        return Err(format!("{label} path must name a file below the root"));
    }
    Ok(())
}

impl VerifiedIncumbentArchive {
    pub fn new(evaluator_git_head: &str, identity: ArchiveIdentity) -> Result<Self, String> {
        if !is_commit_sha(evaluator_git_head) {
            return Err("evaluator Git HEAD must be a 40- or 64-character hexadecimal commit ID".into());
        }
        validate_sha256(&identity.input_board_sha256, "input board SHA-256")?;
        validate_sha256(&identity.dataset_manifest_sha256, "dataset manifest SHA-256")?;
        validate_sha256(&identity.drc_policy_sha256, "DRC policy SHA-256")?;
        if identity.board_id.trim().is_empty() || identity.kicad_version.trim().is_empty() {
            return Err("archive board ID and KiCad version must not be empty".into());
        }
        Ok(Self {
            schema: ANYTIME_ARCHIVE_SCHEMA.to_string(),
            evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
            identity,
            candidate_count: 0,
            eligible_checkpoint_count: 0,
            rejected_checkpoint_count: 0,
            candidates: Vec::new(),
            authority: "monotone_verified_incumbent_archive_no_production_release_authority".to_string(),
            claim_ceiling: "The archive preserves time-stamped candidate evidence and selects only previously verified eligible snapshots. It does not prove physical quality, electrical integrity, manufacturing conformity or safety.".to_string(),
        })
    }

    /// Records failed candidates for audit but never lets them replace a verified
    /// checkpoint. All candidates must refer to the same source board, manifest,
    /// policy and engine version.
    pub fn record(&mut self, evaluation: CheckpointEvaluation) -> Result<(), String> {
        if evaluation.schema != CHECKPOINT_SCHEMA {
            return Err("unsupported checkpoint evaluation schema".into());
        }
        if evaluation.checkpoint.board_id != self.identity.board_id
            || evaluation.checkpoint.input_board_sha256 != self.identity.input_board_sha256
            || evaluation.checkpoint.dataset_manifest_sha256 != self.identity.dataset_manifest_sha256
            || evaluation.drc_policy_sha256 != self.identity.drc_policy_sha256
            || evaluation.checkpoint.kicad_version != self.identity.kicad_version
        {
            return Err("checkpoint identity differs from incumbent archive identity".into());
        }
        if self.candidates.iter().any(|candidate| {
            candidate.checkpoint.checkpoint_id == evaluation.checkpoint.checkpoint_id
        }) {
            return Err("duplicate checkpoint_id in incumbent archive".into());
        }
        if evaluation.eligible_incumbent && !evaluation.rejection_reasons.is_empty() {
            return Err("checkpoint claims eligibility while listing rejection reasons".into());
        }
        if !evaluation.eligible_incumbent && evaluation.rejection_reasons.is_empty() {
            return Err("ineligible checkpoint must retain at least one rejection reason".into());
        }
        self.candidate_count += 1;
        if evaluation.eligible_incumbent {
            self.eligible_checkpoint_count += 1;
        } else {
            self.rejected_checkpoint_count += 1;
        }
        self.candidates.push(evaluation);
        self.candidates.sort_by(|left, right| {
            left.checkpoint
                .elapsed_ms
                .cmp(&right.checkpoint.elapsed_ms)
                .then_with(|| left.checkpoint.checkpoint_id.cmp(&right.checkpoint.checkpoint_id))
        });
        Ok(())
    }

    /// Returns the deterministic Pareto front available by this deadline.
    /// Invalid attempts and checkpoints after the deadline cannot remove an incumbent.
    pub fn select_at_budget(&self, budget_ms: u64) -> BudgetSelection {
        let eligible: Vec<CheckpointEvaluation> = self
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.eligible_incumbent && candidate.checkpoint.elapsed_ms <= budget_ms
            })
            .cloned()
            .collect();
        let mut front = Vec::new();
        for candidate in &eligible {
            let dominated = eligible.iter().any(|other| {
                other.checkpoint.checkpoint_id != candidate.checkpoint.checkpoint_id
                    && dominates(other, candidate)
            });
            if !dominated {
                front.push(candidate.clone());
            }
        }
        front.sort_by(compare_quality);
        let selected = front.first().cloned();
        BudgetSelection {
            budget_ms,
            attempts_seen_by_budget: self
                .candidates
                .iter()
                .filter(|candidate| candidate.checkpoint.elapsed_ms <= budget_ms)
                .count(),
            eligible_checkpoints_by_budget: eligible.len(),
            selected_checkpoint: selected,
            pareto_front: front,
            authority: "best_verified_route_snapshot_observed_by_deadline".to_string(),
            claim_ceiling: "A time-budget selection only uses checkpoint snapshots whose recorded elapsed time is within the budget and whose raw KiCad DRC bytes were verified. This is not proof of a clean checkpoint at a time if no such checkpoint was saved.".to_string(),
        }
    }
}

fn validate_metadata(metadata: &CheckpointMetadata) -> Result<(), String> {
    if metadata.checkpoint_id.trim().is_empty()
        || metadata.board_id.trim().is_empty()
        || metadata.board_filename.trim().is_empty()
        || metadata.kicad_version.trim().is_empty()
    {
        return Err("checkpoint ID, board ID, board filename and KiCad version are required".into());
    }
    validate_sha256(&metadata.input_board_sha256, "input board SHA-256")?;
    validate_sha256(&metadata.dataset_manifest_sha256, "dataset manifest SHA-256")?;
    validate_relative_path(&metadata.output_board_relative_path, "output board")?;
    validate_relative_path(&metadata.drc_report_relative_path, "DRC report")?;
    if metadata.required_nets == 0 || metadata.connected_nets > metadata.required_nets {
        return Err("invalid required/connected net counts".into());
    }
    if metadata.elapsed_ms == 0 {
        return Err("checkpoint elapsed_ms must be positive".into());
    }
    if !metadata.wirelength_mm.is_finite() || metadata.wirelength_mm < 0.0 {
        return Err("checkpoint wirelength must be finite and non-negative".into());
    }
    Ok(())
}

fn dominates(left: &CheckpointEvaluation, right: &CheckpointEvaluation) -> bool {
    let no_worse = left.checkpoint.wirelength_mm <= right.checkpoint.wirelength_mm
        && left.checkpoint.via_count <= right.checkpoint.via_count;
    let strictly_better = left.checkpoint.wirelength_mm < right.checkpoint.wirelength_mm
        || left.checkpoint.via_count < right.checkpoint.via_count;
    let equal_quality = left.checkpoint.wirelength_mm == right.checkpoint.wirelength_mm
        && left.checkpoint.via_count == right.checkpoint.via_count;
    (no_worse && strictly_better)
        || (equal_quality && equal_quality_tiebreak(left, right) == Ordering::Less)
}

fn equal_quality_tiebreak(left: &CheckpointEvaluation, right: &CheckpointEvaluation) -> Ordering {
    left.checkpoint
        .elapsed_ms
        .cmp(&right.checkpoint.elapsed_ms)
        .then_with(|| left.output_board_sha256.cmp(&right.output_board_sha256))
}

fn compare_quality(left: &CheckpointEvaluation, right: &CheckpointEvaluation) -> Ordering {
    left.checkpoint
        .wirelength_mm
        .total_cmp(&right.checkpoint.wirelength_mm)
        .then_with(|| left.checkpoint.via_count.cmp(&right.checkpoint.via_count))
        .then_with(|| left.checkpoint.elapsed_ms.cmp(&right.checkpoint.elapsed_ms))
        .then_with(|| left.output_board_sha256.cmp(&right.output_board_sha256))
}

fn validate_sha256(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{label} must be a 64-character hexadecimal SHA-256"));
    }
    Ok(())
}

fn is_commit_sha(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: &[u8] = include_bytes!("../fixtures/kicad-drc-routing-policy-v0_1.json");
    const EVALUATOR_HEAD: &str = "abababababababababababababababababababab";
    const INPUT_SHA: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const MANIFEST: &str = r#"{
      "schema":"pcb-route-dataset-manifest-v0.1",
      "benchmark_id":"pcb-route-anytime-test",
      "benchmark_version":"0.1",
      "dataset_id":"synthetic-anytime",
      "split_id":"unit-test",
      "evidence_class":"synthetic",
      "source_url":"synthetic://pcb-route-anytime-tests",
      "boards":[{
        "board_id":"coupon",
        "input_board_sha256":"1111111111111111111111111111111111111111111111111111111111111111",
        "required_nets":2,
        "license":"synthetic-fixture",
        "source_url":"synthetic://coupon",
        "relative_path":"boards/coupon.kicad_pcb"
      }]
    }"#;

    fn metadata(id: &str, elapsed_ms: u64, wirelength_mm: f64, via_count: u32) -> CheckpointMetadata {
        CheckpointMetadata {
            checkpoint_id: id.to_string(),
            board_id: "coupon".to_string(),
            input_board_sha256: INPUT_SHA.to_string(),
            dataset_manifest_sha256: sha256_hex(MANIFEST.as_bytes()),
            required_nets: 2,
            connected_nets: 2,
            dangling_track_count: 0,
            dangling_via_count: 0,
            elapsed_ms,
            wirelength_mm,
            via_count,
            output_board_relative_path: format!("outputs/{id}.kicad_pcb"),
            drc_report_relative_path: format!("drc/{id}.json"),
            board_filename: "coupon.kicad_pcb".to_string(),
            kicad_version: "9.0.9".to_string(),
        }
    }

    fn drc_report(source: &str, violations: serde_json::Value, unconnected: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "$schema":"https://schemas.kicad.org/drc.v1.json",
            "source":source,
            "date":"2026-10-22T00:00:00+00:00",
            "kicad_version":"9.0.9",
            "coordinate_units":"mm",
            "included_severities":["error","warning"],
            "violations":violations,
            "unconnected_items":unconnected,
            "schematic_parity":[],
            "ignored_checks":[]
        })).unwrap()
    }

    fn clean_candidate(id: &str, elapsed_ms: u64, wirelength_mm: f64, vias: u32) -> CheckpointEvaluation {
        let report = drc_report("coupon.kicad_pcb", serde_json::json!([]), serde_json::json!([]));
        evaluate_checkpoint(
            metadata(id, elapsed_ms, wirelength_mm, vias),
            MANIFEST.as_bytes(),
            format!("native pcb bytes {id}").as_bytes(),
            &report,
            POLICY,
        ).unwrap()
    }

    #[test]
    fn invalid_later_checkpoint_never_erases_earlier_verified_clean_snapshot() {
        let first = clean_candidate("early-clean", 120_000, 40.0, 2);
        assert!(first.eligible_incumbent);
        let crossed = drc_report(
            "coupon.kicad_pcb",
            serde_json::json!([{
                "type":"tracks_crossing",
                "description":"synthetic crossing control",
                "severity":"error",
                "items":[
                    {"uuid":"11111111-1111-4111-8111-111111111111","description":"track A","pos":{"x":1.0,"y":2.0}},
                    {"uuid":"22222222-2222-4222-8222-222222222222","description":"track B","pos":{"x":1.0,"y":2.0}}
                ]
            }]),
            serde_json::json!([])
        );
        let invalid = evaluate_checkpoint(
            metadata("late-invalid", 210_000, 35.0, 1),
            MANIFEST.as_bytes(),
            b"invalid native board bytes",
            &crossed,
            POLICY,
        ).unwrap();
        assert!(!invalid.eligible_incumbent);
        assert!(invalid.rejection_reasons.contains(&"RAW_KICAD_ROUTING_DRC_FAILED".to_string()));

        let identity = ArchiveIdentity {
            board_id:"coupon".into(),
            input_board_sha256:INPUT_SHA.into(),
            dataset_manifest_sha256:sha256_hex(MANIFEST.as_bytes()),
            drc_policy_sha256:sha256_hex(POLICY),
            kicad_version:"9.0.9".into()
        };
        let mut archive = VerifiedIncumbentArchive::new(EVALUATOR_HEAD, identity).unwrap();
        archive.record(first).unwrap();
        archive.record(invalid).unwrap();

        let at_three_minutes = archive.select_at_budget(180_000);
        assert_eq!(at_three_minutes.eligible_checkpoints_by_budget, 1);
        assert_eq!(at_three_minutes.selected_checkpoint.unwrap().checkpoint.checkpoint_id, "early-clean");
        let at_five_minutes = archive.select_at_budget(300_000);
        assert_eq!(at_five_minutes.eligible_checkpoints_by_budget, 1);
        assert_eq!(archive.rejected_checkpoint_count, 1);
        assert_eq!(archive.eligible_checkpoint_count, 1);
    }

    #[test]
    fn pareto_front_keeps_shorter_wire_and_fewer_vias_tradeoffs() {
        let identity = ArchiveIdentity {
            board_id:"coupon".into(),
            input_board_sha256:INPUT_SHA.into(),
            dataset_manifest_sha256:sha256_hex(MANIFEST.as_bytes()),
            drc_policy_sha256:sha256_hex(POLICY),
            kicad_version:"9.0.9".into()
        };
        let mut archive = VerifiedIncumbentArchive::new(EVALUATOR_HEAD, identity).unwrap();
        archive.record(clean_candidate("low-wire-more-vias", 100_000, 30.0, 4)).unwrap();
        archive.record(clean_candidate("more-wire-low-vias", 110_000, 32.0, 1)).unwrap();
        archive.record(clean_candidate("dominated", 130_000, 35.0, 5)).unwrap();

        let selection = archive.select_at_budget(180_000);
        assert_eq!(selection.pareto_front.len(), 2);
        assert!(!selection.pareto_front.iter().any(|item| item.checkpoint.checkpoint_id == "dominated"));
        assert_eq!(selection.selected_checkpoint.unwrap().checkpoint.checkpoint_id, "low-wire-more-vias");
    }

    #[test]
    fn missing_net_connectivity_or_drc_bytes_cannot_create_an_incumbent() {
        let mut meta = metadata("missing-net", 100_000, 20.0, 0);
        meta.connected_nets = 1;
        let report = drc_report("coupon.kicad_pcb", serde_json::json!([]), serde_json::json!([]));
        let evaluation = evaluate_checkpoint(meta, MANIFEST.as_bytes(), b"board", &report, POLICY).unwrap();
        assert!(!evaluation.eligible_incumbent);
        assert!(evaluation.rejection_reasons.contains(&"REQUIRED_NETS_NOT_ALL_CONNECTED".to_string()));
        assert!(evaluate_checkpoint(metadata("empty-drc", 100_000, 20.0, 0), MANIFEST.as_bytes(), b"board", b"", POLICY).is_err());
    }

    #[test]
    fn archive_rejects_mixed_board_identity_and_duplicate_checkpoint_ids() {
        let identity = ArchiveIdentity {
            board_id:"coupon".into(),
            input_board_sha256:INPUT_SHA.into(),
            dataset_manifest_sha256:sha256_hex(MANIFEST.as_bytes()),
            drc_policy_sha256:sha256_hex(POLICY),
            kicad_version:"9.0.9".into()
        };
        let mut archive = VerifiedIncumbentArchive::new(EVALUATOR_HEAD, identity).unwrap();
        archive.record(clean_candidate("one", 100_000, 20.0, 1)).unwrap();
        assert!(archive.record(clean_candidate("one", 120_000, 18.0, 0)).unwrap_err().contains("duplicate checkpoint_id"));
        let mut altered = clean_candidate("other-board", 130_000, 18.0, 0);
        altered.checkpoint.board_id = "different".into();
        assert!(archive.record(altered).unwrap_err().contains("identity differs"));
    }
}
