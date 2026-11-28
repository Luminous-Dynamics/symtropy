//! Paired virgin/recovered feedstock cohort comparison.
//!
//! This evaluator checks finite, preregistered matched manufacturing blocks.
//! It is not a process solver, statistical population model, M5 qualification,
//! life-cycle assessment, or manufacturing authority.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessRoute {
    PowderExtrusionMouldingSintering,
    MaterialExtrusionBonded,
    LaserPowderBedFusion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialCondition {
    Virgin,
    Recovered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomizedOrder {
    VirginFirst,
    RecoveredFirst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildDisposition {
    Qualified,
    Fail,
    MissingInspection,
    NotEvaluable,
    Scrapped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    PhysicalMeasurement,
    ProductionRecord,
    PublishedLiterature,
    Simulation,
    SyntheticControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Minimize,
    Maximize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    NotComparable,
    Fail,
    Indeterminate,
    InferiorToVirgin,
    RecoveredRouteNonInferior,
    RecoveredRouteAdvantage,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessScope {
    pub application_id: String,
    pub material_family_id: String,
    pub feedstock_form_id: String,
    pub route: ProcessRoute,
    pub recipe_digest: String,
    pub process_revision_digest: String,
    pub geometry_digest: String,
    pub orientation_digest: String,
    pub magnetization_digest: String,
    pub measurement_setup_digest: String,
    pub acceptance_policy_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricSpec {
    pub metric_id: String,
    pub unit: String,
    pub direction: Direction,
    pub hard_min: Option<f64>,
    pub hard_max: Option<f64>,
    pub noninferiority_margin: f64,
    pub improvement_margin: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CohortSpec {
    pub campaign_id: String,
    pub scope: ProcessScope,
    pub metrics: Vec<MetricSpec>,
    pub minimum_independent_lot_pairs: u32,
    pub minimum_build_pairs_per_lot: u32,
    pub first_pass_yield_noninferiority_margin: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceLotPair {
    pub lot_pair_id: String,
    pub virgin_source_lot_id: String,
    pub recovered_source_lot_id: String,
    pub virgin_eligibility_receipt_id: String,
    pub virgin_eligibility_digest: String,
    pub recovered_eligibility_receipt_id: String,
    pub recovered_eligibility_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairBlock {
    pub pair_block_id: String,
    pub lot_pair_id: String,
    pub randomized_order: RandomizedOrder,
    pub randomization_receipt_id: String,
    pub randomization_digest: String,
    pub seed_commitment_digest: String,
    pub virgin_build_id: String,
    pub recovered_build_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricObservation {
    pub metric_id: String,
    pub unit: String,
    pub value: Interval,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub method_id: String,
    pub calibration_digest: String,
    pub uncertainty_record_digest: String,
    pub evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuildEvidence {
    pub build_id: String,
    pub pair_block_id: String,
    pub condition: MaterialCondition,
    pub source_lot_id: String,
    pub specimen_id: String,
    pub blind_sample_id: String,
    pub scope: ProcessScope,
    pub first_pass_disposition: BuildDisposition,
    pub final_disposition: BuildDisposition,
    pub rework_count: u32,
    pub rework_receipt_id: Option<String>,
    pub rework_receipt_digest: Option<String>,
    pub build_receipt_id: String,
    pub build_receipt_digest: String,
    pub evidence_class: EvidenceClass,
    pub observations: Vec<MetricObservation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairedCohort {
    /// The initiated-build manifest is the denominator source.
    pub initiated_build_ids: Vec<String>,
    pub source_lot_pairs: Vec<SourceLotPair>,
    pub pair_blocks: Vec<PairBlock>,
    /// Must contain one record for every initiated build, including failures,
    /// rework, missing inspections and scrap.
    pub builds: Vec<BuildEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    InvalidSpec,
    InvalidDigest,
    MissingIdentity,
    DuplicateMetricSpec,
    DuplicateLotPair,
    DuplicateSourceLot,
    DuplicatePairBlock,
    DuplicateBuildIdentity,
    DuplicateBlindSample,
    MissingSourceLotPair,
    InsufficientIndependentLotPairs,
    InsufficientBuildPairsPerLot,
    MissingPairBlock,
    DuplicateInitiatedBuild,
    MissingBuildEvidence,
    ExtraBuildEvidence,
    UnpairedBuild,
    PairBlockBuildMismatch,
    PairBlockLotMismatch,
    RandomizationReceiptMissing,
    InvalidRandomizationReceipt,
    EligibilityReceiptMissing,
    InvalidEligibilityReceipt,
    ScopeMismatch,
    SourceLotMismatch,
    WrongCondition,
    NonPhysicalBuildEvidence,
    MissingBuildReceipt,
    MissingReworkReceipt,
    ReworkReceiptUnexpected,
    MissingMetricObservation,
    DuplicateMetricObservation,
    ExtraMetricObservation,
    InvalidMetricInterval,
    NonPhysicalMetricEvidence,
    MissingMeasurementMethod,
    InvalidMeasurementLineage,
    VirginBaselineHardFailure,
    RecoveredHardFailure,
    RecoveredHardLimitUnresolved,
    VirginHardLimitUnresolved,
    NoninferiorityViolation,
    FirstPassYieldInferior,
    FirstPassYieldUnresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairMetricComparison {
    pub metric_id: String,
    pub virgin_interval: Interval,
    pub recovered_interval: Interval,
    pub noninferior: bool,
    pub robustly_improved: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairBlockEvaluation {
    pub pair_block_id: String,
    pub virgin_build_id: String,
    pub recovered_build_id: String,
    pub metrics: Vec<PairMetricComparison>,
    pub noninferior: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct YieldSummary {
    pub virgin_initiated: u32,
    pub recovered_initiated: u32,
    pub virgin_first_pass_qualified: u32,
    pub recovered_first_pass_qualified: u32,
    pub virgin_first_pass_yield: f64,
    pub recovered_first_pass_yield: f64,
    pub virgin_final_qualified: u32,
    pub recovered_final_qualified: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CohortEvaluation {
    pub campaign_id: String,
    pub decision: Decision,
    pub yield_summary: Option<YieldSummary>,
    pub pair_evaluations: Vec<PairBlockEvaluation>,
    pub robustly_improved_metrics: Vec<String>,
    pub blockers: Vec<Blocker>,
    pub claim_ceiling: String,
}

pub fn evaluate_paired_cohort(
    spec: &CohortSpec,
    cohort: &PairedCohort,
) -> CohortEvaluation {
    let mut global_blockers = validate_spec(spec);
    validate_manifest_and_pairs(spec, cohort, &mut global_blockers);

    let builds_by_id = cohort
        .builds
        .iter()
        .map(|build| (build.build_id.as_str(), build))
        .collect::<BTreeMap<_, _>>();

    let lot_pairs_by_id = cohort
        .source_lot_pairs
        .iter()
        .map(|pair| (pair.lot_pair_id.as_str(), pair))
        .collect::<BTreeMap<_, _>>();

    let mut pair_evaluations = Vec::new();
    let mut observed_yield_inferior = false;
    let mut definitely_inferior = false;
    let mut unresolved_pair = false;
    let mut all_pairs_noninferior = !cohort.pair_blocks.is_empty();
    let mut improvement_intersection: Option<BTreeSet<String>> = None;

    for block in &cohort.pair_blocks {
        let (Some(lot_pair), Some(virgin), Some(recovered)) = (
            lot_pairs_by_id.get(block.lot_pair_id.as_str()).copied(),
            builds_by_id.get(block.virgin_build_id.as_str()).copied(),
            builds_by_id.get(block.recovered_build_id.as_str()).copied(),
        ) else {
            unresolved_pair = true;
            all_pairs_noninferior = false;
            continue;
        };

        let mut pair_blockers = Vec::new();
        validate_build(
            spec,
            block,
            lot_pair,
            virgin,
            MaterialCondition::Virgin,
            &mut pair_blockers,
        );
        validate_build(
            spec,
            block,
            lot_pair,
            recovered,
            MaterialCondition::Recovered,
            &mut pair_blockers,
        );

        if virgin.final_disposition != BuildDisposition::Qualified
            || recovered.final_disposition != BuildDisposition::Qualified
        {
            unresolved_pair = true;
            all_pairs_noninferior = false;
            global_blockers.extend(pair_blockers);
            continue;
        }

        let virgin_values = validate_observations(spec, virgin, &mut pair_blockers);
        let recovered_values = validate_observations(spec, recovered, &mut pair_blockers);

        let mut comparisons = Vec::new();
        let mut pair_noninferior = true;
        let mut pair_improvements = BTreeSet::new();

        for metric in &spec.metrics {
            let (Some(virgin_interval), Some(recovered_interval)) = (
                virgin_values.get(metric.metric_id.as_str()).copied(),
                recovered_values.get(metric.metric_id.as_str()).copied(),
            ) else {
                pair_noninferior = false;
                unresolved_pair = true;
                pair_blockers.push(Blocker::MissingMetricObservation);
                continue;
            };

            let noninferior = match metric.direction {
                Direction::Maximize => {
                    recovered_interval.lower
                        >= virgin_interval.upper - metric.noninferiority_margin
                }
                Direction::Minimize => {
                    recovered_interval.upper
                        <= virgin_interval.lower + metric.noninferiority_margin
                }
            };
            if !noninferior {
                pair_noninferior = false;
                let definitely_worse = match metric.direction {
                    Direction::Maximize => {
                        recovered_interval.upper
                            < virgin_interval.lower - metric.noninferiority_margin
                    }
                    Direction::Minimize => {
                        recovered_interval.lower
                            > virgin_interval.upper + metric.noninferiority_margin
                    }
                };
                if definitely_worse {
                    definitely_inferior = true;
                    pair_blockers.push(Blocker::NoninferiorityViolation);
                } else {
                    unresolved_pair = true;
                }
            }

            let improved = match metric.direction {
                Direction::Maximize => {
                    recovered_interval.lower
                        > virgin_interval.upper + metric.improvement_margin
                }
                Direction::Minimize => {
                    recovered_interval.upper
                        < virgin_interval.lower - metric.improvement_margin
                }
            };
            if improved {
                pair_improvements.insert(metric.metric_id.clone());
            }
            comparisons.push(PairMetricComparison {
                metric_id: metric.metric_id.clone(),
                virgin_interval,
                recovered_interval,
                noninferior,
                robustly_improved: improved,
            });
        }

        all_pairs_noninferior &= pair_noninferior;
        improvement_intersection = Some(match improvement_intersection.take() {
            Some(previous) => previous
                .intersection(&pair_improvements)
                .cloned()
                .collect(),
            None => pair_improvements,
        });

        if !pair_blockers.is_empty() {
            global_blockers.extend(pair_blockers);
        }
        pair_evaluations.push(PairBlockEvaluation {
            pair_block_id: block.pair_block_id.clone(),
            virgin_build_id: virgin.build_id.clone(),
            recovered_build_id: recovered.build_id.clone(),
            metrics: comparisons,
            noninferior: pair_noninferior,
        });
    }

    let yield_summary = calculate_yield_summary(cohort);
    if let Some(summary) = &yield_summary {
        if summary.recovered_first_pass_yield
            + spec.first_pass_yield_noninferiority_margin
            < summary.virgin_first_pass_yield
        {
            observed_yield_inferior = true;
            global_blockers.push(Blocker::FirstPassYieldInferior);
        }
    }

    let improvements = improvement_intersection
        .unwrap_or_default()
        .into_iter()
        .collect::<Vec<_>>();

    let not_comparable = global_blockers.iter().any(is_not_comparable_blocker);
    let decision = if not_comparable {
        Decision::NotComparable
    } else if global_blockers.contains(&Blocker::RecoveredHardFailure) {
        Decision::Fail
    } else if observed_yield_inferior || definitely_inferior {
        Decision::InferiorToVirgin
    } else if unresolved_pair
        || !all_pairs_noninferior
        || global_blockers.iter().any(is_evidence_blocker)
    {
        Decision::Indeterminate
    } else if global_blockers.contains(&Blocker::NoninferiorityViolation) {
        Decision::InferiorToVirgin
    } else if !improvements.is_empty() {
        Decision::RecoveredRouteAdvantage
    } else {
        Decision::RecoveredRouteNonInferior
    };

    global_blockers.sort_by_key(|blocker| format!("{blocker:?}"));
    global_blockers.dedup();

    CohortEvaluation {
        campaign_id: spec.campaign_id.clone(),
        decision,
        yield_summary,
        pair_evaluations,
        robustly_improved_metrics: improvements,
        blockers: global_blockers,
        claim_ceiling: format!(
            "{decision:?}: finite paired cohort result only; no M5 process capability, population generalization, LCA, certification or production-release claim"
        ),
    }
}

fn validate_spec(spec: &CohortSpec) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if spec.campaign_id.trim().is_empty()
        || spec.scope.application_id.trim().is_empty()
        || spec.scope.material_family_id.trim().is_empty()
        || spec.scope.feedstock_form_id.trim().is_empty()
        || spec.metrics.is_empty()
        || spec.minimum_independent_lot_pairs < 3
        || spec.minimum_build_pairs_per_lot < 3
        || !spec.first_pass_yield_noninferiority_margin.is_finite()
        || spec.first_pass_yield_noninferiority_margin < 0.0
    {
        blockers.push(Blocker::InvalidSpec);
    }
    for digest in [
        &spec.scope.recipe_digest,
        &spec.scope.process_revision_digest,
        &spec.scope.geometry_digest,
        &spec.scope.orientation_digest,
        &spec.scope.magnetization_digest,
        &spec.scope.measurement_setup_digest,
        &spec.scope.acceptance_policy_digest,
    ] {
        if !valid_digest(digest) {
            blockers.push(Blocker::InvalidDigest);
        }
    }
    let mut metric_ids = BTreeSet::new();
    for metric in &spec.metrics {
        if metric.metric_id.trim().is_empty()
            || metric.unit.trim().is_empty()
            || !metric_ids.insert(metric.metric_id.as_str())
            || !metric.noninferiority_margin.is_finite()
            || metric.noninferiority_margin < 0.0
            || !metric.improvement_margin.is_finite()
            || metric.improvement_margin < 0.0
            || metric.hard_min.is_some_and(|x| !x.is_finite())
            || metric.hard_max.is_some_and(|x| !x.is_finite())
            || metric.hard_min.zip(metric.hard_max).is_some_and(|(a, b)| a > b)
        {
            blockers.push(Blocker::DuplicateMetricSpec);
        }
    }
    blockers
}

fn validate_manifest_and_pairs(
    spec: &CohortSpec,
    cohort: &PairedCohort,
    blockers: &mut Vec<Blocker>,
) {
    if cohort.source_lot_pairs.is_empty() {
        blockers.push(Blocker::MissingSourceLotPair);
    }
    if cohort.source_lot_pairs.len() < spec.minimum_independent_lot_pairs as usize {
        blockers.push(Blocker::InsufficientIndependentLotPairs);
    }
    let mut lot_pair_ids = BTreeSet::new();
    let mut source_lots = BTreeSet::new();
    for pair in &cohort.source_lot_pairs {
        if pair.lot_pair_id.trim().is_empty()
            || !lot_pair_ids.insert(pair.lot_pair_id.as_str())
        {
            blockers.push(Blocker::DuplicateLotPair);
        }
        if pair.virgin_source_lot_id.trim().is_empty()
            || pair.recovered_source_lot_id.trim().is_empty()
            || pair.virgin_source_lot_id == pair.recovered_source_lot_id
        {
            blockers.push(Blocker::MissingIdentity);
        }
        if !source_lots.insert(pair.virgin_source_lot_id.as_str())
            || !source_lots.insert(pair.recovered_source_lot_id.as_str())
        {
            blockers.push(Blocker::DuplicateSourceLot);
        }
        if pair.virgin_eligibility_receipt_id.trim().is_empty()
            || pair.recovered_eligibility_receipt_id.trim().is_empty()
        {
            blockers.push(Blocker::EligibilityReceiptMissing);
        }
        if !valid_digest(&pair.virgin_eligibility_digest)
            || !valid_digest(&pair.recovered_eligibility_digest)
        {
            blockers.push(Blocker::InvalidEligibilityReceipt);
        }
    }

    let mut block_ids = BTreeSet::new();
    let mut paired_build_ids = BTreeSet::new();
    let mut block_counts: BTreeMap<&str, u32> = BTreeMap::new();
    for block in &cohort.pair_blocks {
        if block.pair_block_id.trim().is_empty()
            || !block_ids.insert(block.pair_block_id.as_str())
        {
            blockers.push(Blocker::DuplicatePairBlock);
        }
        if !lot_pair_ids.contains(block.lot_pair_id.as_str()) {
            blockers.push(Blocker::PairBlockLotMismatch);
        }
        *block_counts.entry(block.lot_pair_id.as_str()).or_default() += 1;
        if block.virgin_build_id == block.recovered_build_id
            || !paired_build_ids.insert(block.virgin_build_id.as_str())
            || !paired_build_ids.insert(block.recovered_build_id.as_str())
        {
            blockers.push(Blocker::PairBlockBuildMismatch);
        }
        if block.randomization_receipt_id.trim().is_empty() {
            blockers.push(Blocker::RandomizationReceiptMissing);
        }
        if !valid_digest(&block.randomization_digest)
            || !valid_digest(&block.seed_commitment_digest)
        {
            blockers.push(Blocker::InvalidRandomizationReceipt);
        }
    }

    for pair in &cohort.source_lot_pairs {
        if block_counts.get(pair.lot_pair_id.as_str()).copied().unwrap_or(0)
            < spec.minimum_build_pairs_per_lot
        {
            blockers.push(Blocker::InsufficientBuildPairsPerLot);
        }
    }

    let mut initiated = BTreeSet::new();
    for build_id in &cohort.initiated_build_ids {
        if build_id.trim().is_empty() || !initiated.insert(build_id.as_str()) {
            blockers.push(Blocker::DuplicateInitiatedBuild);
        }
    }
    if initiated != paired_build_ids {
        blockers.push(Blocker::UnpairedBuild);
    }

    let mut build_ids = BTreeSet::new();
    let mut blind_ids = BTreeSet::new();
    for build in &cohort.builds {
        if !build_ids.insert(build.build_id.as_str()) {
            blockers.push(Blocker::DuplicateBuildIdentity);
        }
        if !blind_ids.insert(build.blind_sample_id.as_str()) {
            blockers.push(Blocker::DuplicateBlindSample);
        }
    }
    if build_ids.len() < initiated.len() {
        blockers.push(Blocker::MissingBuildEvidence);
    }
    if build_ids.len() > initiated.len() || !build_ids.iter().all(|id| initiated.contains(id)) {
        blockers.push(Blocker::ExtraBuildEvidence);
    }
}

fn validate_build(
    spec: &CohortSpec,
    block: &PairBlock,
    lot_pair: &SourceLotPair,
    build: &BuildEvidence,
    expected_condition: MaterialCondition,
    blockers: &mut Vec<Blocker>,
) {
    if build.build_id.trim().is_empty()
        || build.specimen_id.trim().is_empty()
        || build.blind_sample_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }
    if build.pair_block_id != block.pair_block_id {
        blockers.push(Blocker::PairBlockBuildMismatch);
    }
    if build.scope != spec.scope {
        blockers.push(Blocker::ScopeMismatch);
    }
    let expected_lot = match expected_condition {
        MaterialCondition::Virgin => &lot_pair.virgin_source_lot_id,
        MaterialCondition::Recovered => &lot_pair.recovered_source_lot_id,
    };
    if &build.source_lot_id != expected_lot {
        blockers.push(Blocker::SourceLotMismatch);
    }
    if build.condition != expected_condition {
        blockers.push(Blocker::WrongCondition);
    }
    if build.evidence_class != EvidenceClass::PhysicalMeasurement {
        blockers.push(Blocker::NonPhysicalBuildEvidence);
    }
    if build.build_receipt_id.trim().is_empty() {
        blockers.push(Blocker::MissingBuildReceipt);
    }
    if !valid_digest(&build.build_receipt_digest) {
        blockers.push(Blocker::InvalidDigest);
    }
    if build.rework_count == 0 {
        if build.rework_receipt_id.is_some() || build.rework_receipt_digest.is_some() {
            blockers.push(Blocker::ReworkReceiptUnexpected);
        }
    } else if build.rework_receipt_id.as_deref().is_none_or(str::is_empty)
        || build.rework_receipt_digest.as_deref().is_none_or(|x| !valid_digest(x))
    {
        blockers.push(Blocker::MissingReworkReceipt);
    }
}

fn validate_observations(
    spec: &CohortSpec,
    build: &BuildEvidence,
    blockers: &mut Vec<Blocker>,
) -> BTreeMap<String, Interval> {
    let mut by_metric = BTreeMap::new();
    for observation in &build.observations {
        if by_metric
            .insert(observation.metric_id.as_str(), observation)
            .is_some()
        {
            blockers.push(Blocker::DuplicateMetricObservation);
        }
        if !valid_digest(&observation.evidence_digest)
            || !valid_digest(&observation.calibration_digest)
            || !valid_digest(&observation.uncertainty_record_digest)
        {
            blockers.push(Blocker::InvalidMeasurementLineage);
        }
        if observation.evidence_id.trim().is_empty() || observation.method_id.trim().is_empty() {
            blockers.push(Blocker::MissingMeasurementMethod);
        }
        if observation.evidence_class != EvidenceClass::PhysicalMeasurement {
            blockers.push(Blocker::NonPhysicalMetricEvidence);
        }
        if !observation.value.is_valid() {
            blockers.push(Blocker::InvalidMetricInterval);
        }
    }

    let mut values = BTreeMap::new();
    for metric in &spec.metrics {
        let Some(observation) = by_metric.get(metric.metric_id.as_str()) else {
            blockers.push(Blocker::MissingMetricObservation);
            continue;
        };
        if observation.unit != metric.unit {
            blockers.push(Blocker::InvalidMetricInterval);
            continue;
        }
        let interval = observation.value;
        if metric.hard_min.is_some_and(|minimum| interval.upper < minimum)
            || metric.hard_max.is_some_and(|maximum| interval.lower > maximum)
        {
            let blocker = if build.condition == MaterialCondition::Recovered {
                Blocker::RecoveredHardFailure
            } else {
                Blocker::VirginBaselineHardFailure
            };
            blockers.push(blocker);
        } else if metric.hard_min.is_some_and(|minimum| interval.lower < minimum)
            || metric.hard_max.is_some_and(|maximum| interval.upper > maximum)
        {
            let blocker = if build.condition == MaterialCondition::Recovered {
                Blocker::RecoveredHardLimitUnresolved
            } else {
                Blocker::VirginHardLimitUnresolved
            };
            blockers.push(blocker);
        }
        values.insert(metric.metric_id.clone(), interval);
    }
    for observation in &build.observations {
        if !spec
            .metrics
            .iter()
            .any(|metric| metric.metric_id == observation.metric_id)
        {
            blockers.push(Blocker::ExtraMetricObservation);
        }
    }
    values
}

fn calculate_yield_summary(cohort: &PairedCohort) -> Option<YieldSummary> {
    if cohort.pair_blocks.is_empty() {
        return None;
    }

    // Pair blocks are the denominator manifest, not the subset of build
    // receipts that happened to arrive.
    let virgin_build_ids = cohort
        .pair_blocks
        .iter()
        .map(|block| block.virgin_build_id.as_str())
        .collect::<BTreeSet<_>>();
    let recovered_build_ids = cohort
        .pair_blocks
        .iter()
        .map(|block| block.recovered_build_id.as_str())
        .collect::<BTreeSet<_>>();
    let builds_by_id = cohort
        .builds
        .iter()
        .map(|build| (build.build_id.as_str(), build))
        .collect::<BTreeMap<_, _>>();

    let virgin_first_pass = virgin_build_ids
        .iter()
        .filter(|id| {
            builds_by_id
                .get(*id)
                .is_some_and(|build| build.first_pass_disposition == BuildDisposition::Qualified)
        })
        .count() as u32;
    let recovered_first_pass = recovered_build_ids
        .iter()
        .filter(|id| {
            builds_by_id
                .get(*id)
                .is_some_and(|build| build.first_pass_disposition == BuildDisposition::Qualified)
        })
        .count() as u32;
    let virgin_final = virgin_build_ids
        .iter()
        .filter(|id| {
            builds_by_id
                .get(*id)
                .is_some_and(|build| build.final_disposition == BuildDisposition::Qualified)
        })
        .count() as u32;
    let recovered_final = recovered_build_ids
        .iter()
        .filter(|id| {
            builds_by_id
                .get(*id)
                .is_some_and(|build| build.final_disposition == BuildDisposition::Qualified)
        })
        .count() as u32;

    let virgin_initiated = cohort.pair_blocks.len() as u32;
    let recovered_initiated = cohort.pair_blocks.len() as u32;
    Some(YieldSummary {
        virgin_initiated,
        recovered_initiated,
        virgin_first_pass_qualified: virgin_first_pass,
        recovered_first_pass_qualified: recovered_first_pass,
        virgin_first_pass_yield: f64::from(virgin_first_pass) / f64::from(virgin_initiated),
        recovered_first_pass_yield: f64::from(recovered_first_pass) / f64::from(recovered_initiated),
        virgin_final_qualified: virgin_final,
        recovered_final_qualified: recovered_final,
    })
}

fn is_not_comparable_blocker(blocker: &Blocker) -> bool {
    matches!(
        blocker,
        Blocker::InvalidSpec
            | Blocker::InvalidDigest
            | Blocker::MissingIdentity
            | Blocker::DuplicateMetricSpec
            | Blocker::DuplicateLotPair
            | Blocker::DuplicateSourceLot
            | Blocker::MissingSourceLotPair
            | Blocker::DuplicatePairBlock
            | Blocker::DuplicateBuildIdentity
            | Blocker::DuplicateBlindSample
            | Blocker::PairBlockLotMismatch
            | Blocker::PairBlockBuildMismatch
            | Blocker::DuplicateInitiatedBuild
            | Blocker::UnpairedBuild
            | Blocker::ExtraBuildEvidence
            | Blocker::ScopeMismatch
            | Blocker::SourceLotMismatch
            | Blocker::WrongCondition
    )
}

fn is_evidence_blocker(blocker: &Blocker) -> bool {
    matches!(
        blocker,
        Blocker::MissingBuildEvidence
            | Blocker::InsufficientIndependentLotPairs
            | Blocker::InsufficientBuildPairsPerLot
            | Blocker::MissingPairBlock
            | Blocker::RandomizationReceiptMissing
            | Blocker::InvalidRandomizationReceipt
            | Blocker::EligibilityReceiptMissing
            | Blocker::InvalidEligibilityReceipt
            | Blocker::NonPhysicalBuildEvidence
            | Blocker::MissingBuildReceipt
            | Blocker::MissingReworkReceipt
            | Blocker::MissingMetricObservation
            | Blocker::DuplicateMetricObservation
            | Blocker::ExtraMetricObservation
            | Blocker::InvalidMetricInterval
            | Blocker::NonPhysicalMetricEvidence
            | Blocker::MissingMeasurementMethod
            | Blocker::InvalidMeasurementLineage
            | Blocker::VirginBaselineHardFailure
            | Blocker::RecoveredHardLimitUnresolved
            | Blocker::VirginHardLimitUnresolved
            | Blocker::FirstPassYieldUnresolved
    )
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn scope() -> ProcessScope {
        ProcessScope {
            application_id: "MAGNET-ROUTE-COMP-001".to_string(),
            material_family_id: "NdFeB".to_string(),
            feedstock_form_id: "Powder".to_string(),
            route: ProcessRoute::PowderExtrusionMouldingSintering,
            recipe_digest: digest('a'),
            process_revision_digest: digest('b'),
            geometry_digest: digest('c'),
            orientation_digest: digest('d'),
            magnetization_digest: digest('e'),
            measurement_setup_digest: digest('f'),
            acceptance_policy_digest: digest('1'),
        }
    }

    fn spec() -> CohortSpec {
        CohortSpec {
            campaign_id: "PAIRED-HPMS-PEM-001".to_string(),
            scope: scope(),
            metrics: vec![
                MetricSpec {
                    metric_id: "absolute_br".to_string(),
                    unit: "T".to_string(),
                    direction: Direction::Maximize,
                    hard_min: Some(0.5),
                    hard_max: None,
                    noninferiority_margin: 0.05,
                    improvement_margin: 0.01,
                },
                MetricSpec {
                    metric_id: "intrinsic_hcj".to_string(),
                    unit: "kA_per_m".to_string(),
                    direction: Direction::Maximize,
                    hard_min: Some(100.0),
                    hard_max: None,
                    noninferiority_margin: 10.0,
                    improvement_margin: 2.0,
                },
                MetricSpec {
                    metric_id: "porosity".to_string(),
                    unit: "vol_percent".to_string(),
                    direction: Direction::Minimize,
                    hard_min: None,
                    hard_max: Some(5.0),
                    noninferiority_margin: 1.0,
                    improvement_margin: 0.2,
                },
            ],
            minimum_independent_lot_pairs: 3,
            minimum_build_pairs_per_lot: 3,
            first_pass_yield_noninferiority_margin: 0.1,
        }
    }

    fn lot_pairs() -> Vec<SourceLotPair> {
        (0..3)
            .map(|i| SourceLotPair {
                lot_pair_id: format!("LOT-PAIR-{i}"),
                virgin_source_lot_id: format!("VIRGIN-LOT-{i}"),
                recovered_source_lot_id: format!("RECOVERED-LOT-{i}"),
                virgin_eligibility_receipt_id: format!("VIRGIN-ELIG-{i}"),
                virgin_eligibility_digest: digest('2'),
                recovered_eligibility_receipt_id: format!("RECOVERED-ELIG-{i}"),
                recovered_eligibility_digest: digest('3'),
            })
            .collect()
    }

    fn observation(metric_id: &str, unit: &str, value: f64, suffix: char) -> MetricObservation {
        MetricObservation {
            metric_id: metric_id.to_string(),
            unit: unit.to_string(),
            value: Interval::new(value - 0.005, value + 0.005),
            evidence_id: format!("OBS-{metric_id}-{suffix}"),
            evidence_digest: digest(suffix),
            method_id: format!("METHOD-{metric_id}"),
            calibration_digest: digest('4'),
            uncertainty_record_digest: digest('5'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
        }
    }

    fn build(
        block: &str,
        id: &str,
        condition: MaterialCondition,
        source_lot: &str,
        br: f64,
        hcj: f64,
        porosity: f64,
        suffix: char,
    ) -> BuildEvidence {
        BuildEvidence {
            build_id: id.to_string(),
            pair_block_id: block.to_string(),
            condition,
            source_lot_id: source_lot.to_string(),
            specimen_id: format!("SPEC-{id}"),
            blind_sample_id: format!("BLIND-{id}"),
            scope: scope(),
            first_pass_disposition: BuildDisposition::Qualified,
            final_disposition: BuildDisposition::Qualified,
            rework_count: 0,
            rework_receipt_id: None,
            rework_receipt_digest: None,
            build_receipt_id: format!("BUILD-RECEIPT-{id}"),
            build_receipt_digest: digest('6'),
            evidence_class: EvidenceClass::PhysicalMeasurement,
            observations: vec![
                observation("absolute_br", "T", br, suffix),
                observation("intrinsic_hcj", "kA_per_m", hcj, char::from(suffix as u8 + 1)),
                observation("porosity", "vol_percent", porosity, char::from(suffix as u8 + 2)),
            ],
        }
    }

    fn cohort(recovered_br: f64, recovered_hcj: f64, recovered_porosity: f64) -> PairedCohort {
        let lot_pairs = lot_pairs();
        let mut pair_blocks = Vec::new();
        let mut builds = Vec::new();
        let mut initiated_build_ids = Vec::new();
        let mut suffix = b'a';
        for pair in &lot_pairs {
            for j in 0..3 {
                let block_id = format!("BLOCK-{}-{j}", pair.lot_pair_id);
                let virgin_id = format!("V-{}-{j}", pair.lot_pair_id);
                let recovered_id = format!("R-{}-{j}", pair.lot_pair_id);
                pair_blocks.push(PairBlock {
                    pair_block_id: block_id.clone(),
                    lot_pair_id: pair.lot_pair_id.clone(),
                    randomized_order: if j % 2 == 0 {
                        RandomizedOrder::VirginFirst
                    } else {
                        RandomizedOrder::RecoveredFirst
                    },
                    randomization_receipt_id: format!("RANDOM-{block_id}"),
                    randomization_digest: digest('7'),
                    seed_commitment_digest: digest('8'),
                    virgin_build_id: virgin_id.clone(),
                    recovered_build_id: recovered_id.clone(),
                });
                initiated_build_ids.push(virgin_id.clone());
                initiated_build_ids.push(recovered_id.clone());
                builds.push(build(
                    &block_id,
                    &virgin_id,
                    MaterialCondition::Virgin,
                    &pair.virgin_source_lot_id,
                    0.90,
                    150.0,
                    2.0,
                    char::from(suffix),
                ));
                suffix += 1;
                builds.push(build(
                    &block_id,
                    &recovered_id,
                    MaterialCondition::Recovered,
                    &pair.recovered_source_lot_id,
                    recovered_br,
                    recovered_hcj,
                    recovered_porosity,
                    char::from(suffix),
                ));
                suffix += 1;
            }
        }
        PairedCohort {
            initiated_build_ids,
            source_lot_pairs: lot_pairs,
            pair_blocks,
            builds,
        }
    }

    #[test]
    fn paired_recovered_cohort_is_noninferior_with_complete_denominators() {
        let result = evaluate_paired_cohort(&spec(), &cohort(0.89, 148.0, 2.1));
        assert_eq!(result.decision, Decision::RecoveredRouteNonInferior);
        let yield_summary = result.yield_summary.unwrap();
        assert_eq!(yield_summary.virgin_initiated, 9);
        assert_eq!(yield_summary.recovered_initiated, 9);
        assert_eq!(yield_summary.virgin_first_pass_qualified, 9);
        assert_eq!(yield_summary.recovered_first_pass_qualified, 9);
    }

    #[test]
    fn robust_improvement_across_every_pair_is_recovered_route_advantage() {
        let result = evaluate_paired_cohort(&spec(), &cohort(0.93, 155.0, 1.5));
        assert_eq!(result.decision, Decision::RecoveredRouteAdvantage);
        assert!(result.robustly_improved_metrics.contains(&"absolute_br".to_string()));
        assert!(result.robustly_improved_metrics.contains(&"intrinsic_hcj".to_string()));
        assert!(result.robustly_improved_metrics.contains(&"porosity".to_string()));
    }

    #[test]
    fn noninferiority_failure_is_not_hidden_by_another_metric_improvement() {
        let result = evaluate_paired_cohort(&spec(), &cohort(0.80, 160.0, 1.0));
        assert_eq!(result.decision, Decision::InferiorToVirgin);
        assert!(result.blockers.contains(&Blocker::NoninferiorityViolation));
    }

    #[test]
    fn recovered_absolute_hard_limit_failure_is_fail() {
        let result = evaluate_paired_cohort(&spec(), &cohort(0.40, 150.0, 2.0));
        assert_eq!(result.decision, Decision::Fail);
        assert!(result.blockers.contains(&Blocker::RecoveredHardFailure));
    }

    #[test]
    fn recovered_interval_crossing_hard_limit_is_indeterminate() {
        let result = evaluate_paired_cohort(&spec(), &cohort(0.504, 150.0, 2.0));
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::RecoveredHardLimitUnresolved));
    }

    #[test]
    fn missing_build_evidence_cannot_leave_initiated_denominator() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds.pop();
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingBuildEvidence));
    }

    #[test]
    fn missing_build_record_remains_in_first_pass_yield_denominator() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds.retain(|build| build.build_id != "R-0-0");
        let result = evaluate_paired_cohort(&spec(), &data);
        let summary = result.yield_summary.unwrap();
        assert_eq!(summary.recovered_initiated, 9);
        assert_eq!(summary.recovered_first_pass_qualified, 8);
        assert!((summary.recovered_first_pass_yield - (8.0 / 9.0)).abs() < 1e-12);
        assert_eq!(result.decision, Decision::Indeterminate);
    }

    #[test]
    fn extra_build_evidence_is_not_silently_ignored() {
        let mut data = cohort(0.89, 148.0, 2.1);
        let mut extra = data.builds[0].clone();
        extra.build_id = "ORPHAN-EXTRA".to_string();
        extra.blind_sample_id = "BLIND-EXTRA".to_string();
        data.builds.push(extra);
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::ExtraBuildEvidence));
    }

    #[test]
    fn duplicate_build_identity_is_not_counted_twice() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds[1].build_id = data.builds[0].build_id.clone();
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::DuplicateBuildIdentity));
    }

    #[test]
    fn process_route_mismatch_is_not_comparable() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds[0].scope.route = ProcessRoute::MaterialExtrusionBonded;
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::ScopeMismatch));
    }

    #[test]
    fn recipe_mismatch_is_not_comparable() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds[1].scope.recipe_digest = digest('0');
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::ScopeMismatch));
    }

    #[test]
    fn source_lot_mismatch_is_not_comparable() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds[1].source_lot_id = "UNPAIRED-LOT".to_string();
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::SourceLotMismatch));
    }

    #[test]
    fn minimum_independent_lot_pair_floor_is_required() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.source_lot_pairs.pop();
        let removed_id = "LOT-PAIR-2";
        data.pair_blocks.retain(|block| !block.lot_pair_id.contains(removed_id));
        let removed_builds: BTreeSet<String> = data
            .builds
            .iter()
            .filter(|build| build.pair_block_id.contains(removed_id))
            .map(|build| build.build_id.clone())
            .collect();
        data.builds.retain(|build| !removed_builds.contains(&build.build_id));
        data.initiated_build_ids
            .retain(|build_id| !removed_builds.contains(build_id));
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InsufficientIndependentLotPairs));
    }

    #[test]
    fn minimum_build_pair_floor_per_lot_is_required() {
        let mut specification = spec();
        specification.minimum_build_pairs_per_lot = 4;
        let result = evaluate_paired_cohort(&specification, &cohort(0.89, 148.0, 2.1));
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InsufficientBuildPairsPerLot));
    }

    #[test]
    fn randomization_receipt_is_required_for_each_pair_block() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.pair_blocks[0].randomization_receipt_id.clear();
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::RandomizationReceiptMissing));
    }

    #[test]
    fn rework_does_not_inflate_first_pass_yield() {
        let mut data = cohort(0.89, 148.0, 2.1);
        let recovered = data
            .builds
            .iter_mut()
            .find(|build| build.condition == MaterialCondition::Recovered)
            .unwrap();
        recovered.first_pass_disposition = BuildDisposition::Fail;
        recovered.final_disposition = BuildDisposition::Qualified;
        recovered.rework_count = 1;
        recovered.rework_receipt_id = Some("REWORK-001".to_string());
        recovered.rework_receipt_digest = Some(digest('9'));
        let result = evaluate_paired_cohort(&spec(), &data);
        let summary = result.yield_summary.unwrap();
        assert_eq!(summary.recovered_first_pass_qualified, 8);
        assert_eq!(summary.recovered_final_qualified, 9);
        assert_eq!(summary.virgin_first_pass_qualified, 9);
    }

    #[test]
    fn reworked_build_requires_a_receipt() {
        let mut data = cohort(0.89, 148.0, 2.1);
        let recovered = data
            .builds
            .iter_mut()
            .find(|build| build.condition == MaterialCondition::Recovered)
            .unwrap();
        recovered.first_pass_disposition = BuildDisposition::Fail;
        recovered.rework_count = 1;
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingReworkReceipt));
    }

    #[test]
    fn synthetic_build_receipts_cannot_qualify_a_physical_paired_cohort() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds[0].evidence_class = EvidenceClass::SyntheticControl;
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::NonPhysicalBuildEvidence));
    }

    #[test]
    fn blind_sample_ids_must_be_unique() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds[1].blind_sample_id = data.builds[0].blind_sample_id.clone();
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::NotComparable);
        assert!(result.blockers.contains(&Blocker::DuplicateBlindSample));
    }

    #[test]
    fn missing_metric_observation_is_not_imputed() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds[0]
            .observations
            .retain(|observation| observation.metric_id != "absolute_br");
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::MissingMetricObservation));
    }

    #[test]
    fn virgin_baseline_hard_failure_makes_comparison_indeterminate() {
        let mut data = cohort(0.89, 148.0, 2.1);
        data.builds[0].observations[0].value = Interval::new(0.3, 0.4);
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::Indeterminate);
        assert!(result.blockers.contains(&Blocker::VirginBaselineHardFailure));
    }

    #[test]
    fn first_pass_yield_is_a_separate_noninferiority_gate() {
        let mut data = cohort(0.89, 148.0, 2.1);
        for build in data
            .builds
            .iter_mut()
            .filter(|build| build.condition == MaterialCondition::Recovered)
            .take(3)
        {
            build.first_pass_disposition = BuildDisposition::Fail;
        }
        let result = evaluate_paired_cohort(&spec(), &data);
        assert_eq!(result.decision, Decision::InferiorToVirgin);
        assert!(result.blockers.contains(&Blocker::FirstPassYieldInferior));
    }
}
