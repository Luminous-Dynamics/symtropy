//! Denominator-safe cohort accounting for manufactured magnetic components.
//!
//! The evaluator composes specimen-level acceptance receipts. It does not
//! calculate fields, infer process distributions, estimate population yield,
//! authenticate evidence producers, or authorize physical manufacturing.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RouteFamily {
    Mex,
    LpbfSlm,
    Sls,
    BinderJet,
    LcmPhotopolymer,
    PemAdjacent,
    ConventionalBonded,
    SinteredOrAssembled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildStatus {
    Completed,
    FailedBeforeInspection,
    Incomplete,
    Aborted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecimenDisposition {
    Qualified,
    Fail,
    Indeterminate,
    NotEvaluable,
    Missing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneralizationStatus {
    GeneralizationUnavailable,
    ReplicationCoverageMeetsPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CohortScope {
    pub application_id: String,
    pub target_function_digest: String,
    pub measurement_setup_digest: String,
    pub acceptance_policy_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeedstockLot {
    pub lot_id: String,
    pub feedstock_family: String,
    pub material_grade: String,
    pub recycled_mass_fraction: f64,
    pub lot_record_digest: String,
    pub source_provenance_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildRecord {
    pub build_id: String,
    pub material_lot_id: String,
    pub route_revision: String,
    pub machine_tool_revision: String,
    pub recipe_digest: String,
    pub build_record_digest: String,
    pub genealogy_digest: String,
    pub status: BuildStatus,
    /// The denominator of initiated specimens is declared here, not inferred
    /// from whichever specimen records happen to be present.
    pub initiated_specimen_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecimenRecord {
    pub specimen_id: String,
    pub build_id: String,
    pub material_lot_id: String,
    pub scope: CohortScope,
    pub manufacturing_complete: bool,
    pub inspection_completed: bool,
    pub first_pass_disposition: SpecimenDisposition,
    pub qualification_receipt_id: Option<String>,
    pub qualification_receipt_digest: Option<String>,
    pub rework_attempts: u32,
    pub rework_receipt_id: Option<String>,
    pub rework_receipt_digest: Option<String>,
    pub final_disposition: SpecimenDisposition,
    pub scrapped: bool,
    pub scrap_receipt_id: Option<String>,
    pub scrap_receipt_digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationPolicy {
    pub minimum_independent_builds: u32,
    pub minimum_distinct_material_lots: u32,
    /// A met coverage policy is not itself a statistical yield qualification.
    pub policy_id: String,
    pub policy_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignCostReceipt {
    pub receipt_id: String,
    pub digest: String,
    pub currency: String,
    /// All initiated campaign cost, including failed builds, rework and scrap.
    pub total_campaign_cost_minor: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteCohort {
    pub cohort_id: String,
    pub route_family: RouteFamily,
    pub route_revision: String,
    pub machine_tool_revision: String,
    pub process_recipe_digest: String,
    pub feedstock_family: String,
    pub material_grade: String,
    pub scope: CohortScope,
    pub replication_policy: ReplicationPolicy,
    pub lots: Vec<FeedstockLot>,
    pub builds: Vec<BuildRecord>,
    pub specimens: Vec<SpecimenRecord>,
    pub campaign_cost: Option<CampaignCostReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    MissingIdentity,
    InvalidDigest,
    InvalidScope,
    InvalidRecycledFraction,
    InvalidCurrency,
    InvalidReplicationPolicy,
    DuplicateLotId,
    DuplicateBuildId,
    DuplicateSpecimenId,
    DuplicateInitiatedSpecimenId,
    OrphanSpecimenRecord,
    MissingLotRecord,
    BuildLotMismatch,
    BuildRouteRevisionMismatch,
    BuildMachineRevisionMismatch,
    BuildRecipeMismatch,
    InvalidBuildRecord,
    InitiatedSpecimenMissingRecord,
    SpecimenBuildMismatch,
    SpecimenLotMismatch,
    SpecimenScopeMismatch,
    InvalidQualificationReceipt,
    InvalidInspectionState,
    InvalidManufacturingState,
    InvalidReworkReceipt,
    InvalidReworkState,
    InvalidScrapReceipt,
    FailedBuildHasCompletedSpecimen,
    DuplicateQualificationReceiptId,
    DuplicateReworkReceiptId,
    DuplicateScrapReceiptId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate {
    pub numerator: u64,
    pub denominator: u64,
    pub ratio_ppm: Option<u64>,
}

impl Rate {
    fn new(numerator: u64, denominator: u64) -> Self {
        let ratio_ppm = if denominator == 0 {
            None
        } else {
            let scaled = (u128::from(numerator) * 1_000_000) / u128::from(denominator);
            Some(scaled.min(1_000_000) as u64)
        };
        Self {
            numerator,
            denominator,
            ratio_ppm,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostPerAcceptedPart {
    pub currency: String,
    pub total_campaign_cost_minor: u64,
    pub accepted_part_count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CohortCounts {
    pub initiated_builds: u64,
    pub completed_builds: u64,
    pub pre_inspection_build_failures: u64,
    pub initiated_specimens: u64,
    pub completed_specimens: u64,
    pub inspected_specimens: u64,
    pub first_pass_qualified: u64,
    pub first_pass_fail: u64,
    pub first_pass_indeterminate: u64,
    pub first_pass_not_evaluable: u64,
    pub missing_specimen_outcomes: u64,
    pub reworked_specimens: u64,
    pub rework_recovered: u64,
    pub scrapped_specimens: u64,
    pub final_accepted_specimens: u64,
    pub distinct_material_lots: u64,
    pub replication_eligible_builds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CohortSummary {
    pub counts: CohortCounts,
    pub first_pass_acceptance: Rate,
    pub confirmed_failure: Rate,
    pub unresolved: Rate,
    pub inspection_coverage: Rate,
    pub build_completion: Rate,
    pub rework_recovery: Rate,
    pub final_accepted_fraction: Rate,
    pub generalization_status: GeneralizationStatus,
    /// v0.1 deliberately never converts observed fractions into inferential
    /// production-yield claims.
    pub inferential_yield_claim_available: bool,
    pub cost_per_accepted_part: Option<CostPerAcceptedPart>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CohortEvaluation {
    pub cohort_id: String,
    pub blockers: Vec<Blocker>,
    pub summary: CohortSummary,
}

impl CohortEvaluation {
    pub fn structurally_valid(&self) -> bool {
        self.blockers.is_empty()
    }
}

/// Evaluate a declared cohort. The build records, not the set of available
/// inspection records, define the initiated specimen denominator.
pub fn evaluate_cohort(cohort: &RouteCohort) -> CohortEvaluation {
    let mut blockers = validate_cohort_identity(cohort);
    let mut counts = CohortCounts::default();

    let mut lot_by_id = BTreeMap::new();
    let mut valid_lot_ids = BTreeSet::new();
    let mut duplicate_lot_ids = BTreeSet::new();
    for lot in &cohort.lots {
        if lot_by_id.insert(lot.lot_id.as_str(), lot).is_some() {
            blockers.push(Blocker::DuplicateLotId);
            duplicate_lot_ids.insert(lot.lot_id.as_str());
        }
        if validate_lot_record(lot, cohort, &mut blockers) {
            valid_lot_ids.insert(lot.lot_id.as_str());
        }
    }
    for duplicate_id in &duplicate_lot_ids {
        valid_lot_ids.remove(duplicate_id);
    }

    let mut build_id_counts = BTreeMap::new();
    for build in &cohort.builds {
        *build_id_counts.entry(build.build_id.as_str()).or_insert(0usize) += 1;
    }
    counts.initiated_builds = cohort.builds.len() as u64;

    let mut used_valid_lot_ids = BTreeSet::new();
    let mut valid_build_ids = BTreeSet::new();
    let mut declared_specimen_to_build = BTreeMap::new();
    for build in &cohort.builds {
        let build_valid =
            validate_build_record(cohort, build, &lot_by_id, &valid_lot_ids, &mut blockers);
        let unique_build_id =
            build_id_counts.get(build.build_id.as_str()).copied().unwrap_or(0) == 1;
        if build_valid && unique_build_id {
            valid_build_ids.insert(build.build_id.as_str());
            used_valid_lot_ids.insert(build.material_lot_id.as_str());
            if build.status == BuildStatus::Completed {
                counts.completed_builds += 1;
            }
            if build.status == BuildStatus::FailedBeforeInspection {
                counts.pre_inspection_build_failures += 1;
            }
        } else {
            blockers.push(Blocker::InvalidBuildRecord);
        }

        let mut local_ids = BTreeSet::new();
        for specimen_id in &build.initiated_specimen_ids {
            if specimen_id.trim().is_empty() {
                blockers.push(Blocker::MissingIdentity);
                continue;
            }
            if !local_ids.insert(specimen_id.as_str()) {
                blockers.push(Blocker::DuplicateInitiatedSpecimenId);
            }
            if declared_specimen_to_build
                .insert(specimen_id.as_str(), build)
                .is_some()
            {
                blockers.push(Blocker::DuplicateInitiatedSpecimenId);
            }
        }
    }
    counts.distinct_material_lots = used_valid_lot_ids.len() as u64;
    counts.replication_eligible_builds = valid_build_ids.len() as u64;

    let mut specimen_records_by_id: BTreeMap<&str, Vec<&SpecimenRecord>> = BTreeMap::new();
    let mut qualification_receipt_ids = BTreeSet::new();
    let mut rework_receipt_ids = BTreeSet::new();
    let mut scrap_receipt_ids = BTreeSet::new();
    for specimen in &cohort.specimens {
        if let Some(id) = &specimen.qualification_receipt_id
            && !qualification_receipt_ids.insert(id.as_str())
        {
            blockers.push(Blocker::DuplicateQualificationReceiptId);
        }
        if let Some(id) = &specimen.rework_receipt_id
            && !rework_receipt_ids.insert(id.as_str())
        {
            blockers.push(Blocker::DuplicateReworkReceiptId);
        }
        if let Some(id) = &specimen.scrap_receipt_id
            && !scrap_receipt_ids.insert(id.as_str())
        {
            blockers.push(Blocker::DuplicateScrapReceiptId);
        }
    }

    for specimen in &cohort.specimens {
        specimen_records_by_id
            .entry(specimen.specimen_id.as_str())
            .or_default()
            .push(specimen);
        if !declared_specimen_to_build.contains_key(specimen.specimen_id.as_str()) {
            blockers.push(Blocker::OrphanSpecimenRecord);
        }
    }

    counts.initiated_specimens = declared_specimen_to_build.len() as u64;

    for (specimen_id, build) in &declared_specimen_to_build {
        let Some(records) = specimen_records_by_id.get(specimen_id) else {
            blockers.push(Blocker::InitiatedSpecimenMissingRecord);
            counts.missing_specimen_outcomes += 1;
            continue;
        };
        if records.len() != 1 {
            blockers.push(Blocker::DuplicateSpecimenId);
            counts.missing_specimen_outcomes += 1;
            continue;
        }

        let specimen = records[0];
        if !validate_specimen_record(cohort, build, specimen, &mut blockers) {
            counts.missing_specimen_outcomes += 1;
            continue;
        }

        if specimen.manufacturing_complete {
            counts.completed_specimens += 1;
        }
        if specimen.inspection_completed {
            counts.inspected_specimens += 1;
        }
        match specimen.first_pass_disposition {
            SpecimenDisposition::Qualified => counts.first_pass_qualified += 1,
            SpecimenDisposition::Fail => counts.first_pass_fail += 1,
            SpecimenDisposition::Indeterminate => counts.first_pass_indeterminate += 1,
            SpecimenDisposition::NotEvaluable => counts.first_pass_not_evaluable += 1,
            SpecimenDisposition::Missing => counts.missing_specimen_outcomes += 1,
        }
        if specimen.rework_attempts > 0 {
            counts.reworked_specimens += 1;
            if specimen.final_disposition == SpecimenDisposition::Qualified && !specimen.scrapped {
                counts.rework_recovered += 1;
            }
        }
        if specimen.scrapped {
            counts.scrapped_specimens += 1;
        }
        if specimen.final_disposition == SpecimenDisposition::Qualified && !specimen.scrapped {
            counts.final_accepted_specimens += 1;
        }
    }

    let unresolved_count = counts.first_pass_indeterminate
        + counts.first_pass_not_evaluable
        + counts.missing_specimen_outcomes;

    let generalization_status = if !blockers.is_empty()
        || counts.replication_eligible_builds
            < u64::from(cohort.replication_policy.minimum_independent_builds)
        || counts.distinct_material_lots
            < u64::from(cohort.replication_policy.minimum_distinct_material_lots)
    {
        GeneralizationStatus::GeneralizationUnavailable
    } else {
        GeneralizationStatus::ReplicationCoverageMeetsPolicy
    };

    let cost_per_accepted_part = cohort
        .campaign_cost
        .as_ref()
        .map(|cost| CostPerAcceptedPart {
            currency: cost.currency.clone(),
            total_campaign_cost_minor: cost.total_campaign_cost_minor,
            accepted_part_count: counts.final_accepted_specimens,
        });

    CohortEvaluation {
        cohort_id: cohort.cohort_id.clone(),
        blockers,
        summary: CohortSummary {
            first_pass_acceptance: Rate::new(
                counts.first_pass_qualified,
                counts.initiated_specimens,
            ),
            confirmed_failure: Rate::new(counts.first_pass_fail, counts.initiated_specimens),
            unresolved: Rate::new(unresolved_count, counts.initiated_specimens),
            inspection_coverage: Rate::new(counts.inspected_specimens, counts.initiated_specimens),
            build_completion: Rate::new(counts.completed_builds, counts.initiated_builds),
            rework_recovery: Rate::new(counts.rework_recovered, counts.reworked_specimens),
            final_accepted_fraction: Rate::new(
                counts.final_accepted_specimens,
                counts.initiated_specimens,
            ),
            generalization_status,
            inferential_yield_claim_available: false,
            cost_per_accepted_part,
            counts,
        },
    }
}

fn validate_cohort_identity(cohort: &RouteCohort) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if cohort.cohort_id.trim().is_empty()
        || cohort.route_revision.trim().is_empty()
        || cohort.machine_tool_revision.trim().is_empty()
        || cohort.feedstock_family.trim().is_empty()
        || cohort.material_grade.trim().is_empty()
        || cohort.scope.application_id.trim().is_empty()
        || cohort.replication_policy.policy_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }
    for digest in [
        &cohort.process_recipe_digest,
        &cohort.scope.target_function_digest,
        &cohort.scope.measurement_setup_digest,
        &cohort.scope.acceptance_policy_digest,
        &cohort.replication_policy.policy_digest,
    ] {
        if !valid_sha256(digest) {
            blockers.push(Blocker::InvalidDigest);
        }
    }
    if cohort.replication_policy.minimum_independent_builds == 0
        || cohort.replication_policy.minimum_distinct_material_lots == 0
    {
        blockers.push(Blocker::InvalidReplicationPolicy);
    }
    if cohort.builds.is_empty() {
        blockers.push(Blocker::InvalidBuildRecord);
    }
    if let Some(cost) = &cohort.campaign_cost {
        if cost.receipt_id.trim().is_empty() {
            blockers.push(Blocker::MissingIdentity);
        }
        if !valid_sha256(&cost.digest) {
            blockers.push(Blocker::InvalidDigest);
        }
        if cost.currency.len() != 3 || !cost.currency.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            blockers.push(Blocker::InvalidCurrency);
        }
    }
    blockers
}

fn validate_lot_record(
    lot: &FeedstockLot,
    cohort: &RouteCohort,
    blockers: &mut Vec<Blocker>,
) -> bool {
    let mut valid = true;
    if lot.lot_id.trim().is_empty()
        || lot.feedstock_family.trim().is_empty()
        || lot.material_grade.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
        valid = false;
    }
    if !valid_sha256(&lot.lot_record_digest) || !valid_sha256(&lot.source_provenance_digest) {
        blockers.push(Blocker::InvalidDigest);
        valid = false;
    }
    if !lot.recycled_mass_fraction.is_finite() || !(0.0..=1.0).contains(&lot.recycled_mass_fraction)
    {
        blockers.push(Blocker::InvalidRecycledFraction);
        valid = false;
    }
    if lot.feedstock_family != cohort.feedstock_family
        || lot.material_grade != cohort.material_grade
    {
        blockers.push(Blocker::BuildLotMismatch);
        valid = false;
    }
    valid
}

fn validate_build_record(
    cohort: &RouteCohort,
    build: &BuildRecord,
    lot_by_id: &BTreeMap<&str, &FeedstockLot>,
    valid_lot_ids: &BTreeSet<&str>,
    blockers: &mut Vec<Blocker>,
) -> bool {
    let mut valid = true;
    if build.build_id.trim().is_empty()
        || build.material_lot_id.trim().is_empty()
        || build.route_revision.trim().is_empty()
        || build.machine_tool_revision.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
        valid = false;
    }
    if !valid_sha256(&build.recipe_digest)
        || !valid_sha256(&build.build_record_digest)
        || !valid_sha256(&build.genealogy_digest)
    {
        blockers.push(Blocker::InvalidDigest);
        valid = false;
    }
    if build.route_revision != cohort.route_revision {
        blockers.push(Blocker::BuildRouteRevisionMismatch);
        valid = false;
    }
    if build.machine_tool_revision != cohort.machine_tool_revision {
        blockers.push(Blocker::BuildMachineRevisionMismatch);
        valid = false;
    }
    if build.recipe_digest != cohort.process_recipe_digest {
        blockers.push(Blocker::BuildRecipeMismatch);
        valid = false;
    }
    let Some(lot) = lot_by_id.get(build.material_lot_id.as_str()) else {
        blockers.push(Blocker::MissingLotRecord);
        return false;
    };
    if !valid_lot_ids.contains(build.material_lot_id.as_str()) {
        blockers.push(Blocker::MissingLotRecord);
        valid = false;
    }
    if lot.feedstock_family != cohort.feedstock_family
        || lot.material_grade != cohort.material_grade
    {
        blockers.push(Blocker::BuildLotMismatch);
        valid = false;
    }
    valid
}

fn validate_specimen_record(
    cohort: &RouteCohort,
    build: &BuildRecord,
    specimen: &SpecimenRecord,
    blockers: &mut Vec<Blocker>,
) -> bool {
    let mut valid = true;
    if specimen.specimen_id.trim().is_empty()
        || specimen.build_id.trim().is_empty()
        || specimen.material_lot_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
        valid = false;
    }
    if specimen.build_id != build.build_id {
        blockers.push(Blocker::SpecimenBuildMismatch);
        valid = false;
    }
    if specimen.material_lot_id != build.material_lot_id {
        blockers.push(Blocker::SpecimenLotMismatch);
        valid = false;
    }
    if specimen.scope != cohort.scope {
        blockers.push(Blocker::SpecimenScopeMismatch);
        valid = false;
    }

    match specimen.first_pass_disposition {
        SpecimenDisposition::Missing => {
            if specimen.inspection_completed
                || specimen.qualification_receipt_id.is_some()
                || specimen.qualification_receipt_digest.is_some()
            {
                blockers.push(Blocker::InvalidInspectionState);
                valid = false;
            }
        }
        _ => {
            if !specimen.inspection_completed
                || specimen
                    .qualification_receipt_id
                    .as_ref()
                    .is_none_or(|id| id.trim().is_empty())
                || specimen
                    .qualification_receipt_digest
                    .as_ref()
                    .is_none_or(|digest| !valid_sha256(digest))
            {
                blockers.push(Blocker::InvalidQualificationReceipt);
                valid = false;
            }
        }
    }

    if !specimen.manufacturing_complete && specimen.inspection_completed {
        blockers.push(Blocker::InvalidManufacturingState);
        valid = false;
    }
    if specimen.manufacturing_complete && build.status == BuildStatus::FailedBeforeInspection {
        blockers.push(Blocker::FailedBuildHasCompletedSpecimen);
        valid = false;
    }
    if specimen.rework_attempts == 0 {
        if specimen.rework_receipt_id.is_some() || specimen.rework_receipt_digest.is_some() {
            blockers.push(Blocker::InvalidReworkReceipt);
            valid = false;
        }
        if specimen.final_disposition != specimen.first_pass_disposition {
            blockers.push(Blocker::InvalidReworkState);
            valid = false;
        }
    } else {
        if specimen.first_pass_disposition == SpecimenDisposition::Qualified
            || specimen.first_pass_disposition == SpecimenDisposition::Missing
            || specimen
                .rework_receipt_id
                .as_ref()
                .is_none_or(|id| id.trim().is_empty())
            || specimen
                .rework_receipt_digest
                .as_ref()
                .is_none_or(|digest| !valid_sha256(digest))
            || specimen.final_disposition == SpecimenDisposition::Missing
        {
            blockers.push(Blocker::InvalidReworkState);
            valid = false;
        }
    }

    if specimen.scrapped {
        if specimen
            .scrap_receipt_id
            .as_ref()
            .is_none_or(|id| id.trim().is_empty())
            || specimen
                .scrap_receipt_digest
                .as_ref()
                .is_none_or(|digest| !valid_sha256(digest))
        {
            blockers.push(Blocker::InvalidScrapReceipt);
            valid = false;
        }
    } else if specimen.scrap_receipt_id.is_some() || specimen.scrap_receipt_digest.is_some() {
        blockers.push(Blocker::InvalidScrapReceipt);
        valid = false;
    }

    valid
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn scope() -> CohortScope {
        CohortScope {
            application_id: "ENCODER-80MM-001".to_string(),
            target_function_digest: digest('a'),
            measurement_setup_digest: digest('b'),
            acceptance_policy_digest: digest('c'),
        }
    }

    fn lot(id: &str, ch: char) -> FeedstockLot {
        FeedstockLot {
            lot_id: id.to_string(),
            feedstock_family: "BONDED-NDFEB-PA12".to_string(),
            material_grade: "NDFEB-GRADE-X".to_string(),
            recycled_mass_fraction: 0.75,
            lot_record_digest: digest(ch),
            source_provenance_digest: digest('d'),
        }
    }

    fn build(id: &str, lot_id: &str, status: BuildStatus, specimens: &[&str]) -> BuildRecord {
        BuildRecord {
            build_id: id.to_string(),
            material_lot_id: lot_id.to_string(),
            route_revision: "MEX-v1".to_string(),
            machine_tool_revision: "MACHINE-v1".to_string(),
            recipe_digest: digest('e'),
            build_record_digest: digest('f'),
            genealogy_digest: digest('1'),
            status,
            initiated_specimen_ids: specimens.iter().map(|id| (*id).to_string()).collect(),
        }
    }

    fn specimen(
        id: &str,
        build_id: &str,
        lot_id: &str,
        first: SpecimenDisposition,
        final_disposition: SpecimenDisposition,
        rework_attempts: u32,
        scrapped: bool,
    ) -> SpecimenRecord {
        let has_inspection = first != SpecimenDisposition::Missing;
        SpecimenRecord {
            specimen_id: id.to_string(),
            build_id: build_id.to_string(),
            material_lot_id: lot_id.to_string(),
            scope: scope(),
            manufacturing_complete: has_inspection,
            inspection_completed: has_inspection,
            first_pass_disposition: first,
            qualification_receipt_id: has_inspection.then(|| format!("QUAL-{id}")),
            qualification_receipt_digest: has_inspection.then(|| digest('2')),
            rework_attempts,
            rework_receipt_id: (rework_attempts > 0).then(|| format!("REWORK-{id}")),
            rework_receipt_digest: (rework_attempts > 0).then(|| digest('3')),
            final_disposition,
            scrapped,
            scrap_receipt_id: scrapped.then(|| format!("SCRAP-{id}")),
            scrap_receipt_digest: scrapped.then(|| digest('4')),
        }
    }

    fn valid_cohort() -> RouteCohort {
        RouteCohort {
            cohort_id: "MEX-COHORT-001".to_string(),
            route_family: RouteFamily::Mex,
            route_revision: "MEX-v1".to_string(),
            machine_tool_revision: "MACHINE-v1".to_string(),
            process_recipe_digest: digest('e'),
            feedstock_family: "BONDED-NDFEB-PA12".to_string(),
            material_grade: "NDFEB-GRADE-X".to_string(),
            scope: scope(),
            replication_policy: ReplicationPolicy {
                minimum_independent_builds: 3,
                minimum_distinct_material_lots: 2,
                policy_id: "REPLICATION-POLICY-v1".to_string(),
                policy_digest: digest('5'),
            },
            lots: vec![lot("LOT-1", '6'), lot("LOT-2", '7')],
            builds: vec![
                build(
                    "BUILD-1",
                    "LOT-1",
                    BuildStatus::Completed,
                    &["SPEC-1", "SPEC-2"],
                ),
                build(
                    "BUILD-2",
                    "LOT-2",
                    BuildStatus::FailedBeforeInspection,
                    &["SPEC-3"],
                ),
            ],
            specimens: vec![
                specimen(
                    "SPEC-1",
                    "BUILD-1",
                    "LOT-1",
                    SpecimenDisposition::Qualified,
                    SpecimenDisposition::Qualified,
                    0,
                    false,
                ),
                specimen(
                    "SPEC-2",
                    "BUILD-1",
                    "LOT-1",
                    SpecimenDisposition::Fail,
                    SpecimenDisposition::Qualified,
                    1,
                    false,
                ),
                specimen(
                    "SPEC-3",
                    "BUILD-2",
                    "LOT-2",
                    SpecimenDisposition::Missing,
                    SpecimenDisposition::Missing,
                    0,
                    false,
                ),
            ],
            campaign_cost: Some(CampaignCostReceipt {
                receipt_id: "COST-001".to_string(),
                digest: digest('8'),
                currency: "USD".to_string(),
                total_campaign_cost_minor: 10_000,
            }),
        }
    }

    #[test]
    fn denominator_includes_failed_missing_specimens_and_rework_does_not_change_first_pass() {
        let result = evaluate_cohort(&valid_cohort());
        assert!(result.structurally_valid());
        assert_eq!(result.summary.counts.initiated_specimens, 3);
        assert_eq!(result.summary.counts.first_pass_qualified, 1);
        assert_eq!(result.summary.counts.first_pass_fail, 1);
        assert_eq!(result.summary.counts.missing_specimen_outcomes, 1);
        assert_eq!(result.summary.first_pass_acceptance, Rate::new(1, 3));
        assert_eq!(result.summary.unresolved, Rate::new(1, 3));
        assert_eq!(result.summary.reworked_specimens, 1);
        assert_eq!(result.summary.rework_recovered, 1);
        assert_eq!(result.summary.rework_recovery, Rate::new(1, 1));
        assert_eq!(result.summary.final_accepted_fraction, Rate::new(2, 3));
    }

    #[test]
    fn multiple_parts_in_one_build_count_as_one_build_replication() {
        let result = evaluate_cohort(&valid_cohort());
        assert_eq!(result.summary.counts.initiated_builds, 2);
        assert_eq!(result.summary.counts.replication_eligible_builds, 2);
        assert_eq!(result.summary.counts.distinct_material_lots, 2);
        assert_eq!(
            result.summary.generalization_status,
            GeneralizationStatus::GeneralizationUnavailable
        );
        assert!(!result.summary.inferential_yield_claim_available);
    }

    #[test]
    fn cost_per_accepted_part_preserves_campaign_numerator_and_output_denominator() {
        let result = evaluate_cohort(&valid_cohort());
        let cost = result.summary.cost_per_accepted_part.expect("cost receipt");
        assert_eq!(cost.total_campaign_cost_minor, 10_000);
        assert_eq!(cost.accepted_part_count, 2);
        assert_eq!(cost.currency, "USD");
    }

    #[test]
    fn build_count_not_specimen_count_drives_replication_policy() {
        let mut cohort = valid_cohort();
        cohort.replication_policy.minimum_independent_builds = 1;
        assert_eq!(
            evaluate_cohort(&cohort).summary.generalization_status,
            GeneralizationStatus::ReplicationCoverageMeetsPolicy
        );
    }

    #[test]
    fn missing_specimen_record_is_kept_in_denominator() {
        let mut cohort = valid_cohort();
        cohort
            .specimens
            .retain(|record| record.specimen_id != "SPEC-3");
        let result = evaluate_cohort(&cohort);
        assert!(
            result
                .blockers
                .contains(&Blocker::InitiatedSpecimenMissingRecord)
        );
        assert_eq!(result.summary.counts.initiated_specimens, 3);
        assert_eq!(result.summary.counts.missing_specimen_outcomes, 1);
        assert_eq!(result.summary.first_pass_acceptance, Rate::new(1, 3));
        assert_eq!(
            result.summary.generalization_status,
            GeneralizationStatus::GeneralizationUnavailable
        );
    }

    #[test]
    fn orphan_specimen_cannot_inflate_route_denominator_or_yield() {
        let mut cohort = valid_cohort();
        let orphan = specimen(
            "SPEC-ORPHAN",
            "BUILD-1",
            "LOT-1",
            SpecimenDisposition::Qualified,
            SpecimenDisposition::Qualified,
            0,
            false,
        );
        cohort.specimens.push(orphan);
        let result = evaluate_cohort(&cohort);
        assert!(result.blockers.contains(&Blocker::OrphanSpecimenRecord));
        assert_eq!(result.summary.counts.initiated_specimens, 3);
        assert_eq!(result.summary.first_pass_acceptance, Rate::new(1, 3));
    }

    #[test]
    fn duplicated_specimen_records_are_counted_as_missing_not_twice() {
        let mut cohort = valid_cohort();
        cohort.specimens.push(cohort.specimens[0].clone());
        let result = evaluate_cohort(&cohort);
        assert!(result.blockers.contains(&Blocker::DuplicateSpecimenId));
        assert_eq!(result.summary.counts.initiated_specimens, 3);
        assert_eq!(result.summary.counts.missing_specimen_outcomes, 1);
    }

    #[test]
    fn duplicate_specimen_ids_in_two_build_manifests_are_rejected() {
        let mut cohort = valid_cohort();
        cohort.builds[1]
            .initiated_specimen_ids
            .push("SPEC-1".to_string());
        let result = evaluate_cohort(&cohort);
        assert!(
            result
                .blockers
                .contains(&Blocker::DuplicateInitiatedSpecimenId)
        );
    }

    #[test]
    fn route_revision_mismatch_cannot_be_pooled() {
        let mut cohort = valid_cohort();
        cohort.builds[0].route_revision = "MEX-v2".to_string();
        let result = evaluate_cohort(&cohort);
        assert!(
            result
                .blockers
                .contains(&Blocker::BuildRouteRevisionMismatch)
        );
    }

    #[test]
    fn feedstock_lot_grade_mismatch_cannot_be_silently_pooled() {
        let mut cohort = valid_cohort();
        cohort.lots[1].material_grade = "NDFEB-GRADE-Y".to_string();
        let result = evaluate_cohort(&cohort);
        assert!(result.blockers.contains(&Blocker::BuildLotMismatch));
    }

    #[test]
    fn rework_without_receipt_is_rejected() {
        let mut cohort = valid_cohort();
        cohort.specimens[1].rework_receipt_digest = None;
        let result = evaluate_cohort(&cohort);
        assert!(result.blockers.contains(&Blocker::InvalidReworkState));
    }

    #[test]
    fn rework_cannot_inflate_first_pass_accepted_fraction() {
        let mut cohort = valid_cohort();
        cohort.specimens[1].final_disposition = SpecimenDisposition::Qualified;
        let result = evaluate_cohort(&cohort);
        assert_eq!(result.summary.first_pass_acceptance, Rate::new(1, 3));
        assert_eq!(result.summary.final_accepted_fraction, Rate::new(2, 3));
    }

    #[test]
    fn scrapped_specimen_is_not_counted_as_final_accepted() {
        let mut cohort = valid_cohort();
        cohort.specimens[0].scrapped = true;
        cohort.specimens[0].scrap_receipt_id = Some("SCRAP-SPEC-1".to_string());
        cohort.specimens[0].scrap_receipt_digest = Some(digest('9'));
        let result = evaluate_cohort(&cohort);
        assert_eq!(result.summary.counts.first_pass_qualified, 1);
        assert_eq!(result.summary.counts.scrapped_specimens, 1);
        assert_eq!(result.summary.counts.final_accepted_specimens, 1);
    }

    #[test]
    fn insufficient_lot_replication_keeps_generalization_unavailable() {
        let mut cohort = valid_cohort();
        cohort.lots.truncate(1);
        cohort.builds.truncate(1);
        cohort.specimens.truncate(2);
        cohort.replication_policy.minimum_independent_builds = 2;
        cohort.replication_policy.minimum_distinct_material_lots = 2;
        let result = evaluate_cohort(&cohort);
        assert_eq!(
            result.summary.generalization_status,
            GeneralizationStatus::GeneralizationUnavailable
        );
    }

    #[test]
    fn inferential_yield_is_never_claimed_by_descriptive_evaluator() {
        let mut cohort = valid_cohort();
        cohort.replication_policy.minimum_independent_builds = 1;
        let result = evaluate_cohort(&cohort);
        assert_eq!(
            result.summary.generalization_status,
            GeneralizationStatus::ReplicationCoverageMeetsPolicy
        );
        assert!(!result.summary.inferential_yield_claim_available);
    }

    #[test]
    fn first_pass_missing_record_cannot_claim_inspection_completion() {
        let mut cohort = valid_cohort();
        cohort.specimens[2].first_pass_disposition = SpecimenDisposition::Missing;
        cohort.specimens[2].inspection_completed = true;
        let result = evaluate_cohort(&cohort);
        assert!(result.blockers.contains(&Blocker::InvalidInspectionState));
    }

    #[test]
    fn failed_before_inspection_build_cannot_contain_completed_part() {
        let mut cohort = valid_cohort();
        cohort.specimens[2].manufacturing_complete = true;
        let result = evaluate_cohort(&cohort);
        assert!(
            result
                .blockers
                .contains(&Blocker::FailedBuildHasInspectedSpecimen)
        );
    }

    #[test]
    fn unused_material_lot_does_not_count_toward_replication() {
        let mut cohort = valid_cohort();
        cohort.lots.push(lot("LOT-UNUSED", '9'));
        cohort.replication_policy.minimum_distinct_material_lots = 3;
        let result = evaluate_cohort(&cohort);
        assert_eq!(result.summary.counts.distinct_material_lots, 2);
        assert_eq!(
            result.summary.generalization_status,
            GeneralizationStatus::GeneralizationUnavailable
        );
    }

    #[test]
    fn duplicate_specimen_receipt_ids_invalidate_cohort() {
        let mut cohort = valid_cohort();
        cohort.specimens[1].qualification_receipt_id =
            cohort.specimens[0].qualification_receipt_id.clone();
        let result = evaluate_cohort(&cohort);
        assert!(result
            .blockers
            .contains(&Blocker::DuplicateQualificationReceiptId));
        assert_eq!(
            result.summary.generalization_status,
            GeneralizationStatus::GeneralizationUnavailable
        );
    }

    #[test]
    fn invalid_recycled_fraction_is_rejected() {
        let mut cohort = valid_cohort();
        cohort.lots[0].recycled_mass_fraction = 1.2;
        let result = evaluate_cohort(&cohort);
        assert!(result.blockers.contains(&Blocker::InvalidRecycledFraction));
    }

    #[test]
    fn missing_cost_receipt_does_not_invent_cost_per_accepted_part() {
        let mut cohort = valid_cohort();
        cohort.campaign_cost = None;
        let result = evaluate_cohort(&cohort);
        assert!(result.summary.cost_per_accepted_part.is_none());
        assert!(result.structurally_valid());
    }
}
