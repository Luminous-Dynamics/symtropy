//! Engine-neutral evidence manifests for reproducible locomotion runs.
//!
//! This crate validates provenance and paired-comparison compatibility. It
//! does not hash files itself; callers supply SHA-256 digests of immutable
//! source assets/configurations/trajectories and may use canonical_bytes() as
//! an unambiguous, deterministic manifest encoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceTier {
    Synthetic,
    Simulated,
    Hardware,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareProfile {
    pub platform: String,
    pub cpu_model: String,
    pub accelerator_model: Option<String>,
    pub os_build: String,
    pub driver_version: Option<String>,
    pub calibration_record_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunManifest {
    pub schema_version: u16,
    pub evidence_tier: EvidenceTier,
    pub run_id: String,
    pub engine_name: String,
    pub engine_version: String,
    pub engine_backend: String,
    /// Git object ID for the engine/source adapter (40- or 64-hex).
    pub engine_commit: String,
    /// Engine-native configuration may differ across engines and is retained
    /// for audit; paired comparison does not require it to be byte-identical.
    pub engine_native_config_sha256: String,
    pub controller_name: String,
    pub controller_sha256: String,
    pub morphology_name: String,
    pub morphology_sha256: String,
    /// Engine-neutral physical contract: mass/inertia/gravity/contact material
    /// semantics and other declared physical quantities.
    pub physical_contract_sha256: String,
    pub sensor_config_sha256: String,
    pub actuator_config_sha256: String,
    pub terrain_name: String,
    pub terrain_sha256: String,
    pub initial_state_sha256: String,
    /// Intended intervention/action assignment, common across paired runs.
    pub action_assignment_sha256: String,
    pub has_intervention: bool,
    /// Run-specific receipt proving what the simulator/robot actually applied.
    pub action_delivery_receipt_sha256: Option<String>,
    pub raw_trajectory_sha256: String,
    pub metric_version: String,
    pub terrain_seed: u64,
    pub trial_seed: u64,
    pub timestep_seconds: f64,
    pub control_frequency_hz: f64,
    pub sensor_frequency_hz: f64,
    /// Recorded for audit. Engine-specific solver choices need not be identical
    /// across backends, but must be disclosed in a fair comparison.
    pub integrator_name: String,
    pub solver_name: String,
    pub solver_tolerance: f64,
    pub solver_iterations: u32,
    /// Effective end-to-end delays declared by the sensor/actuator contract.
    pub sensor_latency_seconds: f64,
    pub actuator_latency_seconds: f64,
    pub duration_seconds: f64,
    pub recorded_samples: u64,
    /// May differ for throughput benchmarks; recorded but not paired-gated.
    pub hardware_profile: Option<HardwareProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairedRunMismatch {
    EvidenceTier,
    Controller,
    Morphology,
    PhysicalContract,
    SensorConfiguration,
    ActuatorConfiguration,
    Terrain,
    InitialState,
    ActionAssignment,
    InterventionPresence,
    MetricVersion,
    Seeds,
    Timebase,
    Duration,
    SampleCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    UnsupportedSchemaVersion,
    MissingRequiredField,
    InvalidSha256,
    InvalidEngineCommit,
    InvalidTimebase,
    InvalidNumericalConfiguration,
    InvalidLatency,
    InvalidDuration,
    InvalidSampleCount,
    MissingInterventionReceipt,
    UnexpectedInterventionReceipt,
    MissingHardwareProfile,
    MissingHardwareCalibration,
    PairedRunMismatch(PairedRunMismatch),
}

impl RunManifest {
    pub fn validate(&self) -> Result<(), EvidenceError> {
        if self.schema_version != 1 {
            return Err(EvidenceError::UnsupportedSchemaVersion);
        }

        for value in [
            self.run_id.as_str(),
            self.engine_name.as_str(),
            self.engine_version.as_str(),
            self.engine_backend.as_str(),
            self.controller_name.as_str(),
            self.morphology_name.as_str(),
            self.terrain_name.as_str(),
            self.metric_version.as_str(),
            self.integrator_name.as_str(),
            self.solver_name.as_str(),
        ] {
            require_text(value)?;
        }

        if !valid_commit_id(&self.engine_commit) {
            return Err(EvidenceError::InvalidEngineCommit);
        }

        for digest in [
            &self.engine_native_config_sha256,
            &self.controller_sha256,
            &self.morphology_sha256,
            &self.physical_contract_sha256,
            &self.sensor_config_sha256,
            &self.actuator_config_sha256,
            &self.terrain_sha256,
            &self.initial_state_sha256,
            &self.action_assignment_sha256,
            &self.raw_trajectory_sha256,
        ] {
            require_sha256(digest)?;
        }

        match (&self.has_intervention, &self.action_delivery_receipt_sha256) {
            (true, Some(digest)) => require_sha256(digest)?,
            (true, None) => return Err(EvidenceError::MissingInterventionReceipt),
            (false, Some(_)) => return Err(EvidenceError::UnexpectedInterventionReceipt),
            (false, None) => {}
        }

        if !self.timestep_seconds.is_finite()
            || self.timestep_seconds <= 0.0
            || !self.control_frequency_hz.is_finite()
            || self.control_frequency_hz <= 0.0
            || !self.sensor_frequency_hz.is_finite()
            || self.sensor_frequency_hz <= 0.0
        {
            return Err(EvidenceError::InvalidTimebase);
        }
        if !self.solver_tolerance.is_finite()
            || self.solver_tolerance <= 0.0
            || self.solver_iterations == 0
        {
            return Err(EvidenceError::InvalidNumericalConfiguration);
        }
        if !self.sensor_latency_seconds.is_finite()
            || self.sensor_latency_seconds < 0.0
            || !self.actuator_latency_seconds.is_finite()
            || self.actuator_latency_seconds < 0.0
        {
            return Err(EvidenceError::InvalidLatency);
        }
        if !self.duration_seconds.is_finite() || self.duration_seconds <= 0.0 {
            return Err(EvidenceError::InvalidDuration);
        }
        if self.recorded_samples < 2 {
            return Err(EvidenceError::InvalidSampleCount);
        }

        if self.evidence_tier == EvidenceTier::Hardware {
            let profile = self
                .hardware_profile
                .as_ref()
                .ok_or(EvidenceError::MissingHardwareProfile)?;
            profile.validate()?;
            if profile.calibration_record_sha256.is_none() {
                return Err(EvidenceError::MissingHardwareCalibration);
            }
        } else if let Some(profile) = &self.hardware_profile {
            profile.validate()?;
        }
        Ok(())
    }

    /// Canonical length-prefixed encoding. It is deterministic but not itself
    /// a cryptographic hash; hash the returned bytes with the declared
    /// algorithm in the surrounding evidence pipeline.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EvidenceError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        bytes.push(match self.evidence_tier {
            EvidenceTier::Synthetic => 0,
            EvidenceTier::Simulated => 1,
            EvidenceTier::Hardware => 2,
        });

        for value in [
            self.run_id.as_str(),
            self.engine_name.as_str(),
            self.engine_version.as_str(),
            self.engine_backend.as_str(),
            self.engine_commit.as_str(),
            self.engine_native_config_sha256.as_str(),
            self.controller_name.as_str(),
            self.controller_sha256.as_str(),
            self.morphology_name.as_str(),
            self.morphology_sha256.as_str(),
            self.physical_contract_sha256.as_str(),
            self.sensor_config_sha256.as_str(),
            self.actuator_config_sha256.as_str(),
            self.terrain_name.as_str(),
            self.terrain_sha256.as_str(),
            self.initial_state_sha256.as_str(),
            self.action_assignment_sha256.as_str(),
        ] {
            push_string(&mut bytes, value);
        }

        bytes.push(u8::from(self.has_intervention));
        push_optional_string(&mut bytes, self.action_delivery_receipt_sha256.as_deref());
        push_string(&mut bytes, &self.raw_trajectory_sha256);
        push_string(&mut bytes, &self.metric_version);
        bytes.extend_from_slice(&self.terrain_seed.to_le_bytes());
        bytes.extend_from_slice(&self.trial_seed.to_le_bytes());
        bytes.extend_from_slice(&self.timestep_seconds.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.control_frequency_hz.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.sensor_frequency_hz.to_bits().to_le_bytes());
        push_string(&mut bytes, &self.integrator_name);
        push_string(&mut bytes, &self.solver_name);
        bytes.extend_from_slice(&self.solver_tolerance.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.solver_iterations.to_le_bytes());
        bytes.extend_from_slice(&self.sensor_latency_seconds.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.actuator_latency_seconds.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.duration_seconds.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.recorded_samples.to_le_bytes());

        match &self.hardware_profile {
            Some(profile) => {
                bytes.push(1);
                push_string(&mut bytes, &profile.platform);
                push_string(&mut bytes, &profile.cpu_model);
                push_optional_string(&mut bytes, profile.accelerator_model.as_deref());
                push_string(&mut bytes, &profile.os_build);
                push_optional_string(&mut bytes, profile.driver_version.as_deref());
                push_optional_string(&mut bytes, profile.calibration_record_sha256.as_deref());
            }
            None => bytes.push(0),
        }
        Ok(bytes)
    }
}

impl HardwareProfile {
    fn validate(&self) -> Result<(), EvidenceError> {
        require_text(&self.platform)?;
        require_text(&self.cpu_model)?;
        require_text(&self.os_build)?;
        if let Some(accelerator) = &self.accelerator_model {
            require_text(accelerator)?;
        }
        if let Some(driver) = &self.driver_version {
            require_text(driver)?;
        }
        if let Some(digest) = &self.calibration_record_sha256 {
            require_sha256(digest)?;
        }
        Ok(())
    }
}

/// Require the pair to differ only in the engine/backend/run-specific outputs
/// and hardware context. This is deliberately strict: a different seed,
/// morphology, physics contract, sensor/actuator model, action assignment,
/// metric version or timebase is not a paired comparison.
pub fn validate_paired_comparison(
    left: &RunManifest,
    right: &RunManifest,
) -> Result<(), EvidenceError> {
    left.validate()?;
    right.validate()?;

    if left.evidence_tier != right.evidence_tier {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::EvidenceTier,
        ));
    }
    if left.controller_name != right.controller_name
        || left.controller_sha256 != right.controller_sha256
    {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::Controller,
        ));
    }
    if left.morphology_name != right.morphology_name
        || left.morphology_sha256 != right.morphology_sha256
    {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::Morphology,
        ));
    }
    if left.physical_contract_sha256 != right.physical_contract_sha256 {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::PhysicalContract,
        ));
    }
    if left.sensor_config_sha256 != right.sensor_config_sha256 {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::SensorConfiguration,
        ));
    }
    if left.actuator_config_sha256 != right.actuator_config_sha256 {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::ActuatorConfiguration,
        ));
    }
    if left.terrain_name != right.terrain_name || left.terrain_sha256 != right.terrain_sha256 {
        return Err(EvidenceError::PairedRunMismatch(PairedRunMismatch::Terrain));
    }
    if left.initial_state_sha256 != right.initial_state_sha256 {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::InitialState,
        ));
    }
    if left.action_assignment_sha256 != right.action_assignment_sha256 {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::ActionAssignment,
        ));
    }
    if left.has_intervention != right.has_intervention {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::InterventionPresence,
        ));
    }
    if left.metric_version != right.metric_version {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::MetricVersion,
        ));
    }
    if left.terrain_seed != right.terrain_seed || left.trial_seed != right.trial_seed {
        return Err(EvidenceError::PairedRunMismatch(PairedRunMismatch::Seeds));
    }
    if left.timestep_seconds != right.timestep_seconds
        || left.control_frequency_hz != right.control_frequency_hz
        || left.sensor_frequency_hz != right.sensor_frequency_hz
        || left.sensor_latency_seconds != right.sensor_latency_seconds
        || left.actuator_latency_seconds != right.actuator_latency_seconds
    {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::Timebase,
        ));
    }
    if left.duration_seconds != right.duration_seconds {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::Duration,
        ));
    }
    if left.recorded_samples != right.recorded_samples {
        return Err(EvidenceError::PairedRunMismatch(
            PairedRunMismatch::SampleCount,
        ));
    }
    Ok(())
}

fn require_text(value: &str) -> Result<(), EvidenceError> {
    if value.trim().is_empty() {
        Err(EvidenceError::MissingRequiredField)
    } else {
        Ok(())
    }
}

fn require_sha256(value: &str) -> Result<(), EvidenceError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
    {
        Ok(())
    } else {
        Err(EvidenceError::InvalidSha256)
    }
}

fn valid_commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|character| character.is_ascii_digit() || (b'a'..=b'f').contains(&character))
}

fn push_string(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&(value.len() as u64).to_le_bytes());
    output.extend_from_slice(value.as_bytes());
}

fn push_optional_string(output: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            output.push(1);
            push_string(output, value);
        }
        None => output.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn manifest(run_id: &str, engine: &str) -> RunManifest {
        RunManifest {
            schema_version: 1,
            evidence_tier: EvidenceTier::Simulated,
            run_id: run_id.to_owned(),
            engine_name: engine.to_owned(),
            engine_version: if engine == "flygym" {
                "2.1.1".to_owned()
            } else {
                "3.17.0".to_owned()
            },
            engine_backend: if engine == "flygym" {
                "warp-mjwarp".to_owned()
            } else {
                "mjbatch".to_owned()
            },
            engine_commit: "a".repeat(40),
            engine_native_config_sha256: digest(if engine == "flygym" { '1' } else { '2' }),
            controller_name: "phase-controller".to_owned(),
            controller_sha256: digest('3'),
            morphology_name: "hexapod-v1".to_owned(),
            morphology_sha256: digest('4'),
            physical_contract_sha256: digest('5'),
            sensor_config_sha256: digest('6'),
            actuator_config_sha256: digest('7'),
            terrain_name: "flat-rubber-v1".to_owned(),
            terrain_sha256: digest('8'),
            initial_state_sha256: digest('9'),
            action_assignment_sha256: digest('a'),
            has_intervention: true,
            action_delivery_receipt_sha256: Some(digest(if engine == "flygym" {
                'b'
            } else {
                'c'
            })),
            raw_trajectory_sha256: digest(if engine == "flygym" { 'd' } else { 'e' }),
            metric_version: "locomotion-metrics-v1".to_owned(),
            terrain_seed: 42,
            trial_seed: 9001,
            timestep_seconds: 0.0001,
            control_frequency_hz: 1000.0,
            sensor_frequency_hz: 500.0,
            integrator_name: if engine == "flygym" {
                "semi-implicit-euler".to_owned()
            } else {
                "implicit-fast".to_owned()
            },
            solver_name: if engine == "flygym" {
                "mujoco-constraint-solver".to_owned()
            } else {
                "newton-solver".to_owned()
            },
            solver_tolerance: 1.0e-8,
            solver_iterations: 100,
            sensor_latency_seconds: 0.002,
            actuator_latency_seconds: 0.001,
            duration_seconds: 2.0,
            recorded_samples: 20_001,
            hardware_profile: Some(HardwareProfile {
                platform: "linux-x86_64".to_owned(),
                cpu_model: if engine == "flygym" {
                    "CPU-A".to_owned()
                } else {
                    "CPU-B".to_owned()
                },
                accelerator_model: Some("GPU-X".to_owned()),
                os_build: "nixos-test".to_owned(),
                driver_version: Some("driver-1".to_owned()),
                calibration_record_sha256: None,
            }),
        }
    }

    #[test]
    fn manifest_validates_and_canonical_encoding_is_deterministic() {
        let run = manifest("run-a", "flygym");
        assert_eq!(run.validate(), Ok(()));
        let first = run.canonical_bytes().unwrap();
        let second = run.canonical_bytes().unwrap();
        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn paired_comparison_allows_engine_and_run_specific_receipt_differences() {
        let flygym = manifest("run-flygym", "flygym");
        let mujoco = manifest("run-mujoco", "mujoco");
        assert_eq!(validate_paired_comparison(&flygym, &mujoco), Ok(()));
        assert_ne!(
            flygym.canonical_bytes().unwrap(),
            mujoco.canonical_bytes().unwrap()
        );
    }

    #[test]
    fn paired_comparison_rejects_different_physical_contracts() {
        let flygym = manifest("run-flygym", "flygym");
        let mut mujoco = manifest("run-mujoco", "mujoco");
        mujoco.physical_contract_sha256 = digest('f');
        assert_eq!(
            validate_paired_comparison(&flygym, &mujoco),
            Err(EvidenceError::PairedRunMismatch(
                PairedRunMismatch::PhysicalContract
            ))
        );
    }

    #[test]
    fn paired_comparison_rejects_different_sensor_configurations() {
        let flygym = manifest("run-flygym", "flygym");
        let mut mujoco = manifest("run-mujoco", "mujoco");
        mujoco.sensor_config_sha256 = digest('f');
        assert_eq!(
            validate_paired_comparison(&flygym, &mujoco),
            Err(EvidenceError::PairedRunMismatch(
                PairedRunMismatch::SensorConfiguration
            ))
        );
    }

    #[test]
    fn paired_comparison_rejects_timebase_and_seed_mismatch() {
        let flygym = manifest("run-flygym", "flygym");
        let mut mujoco = manifest("run-mujoco", "mujoco");
        mujoco.sensor_frequency_hz = 250.0;
        assert_eq!(
            validate_paired_comparison(&flygym, &mujoco),
            Err(EvidenceError::PairedRunMismatch(
                PairedRunMismatch::Timebase
            ))
        );

        mujoco = manifest("run-mujoco", "mujoco");
        mujoco.trial_seed += 1;
        assert_eq!(
            validate_paired_comparison(&flygym, &mujoco),
            Err(EvidenceError::PairedRunMismatch(PairedRunMismatch::Seeds))
        );
    }

    #[test]
    fn interventions_require_a_delivery_receipt() {
        let mut run = manifest("run-a", "flygym");
        run.action_delivery_receipt_sha256 = None;
        assert_eq!(
            run.validate(),
            Err(EvidenceError::MissingInterventionReceipt)
        );

        run.has_intervention = false;
        run.action_delivery_receipt_sha256 = Some(digest('f'));
        assert_eq!(
            run.validate(),
            Err(EvidenceError::UnexpectedInterventionReceipt)
        );
        run.action_delivery_receipt_sha256 = None;
        assert_eq!(run.validate(), Ok(()));
    }

    #[test]
    fn hardware_tier_requires_hardware_profile() {
        let mut run = manifest("run-a", "mujoco");
        run.evidence_tier = EvidenceTier::Hardware;
        run.hardware_profile = None;
        assert_eq!(run.validate(), Err(EvidenceError::MissingHardwareProfile));
    }

    #[test]
    fn hardware_tier_requires_a_calibration_record_digest() {
        let mut run = manifest("run-hardware", "mujoco");
        run.evidence_tier = EvidenceTier::Hardware;
        let mut profile = run.hardware_profile.take().unwrap();
        profile.calibration_record_sha256 = None;
        run.hardware_profile = Some(profile);
        assert_eq!(
            run.validate(),
            Err(EvidenceError::MissingHardwareCalibration)
        );

        run.hardware_profile
            .as_mut()
            .unwrap()
            .calibration_record_sha256 = Some(digest('f'));
        assert_eq!(run.validate(), Ok(()));
    }

    #[test]
    fn rejects_placeholder_or_malformed_digest_and_git_commit() {
        let mut run = manifest("run-a", "flygym");
        run.controller_sha256 = "unknown".to_owned();
        assert_eq!(run.validate(), Err(EvidenceError::InvalidSha256));

        run = manifest("run-a", "flygym");
        run.engine_commit = "not-a-commit".to_owned();
        assert_eq!(run.validate(), Err(EvidenceError::InvalidEngineCommit));
    }

    #[test]
    fn rejects_nonpositive_timebase_and_sample_count() {
        let mut run = manifest("run-a", "flygym");
        run.timestep_seconds = 0.0;
        assert_eq!(run.validate(), Err(EvidenceError::InvalidTimebase));

        run = manifest("run-a", "flygym");
        run.recorded_samples = 1;
        assert_eq!(run.validate(), Err(EvidenceError::InvalidSampleCount));
    }

    #[test]
    fn paired_comparison_rejects_sensor_and_actuator_latency_mismatches() {
        let flygym = manifest("run-flygym", "flygym");
        let mut mujoco = manifest("run-mujoco", "mujoco");
        mujoco.sensor_latency_seconds = 0.004;
        assert_eq!(
            validate_paired_comparison(&flygym, &mujoco),
            Err(EvidenceError::PairedRunMismatch(
                PairedRunMismatch::Timebase
            ))
        );

        mujoco = manifest("run-mujoco", "mujoco");
        mujoco.actuator_latency_seconds = 0.003;
        assert_eq!(
            validate_paired_comparison(&flygym, &mujoco),
            Err(EvidenceError::PairedRunMismatch(
                PairedRunMismatch::Timebase
            ))
        );
    }

    #[test]
    fn records_engine_specific_integrator_and_solver_without_false_equality_gate() {
        let flygym = manifest("run-flygym", "flygym");
        let mujoco = manifest("run-mujoco", "mujoco");
        assert_ne!(flygym.integrator_name, mujoco.integrator_name);
        assert_ne!(flygym.solver_name, mujoco.solver_name);
        assert_eq!(validate_paired_comparison(&flygym, &mujoco), Ok(()));
        assert_ne!(
            flygym.canonical_bytes().unwrap(),
            mujoco.canonical_bytes().unwrap()
        );
    }

    #[test]
    fn rejects_invalid_solver_settings_and_negative_latency() {
        let mut run = manifest("run-a", "flygym");
        run.solver_tolerance = 0.0;
        assert_eq!(
            run.validate(),
            Err(EvidenceError::InvalidNumericalConfiguration)
        );

        run = manifest("run-a", "flygym");
        run.solver_iterations = 0;
        assert_eq!(
            run.validate(),
            Err(EvidenceError::InvalidNumericalConfiguration)
        );

        run = manifest("run-a", "flygym");
        run.sensor_latency_seconds = -0.001;
        assert_eq!(run.validate(), Err(EvidenceError::InvalidLatency));
    }

    #[test]
    fn rejects_noncanonical_empty_required_fields_and_unknown_schema() {
        let mut run = manifest("run-a", "flygym");
        run.engine_name = "  ".to_owned();
        assert_eq!(run.validate(), Err(EvidenceError::MissingRequiredField));

        run = manifest("run-a", "flygym");
        run.schema_version = 2;
        assert_eq!(run.validate(), Err(EvidenceError::UnsupportedSchemaVersion));
    }
}
