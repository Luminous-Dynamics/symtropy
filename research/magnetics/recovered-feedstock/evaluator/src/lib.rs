//! Route-specific qualification of recovered permanent-magnet feedstocks.
//!
//! This evaluator checks evidence supplied for an identified material lot and
//! manufacturing route. It does not perform recovery chemistry, powder
//! characterization, sintering, extrusion, magnetic simulation, or laboratory
//! measurement.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedstockFamily {
    HpmsRecoveredNdFeB,
    HddRecoveredNdFeB,
    VirginNdFeB,
    SmCo,
    SmFeN,
    Ferrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedstockForm {
    Powder,
    CompositeFeedstock,
    AtomizedPowder,
    BulkMagnet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessRoute {
    PowderExtrusionMouldingSintering,
    MaterialExtrusionBonded,
    LaserPowderBedFusion,
    DirectReuse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    PhysicalMeasurement,
    ProductionRecord,
    PublishedLiterature,
    Simulation,
    SyntheticControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputKind {
    MagnetProduct,
    BinderOrLubricantResidue,
    ProcessReject,
    ProcessLoss,
    OtherMeasuredOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    QualifiedFeedstockRoute,
    Fail,
    Indeterminate,
    NotComparable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub const fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    pub fn is_valid(self) -> bool {
        self.lower.is_finite() && self.upper.is_finite() && self.lower <= self.upper
    }

    pub fn is_nonnegative(self) -> bool {
        self.is_valid() && self.lower >= 0.0
    }

    pub fn add(self, rhs: Self) -> Self {
        Self::new(self.lower + rhs.lower, self.upper + rhs.upper)
    }

    pub fn overlaps(self, rhs: Self) -> bool {
        self.is_valid()
            && rhs.is_valid()
            && self.lower <= rhs.upper
            && rhs.lower <= self.upper
    }

    pub fn is_positive(self) -> bool {
        self.is_valid() && self.lower > 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssaySpec {
    pub assay_id: String,
    pub unit: String,
    pub laboratory_method_scope_digest: String,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteSpec {
    pub campaign_id: String,
    pub scope_digest: String,
    pub source_lot_id: String,
    pub product_lot_id: String,
    pub feedstock_family: FeedstockFamily,
    pub feedstock_form: FeedstockForm,
    pub route: ProcessRoute,
    pub recipe_digest: String,
    pub process_revision_digest: String,
    pub feedstock_input_mass_mg: Interval,
    pub added_input_mass_mg: Interval,
    pub required_output_kinds: Vec<OutputKind>,
    pub required_assays: Vec<AssaySpec>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteEligibilityReceipt {
    pub receipt_id: String,
    pub receipt_digest: String,
    pub evidence_class: EvidenceClass,
    pub issuer_id: String,
    pub scope_digest: String,
    pub source_lot_id: String,
    pub feedstock_family: FeedstockFamily,
    pub feedstock_form: FeedstockForm,
    pub route: ProcessRoute,
    pub recipe_digest: String,
    pub eligible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassOutput {
    pub stream_id: String,
    pub lot_id: String,
    pub kind: OutputKind,
    pub mass_mg: Interval,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssayObservation {
    pub assay_id: String,
    pub unit: String,
    pub value: Interval,
    pub product_lot_id: String,
    pub route: ProcessRoute,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
    pub laboratory_method_scope_digest: String,
    pub laboratory_id: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessEvidence {
    pub process_receipt_id: String,
    pub process_receipt_digest: String,
    pub evidence_class: EvidenceClass,
    pub scope_digest: String,
    pub source_lot_id: String,
    pub product_lot_id: String,
    pub feedstock_family: FeedstockFamily,
    pub feedstock_form: FeedstockForm,
    pub route: ProcessRoute,
    pub recipe_digest: String,
    pub process_revision_digest: String,
    pub equipment_id: String,
    pub feedstock_input_mass_mg: Interval,
    pub added_input_mass_mg: Interval,
    pub outputs: Vec<MassOutput>,
    pub assays: Vec<AssayObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    InvalidSpec,
    InvalidDigest,
    MissingIdentity,
    UnsupportedFamilyFormRoute,
    RouteIneligible,
    EligibilityReceiptScopeMismatch,
    MissingProcessReceipt,
    ProcessReceiptScopeMismatch,
    NonProductionProcessEvidence,
    MissingOutputStream,
    MissingOutputKind,
    MissingMagnetProduct,
    DuplicateOutputStream,
    DuplicateOutputLot,
    MissingOutputReceipt,
    InvalidOutputMass,
    NonPhysicalMassEvidence,
    MissingMassMethod,
    InvalidInputMass,
    InputMassReceiptMismatch,
    DefiniteMassBalanceFailure,
    UnresolvedMassBalance,
    MissingAssay,
    DuplicateAssay,
    ExtraAssay,
    AssayScopeMismatch,
    AssayUnitMismatch,
    AssayMethodScopeMismatch,
    InvalidAssayInterval,
    InadmissibleAssayEvidence,
    MissingAssayMethod,
    MissingCalibration,
    MissingUncertaintyRecord,
    MissingLaboratoryIdentity,
    DefiniteAssayLimitViolation,
    UnresolvedAssayLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssayResult {
    pub assay_id: String,
    pub interval: Interval,
    pub disposition: Disposition,
    pub blockers: Vec<Blocker>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeedstockRouteEvaluation {
    pub campaign_id: String,
    pub disposition: Disposition,
    pub source_lot_id: String,
    pub product_lot_id: String,
    pub route: ProcessRoute,
    pub expected_output_streams: usize,
    pub observed_output_streams: usize,
    pub input_mass_mg: Option<Interval>,
    pub output_mass_mg: Option<Interval>,
    pub assay_results: Vec<AssayResult>,
    pub blockers: Vec<Blocker>,
    pub claim_ceiling: String,
}

/// Qualifies one feedstock lot through one explicitly declared process route.
///
/// PEM and MEX are deliberately separate route identifiers. A route-eligibility
/// receipt cannot override the conservative family/form/route admission matrix.
pub fn evaluate_route(
    spec: &RouteSpec,
    eligibility: &RouteEligibilityReceipt,
    process: &ProcessEvidence,
) -> FeedstockRouteEvaluation {
    let mut blockers = validate_spec(spec);
    let mut not_comparable = false;
    let mut hard_failure = false;
    let mut unresolved = false;

    if !supported_family_form_route(
        spec.feedstock_family,
        spec.feedstock_form,
        spec.route,
    ) {
        blockers.push(Blocker::UnsupportedFamilyFormRoute);
        not_comparable = true;
    }

    if eligibility.scope_digest != spec.scope_digest
        || eligibility.source_lot_id != spec.source_lot_id
        || eligibility.feedstock_family != spec.feedstock_family
        || eligibility.feedstock_form != spec.feedstock_form
        || eligibility.route != spec.route
        || eligibility.recipe_digest != spec.recipe_digest
    {
        blockers.push(Blocker::EligibilityReceiptScopeMismatch);
        not_comparable = true;
    }
    if !eligibility.eligible {
        blockers.push(Blocker::RouteIneligible);
        not_comparable = true;
    }
    if eligibility.receipt_id.trim().is_empty()
        || eligibility.issuer_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
        unresolved = true;
    }
    if !valid_digest(&eligibility.receipt_digest) || !valid_digest(&eligibility.scope_digest)
    {
        blockers.push(Blocker::InvalidDigest);
        unresolved = true;
    }
    if !matches!(
        eligibility.evidence_class,
        EvidenceClass::PhysicalMeasurement | EvidenceClass::ProductionRecord
    ) {
        blockers.push(Blocker::InadmissibleAssayEvidence);
        unresolved = true;
    }

    if process.process_receipt_id.trim().is_empty()
        || process.equipment_id.trim().is_empty()
        || process.source_lot_id.trim().is_empty()
        || process.product_lot_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingProcessReceipt);
        unresolved = true;
    }
    if process.scope_digest != spec.scope_digest
        || process.source_lot_id != spec.source_lot_id
        || process.product_lot_id != spec.product_lot_id
        || process.feedstock_family != spec.feedstock_family
        || process.feedstock_form != spec.feedstock_form
        || process.route != spec.route
        || process.recipe_digest != spec.recipe_digest
        || process.process_revision_digest != spec.process_revision_digest
    {
        blockers.push(Blocker::ProcessReceiptScopeMismatch);
        not_comparable = true;
    }
    if !valid_digest(&process.process_receipt_digest)
        || !valid_digest(&process.scope_digest)
        || !valid_digest(&process.recipe_digest)
        || !valid_digest(&process.process_revision_digest)
    {
        blockers.push(Blocker::InvalidDigest);
        unresolved = true;
    }
    if process.evidence_class != EvidenceClass::ProductionRecord {
        blockers.push(Blocker::NonProductionProcessEvidence);
        unresolved = true;
    }
    let input_mass_scope_valid = process.feedstock_input_mass_mg == spec.feedstock_input_mass_mg
        && process.added_input_mass_mg == spec.added_input_mass_mg;
    if !input_mass_scope_valid {
        blockers.push(Blocker::InputMassReceiptMismatch);
        not_comparable = true;
    }

    let input_mass = spec
        .feedstock_input_mass_mg
        .add(spec.added_input_mass_mg);
    if !spec.feedstock_input_mass_mg.is_nonnegative()
        || !spec.added_input_mass_mg.is_nonnegative()
        || !input_mass.is_nonnegative()
    {
        blockers.push(Blocker::InvalidInputMass);
        unresolved = true;
    }

    let mut output_mass = Interval::new(0.0, 0.0);
    let mut output_stream_ids = BTreeSet::new();
    let mut output_lot_ids = BTreeSet::new();
    let mut has_product = false;
    let mut outputs_complete = !process.outputs.is_empty();

    if process.outputs.is_empty() {
        blockers.push(Blocker::MissingOutputStream);
        unresolved = true;
    }

    for output in &process.outputs {
        if !output_stream_ids.insert(output.stream_id.as_str()) {
            blockers.push(Blocker::DuplicateOutputStream);
            unresolved = true;
            outputs_complete = false;
        }
        if !output_lot_ids.insert(output.lot_id.as_str()) {
            blockers.push(Blocker::DuplicateOutputLot);
            unresolved = true;
            outputs_complete = false;
        }
        if output.stream_id.trim().is_empty()
            || output.lot_id.trim().is_empty()
            || output.receipt_id.trim().is_empty()
            || output.evidence_id.trim().is_empty()
        {
            blockers.push(Blocker::MissingIdentity);
            unresolved = true;
            outputs_complete = false;
        }
        if !valid_digest(&output.receipt_digest)
            || !valid_digest(&output.evidence_digest)
            || !valid_digest(&output.calibration_digest)
            || !valid_digest(&output.uncertainty_record_digest)
        {
            blockers.push(Blocker::InvalidDigest);
            unresolved = true;
            outputs_complete = false;
        }
        if !output.mass_mg.is_nonnegative() {
            blockers.push(Blocker::InvalidOutputMass);
            unresolved = true;
            outputs_complete = false;
        }
        if output.evidence_class != EvidenceClass::PhysicalMeasurement {
            blockers.push(Blocker::NonPhysicalMassEvidence);
            unresolved = true;
            outputs_complete = false;
        }
        if output.method_id.trim().is_empty() {
            blockers.push(Blocker::MissingMassMethod);
            unresolved = true;
            outputs_complete = false;
        }
        if output.kind == OutputKind::MagnetProduct {
            has_product = true;
            if output.lot_id != spec.product_lot_id || !output.mass_mg.is_positive() {
                blockers.push(Blocker::ProcessReceiptScopeMismatch);
                not_comparable = true;
                outputs_complete = false;
            }
        }
        output_mass = output_mass.add(output.mass_mg);
    }

    if !has_product {
        blockers.push(Blocker::MissingMagnetProduct);
        unresolved = true;
        outputs_complete = false;
    }
    for required_kind in &spec.required_output_kinds {
        if !process.outputs.iter().any(|output| output.kind == *required_kind) {
            blockers.push(Blocker::MissingOutputKind);
            unresolved = true;
            outputs_complete = false;
        }
    }

    if outputs_complete
        && input_mass_scope_valid
        && input_mass.is_nonnegative()
        && output_mass.is_nonnegative()
    {
        if !input_mass.overlaps(output_mass) {
            blockers.push(Blocker::DefiniteMassBalanceFailure);
            hard_failure = true;
        }
    } else {
        blockers.push(Blocker::UnresolvedMassBalance);
        unresolved = true;
    }

    let mut assay_results = Vec::with_capacity(spec.required_assays.len());
    let mut seen_assays = BTreeSet::new();
    for observation in &process.assays {
        if !seen_assays.insert(observation.assay_id.as_str()) {
            blockers.push(Blocker::DuplicateAssay);
            unresolved = true;
        }
        if observation.product_lot_id != spec.product_lot_id
            || observation.route != spec.route
        {
            blockers.push(Blocker::AssayScopeMismatch);
            not_comparable = true;
        }
    }

    for required in &spec.required_assays {
        let matching = process
            .assays
            .iter()
            .filter(|observation| observation.assay_id == required.assay_id)
            .collect::<Vec<_>>();
        let Some(observation) = matching.first().copied() else {
            blockers.push(Blocker::MissingAssay);
            assay_results.push(AssayResult {
                assay_id: required.assay_id.clone(),
                interval: Interval::new(f64::NAN, f64::NAN),
                disposition: Disposition::Indeterminate,
                blockers: vec![Blocker::MissingAssay],
            });
            unresolved = true;
            continue;
        };

        let mut assay_blockers = Vec::new();
        let mut assay_hard_failure = false;
        let mut assay_unresolved = false;
        if observation.unit != required.unit {
            assay_blockers.push(Blocker::AssayUnitMismatch);
            not_comparable = true;
        }
        if observation.laboratory_method_scope_digest
            != required.laboratory_method_scope_digest
        {
            assay_blockers.push(Blocker::AssayMethodScopeMismatch);
            not_comparable = true;
        }
        if !observation.value.is_valid() {
            assay_blockers.push(Blocker::InvalidAssayInterval);
            assay_unresolved = true;
        }
        if observation.evidence_class != EvidenceClass::PhysicalMeasurement {
            assay_blockers.push(Blocker::InadmissibleAssayEvidence);
            assay_unresolved = true;
        }
        if observation.evidence_id.trim().is_empty()
            || observation.laboratory_id.trim().is_empty()
        {
            assay_blockers.push(Blocker::MissingLaboratoryIdentity);
            assay_unresolved = true;
        }
        if !valid_digest(&observation.evidence_digest) {
            assay_blockers.push(Blocker::InvalidDigest);
            assay_unresolved = true;
        }
        if observation.method_id.trim().is_empty() {
            assay_blockers.push(Blocker::MissingAssayMethod);
            assay_unresolved = true;
        }
        if !valid_digest(&observation.calibration_digest) {
            assay_blockers.push(Blocker::MissingCalibration);
            assay_unresolved = true;
        }
        if !valid_digest(&observation.uncertainty_record_digest) {
            assay_blockers.push(Blocker::MissingUncertaintyRecord);
            assay_unresolved = true;
        }

        if observation.value.is_valid() {
            if required
                .minimum
                .is_some_and(|minimum| observation.value.upper < minimum)
                || required
                    .maximum
                    .is_some_and(|maximum| observation.value.lower > maximum)
            {
                assay_blockers.push(Blocker::DefiniteAssayLimitViolation);
                assay_hard_failure = true;
            } else if required
                .minimum
                .is_some_and(|minimum| observation.value.lower < minimum)
                || required
                    .maximum
                    .is_some_and(|maximum| observation.value.upper > maximum)
            {
                assay_blockers.push(Blocker::UnresolvedAssayLimit);
                assay_unresolved = true;
            }
        }

        let assay_disposition = if assay_hard_failure {
            Disposition::Fail
        } else if not_comparable || assay_blockers.iter().any(is_scope_blocker) {
            Disposition::NotComparable
        } else if assay_unresolved || !assay_blockers.is_empty() {
            Disposition::Indeterminate
        } else {
            Disposition::QualifiedFeedstockRoute
        };
        hard_failure |= assay_hard_failure;
        unresolved |= assay_unresolved;
        assay_results.push(AssayResult {
            assay_id: required.assay_id.clone(),
            interval: observation.value,
            disposition: assay_disposition,
            blockers: dedupe(assay_blockers),
        });
    }

    for observation in &process.assays {
        if !spec
            .required_assays
            .iter()
            .any(|required| required.assay_id == observation.assay_id)
        {
            blockers.push(Blocker::ExtraAssay);
            unresolved = true;
        }
    }

    let disposition = if not_comparable {
        Disposition::NotComparable
    } else if hard_failure {
        Disposition::Fail
    } else if unresolved || !blockers.is_empty() {
        Disposition::Indeterminate
    } else {
        Disposition::QualifiedFeedstockRoute
    };

    if disposition == Disposition::Fail && !hard_failure {
        blockers.push(Blocker::DefiniteAssayLimitViolation);
    }
    if disposition == Disposition::Indeterminate && blockers.is_empty() {
        blockers.push(Blocker::UnresolvedMassBalance);
    }

    FeedstockRouteEvaluation {
        campaign_id: spec.campaign_id.clone(),
        disposition,
        source_lot_id: spec.source_lot_id.clone(),
        product_lot_id: spec.product_lot_id.clone(),
        route: spec.route,
        expected_output_streams: spec.required_output_kinds.len(),
        observed_output_streams: process.outputs.len(),
        input_mass_mg: input_mass.is_valid().then_some(input_mass),
        output_mass_mg: output_mass.is_valid().then_some(output_mass),
        assay_results,
        blockers: dedupe(blockers),
        claim_ceiling: format!(
            "{disposition:?}: route-specific recovered feedstock/process-output evidence only; no M4 component function, M5 process capability, LCA, certification or product-release claim"
        ),
    }
}

fn validate_spec(spec: &RouteSpec) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if spec.campaign_id.trim().is_empty()
        || spec.source_lot_id.trim().is_empty()
        || spec.product_lot_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }
    if !valid_digest(&spec.scope_digest)
        || !valid_digest(&spec.recipe_digest)
        || !valid_digest(&spec.process_revision_digest)
    {
        blockers.push(Blocker::InvalidDigest);
    }
    if !spec.feedstock_input_mass_mg.is_nonnegative()
        || !spec.added_input_mass_mg.is_nonnegative()
    {
        blockers.push(Blocker::InvalidInputMass);
    }
    if spec.required_assays.is_empty() || spec.required_output_kinds.is_empty() {
        blockers.push(Blocker::InvalidSpec);
    }
    let mut required_kinds = BTreeSet::new();
    for kind in &spec.required_output_kinds {
        if !required_kinds.insert(*kind) {
            blockers.push(Blocker::InvalidSpec);
        }
    }
    let mut assays = BTreeSet::new();
    for assay in &spec.required_assays {
        if assay.assay_id.trim().is_empty()
            || assay.unit.trim().is_empty()
            || !valid_digest(&assay.laboratory_method_scope_digest)
            || (assay.minimum.is_none() && assay.maximum.is_none())
            || !assays.insert(assay.assay_id.as_str())
            || assay.minimum.is_some_and(|value| !value.is_finite())
            || assay.maximum.is_some_and(|value| !value.is_finite())
            || assay
                .minimum
                .zip(assay.maximum)
                .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            blockers.push(Blocker::InvalidSpec);
        }
    }
    blockers
}

/// Conservative v0.1 admission matrix. New feedstock/route pairs require an
/// explicit protocol extension and route-specific evidence, not a broad label.
pub fn supported_family_form_route(
    family: FeedstockFamily,
    form: FeedstockForm,
    route: ProcessRoute,
) -> bool {
    matches!(
        (family, form, route),
        (
            FeedstockFamily::HpmsRecoveredNdFeB,
            FeedstockForm::Powder,
            ProcessRoute::PowderExtrusionMouldingSintering
        ) | (
            FeedstockFamily::VirginNdFeB,
            FeedstockForm::Powder,
            ProcessRoute::PowderExtrusionMouldingSintering
        ) | (
            FeedstockFamily::HddRecoveredNdFeB,
            FeedstockForm::Powder,
            ProcessRoute::MaterialExtrusionBonded
        ) | (
            FeedstockFamily::VirginNdFeB,
            FeedstockForm::CompositeFeedstock,
            ProcessRoute::MaterialExtrusionBonded
        ) | (
            FeedstockFamily::VirginNdFeB,
            FeedstockForm::AtomizedPowder,
            ProcessRoute::LaserPowderBedFusion
        )
    )
}

fn is_scope_blocker(blocker: &Blocker) -> bool {
    matches!(
        blocker,
        Blocker::AssayScopeMismatch | Blocker::AssayUnitMismatch
    )
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn dedupe(mut blockers: Vec<Blocker>) -> Vec<Blocker> {
    blockers.sort_by_key(|blocker| format!("{blocker:?}"));
    blockers.dedup();
    blockers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn spec() -> RouteSpec {
        RouteSpec {
            campaign_id: "RECOVERED-NDFEB-PEM-001".to_string(),
            scope_digest: digest('a'),
            source_lot_id: "HPMS-SOURCE-LOT-001".to_string(),
            product_lot_id: "PEM-MAGNET-LOT-001".to_string(),
            feedstock_family: FeedstockFamily::HpmsRecoveredNdFeB,
            feedstock_form: FeedstockForm::Powder,
            route: ProcessRoute::PowderExtrusionMouldingSintering,
            recipe_digest: digest('b'),
            process_revision_digest: digest('c'),
            feedstock_input_mass_mg: Interval::new(999.0, 1001.0),
            added_input_mass_mg: Interval::new(99.0, 101.0),
            required_output_kinds: vec![
                OutputKind::MagnetProduct,
                OutputKind::BinderOrLubricantResidue,
            ],
            required_assays: vec![
                AssaySpec {
                    assay_id: "carbon_wt_percent".to_string(),
                    unit: "wt%".to_string(),
                    laboratory_method_scope_digest: digest('4'),
                    minimum: None,
                    maximum: Some(0.06),
                },
                AssaySpec {
                    assay_id: "oxygen_increase_wt_percent_points".to_string(),
                    unit: "wt%-points".to_string(),
                    laboratory_method_scope_digest: digest('4'),
                    minimum: None,
                    maximum: Some(0.50),
                },
                AssaySpec {
                    assay_id: "remanence_to_saturation_ratio".to_string(),
                    unit: "ratio".to_string(),
                    laboratory_method_scope_digest: digest('4'),
                    minimum: Some(0.90),
                    maximum: Some(1.00),
                },
            ],
        }
    }

    fn eligibility(specification: &RouteSpec) -> RouteEligibilityReceipt {
        RouteEligibilityReceipt {
            receipt_id: "ELIGIBILITY-001".to_string(),
            receipt_digest: digest('d'),
            evidence_class: EvidenceClass::ProductionRecord,
            issuer_id: "PROCESS-AUTHORITY-001".to_string(),
            scope_digest: specification.scope_digest.clone(),
            source_lot_id: specification.source_lot_id.clone(),
            feedstock_family: specification.feedstock_family,
            feedstock_form: specification.feedstock_form,
            route: specification.route,
            recipe_digest: specification.recipe_digest.clone(),
            eligible: true,
        }
    }

    fn mass_output(
        stream_id: &str,
        lot_id: &str,
        kind: OutputKind,
        mass: Interval,
        suffix: char,
    ) -> MassOutput {
        MassOutput {
            stream_id: stream_id.to_string(),
            lot_id: lot_id.to_string(),
            kind,
            mass_mg: mass,
            receipt_id: format!("WEIGH-{stream_id}"),
            receipt_digest: digest('e'),
            evidence_id: format!("MASS-{stream_id}"),
            evidence_digest: digest(suffix),
            method_id: format!("SCALE-METHOD-{stream_id}"),
            calibration_digest: digest('f'),
            uncertainty_record_digest: digest('1'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
        }
    }

    fn assay(
        assay_id: &str,
        unit: &str,
        value: Interval,
        product_lot_id: &str,
        route: ProcessRoute,
        suffix: char,
    ) -> AssayObservation {
        AssayObservation {
            assay_id: assay_id.to_string(),
            unit: unit.to_string(),
            value,
            product_lot_id: product_lot_id.to_string(),
            route,
            evidence_id: format!("ASSAY-{assay_id}"),
            evidence_digest: digest(suffix),
            method_id: format!("METHOD-{assay_id}"),
            calibration_digest: digest('2'),
            uncertainty_record_digest: digest('3'),
            laboratory_method_scope_digest: digest('4'),
            laboratory_id: "LAB-METHOD-SCOPE-001".to_string(),
            evidence_class: EvidenceClass::PhysicalMeasurement,
        }
    }

    fn process(specification: &RouteSpec) -> ProcessEvidence {
        ProcessEvidence {
            process_receipt_id: "PROCESS-RECEIPT-001".to_string(),
            process_receipt_digest: digest('4'),
            evidence_class: EvidenceClass::ProductionRecord,
            scope_digest: specification.scope_digest.clone(),
            source_lot_id: specification.source_lot_id.clone(),
            product_lot_id: specification.product_lot_id.clone(),
            feedstock_family: specification.feedstock_family,
            feedstock_form: specification.feedstock_form,
            route: specification.route,
            recipe_digest: specification.recipe_digest.clone(),
            process_revision_digest: specification.process_revision_digest.clone(),
            equipment_id: "ROUTE-EQUIPMENT-001".to_string(),
            feedstock_input_mass_mg: specification.feedstock_input_mass_mg,
            added_input_mass_mg: specification.added_input_mass_mg,
            outputs: vec![
                mass_output(
                    "magnet-product",
                    &specification.product_lot_id,
                    OutputKind::MagnetProduct,
                    Interval::new(989.0, 993.0),
                    '5',
                ),
                mass_output(
                    "process-residue",
                    "RESIDUE-LOT-001",
                    OutputKind::BinderOrLubricantResidue,
                    Interval::new(105.0, 111.0),
                    '6',
                ),
            ],
            assays: vec![
                assay(
                    "carbon_wt_percent",
                    "wt%",
                    Interval::new(0.042, 0.048),
                    &specification.product_lot_id,
                    specification.route,
                    '7',
                ),
                assay(
                    "oxygen_increase_wt_percent_points",
                    "wt%-points",
                    Interval::new(0.35, 0.45),
                    &specification.product_lot_id,
                    specification.route,
                    '8',
                ),
                assay(
                    "remanence_to_saturation_ratio",
                    "ratio",
                    Interval::new(0.95, 0.97),
                    &specification.product_lot_id,
                    specification.route,
                    '9',
                ),
            ],
        }
    }

    fn evaluate(
        specification: &RouteSpec,
        process_evidence: &ProcessEvidence,
        eligibility_receipt: &RouteEligibilityReceipt,
    ) -> FeedstockRouteEvaluation {
        evaluate_route(specification, eligibility_receipt, process_evidence)
    }

    #[test]
    fn hpms_recovered_ndfeb_pem_route_qualifies_with_complete_physical_evidence() {
        let specification = spec();
        let result = evaluate(
            &specification,
            &process(&specification),
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::QualifiedFeedstockRoute);
        assert_eq!(result.expected_output_streams, 2);
        assert_eq!(result.observed_output_streams, 2);
        assert_eq!(result.assay_results.len(), 3);
        assert!(result.input_mass_mg.unwrap().overlaps(result.output_mass_mg.unwrap()));
    }

    #[test]
    fn hpms_powder_is_not_automatically_admitted_to_lpbf() {
        assert!(!supported_family_form_route(
            FeedstockFamily::HpmsRecoveredNdFeB,
            FeedstockForm::Powder,
            ProcessRoute::LaserPowderBedFusion
        ));
    }

    #[test]
    fn recovered_hard_drive_ndfeb_powder_is_admitted_to_mex_only_with_exact_receipts() {
        let mut specification = spec();
        specification.campaign_id = "RECOVERED-HDD-NDFEB-MEX-001".to_string();
        specification.source_lot_id = "HDD-SOURCE-LOT-001".to_string();
        specification.product_lot_id = "HDD-MEX-PRODUCT-LOT-001".to_string();
        specification.feedstock_family = FeedstockFamily::HddRecoveredNdFeB;
        specification.route = ProcessRoute::MaterialExtrusionBonded;

        let eligibility_receipt = eligibility(&specification);
        let process_evidence = process(&specification);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility_receipt,
        );
        assert_eq!(result.disposition, Disposition::QualifiedFeedstockRoute);
        assert_eq!(result.route, ProcessRoute::MaterialExtrusionBonded);
        assert!(supported_family_form_route(
            FeedstockFamily::HddRecoveredNdFeB,
            FeedstockForm::Powder,
            ProcessRoute::MaterialExtrusionBonded
        ));
    }

    #[test]
    fn pem_and_mex_are_distinct_route_families() {
        assert!(!supported_family_form_route(
            FeedstockFamily::HpmsRecoveredNdFeB,
            FeedstockForm::Powder,
            ProcessRoute::MaterialExtrusionBonded
        ));
        assert!(supported_family_form_route(
            FeedstockFamily::HddRecoveredNdFeB,
            FeedstockForm::Powder,
            ProcessRoute::MaterialExtrusionBonded
        ));
    }

    #[test]
    fn wrong_family_form_route_is_not_comparable() {
        let mut specification = spec();
        specification.feedstock_family = FeedstockFamily::SmCo;
        let eligibility_receipt = eligibility(&specification);
        let process_evidence = process(&specification);
        let result = evaluate(&specification, &process_evidence, &eligibility_receipt);
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::UnsupportedFamilyFormRoute));
    }

    #[test]
    fn eligibility_receipt_must_bind_exact_source_lot_and_recipe() {
        let specification = spec();
        let mut receipt = eligibility(&specification);
        receipt.source_lot_id = "OTHER-LOT".to_string();
        let result = evaluate(
            &specification,
            &process(&specification),
            &receipt,
        );
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::EligibilityReceiptScopeMismatch));
    }

    #[test]
    fn explicitly_ineligible_route_cannot_be_promoted() {
        let specification = spec();
        let mut receipt = eligibility(&specification);
        receipt.eligible = false;
        let result = evaluate(
            &specification,
            &process(&specification),
            &receipt,
        );
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::RouteIneligible));
    }

    #[test]
    fn process_recipe_mismatch_is_not_comparable() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.recipe_digest = digest('0');
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::ProcessReceiptScopeMismatch));
    }

    #[test]
    fn source_lot_cannot_be_substituted_between_eligibility_and_process() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.source_lot_id = "OTHER-SOURCE-LOT".to_string();
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::NotComparable);
    }

    #[test]
    fn definite_mass_balance_failure_is_a_hard_fail() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.outputs[1].mass_mg = Interval::new(10.0, 12.0);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(result.blockers.contains(&Blocker::DefiniteMassBalanceFailure));
    }

    #[test]
    fn overlapping_mass_intervals_support_mass_balance() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.outputs[1].mass_mg = Interval::new(98.0, 112.0);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::QualifiedFeedstockRoute);
    }

    #[test]
    fn missing_output_stream_is_indeterminate_not_a_good_yield() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.outputs.clear();
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingOutputStream));
    }

    #[test]
    fn missing_magnet_product_stream_is_indeterminate() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.outputs.retain(|item| item.kind != OutputKind::MagnetProduct);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingMagnetProduct));
    }

    #[test]
    fn duplicate_output_stream_identity_is_not_silently_counted() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.outputs[1].stream_id = process_evidence.outputs[0].stream_id.clone();
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::DuplicateOutputStream));
    }

    #[test]
    fn a_definite_assay_limit_violation_fails_the_route() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.assays[0].value = Interval::new(0.08, 0.09);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(result.blockers.contains(&Blocker::DefiniteAssayLimitViolation));
    }

    #[test]
    fn an_assay_interval_crossing_a_limit_is_indeterminate() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.assays[1].value = Interval::new(0.45, 0.55);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::UnresolvedAssayLimit));
    }

    #[test]
    fn simulation_cannot_qualify_a_physical_feedstock_route() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.assays[0].evidence_class = EvidenceClass::Simulation;
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InadmissibleAssayEvidence));
    }

    #[test]
    fn every_assay_must_bind_the_declared_product_lot() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.assays[2].product_lot_id = "OTHER-LOT".to_string();
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::AssayScopeMismatch));
    }

    #[test]
    fn br_over_js_is_a_distinct_ratio_assay_not_absolute_remanence() {
        let specification = spec();
        let process_evidence = process(&specification);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        let alignment = result
            .assay_results
            .iter()
            .find(|item| item.assay_id == "remanence_to_saturation_ratio")
            .unwrap();
        assert_eq!(alignment.interval, Interval::new(0.95, 0.97));
        assert_eq!(result.disposition, Disposition::QualifiedFeedstockRoute);
    }

    #[test]
    fn required_output_stream_kinds_are_explicit_not_assumed() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.outputs.retain(|item| item.kind != OutputKind::BinderOrLubricantResidue);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingOutputKind));
        assert_eq!(result.expected_output_streams, 2);
    }

    #[test]
    fn assay_method_scope_must_match_the_declared_laboratory_scope() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        process_evidence.assays[1].laboratory_method_scope_digest = digest('0');
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert!(result.blockers.contains(&Blocker::AssayMethodScopeMismatch));
    }

    #[test]
    fn extra_assays_are_visible_and_do_not_expand_the_acceptance_policy() {
        let specification = spec();
        let mut process_evidence = process(&specification);
        let mut extra = process_evidence.assays[0].clone();
        extra.assay_id = "unregistered_metric".to_string();
        process_evidence.assays.push(extra);
        let result = evaluate(
            &specification,
            &process_evidence,
            &eligibility(&specification),
        );
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::ExtraAssay));
    }
}
