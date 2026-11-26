#![forbid(unsafe_code)]

//! Fail-closed receipt validation for frictional-contact solver benchmarks.
//!
//! Two panels are deliberately distinct:
//! - FixedProblemSolver: solvers must receive identical canonical mathematical
//!   problems and identical solver-row inputs.
//! - FullEnginePipeline: frontends may differ, but eligible-pair, query, contact
//!   and solver-row denominators must be independently accounted.
//!
//! This crate validates declared digests and counts. It does not hash upstream
//! data, generate contacts, prove an oracle correct, or authenticate telemetry.

use std::error::Error;
use std::fmt;

const SHA256_BYTES: usize = 32;
const SHA256_HEX_CHARS: usize = SHA256_BYTES * 2;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Digest([u8; SHA256_BYTES]);

impl Digest {
    pub fn from_hex(value: &str) -> Result<Self, ReceiptError> {
        if value.len() != SHA256_HEX_CHARS {
            return Err(ReceiptError::InvalidDigest);
        }
        let mut output = [0_u8; SHA256_BYTES];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let high = decode_lower_hex(pair[0])?;
            let low = decode_lower_hex(pair[1])?;
            output[index] = (high << 4) | low;
        }
        Ok(Self(output))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; SHA256_BYTES] {
        &self.0
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(SHA256_HEX_CHARS);
        for byte in self.0 {
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        output
    }
}

fn decode_lower_hex(value: u8) -> Result<u8, ReceiptError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(ReceiptError::InvalidDigest),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BenchmarkPanel {
    FixedProblemSolver,
    FullEnginePipeline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SolverOutcome {
    Converged,
    IterationLimit,
    NumericalFailure,
    Unsupported,
    InitializationFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptError {
    InvalidDigest,
    InvalidIdentifier,
    InvalidProblemCounts,
    WrongBenchmarkPanel,
    FrontendPresentForFixedProblem,
    InvalidPairCounts,
    PairCountOverflow,
    InvalidQueryCounts,
    QueryCountOverflow,
    InvalidContactSetCounts,
    InvalidSolverRowCounts,
    SolverRowsExceedCapacity,
    CapacityOverflowFlagMismatch,
    SolverProblemDigestMismatch,
    SolverContactIdentityMismatch,
    InvalidTiming,
    InvalidResidual,
    MissingFrontendForFullPipeline,
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for ReceiptError {}

/// Scene identity is shared by full-engine runs even when engine contact
/// algorithms or constitutive laws differ.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SceneFingerprint {
    pub geometry: Digest,
    pub initial_state: Digest,
    pub motion_or_controls: Digest,
    pub material_catalog: Digest,
    pub collision_filters: Digest,
    pub time_discretization: Digest,
    pub environment: Digest,
}

/// Identity of the fixed mathematical contact problem presented to a solver.
/// Each digest is independently produced from a canonical serialized input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedProblemFingerprint {
    pub model_law: Digest,
    pub mass_properties: Digest,
    pub jacobian: Digest,
    pub contact_frames: Digest,
    pub contact_identities: Digest,
    pub friction_material: Digest,
    pub free_velocity_rhs: Digest,
    pub regularization: Digest,
    pub time_discretization: Digest,
    pub solver_rows: Digest,
    pub body_count: u64,
    pub contact_count: u64,
    pub solver_row_count: u64,
}

impl FixedProblemFingerprint {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if self.body_count == 0 {
            return Err(ReceiptError::InvalidProblemCounts);
        }
        Ok(())
    }

    /// Return only the fields that define the mathematical problem. Solver
    /// implementation, build, backend and stopping policy are intentionally
    /// not included: those are allowed to differ in a solver tournament.
    #[must_use]
    pub fn mismatched_fields(&self, other: &Self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.model_law != other.model_law {
            fields.push("model_law");
        }
        if self.mass_properties != other.mass_properties {
            fields.push("mass_properties");
        }
        if self.jacobian != other.jacobian {
            fields.push("jacobian");
        }
        if self.contact_frames != other.contact_frames {
            fields.push("contact_frames");
        }
        if self.contact_identities != other.contact_identities {
            fields.push("contact_identities");
        }
        if self.friction_material != other.friction_material {
            fields.push("friction_material");
        }
        if self.free_velocity_rhs != other.free_velocity_rhs {
            fields.push("free_velocity_rhs");
        }
        if self.regularization != other.regularization {
            fields.push("regularization");
        }
        if self.time_discretization != other.time_discretization {
            fields.push("time_discretization");
        }
        if self.solver_rows != other.solver_rows {
            fields.push("solver_rows");
        }
        if self.body_count != other.body_count {
            fields.push("body_count");
        }
        if self.contact_count != other.contact_count {
            fields.push("contact_count");
        }
        if self.solver_row_count != other.solver_row_count {
            fields.push("solver_row_count");
        }
        fields
    }
}

/// Solver input rows as actually admitted by the engine, not merely requested
/// by a frontend or predicted by a contact-count estimator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SolverInputAudit {
    pub expected_rows: u64,
    pub emitted_rows: u64,
    pub dropped_rows: u64,
    pub capacity_rows: Option<u64>,
    pub overflow_reported: bool,
    pub solver_rows_digest: Digest,
    pub contact_identities_digest: Digest,
}

impl SolverInputAudit {
    pub fn validate(
        &self,
        problem: &FixedProblemFingerprint,
    ) -> Result<(), ReceiptError> {
        let accounted = self
            .emitted_rows
            .checked_add(self.dropped_rows)
            .ok_or(ReceiptError::InvalidSolverRowCounts)?;
        if self.expected_rows != problem.solver_row_count
            || accounted != self.expected_rows
            || self.emitted_rows > self.expected_rows
        {
            return Err(ReceiptError::InvalidSolverRowCounts);
        }
        if self.solver_rows_digest != problem.solver_rows {
            return Err(ReceiptError::SolverProblemDigestMismatch);
        }
        if self.contact_identities_digest != problem.contact_identities {
            return Err(ReceiptError::SolverContactIdentityMismatch);
        }

        if let Some(capacity) = self.capacity_rows {
            if self.emitted_rows > capacity {
                return Err(ReceiptError::SolverRowsExceedCapacity);
            }
            let overflow_expected = self.expected_rows > capacity;
            if self.overflow_reported != overflow_expected {
                return Err(ReceiptError::CapacityOverflowFlagMismatch);
            }
        } else if self.overflow_reported {
            return Err(ReceiptError::CapacityOverflowFlagMismatch);
        }

        Ok(())
    }

    #[must_use]
    pub fn complete(&self) -> bool {
        self.dropped_rows == 0 && !self.overflow_reported
    }

    /// Compare the admitted mathematical rows, excluding capacity settings:
    /// two different capacities are comparable when both admitted the exact
    /// same complete row set without truncation.
    #[must_use]
    pub fn same_admitted_rows(&self, other: &Self) -> bool {
        self.expected_rows == other.expected_rows
            && self.emitted_rows == other.emitted_rows
            && self.dropped_rows == other.dropped_rows
            && self.solver_rows_digest == other.solver_rows_digest
            && self.contact_identities_digest == other.contact_identities_digest
    }
}

/// Independent broadphase pair-set evidence. The oracle population is the
/// eligible canonical collider-pair set after the declared collision filter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PairSetAudit {
    pub oracle_pair_set_digest: Digest,
    pub eligible_pairs: u64,
    pub oracle_overlap_pairs: u64,
    pub emitted_pairs: u64,
    pub true_positive_pairs: u64,
    pub false_positive_pairs: u64,
    pub false_negative_pairs: u64,
    pub duplicate_pairs: u64,
    pub unknown_endpoints: u64,
    pub oracle_coverage_complete: bool,
    pub receipt_valid: bool,
}

impl PairSetAudit {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if self.oracle_overlap_pairs > self.eligible_pairs
            || self.emitted_pairs > self.eligible_pairs
            || self.true_positive_pairs > self.oracle_overlap_pairs
            || self.true_positive_pairs > self.emitted_pairs
        {
            return Err(ReceiptError::InvalidPairCounts);
        }
        let oracle_count = self
            .true_positive_pairs
            .checked_add(self.false_negative_pairs)
            .ok_or(ReceiptError::PairCountOverflow)?;
        let emitted_count = self
            .true_positive_pairs
            .checked_add(self.false_positive_pairs)
            .ok_or(ReceiptError::PairCountOverflow)?;
        if oracle_count != self.oracle_overlap_pairs || emitted_count != self.emitted_pairs {
            return Err(ReceiptError::InvalidPairCounts);
        }
        Ok(())
    }

    #[must_use]
    pub fn qualifies(&self) -> bool {
        self.receipt_valid
            && self.oracle_coverage_complete
            && self.false_negative_pairs == 0
            && self.duplicate_pairs == 0
            && self.unknown_endpoints == 0
    }
}

/// Narrowphase query denominator. Every requested query must be completed,
/// unsupported, or failed; unsupported/failed cases cannot be relabelled as
/// negatives or disappear from the benchmark denominator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryCoverageAudit {
    pub requested_queries: u64,
    pub completed_queries: u64,
    pub unsupported_queries: u64,
    pub failed_queries: u64,
    pub query_receipt_valid: bool,
}

impl QueryCoverageAudit {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        let accounted = self
            .completed_queries
            .checked_add(self.unsupported_queries)
            .and_then(|value| value.checked_add(self.failed_queries))
            .ok_or(ReceiptError::QueryCountOverflow)?;
        if accounted != self.requested_queries {
            return Err(ReceiptError::InvalidQueryCounts);
        }
        Ok(())
    }

    #[must_use]
    pub fn complete_and_supported(&self) -> bool {
        self.query_receipt_valid
            && self.completed_queries == self.requested_queries
            && self.unsupported_queries == 0
            && self.failed_queries == 0
    }
}

/// A contact-set comparison is only available when an independent oracle
/// supplies its canonical set digest. Counts alone are never treated as
/// contact-set equality.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContactSetAudit {
    pub oracle_contact_count: Option<u64>,
    pub oracle_contact_digest: Option<Digest>,
    pub generated_contact_count: u64,
    pub generated_contact_digest: Digest,
    pub unsupported_contacts: u64,
    pub dropped_contacts: u64,
    pub receipt_valid: bool,
}

impl ContactSetAudit {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        match (self.oracle_contact_count, self.oracle_contact_digest) {
            (Some(_), Some(_)) | (None, None) => Ok(()),
            _ => Err(ReceiptError::InvalidContactSetCounts),
        }
    }

    #[must_use]
    pub fn oracle_available(&self) -> bool {
        self.oracle_contact_count.is_some() && self.oracle_contact_digest.is_some()
    }

    #[must_use]
    pub fn accounted(&self) -> bool {
        self.receipt_valid
            && self.unsupported_contacts == 0
            && self.dropped_contacts == 0
    }

    #[must_use]
    pub fn qualifies(&self) -> bool {
        self.accounted()
            && self.oracle_available()
            && self.oracle_contact_count == Some(self.generated_contact_count)
            && self.oracle_contact_digest == Some(self.generated_contact_digest)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrontendAudit {
    pub pair_set: Option<PairSetAudit>,
    pub narrowphase_queries: Option<QueryCoverageAudit>,
    pub contact_set: Option<ContactSetAudit>,
}

impl FrontendAudit {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if let Some(audit) = self.pair_set {
            audit.validate()?;
        }
        if let Some(audit) = self.narrowphase_queries {
            audit.validate()?;
        }
        if let Some(audit) = self.contact_set {
            audit.validate()?;
        }
        Ok(())
    }

    /// Accounting is usable for a scene/task panel when broadphase and query
    /// denominators are complete and every generated/dropped contact is
    /// explicitly accounted. This does not prove that the generated contact
    /// set is complete without an independent contact oracle.
    #[must_use]
    pub fn accounted(&self) -> bool {
        self.pair_set.is_some_and(|audit| audit.qualifies())
            && self
                .narrowphase_queries
                .is_some_and(|audit| audit.complete_and_supported())
            && self.contact_set.is_some_and(|audit| audit.accounted())
    }

    /// Full frontend coverage requires exact contact-set agreement with an
    /// independent contact oracle. Matching counts alone never suffice.
    #[must_use]
    pub fn fully_qualifies(&self) -> bool {
        self.accounted() && self.contact_set.is_some_and(|audit| audit.qualifies())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidualEvidence {
    pub primal_cone: f64,
    pub dual_cone: f64,
    pub complementarity: f64,
    pub normal_pull: f64,
    pub response_consistency: f64,
    pub energy_injection: f64,
}

impl ResidualEvidence {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        let values = [
            self.primal_cone,
            self.dual_cone,
            self.complementarity,
            self.normal_pull,
            self.response_consistency,
            self.energy_injection,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(ReceiptError::InvalidResidual);
        }
        Ok(())
    }

    #[must_use]
    pub fn max(self) -> f64 {
        self.primal_cone
            .max(self.dual_cone)
            .max(self.complementarity)
            .max(self.normal_pull)
            .max(self.response_consistency)
            .max(self.energy_injection)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimingEvidence {
    pub total_ns: Option<u64>,
    pub solver_ns: Option<u64>,
    pub timer_resolution_ns: Option<u64>,
}

impl TimingEvidence {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if let (Some(total), Some(solver)) = (self.total_ns, self.solver_ns) {
            if solver > total {
                return Err(ReceiptError::InvalidTiming);
            }
        }
        if self.timer_resolution_ns == Some(0) {
            return Err(ReceiptError::InvalidTiming);
        }
        Ok(())
    }

    #[must_use]
    pub fn is_performance_usable(&self) -> bool {
        let (Some(total), Some(solver), Some(resolution)) =
            (self.total_ns, self.solver_ns, self.timer_resolution_ns)
        else {
            return false;
        };
        total > 0 && solver > 0 && total >= resolution && solver >= resolution
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunReceipt {
    pub run_id: String,
    pub panel: BenchmarkPanel,
    pub scene: SceneFingerprint,
    pub problem: FixedProblemFingerprint,
    pub solver_implementation: Digest,
    pub solver_build: Digest,
    pub solver_configuration: Digest,
    pub environment: Digest,
    pub solver_input: SolverInputAudit,
    pub frontend: Option<FrontendAudit>,
    pub outcome: SolverOutcome,
    pub residuals: Option<ResidualEvidence>,
    pub timing: TimingEvidence,
}

impl RunReceipt {
    pub fn validate(&self) -> Result<(), ReceiptError> {
        if self.run_id.trim().is_empty() {
            return Err(ReceiptError::InvalidIdentifier);
        }
        self.problem.validate()?;
        self.solver_input.validate(&self.problem)?;
        self.timing.validate()?;
        if let Some(residuals) = self.residuals {
            residuals.validate()?;
        }

        match (self.panel, self.frontend) {
            (BenchmarkPanel::FixedProblemSolver, None) => {}
            (BenchmarkPanel::FixedProblemSolver, Some(_)) => {
                return Err(ReceiptError::FrontendPresentForFixedProblem);
            }
            (BenchmarkPanel::FullEnginePipeline, Some(frontend)) => frontend.validate()?,
            (BenchmarkPanel::FullEnginePipeline, None) => {
                return Err(ReceiptError::MissingFrontendForFullPipeline);
            }
        }
        Ok(())
    }

    /// Input comparability is independent of whether a solver converged.
    /// Outcome, residual and timing are reported as separate result dimensions.
    #[must_use]
    pub fn solver_input_qualifies(&self) -> bool {
        self.solver_input.complete()
    }

    #[must_use]
    pub fn frontend_accounted(&self) -> bool {
        self.solver_input.complete()
            && self.frontend.is_some_and(|frontend| frontend.accounted())
    }

    #[must_use]
    pub fn frontend_qualifies(&self) -> bool {
        self.solver_input.complete()
            && self.frontend.is_some_and(|frontend| frontend.fully_qualifies())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedProblemComparison {
    /// Same complete mathematical input and same admitted solver rows.
    pub eligible: bool,
    /// Comparable wall-clock evidence additionally requires matching run
    /// environment digests and timings above the declared timer resolution.
    pub performance_comparable: bool,
    pub environment_matched: bool,
    pub mismatched_fields: Vec<&'static str>,
    pub left_run_id: String,
    pub right_run_id: String,
    pub left_outcome: SolverOutcome,
    pub right_outcome: SolverOutcome,
}

pub fn compare_fixed_problem(
    left: &RunReceipt,
    right: &RunReceipt,
) -> Result<FixedProblemComparison, ReceiptError> {
    left.validate()?;
    right.validate()?;
    if left.panel != BenchmarkPanel::FixedProblemSolver
        || right.panel != BenchmarkPanel::FixedProblemSolver
    {
        return Err(ReceiptError::WrongBenchmarkPanel);
    }

    let mismatched_fields = left.problem.mismatched_fields(&right.problem);
    let solver_input_matches = left.solver_input.same_admitted_rows(&right.solver_input);
    let eligible = mismatched_fields.is_empty()
        && solver_input_matches
        && left.solver_input_qualifies()
        && right.solver_input_qualifies();
    let environment_matched = left.environment == right.environment;
    let performance_comparable = eligible
        && environment_matched
        && left.timing.is_performance_usable()
        && right.timing.is_performance_usable();

    let mut fields = mismatched_fields;
    if !solver_input_matches {
        fields.push("solver_input_audit");
    }

    Ok(FixedProblemComparison {
        eligible,
        performance_comparable,
        environment_matched,
        mismatched_fields: fields,
        left_run_id: left.run_id.clone(),
        right_run_id: right.run_id.clone(),
        left_outcome: left.outcome,
        right_outcome: right.outcome,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FullPipelineComparison {
    pub eligible_for_scene_comparison: bool,
    pub constitutive_law_matched: bool,
    pub environment_matched: bool,
    pub pair_oracle_matched: bool,
    pub contact_sets_matched: bool,
    pub performance_comparable: bool,
    pub left_frontend_accounted: bool,
    pub right_frontend_accounted: bool,
    pub left_frontend_qualified: bool,
    pub right_frontend_qualified: bool,
    pub scene_mismatches: Vec<&'static str>,
    pub left_run_id: String,
    pub right_run_id: String,
}

pub fn compare_full_pipeline(
    left: &RunReceipt,
    right: &RunReceipt,
) -> Result<FullPipelineComparison, ReceiptError> {
    left.validate()?;
    right.validate()?;
    if left.panel != BenchmarkPanel::FullEnginePipeline
        || right.panel != BenchmarkPanel::FullEnginePipeline
    {
        return Err(ReceiptError::WrongBenchmarkPanel);
    }

    let mut mismatches = Vec::new();
    if left.scene.geometry != right.scene.geometry {
        mismatches.push("geometry");
    }
    if left.scene.initial_state != right.scene.initial_state {
        mismatches.push("initial_state");
    }
    if left.scene.motion_or_controls != right.scene.motion_or_controls {
        mismatches.push("motion_or_controls");
    }
    if left.scene.material_catalog != right.scene.material_catalog {
        mismatches.push("material_catalog");
    }
    if left.scene.collision_filters != right.scene.collision_filters {
        mismatches.push("collision_filters");
    }
    if left.scene.time_discretization != right.scene.time_discretization {
        mismatches.push("time_discretization");
    }
    if left.scene.environment != right.scene.environment {
        mismatches.push("environment");
    }

    let left_frontend_accounted = left.frontend_accounted();
    let right_frontend_accounted = right.frontend_accounted();
    let left_frontend_qualified = left.frontend_qualifies();
    let right_frontend_qualified = right.frontend_qualifies();
    let constitutive_law_matched = left.problem.model_law == right.problem.model_law;
    let environment_matched = left.environment == right.environment;
    let eligible_for_scene_comparison =
        mismatches.is_empty() && left_frontend_accounted && right_frontend_accounted;

    let left_frontend = left.frontend.expect("validated full pipeline frontend");
    let right_frontend = right.frontend.expect("validated full pipeline frontend");
    let pair_oracle_matched = match (left_frontend.pair_set, right_frontend.pair_set) {
        (Some(left_audit), Some(right_audit)) => {
            left_audit.qualifies()
                && right_audit.qualifies()
                && left_audit.oracle_pair_set_digest == right_audit.oracle_pair_set_digest
                && left_audit.eligible_pairs == right_audit.eligible_pairs
                && left_audit.oracle_overlap_pairs == right_audit.oracle_overlap_pairs
        }
        _ => false,
    };
    let contact_sets_matched = match (left_frontend.contact_set, right_frontend.contact_set) {
        (Some(left_audit), Some(right_audit)) => {
            left_audit.qualifies()
                && right_audit.qualifies()
                && left_audit.oracle_contact_count == right_audit.oracle_contact_count
                && left_audit.oracle_contact_digest == right_audit.oracle_contact_digest
                && left_audit.generated_contact_count == right_audit.generated_contact_count
                && left_audit.generated_contact_digest == right_audit.generated_contact_digest
        }
        _ => false,
    };
    let performance_comparable = eligible_for_scene_comparison
        && constitutive_law_matched
        && environment_matched
        && pair_oracle_matched
        && contact_sets_matched
        && left.timing.is_performance_usable()
        && right.timing.is_performance_usable();

    Ok(FullPipelineComparison {
        eligible_for_scene_comparison,
        constitutive_law_matched,
        environment_matched,
        pair_oracle_matched,
        contact_sets_matched,
        performance_comparable,
        left_frontend_accounted,
        right_frontend_accounted,
        left_frontend_qualified,
        right_frontend_qualified,
        scene_mismatches: mismatches,
        left_run_id: left.run_id.clone(),
        right_run_id: right.run_id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> Digest {
        let text = std::iter::repeat_n(byte, SHA256_HEX_CHARS).collect::<String>();
        Digest::from_hex(&text).unwrap()
    }

    fn problem() -> FixedProblemFingerprint {
        FixedProblemFingerprint {
            model_law: digest('a'),
            mass_properties: digest('b'),
            jacobian: digest('c'),
            contact_frames: digest('d'),
            contact_identities: digest('e'),
            friction_material: digest('f'),
            free_velocity_rhs: digest('1'),
            regularization: digest('2'),
            time_discretization: digest('3'),
            solver_rows: digest('4'),
            body_count: 3,
            contact_count: 2,
            solver_row_count: 6,
        }
    }

    fn input_audit(problem: FixedProblemFingerprint) -> SolverInputAudit {
        SolverInputAudit {
            expected_rows: problem.solver_row_count,
            emitted_rows: problem.solver_row_count,
            dropped_rows: 0,
            capacity_rows: Some(16),
            overflow_reported: false,
            solver_rows_digest: problem.solver_rows,
            contact_identities_digest: problem.contact_identities,
        }
    }

    fn pair_audit() -> PairSetAudit {
        PairSetAudit {
            oracle_pair_set_digest: digest('5'),
            eligible_pairs: 10,
            oracle_overlap_pairs: 3,
            emitted_pairs: 4,
            true_positive_pairs: 3,
            false_positive_pairs: 1,
            false_negative_pairs: 0,
            duplicate_pairs: 0,
            unknown_endpoints: 0,
            oracle_coverage_complete: true,
            receipt_valid: true,
        }
    }

    fn query_audit() -> QueryCoverageAudit {
        QueryCoverageAudit {
            requested_queries: 8,
            completed_queries: 8,
            unsupported_queries: 0,
            failed_queries: 0,
            query_receipt_valid: true,
        }
    }

    fn contact_audit() -> ContactSetAudit {
        ContactSetAudit {
            oracle_contact_count: Some(2),
            oracle_contact_digest: Some(digest('5')),
            generated_contact_count: 2,
            generated_contact_digest: digest('5'),
            unsupported_contacts: 0,
            dropped_contacts: 0,
            receipt_valid: true,
        }
    }

    fn frontend() -> FrontendAudit {
        FrontendAudit {
            pair_set: Some(pair_audit()),
            narrowphase_queries: Some(query_audit()),
            contact_set: Some(contact_audit()),
        }
    }

    fn scene() -> SceneFingerprint {
        SceneFingerprint {
            geometry: digest('6'),
            initial_state: digest('7'),
            motion_or_controls: digest('8'),
            material_catalog: digest('9'),
            collision_filters: digest('a'),
            time_discretization: digest('b'),
            environment: digest('c'),
        }
    }

    fn receipt(panel: BenchmarkPanel, run_id: &str) -> RunReceipt {
        let problem = problem();
        RunReceipt {
            run_id: run_id.to_owned(),
            panel,
            scene: scene(),
            problem,
            solver_implementation: digest('d'),
            solver_build: digest('e'),
            solver_configuration: digest('f'),
            environment: digest('a'),
            solver_input: input_audit(problem),
            frontend: (panel == BenchmarkPanel::FullEnginePipeline).then(frontend),
            outcome: SolverOutcome::Converged,
            residuals: Some(ResidualEvidence {
                primal_cone: 0.0,
                dual_cone: 0.0,
                complementarity: 1.0e-10,
                normal_pull: 0.0,
                response_consistency: 0.0,
                energy_injection: 0.0,
            }),
            timing: TimingEvidence {
                total_ns: Some(1200),
                solver_ns: Some(800),
                timer_resolution_ns: Some(1),
            },
        }
    }

    #[test]
    fn digest_accepts_canonical_lowercase_sha256_and_roundtrips() {
        let value = "0123456789abcdef".repeat(4);
        let digest = Digest::from_hex(&value).unwrap();
        assert_eq!(digest.to_hex(), value);
    }

    #[test]
    fn digest_rejects_uppercase_and_wrong_length() {
        assert_eq!(Digest::from_hex(&"A".repeat(64)), Err(ReceiptError::InvalidDigest));
        assert_eq!(Digest::from_hex("abc"), Err(ReceiptError::InvalidDigest));
    }

    #[test]
    fn fixed_problem_comparison_allows_solver_builds_to_differ() {
        let mut left = receipt(BenchmarkPanel::FixedProblemSolver, "left");
        let mut right = receipt(BenchmarkPanel::FixedProblemSolver, "right");
        right.solver_implementation = digest('7');
        right.solver_build = digest('8');
        right.solver_configuration = digest('9');
        right.outcome = SolverOutcome::IterationLimit;
        let comparison = compare_fixed_problem(&left, &right).unwrap();
        assert!(comparison.eligible);
        assert!(comparison.performance_comparable);
        assert!(comparison.environment_matched);
        assert!(comparison.mismatched_fields.is_empty());
        assert_eq!(comparison.left_outcome, SolverOutcome::Converged);
        assert_eq!(comparison.right_outcome, SolverOutcome::IterationLimit);
        left.solver_input.capacity_rows = Some(6);
        assert!(compare_fixed_problem(&left, &right).unwrap().eligible);

        left.environment = digest('7');
        let environment_mismatch = compare_fixed_problem(&left, &right).unwrap();
        assert!(environment_mismatch.eligible);
        assert!(!environment_mismatch.performance_comparable);
        assert!(!environment_mismatch.environment_matched);
    }

    #[test]
    fn fixed_problem_rejects_different_model_law_or_contact_frames() {
        let left = receipt(BenchmarkPanel::FixedProblemSolver, "left");
        let mut right = receipt(BenchmarkPanel::FixedProblemSolver, "right");
        right.problem.model_law = digest('7');
        right.problem.contact_frames = digest('8');
        let comparison = compare_fixed_problem(&left, &right).unwrap();
        assert!(!comparison.eligible);
        assert_eq!(
            comparison.mismatched_fields,
            vec!["model_law", "contact_frames"]
        );
    }

    #[test]
    fn same_contact_count_does_not_hide_different_contact_identities() {
        let left = receipt(BenchmarkPanel::FixedProblemSolver, "left");
        let mut right = receipt(BenchmarkPanel::FixedProblemSolver, "right");
        right.problem.contact_identities = digest('8');
        let comparison = compare_fixed_problem(&left, &right).unwrap();
        assert!(!comparison.eligible);
        assert!(comparison.mismatched_fields.contains(&"contact_identities"));
    }

    #[test]
    fn solver_row_digest_mismatch_is_rejected_even_when_counts_match() {
        let left = receipt(BenchmarkPanel::FixedProblemSolver, "left");
        let mut right = receipt(BenchmarkPanel::FixedProblemSolver, "right");
        right.problem.solver_rows = digest('8');
        right.solver_input.solver_rows_digest = digest('8');
        let comparison = compare_fixed_problem(&left, &right).unwrap();
        assert!(!comparison.eligible);
        assert!(comparison.mismatched_fields.contains(&"solver_rows"));
    }

    #[test]
    fn dropped_solver_rows_disqualify_fixed_problem_comparison() {
        let mut left = receipt(BenchmarkPanel::FixedProblemSolver, "left");
        let right = receipt(BenchmarkPanel::FixedProblemSolver, "right");
        left.solver_input.emitted_rows = 5;
        left.solver_input.dropped_rows = 1;
        assert_eq!(
            compare_fixed_problem(&left, &right),
            Err(ReceiptError::InvalidSolverRowCounts)
        );
    }

    #[test]
    fn capacity_overflow_is_detected_and_report_must_match_counts() {
        let mut value = receipt(BenchmarkPanel::FixedProblemSolver, "left");
        value.solver_input.capacity_rows = Some(4);
        value.solver_input.overflow_reported = true;
        assert_eq!(value.validate(), Err(ReceiptError::SolverRowsExceedCapacity));
        value.solver_input.capacity_rows = Some(8);
        value.solver_input.overflow_reported = true;
        assert_eq!(value.validate(), Err(ReceiptError::CapacityOverflowFlagMismatch));
    }

    #[test]
    fn pair_recall_count_conservation_is_independently_checked() {
        let mut value = pair_audit();
        value.false_negative_pairs = 1;
        assert_eq!(value.validate(), Err(ReceiptError::InvalidPairCounts));
        value.false_negative_pairs = 0;
        assert!(value.qualifies());
    }

    #[test]
    fn query_denominator_cannot_drop_failed_or_unsupported_queries() {
        let mut value = query_audit();
        value.completed_queries = 7;
        assert_eq!(value.validate(), Err(ReceiptError::InvalidQueryCounts));
        value.unsupported_queries = 1;
        assert!(value.validate().is_ok());
        assert!(!value.complete_and_supported());
    }

    #[test]
    fn contact_count_alone_never_proves_contact_set_equality() {
        let mut value = contact_audit();
        value.generated_contact_digest = digest('7');
        assert_eq!(value.validate(), Ok(()));
        assert!(!value.qualifies());
    }

    #[test]
    fn missing_contact_oracle_remains_unavailable() {
        let mut value = contact_audit();
        value.oracle_contact_count = None;
        value.oracle_contact_digest = None;
        assert_eq!(value.validate(), Ok(()));
        assert!(!value.oracle_available());
        assert!(!value.qualifies());
    }

    #[test]
    fn full_pipeline_comparison_exposes_model_law_mismatch() {
        let left = receipt(BenchmarkPanel::FullEnginePipeline, "left");
        let mut right = receipt(BenchmarkPanel::FullEnginePipeline, "right");
        right.problem.model_law = digest('7');
        let comparison = compare_full_pipeline(&left, &right).unwrap();
        assert!(comparison.eligible_for_scene_comparison);
        assert!(comparison.left_frontend_accounted);
        assert!(comparison.left_frontend_qualified);
        assert!(comparison.pair_oracle_matched);
        assert!(comparison.contact_sets_matched);
        assert!(!comparison.constitutive_law_matched);
        assert!(!comparison.performance_comparable);
    }

    #[test]
    fn full_pipeline_comparison_rejects_capacity_dropped_contacts() {
        let left = receipt(BenchmarkPanel::FullEnginePipeline, "left");
        let mut right = receipt(BenchmarkPanel::FullEnginePipeline, "right");
        let mut audit = right.frontend.unwrap();
        let mut contacts = audit.contact_set.unwrap();
        contacts.dropped_contacts = 1;
        audit.contact_set = Some(contacts);
        right.frontend = Some(audit);
        let comparison = compare_full_pipeline(&left, &right).unwrap();
        assert!(!comparison.eligible_for_scene_comparison);
        assert!(comparison.left_frontend_accounted);
        assert!(comparison.left_frontend_qualified);
        assert!(!comparison.right_frontend_accounted);
        assert!(!comparison.right_frontend_qualified);
    }

    #[test]
    fn full_pipeline_rejects_missing_pair_oracle_coverage() {
        let left = receipt(BenchmarkPanel::FullEnginePipeline, "left");
        let mut right = receipt(BenchmarkPanel::FullEnginePipeline, "right");
        let mut audit = right.frontend.unwrap();
        let mut pairs = audit.pair_set.unwrap();
        pairs.oracle_coverage_complete = false;
        audit.pair_set = Some(pairs);
        right.frontend = Some(audit);
        let comparison = compare_full_pipeline(&left, &right).unwrap();
        assert!(!comparison.eligible_for_scene_comparison);
    }

    #[test]
    fn same_fixed_problem_with_unusable_timings_is_not_a_speed_comparison() {
        let mut left = receipt(BenchmarkPanel::FixedProblemSolver, "left");
        let right = receipt(BenchmarkPanel::FixedProblemSolver, "right");
        left.timing.timer_resolution_ns = None;
        let comparison = compare_fixed_problem(&left, &right).unwrap();
        assert!(comparison.eligible);
        assert!(!comparison.performance_comparable);
    }

    #[test]
    fn missing_contact_oracle_allows_accounted_scene_report_but_not_speed_claim() {
        let mut left = receipt(BenchmarkPanel::FullEnginePipeline, "left");
        let right = receipt(BenchmarkPanel::FullEnginePipeline, "right");
        let mut audit = left.frontend.unwrap();
        let mut contacts = audit.contact_set.unwrap();
        contacts.oracle_contact_count = None;
        contacts.oracle_contact_digest = None;
        audit.contact_set = Some(contacts);
        left.frontend = Some(audit);

        let comparison = compare_full_pipeline(&left, &right).unwrap();
        assert!(comparison.eligible_for_scene_comparison);
        assert!(comparison.left_frontend_accounted);
        assert!(!comparison.left_frontend_qualified);
        assert!(comparison.right_frontend_qualified);
        assert!(!comparison.performance_comparable);
    }

    #[test]
    fn contact_count_mismatch_is_a_qualification_failure_not_parse_failure() {
        let mut value = contact_audit();
        value.generated_contact_count = 1;
        assert!(value.validate().is_ok());
        assert!(!value.qualifies());
    }

    #[test]
    fn full_pipeline_speed_comparison_requires_matching_pair_and_contact_oracles() {
        let left = receipt(BenchmarkPanel::FullEnginePipeline, "left");
        let mut right = receipt(BenchmarkPanel::FullEnginePipeline, "right");
        let mut audit = right.frontend.unwrap();
        let mut pairs = audit.pair_set.unwrap();
        pairs.oracle_pair_set_digest = digest('9');
        audit.pair_set = Some(pairs);
        right.frontend = Some(audit);
        let comparison = compare_full_pipeline(&left, &right).unwrap();
        assert!(comparison.eligible_for_scene_comparison);
        assert!(!comparison.pair_oracle_matched);
        assert!(comparison.contact_sets_matched);
        assert!(!comparison.performance_comparable);
    }

    #[test]
    fn full_pipeline_solver_row_drop_blocks_scene_qualification() {
        let mut left = receipt(BenchmarkPanel::FullEnginePipeline, "left");
        let right = receipt(BenchmarkPanel::FullEnginePipeline, "right");
        left.solver_input.emitted_rows = 5;
        left.solver_input.dropped_rows = 1;
        assert_eq!(
            compare_full_pipeline(&left, &right),
            Err(ReceiptError::InvalidSolverRowCounts)
        );
    }

    #[test]
    fn missing_frontend_and_frontend_on_fixed_panel_are_rejected() {
        let mut fixed = receipt(BenchmarkPanel::FixedProblemSolver, "fixed");
        fixed.frontend = Some(frontend());
        assert_eq!(fixed.validate(), Err(ReceiptError::FrontendPresentForFixedProblem));
        let mut full = receipt(BenchmarkPanel::FullEnginePipeline, "full");
        full.frontend = None;
        assert_eq!(full.validate(), Err(ReceiptError::MissingFrontendForFullPipeline));
    }

    #[test]
    fn invalid_timing_and_residuals_are_rejected() {
        let mut value = receipt(BenchmarkPanel::FixedProblemSolver, "fixed");
        value.timing.solver_ns = Some(1300);
        assert_eq!(value.validate(), Err(ReceiptError::InvalidTiming));
        value.timing.solver_ns = Some(800);
        value.residuals.as_mut().unwrap().complementarity = f64::NAN;
        assert_eq!(value.validate(), Err(ReceiptError::InvalidResidual));
    }

    #[test]
    fn empty_or_unqualified_contact_receipts_are_not_converged_comparisons() {
        let mut value = receipt(BenchmarkPanel::FixedProblemSolver, "fixed");
        value.solver_input.dropped_rows = 0;
        value.solver_input.emitted_rows = 5;
        assert_eq!(value.validate(), Err(ReceiptError::InvalidSolverRowCounts));
    }
}
