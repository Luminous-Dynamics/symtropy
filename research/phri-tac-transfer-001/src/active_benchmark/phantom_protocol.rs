//! Traceable non-human tactile phantom campaign admission.
//!
//! This module audits whether a physical phantom campaign binds the exact
//! phantom, indenter, sensorized body, as-built taxel geometry, independent
//! six-axis reference, synchronization evidence and episode-level raw data.
//!
//! It checks metadata/receipt bindings and protocol completeness only. Receipt
//! identifiers are references, not proof that a calibration certificate,
//! reference chain or physical stimulus is valid. The module cannot verify
//! signatures, traceability, as-built dimensions or raw media contents.
//!
//! A six-axis wrench is an aggregate reference. It does not provide local
//! pressure ground truth for every taxel. Local-field accuracy requires a
//! separately characterized spatial reference or must remain an explicitly
//! underdetermined measurand.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use super::wrench_consistency::{
    ReferenceIndependenceDisposition, RigidFrameTransform3, TaxelGeometrySet,
    WrenchObservation, WrenchObservationBinding, WrenchReferenceEvidence, Vec3,
    build_wrench_operator,
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PhantomArticleIdentity {
    pub article_id: String,
    pub revision: String,
    pub material_lot_id: String,
    pub surface_profile_digest: String,
    pub nominal_geometry_digest: String,
    pub as_built_geometry_measurement_digest: String,
    pub material_characterization_receipt_id: String,
    pub dimensional_traceability_receipt_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct IndenterArticleIdentity {
    pub article_id: String,
    pub revision: String,
    pub material_lot_id: String,
    pub qualified_tip_profile_digests: Vec<String>,
    pub tip_geometry_characterization_receipt_id: String,
    pub pose_calibration_receipt_id: String,
    pub wear_state_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SensorizedBodyIdentity {
    pub sensor_article_id: String,
    pub sensor_family: String,
    pub manufacturing_lot_id: String,
    pub skin_article_id: String,
    pub skin_material_lot_id: String,
    pub electronics_revision: String,
    pub firmware_revision: String,
    pub readout_chain_digest: String,
    pub sensorized_body_context_digest: String,
    pub geometry_digest: String,
    pub frame_transform_digest: String,
    pub calibration_record_id: String,
    pub operating_envelope_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SixAxisReferenceIdentity {
    pub instrument_article_id: String,
    pub model: String,
    pub calibration_record_id: String,
    pub calibration_validity_receipt_id: String,
    pub covariance_characterization_receipt_id: String,
    pub reference_frame_digest: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub verifier_receipt_id: String,
    pub independence: ReferenceIndependenceDisposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum PhantomTestFamily {
    FlatIndentation,
    CurvedSurfaceIndentation,
    ContactLocalizationGrid,
    EdgeAndTileSeam,
    NormalLoadSweep,
    TangentialShear,
    TorsionalLoading,
    LoadingUnloadingHysteresis,
    StaticDwellAndCreep,
    ContactMigration,
    PartialSlip,
    TemperatureSweep,
    HumiditySweep,
    RepeatedCycleFatigue,
    DeadChannelNegativeControl,
    SaturationNegativeControl,
    FrameMisregistrationNegativeControl,
    RemountAndPreloadChange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum PhantomEpisodePartition {
    Calibration,
    Validation,
    HeldOutTest,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RequiredPhantomFamily {
    pub family: PhantomTestFamily,
    pub minimum_total_episodes: usize,
    pub minimum_heldout_episodes: usize,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PhantomEpisode {
    pub episode_id: String,
    pub contact_episode_id: String,
    pub session_id: String,
    pub partition: PhantomEpisodePartition,
    pub family: PhantomTestFamily,
    pub repeat_index: u32,
    pub sample_identity: String,
    pub native_observation_set_digest: String,
    pub raw_sensor_observation_digest: String,
    pub raw_reference_observation_digest: String,
    pub applied_stimulus_digest: String,
    pub contact_target_region_id: String,
    pub indenter_tip_profile_digest: String,
    pub surface_profile_digest: String,
    pub sensorized_body_context_digest: String,
    pub readout_chain_digest: String,
    pub geometry_digest: String,
    pub frame_transform_digest: String,
    pub operating_envelope_cell_id: String,
    pub temperature_c: f64,
    pub relative_humidity_percent: f64,
    pub sensor_reference_timestamp_ns: u64,
    pub wrench_reference_timestamp_ns: u64,
    pub maximum_permitted_sample_skew_ns: u64,
    pub observation: WrenchObservation,
    pub observation_binding: WrenchObservationBinding,
    pub reference_evidence: WrenchReferenceEvidence,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PhantomCampaignManifest {
    pub manifest_id: String,
    pub protocol_revision: String,
    pub phantom: PhantomArticleIdentity,
    pub indenter: IndenterArticleIdentity,
    pub sensorized_body: SensorizedBodyIdentity,
    pub six_axis_reference: SixAxisReferenceIdentity,
    pub geometry: TaxelGeometrySet,
    pub frame_transform: RigidFrameTransform3,
    pub required_families: Vec<RequiredPhantomFamily>,
    pub episodes: Vec<PhantomEpisode>,
    /// Set true for campaigns claiming probe-geometry generalization. The
    /// held-out probe profile must not appear in calibration or validation.
    pub require_heldout_indenter_geometry: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum PhantomAuditDisposition {
    AdmissibleCampaignManifest,
    InvalidManifest,
    EpisodeEvidenceUnresolved,
    CoverageIncomplete,
    PartitionLeakage,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PhantomFamilyCoverage {
    pub family: PhantomTestFamily,
    pub total_episodes: usize,
    pub calibration_episodes: usize,
    pub validation_episodes: usize,
    pub heldout_episodes: usize,
    pub required_total_episodes: usize,
    pub required_heldout_episodes: usize,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PhantomLeakageFinding {
    pub identity_dimension: String,
    pub identity: String,
    pub partitions: Vec<PhantomEpisodePartition>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PhantomCampaignAudit {
    pub disposition: PhantomAuditDisposition,
    pub manifest_id: String,
    pub episode_count: usize,
    pub family_coverage: Vec<PhantomFamilyCoverage>,
    pub leakage_findings: Vec<PhantomLeakageFinding>,
    pub invalid_episode_ids: Vec<String>,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

fn nonempty_fields(fields: &[(&str, &str)]) -> Vec<String> {
    fields
        .iter()
        .filter(|(_, value)| value.trim().is_empty())
        .map(|(name, _)| format!("required phantom manifest field {name} is missing"))
        .collect()
}

fn validate_spd_covariance(values: &[f64]) -> Result<(), String> {
    if values.len() != 36 || values.iter().any(|value| !value.is_finite()) {
        return Err("six-axis reference covariance must contain 36 finite values".to_owned());
    }
    let mut matrix = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            let value = values[row * 6 + column];
            let transpose = values[column * 6 + row];
            let scale = value.abs().max(transpose.abs()).max(f64::MIN_POSITIVE);
            if (value - transpose).abs() > 1.0e-12 * scale {
                return Err("six-axis reference covariance must be symmetric".to_owned());
            }
            matrix[row][column] = value;
        }
    }
    let mut lower = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..=row {
            let correction = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            if row == column {
                let diagonal = matrix[row][row] - correction;
                if !diagonal.is_finite() || diagonal <= 0.0 {
                    return Err("six-axis reference covariance must be positive definite".to_owned());
                }
                lower[row][column] = diagonal.sqrt();
            } else {
                lower[row][column] = (matrix[row][column] - correction) / lower[column][column];
            }
        }
    }
    Ok(())
}

fn validate_manifest_identity(manifest: &PhantomCampaignManifest) -> Vec<String> {
    let mut errors = nonempty_fields(&[
        ("manifest_id", &manifest.manifest_id),
        ("protocol_revision", &manifest.protocol_revision),
        ("phantom.article_id", &manifest.phantom.article_id),
        ("phantom.revision", &manifest.phantom.revision),
        ("phantom.material_lot_id", &manifest.phantom.material_lot_id),
        ("phantom.surface_profile_digest", &manifest.phantom.surface_profile_digest),
        ("phantom.nominal_geometry_digest", &manifest.phantom.nominal_geometry_digest),
        ("phantom.as_built_geometry_measurement_digest", &manifest.phantom.as_built_geometry_measurement_digest),
        ("phantom.material_characterization_receipt_id", &manifest.phantom.material_characterization_receipt_id),
        ("phantom.dimensional_traceability_receipt_id", &manifest.phantom.dimensional_traceability_receipt_id),
        ("indenter.article_id", &manifest.indenter.article_id),
        ("indenter.revision", &manifest.indenter.revision),
        ("indenter.material_lot_id", &manifest.indenter.material_lot_id),
        ("indenter.tip_geometry_characterization_receipt_id", &manifest.indenter.tip_geometry_characterization_receipt_id),
        ("indenter.pose_calibration_receipt_id", &manifest.indenter.pose_calibration_receipt_id),
        ("indenter.wear_state_digest", &manifest.indenter.wear_state_digest),
        ("sensor.sensor_article_id", &manifest.sensorized_body.sensor_article_id),
        ("sensor.sensor_family", &manifest.sensorized_body.sensor_family),
        ("sensor.manufacturing_lot_id", &manifest.sensorized_body.manufacturing_lot_id),
        ("sensor.skin_article_id", &manifest.sensorized_body.skin_article_id),
        ("sensor.skin_material_lot_id", &manifest.sensorized_body.skin_material_lot_id),
        ("sensor.electronics_revision", &manifest.sensorized_body.electronics_revision),
        ("sensor.firmware_revision", &manifest.sensorized_body.firmware_revision),
        ("sensor.readout_chain_digest", &manifest.sensorized_body.readout_chain_digest),
        ("sensor.sensorized_body_context_digest", &manifest.sensorized_body.sensorized_body_context_digest),
        ("sensor.geometry_digest", &manifest.sensorized_body.geometry_digest),
        ("sensor.frame_transform_digest", &manifest.sensorized_body.frame_transform_digest),
        ("sensor.calibration_record_id", &manifest.sensorized_body.calibration_record_id),
        ("sensor.operating_envelope_id", &manifest.sensorized_body.operating_envelope_id),
        ("reference.instrument_article_id", &manifest.six_axis_reference.instrument_article_id),
        ("reference.model", &manifest.six_axis_reference.model),
        ("reference.calibration_record_id", &manifest.six_axis_reference.calibration_record_id),
        ("reference.calibration_validity_receipt_id", &manifest.six_axis_reference.calibration_validity_receipt_id),
        ("reference.covariance_characterization_receipt_id", &manifest.six_axis_reference.covariance_characterization_receipt_id),
        ("reference.reference_frame_digest", &manifest.six_axis_reference.reference_frame_digest),
        ("reference.reference_ancestry_receipt_id", &manifest.six_axis_reference.reference_ancestry_receipt_id),
        ("reference.common_cause_analysis_receipt_id", &manifest.six_axis_reference.common_cause_analysis_receipt_id),
        ("reference.verifier_receipt_id", &manifest.six_axis_reference.verifier_receipt_id),
    ]);
    if manifest.indenter.qualified_tip_profile_digests.is_empty() {
        errors.push("indenter has no qualified tip profiles".to_owned());
    }
    if manifest.six_axis_reference.independence != ReferenceIndependenceDisposition::IndependentlyCharacterized {
        errors.push("six-axis reference independence is unresolved or shares a common cause".to_owned());
    }
    if manifest.phantom.nominal_geometry_digest != manifest.geometry.geometry_digest
        || manifest.phantom.as_built_geometry_measurement_digest.trim().is_empty()
        || manifest.sensorized_body.geometry_digest != manifest.geometry.geometry_digest
    {
        errors.push("phantom, sensor and provided as-built geometry digests are inconsistent".to_owned());
    }
    if manifest.sensorized_body.frame_transform_digest != manifest.frame_transform.transform_digest {
        errors.push("sensor identity does not bind the supplied rigid frame transform".to_owned());
    }
    if manifest.six_axis_reference.reference_frame_digest != manifest.geometry.reference_frame_digest {
        errors.push("six-axis reference frame does not match the as-built geometry frame".to_owned());
    }
    if let Err(error) = build_wrench_operator(&manifest.geometry) {
        errors.push(format!("as-built tactile geometry is invalid: {error}"));
    }
    if manifest.frame_transform.target_frame_digest != manifest.geometry.reference_frame_digest {
        errors.push("frame transform target does not match the declared geometry/reference frame".to_owned());
    }
    if manifest.required_families.is_empty() {
        errors.push("phantom campaign declares no required test families".to_owned());
    }
    let mut family_ids = BTreeSet::new();
    for requirement in &manifest.required_families {
        if !family_ids.insert(requirement.family) {
            errors.push(format!("required test family {:?} is duplicated", requirement.family));
        }
        if requirement.minimum_total_episodes == 0 || requirement.minimum_heldout_episodes == 0 {
            errors.push(format!("required test family {:?} must request at least one total and held-out episode", requirement.family));
        }
        if requirement.minimum_heldout_episodes > requirement.minimum_total_episodes {
            errors.push(format!("required held-out count exceeds total for family {:?}", requirement.family));
        }
    }
    errors
}

fn validate_episode(
    episode: &PhantomEpisode,
    manifest: &PhantomCampaignManifest,
) -> Vec<String> {
    let mut errors = nonempty_fields(&[
        ("episode_id", &episode.episode_id),
        ("contact_episode_id", &episode.contact_episode_id),
        ("session_id", &episode.session_id),
        ("sample_identity", &episode.sample_identity),
        ("native_observation_set_digest", &episode.native_observation_set_digest),
        ("raw_sensor_observation_digest", &episode.raw_sensor_observation_digest),
        ("raw_reference_observation_digest", &episode.raw_reference_observation_digest),
        ("applied_stimulus_digest", &episode.applied_stimulus_digest),
        ("contact_target_region_id", &episode.contact_target_region_id),
        ("indenter_tip_profile_digest", &episode.indenter_tip_profile_digest),
        ("surface_profile_digest", &episode.surface_profile_digest),
        ("sensorized_body_context_digest", &episode.sensorized_body_context_digest),
        ("readout_chain_digest", &episode.readout_chain_digest),
        ("geometry_digest", &episode.geometry_digest),
        ("frame_transform_digest", &episode.frame_transform_digest),
        ("operating_envelope_cell_id", &episode.operating_envelope_cell_id),
    ]);
    if !episode.temperature_c.is_finite()
        || !(-80.0..=250.0).contains(&episode.temperature_c)
        || !episode.relative_humidity_percent.is_finite()
        || !(0.0..=100.0).contains(&episode.relative_humidity_percent)
    {
        errors.push("episode temperature or relative humidity is physically invalid".to_owned());
    }
    if episode.indenter_tip_profile_digest.trim().is_empty()
        || !manifest.indenter.qualified_tip_profile_digests.contains(&episode.indenter_tip_profile_digest)
    {
        errors.push("episode indenter tip profile is not part of the characterized indenter article".to_owned());
    }
    if episode.surface_profile_digest != manifest.phantom.surface_profile_digest {
        errors.push("episode surface profile does not match the characterized phantom surface".to_owned());
    }
    if episode.sensorized_body_context_digest != manifest.sensorized_body.sensorized_body_context_digest
        || episode.readout_chain_digest != manifest.sensorized_body.readout_chain_digest
    {
        errors.push("episode does not bind the exact sensorized-body and readout context".to_owned());
    }
    if episode.geometry_digest != manifest.geometry.geometry_digest
        || episode.frame_transform_digest != manifest.frame_transform.transform_digest
    {
        errors.push("episode geometry or frame-transform digest does not match the campaign".to_owned());
    }
    if !episode.observation.force_n.is_finite()
        || !episode.observation.moment_nm.is_finite()
    {
        errors.push("six-axis reference observation contains non-finite values".to_owned());
    }
    if let Err(error) = validate_spd_covariance(&episode.observation.covariance.matrix_row_major) {
        errors.push(format!("episode reference covariance invalid: {error}"));
    }
    let sample_skew_ns = episode.sensor_reference_timestamp_ns.abs_diff(episode.wrench_reference_timestamp_ns);
    if sample_skew_ns > episode.maximum_permitted_sample_skew_ns {
        errors.push("sensor and six-axis reference timestamps exceed the permitted same-contact sample skew".to_owned());
    }
    if episode.maximum_permitted_sample_skew_ns == 0 {
        errors.push("episode maximum permitted sample skew must be positive".to_owned());
    }

    let binding = &episode.observation_binding;
    if binding.sample_identity != episode.sample_identity
        || binding.native_observation_set_digest != episode.native_observation_set_digest
        || binding.wrench_observation_digest != episode.raw_reference_observation_digest
    {
        errors.push("wrench observation binding does not match episode sample/native/raw-reference identities".to_owned());
    }

    let evidence = &episode.reference_evidence;
    if evidence.sample_identity != episode.sample_identity
        || evidence.native_observation_set_digest != episode.native_observation_set_digest
        || evidence.wrench_observation_digest != episode.raw_reference_observation_digest
    {
        errors.push("reference evidence does not bind the exact episode sample/native/raw-reference identities".to_owned());
    }
    if evidence.reference_frame_digest != manifest.geometry.reference_frame_digest
        || evidence.contact_boundary_digest != manifest.geometry.contact_boundary_digest
    {
        errors.push("reference evidence frame or contact boundary does not match the physical geometry".to_owned());
    }
    if evidence.reference_independence != ReferenceIndependenceDisposition::IndependentlyCharacterized
        || manifest.six_axis_reference.independence != ReferenceIndependenceDisposition::IndependentlyCharacterized
    {
        errors.push("episode six-axis reference ancestry/common-cause evidence is unresolved".to_owned());
    }
    if evidence.timestamp_alignment_uncertainty_ns > evidence.maximum_permitted_alignment_uncertainty_ns {
        errors.push("episode reference evidence exceeds its own timestamp-alignment uncertainty bound".to_owned());
    }
    if evidence.timestamp_alignment_uncertainty_ns > episode.maximum_permitted_sample_skew_ns {
        errors.push("episode timestamp-alignment uncertainty exceeds the declared sample-skew budget".to_owned());
    }
    for (name, value) in [
        ("reference instrument receipt", evidence.reference_instrument_receipt_id.as_str()),
        ("reference ancestry receipt", evidence.reference_ancestry_receipt_id.as_str()),
        ("common-cause analysis receipt", evidence.common_cause_analysis_receipt_id.as_str()),
        ("verifier receipt", evidence.verifier_receipt_id.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("episode {name} is missing"));
        }
    }
    if evidence.reference_instrument_receipt_id != manifest.six_axis_reference.calibration_record_id
        && evidence.reference_instrument_receipt_id != manifest.six_axis_reference.calibration_validity_receipt_id
    {
        errors.push("episode reference instrument receipt is not bound to the declared six-axis reference identity".to_owned());
    }
    errors
}

fn add_leakage(
    findings: &mut Vec<PhantomLeakageFinding>,
    dimension: &str,
    identity: &str,
    partition: PhantomEpisodePartition,
) {
    if let Some(existing) = findings
        .iter_mut()
        .find(|finding| finding.identity_dimension == dimension && finding.identity == identity)
    {
        if !existing.partitions.contains(&partition) {
            existing.partitions.push(partition);
            existing.partitions.sort();
        }
    } else {
        findings.push(PhantomLeakageFinding {
            identity_dimension: dimension.to_owned(),
            identity: identity.to_owned(),
            partitions: vec![partition],
        });
    }
}

pub fn audit_phantom_campaign(manifest: &PhantomCampaignManifest) -> PhantomCampaignAudit {
    let mut reasons = validate_manifest_identity(manifest);
    let mut invalid_episode_ids = Vec::new();
    let mut seen_episode_ids = BTreeSet::new();
    let mut seen_contact_ids = BTreeSet::new();
    let mut seen_sample_ids = BTreeSet::new();
    let mut seen_raw_sensor = BTreeSet::new();
    let mut seen_raw_reference = BTreeSet::new();
    let mut counts: BTreeMap<PhantomTestFamily, [usize; 3]> = BTreeMap::new();
    let mut identity_partitions: BTreeMap<(&'static str, String), BTreeSet<PhantomEpisodePartition>> =
        BTreeMap::new();
    let mut profile_partitions: BTreeMap<String, BTreeSet<PhantomEpisodePartition>> = BTreeMap::new();

    for episode in &manifest.episodes {
        let mut episode_errors = validate_episode(episode, manifest);
        for (dimension, identity, seen) in [
            ("episode", episode.episode_id.as_str(), &mut seen_episode_ids),
            ("contact_episode", episode.contact_episode_id.as_str(), &mut seen_contact_ids),
            ("sample", episode.sample_identity.as_str(), &mut seen_sample_ids),
            ("raw_sensor", episode.raw_sensor_observation_digest.as_str(), &mut seen_raw_sensor),
            ("raw_reference", episode.raw_reference_observation_digest.as_str(), &mut seen_raw_reference),
        ] {
            if !seen.insert(identity.to_owned()) {
                episode_errors.push(format!("duplicate {dimension} identity {identity}"));
            }
        }

        let partition_index = match episode.partition {
            PhantomEpisodePartition::Calibration => 0,
            PhantomEpisodePartition::Validation => 1,
            PhantomEpisodePartition::HeldOutTest => 2,
        };
        counts.entry(episode.family).or_insert([0; 3])[partition_index] += 1;
        identity_partitions
            .entry(("session", episode.session_id.clone()))
            .or_default()
            .insert(episode.partition);
        if manifest.require_heldout_indenter_geometry {
            profile_partitions
                .entry(episode.indenter_tip_profile_digest.clone())
                .or_default()
                .insert(episode.partition);
        }

        if !episode_errors.is_empty() {
            invalid_episode_ids.push(episode.episode_id.clone());
            reasons.extend(
                episode_errors
                    .into_iter()
                    .map(|reason| format!("episode {}: {reason}", episode.episode_id)),
            );
        }
    }

    let mut leakage_findings = Vec::new();
    for ((dimension, identity), partitions) in identity_partitions {
        if partitions.len() > 1 {
            for partition in &partitions {
                add_leakage(&mut leakage_findings, dimension, &identity, *partition);
            }
        }
    }
    if manifest.require_heldout_indenter_geometry {
        for (profile, partitions) in profile_partitions {
            if partitions.contains(&PhantomEpisodePartition::HeldOutTest)
                && (partitions.contains(&PhantomEpisodePartition::Calibration)
                    || partitions.contains(&PhantomEpisodePartition::Validation))
            {
                for partition in &partitions {
                    add_leakage(&mut leakage_findings, "indenter_tip_profile", &profile, *partition);
                }
            }
        }
    }

    let family_coverage = manifest
        .required_families
        .iter()
        .map(|required| {
            let counts = counts.get(&required.family).copied().unwrap_or([0; 3]);
            let total = counts.iter().sum::<usize>();
            let complete = total >= required.minimum_total_episodes
                && counts[2] >= required.minimum_heldout_episodes;
            if !complete {
                reasons.push(format!(
                    "family {:?} coverage incomplete: total {total}/{}, held-out {}/{}",
                    required.family,
                    required.minimum_total_episodes,
                    counts[2],
                    required.minimum_heldout_episodes
                ));
            }
            PhantomFamilyCoverage {
                family: required.family,
                total_episodes: total,
                calibration_episodes: counts[0],
                validation_episodes: counts[1],
                heldout_episodes: counts[2],
                required_total_episodes: required.minimum_total_episodes,
                required_heldout_episodes: required.minimum_heldout_episodes,
                complete,
            }
        })
        .collect::<Vec<_>>();

    if !leakage_findings.is_empty() {
        reasons.push("session or declared held-out indenter-profile identity crosses partition boundaries".to_owned());
    }
    let coverage_incomplete = family_coverage.iter().any(|coverage| !coverage.complete);
    let disposition = if !leakage_findings.is_empty() {
        PhantomAuditDisposition::PartitionLeakage
    } else if !invalid_episode_ids.is_empty() {
        PhantomAuditDisposition::EpisodeEvidenceUnresolved
    } else if !reasons.is_empty() {
        PhantomAuditDisposition::InvalidManifest
    } else if coverage_incomplete {
        PhantomAuditDisposition::CoverageIncomplete
    } else {
        PhantomAuditDisposition::AdmissibleCampaignManifest
    };

    PhantomCampaignAudit {
        disposition,
        manifest_id: manifest.manifest_id.clone(),
        episode_count: manifest.episodes.len(),
        family_coverage,
        leakage_findings,
        invalid_episode_ids,
        reasons,
        claim_class: "phantom_campaign_manifest_admission_only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::wrench_consistency::{WrenchCovariance6, WrenchVector};

    fn geometry() -> TaxelGeometrySet {
        TaxelGeometrySet {
            geometry_digest: "sha256:phantom-as-built-geometry".to_owned(),
            reference_frame_digest: "sha256:phantom-reference-frame".to_owned(),
            contact_boundary_digest: "sha256:phantom-contact-boundary".to_owned(),
            wrench_origin_m: Vec3::default(),
            taxels: vec![
                super::super::wrench_consistency::TaxelGeometry {
                    taxel_id: "taxel-0".to_owned(),
                    area_m2: 1.0e-4,
                    application_point_m: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                    normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                    tangent_u: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
                    tangent_v: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                }
            ],
        }
    }

    fn episode(family: PhantomTestFamily, partition: PhantomEpisodePartition, id: &str) -> PhantomEpisode {
        let cov = (0..6).map(|index| if index % 7 == 0 { 1.0e-4 } else { 0.0 }).collect();
        PhantomEpisode {
            episode_id: format!("episode-{id}"),
            contact_episode_id: format!("contact-{id}"),
            session_id: format!("session-{id}"),
            partition,
            family,
            repeat_index: 0,
            sample_identity: format!("sample-{id}"),
            native_observation_set_digest: format!("sha256:native-{id}"),
            raw_sensor_observation_digest: format!("sha256:sensor-{id}"),
            raw_reference_observation_digest: format!("sha256:reference-{id}"),
            applied_stimulus_digest: "sha256:stimulus-profile".to_owned(),
            contact_target_region_id: "center-cell".to_owned(),
            indenter_tip_profile_digest: "sha256:tip-flat-2mm".to_owned(),
            surface_profile_digest: "sha256:phantom-surface".to_owned(),
            sensorized_body_context_digest: "sha256:sensorized-body".to_owned(),
            readout_chain_digest: "sha256:readout-chain".to_owned(),
            geometry_digest: "sha256:phantom-as-built-geometry".to_owned(),
            frame_transform_digest: "sha256:frame-transform".to_owned(),
            operating_envelope_cell_id: "cell-room-temp-low-rate".to_owned(),
            temperature_c: 23.0,
            relative_humidity_percent: 50.0,
            sensor_reference_timestamp_ns: 1_000_000_000,
            wrench_reference_timestamp_ns: 1_000_000_200,
            maximum_permitted_sample_skew_ns: 1_000,
            observation: WrenchObservation {
                force_n: Vec3 { x: 0.0, y: 0.0, z: 2.0 },
                moment_nm: Vec3::default(),
                covariance: WrenchCovariance6 { matrix_row_major: cov },
            },
            observation_binding: WrenchObservationBinding {
                sample_identity: format!("sample-{id}"),
                native_observation_set_digest: format!("sha256:native-{id}"),
                wrench_observation_digest: format!("sha256:reference-{id}"),
            },
            reference_evidence: WrenchReferenceEvidence {
                native_observation_set_digest: format!("sha256:native-{id}"),
                wrench_observation_digest: format!("sha256:reference-{id}"),
                sample_identity: format!("sample-{id}"),
                reference_frame_digest: "sha256:phantom-reference-frame".to_owned(),
                contact_boundary_digest: "sha256:phantom-contact-boundary".to_owned(),
                reference_instrument_receipt_id: "receipt:ref-calibration-validity".to_owned(),
                reference_ancestry_receipt_id: "receipt:ref-ancestry".to_owned(),
                common_cause_analysis_receipt_id: "receipt:ref-common-cause".to_owned(),
                verifier_receipt_id: "receipt:episode-verifier".to_owned(),
                reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
                timestamp_alignment_uncertainty_ns: 100,
                maximum_permitted_alignment_uncertainty_ns: 500,
            },
        }
    }

    fn manifest() -> PhantomCampaignManifest {
        PhantomCampaignManifest {
            manifest_id: "phantom-campaign-001".to_owned(),
            protocol_revision: "phri-tac-phantom-v0.1".to_owned(),
            phantom: PhantomArticleIdentity {
                article_id: "phantom-article-01".to_owned(),
                revision: "phantom-r1".to_owned(),
                material_lot_id: "silicone-lot-01".to_owned(),
                surface_profile_digest: "sha256:phantom-surface".to_owned(),
                nominal_geometry_digest: "sha256:phantom-as-built-geometry".to_owned(),
                as_built_geometry_measurement_digest: "sha256:as-built-measurement".to_owned(),
                material_characterization_receipt_id: "receipt:material-char".to_owned(),
                dimensional_traceability_receipt_id: "receipt:dimension-trace".to_owned(),
            },
            indenter: IndenterArticleIdentity {
                article_id: "indenter-01".to_owned(),
                revision: "indenter-r1".to_owned(),
                material_lot_id: "steel-tip-lot-01".to_owned(),
                qualified_tip_profile_digests: vec!["sha256:tip-flat-2mm".to_owned()],
                tip_geometry_characterization_receipt_id: "receipt:tip-geometry".to_owned(),
                pose_calibration_receipt_id: "receipt:pose-cal".to_owned(),
                wear_state_digest: "sha256:tip-wear-state".to_owned(),
            },
            sensorized_body: SensorizedBodyIdentity {
                sensor_article_id: "sensor-article-01".to_owned(),
                sensor_family: "resistive-array".to_owned(),
                manufacturing_lot_id: "sensor-lot-01".to_owned(),
                skin_article_id: "skin-article-01".to_owned(),
                skin_material_lot_id: "skin-lot-01".to_owned(),
                electronics_revision: "pcb-r1".to_owned(),
                firmware_revision: "fw-r1".to_owned(),
                readout_chain_digest: "sha256:readout-chain".to_owned(),
                sensorized_body_context_digest: "sha256:sensorized-body".to_owned(),
                geometry_digest: "sha256:phantom-as-built-geometry".to_owned(),
                frame_transform_digest: "sha256:frame-transform".to_owned(),
                calibration_record_id: "calibration:array-r1".to_owned(),
                operating_envelope_id: "envelope:room-temp".to_owned(),
            },
            six_axis_reference: SixAxisReferenceIdentity {
                instrument_article_id: "ref-article-01".to_owned(),
                model: "ATI Axia80".to_owned(),
                calibration_record_id: "calibration:reference".to_owned(),
                calibration_validity_receipt_id: "receipt:ref-calibration-validity".to_owned(),
                covariance_characterization_receipt_id: "receipt:ref-covariance".to_owned(),
                reference_frame_digest: "sha256:phantom-reference-frame".to_owned(),
                reference_ancestry_receipt_id: "receipt:ref-ancestry".to_owned(),
                common_cause_analysis_receipt_id: "receipt:ref-common-cause".to_owned(),
                verifier_receipt_id: "receipt:ref-verifier".to_owned(),
                independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
            },
            geometry: geometry(),
            frame_transform: RigidFrameTransform3 {
                transform_digest: "sha256:frame-transform".to_owned(),
                source_frame_digest: "sha256:phantom-fixture-frame".to_owned(),
                target_frame_digest: "sha256:phantom-reference-frame".to_owned(),
                rotation_row_major: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
                translation_m: Vec3::default(),
            },
            required_families: vec![
                RequiredPhantomFamily {
                    family: PhantomTestFamily::FlatIndentation,
                    minimum_total_episodes: 3,
                    minimum_heldout_episodes: 1,
                },
                RequiredPhantomFamily {
                    family: PhantomTestFamily::ContactLocalizationGrid,
                    minimum_total_episodes: 3,
                    minimum_heldout_episodes: 1,
                },
                RequiredPhantomFamily {
                    family: PhantomTestFamily::TangentialShear,
                    minimum_total_episodes: 3,
                    minimum_heldout_episodes: 1,
                },
                RequiredPhantomFamily {
                    family: PhantomTestFamily::LoadingUnloadingHysteresis,
                    minimum_total_episodes: 3,
                    minimum_heldout_episodes: 1,
                },
            ],
            episodes: vec![
                episode(PhantomTestFamily::FlatIndentation, PhantomEpisodePartition::Calibration, "a"),
                episode(PhantomTestFamily::FlatIndentation, PhantomEpisodePartition::Validation, "b"),
                episode(PhantomTestFamily::FlatIndentation, PhantomEpisodePartition::HeldOutTest, "c"),
                episode(PhantomTestFamily::ContactLocalizationGrid, PhantomEpisodePartition::Calibration, "d"),
                episode(PhantomTestFamily::ContactLocalizationGrid, PhantomEpisodePartition::Validation, "e"),
                episode(PhantomTestFamily::ContactLocalizationGrid, PhantomEpisodePartition::HeldOutTest, "f"),
                episode(PhantomTestFamily::TangentialShear, PhantomEpisodePartition::Calibration, "g"),
                episode(PhantomTestFamily::TangentialShear, PhantomEpisodePartition::Validation, "h"),
                episode(PhantomTestFamily::TangentialShear, PhantomEpisodePartition::HeldOutTest, "i"),
                episode(PhantomTestFamily::LoadingUnloadingHysteresis, PhantomEpisodePartition::Calibration, "j"),
                episode(PhantomTestFamily::LoadingUnloadingHysteresis, PhantomEpisodePartition::Validation, "k"),
                episode(PhantomTestFamily::LoadingUnloadingHysteresis, PhantomEpisodePartition::HeldOutTest, "l"),
            ],
            require_heldout_indenter_geometry: false,
        }
    }

    #[test]
    fn fully_bound_phantom_campaign_manifest_is_admitted() {
        let report = audit_phantom_campaign(&manifest());
        assert_eq!(report.disposition, PhantomAuditDisposition::AdmissibleCampaignManifest);
        assert_eq!(report.episode_count, 12);
        assert!(report.family_coverage.iter().all(|family| family.complete));
    }

    #[test]
    fn missing_family_or_heldout_episodes_is_not_admitted() {
        let mut incomplete = manifest();
        incomplete.episodes.retain(|episode| {
            !(episode.family == PhantomTestFamily::TangentialShear
                && episode.partition == PhantomEpisodePartition::HeldOutTest)
        });
        let report = audit_phantom_campaign(&incomplete);
        assert_eq!(report.disposition, PhantomAuditDisposition::CoverageIncomplete);
        assert!(report.reasons.iter().any(|reason| reason.contains("TangentialShear")));
    }

    #[test]
    fn episode_requires_same_contact_reference_binding_and_time_alignment() {
        let mut bad = manifest();
        bad.episodes[0].sample_identity = "wrong-sample".to_owned();
        bad.episodes[1].wrench_reference_timestamp_ns += 100_000;
        let report = audit_phantom_campaign(&bad);
        assert_eq!(report.disposition, PhantomAuditDisposition::EpisodeEvidenceUnresolved);
        assert_eq!(report.invalid_episode_ids.len(), 2);
    }

    #[test]
    fn shared_or_unresolved_reference_ancestry_fails_closed() {
        let mut bad = manifest();
        bad.six_axis_reference.independence = ReferenceIndependenceDisposition::SharedCommonCause;
        bad.episodes[0].reference_evidence.reference_independence =
            ReferenceIndependenceDisposition::SharedCommonCause;
        let report = audit_phantom_campaign(&bad);
        assert_eq!(report.disposition, PhantomAuditDisposition::InvalidManifest);
        assert!(report.reasons.iter().any(|reason| reason.contains("reference independence")));
    }

    #[test]
    fn session_and_heldout_probe_geometry_leakage_are_detected() {
        let mut bad = manifest();
        bad.require_heldout_indenter_geometry = true;
        bad.episodes[0].session_id = "shared-session".to_owned();
        bad.episodes[2].session_id = "shared-session".to_owned();
        let report = audit_phantom_campaign(&bad);
        assert_eq!(report.disposition, PhantomAuditDisposition::PartitionLeakage);
        assert!(report.leakage_findings.iter().any(|finding| finding.identity_dimension == "session"));
        assert!(report.leakage_findings.iter().any(|finding| finding.identity_dimension == "indenter_tip_profile"));
    }

    #[test]
    fn covariance_geometry_context_and_duplicate_identity_fail_closed() {
        let mut duplicate = manifest();
        duplicate.episodes[1].sample_identity = duplicate.episodes[0].sample_identity.clone();
        let report = audit_phantom_campaign(&duplicate);
        assert_eq!(report.disposition, PhantomAuditDisposition::EpisodeEvidenceUnresolved);

        let mut invalid_covariance = manifest();
        invalid_covariance.episodes[0].observation.covariance.matrix_row_major[0] = -1.0;
        let report = audit_phantom_campaign(&invalid_covariance);
        assert_eq!(report.disposition, PhantomAuditDisposition::EpisodeEvidenceUnresolved);

        let mut wrong_geometry = manifest();
        wrong_geometry.episodes[0].geometry_digest = "sha256:stale-geometry".to_owned();
        let report = audit_phantom_campaign(&wrong_geometry);
        assert_eq!(report.disposition, PhantomAuditDisposition::EpisodeEvidenceUnresolved);
    }
}
