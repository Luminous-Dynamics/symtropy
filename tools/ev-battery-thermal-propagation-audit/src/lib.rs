#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationDomain {
    EvTraction,
    StationaryStorage,
    Industrial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestLevel {
    Cell,
    ModuleOrRpu,
    ReessPack,
    Vehicle,
    StationaryInstallation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    PhysicalTest,
    IndependentLaboratoryTest,
    RiskAnalysis,
    Simulation,
    Literature,
    Synthetic,
}

impl EvidenceClass {
    fn is_physical(self) -> bool {
        matches!(self, Self::PhysicalTest | Self::IndependentLaboratoryTest)
    }

    fn is_admissible_review_source(self) -> bool {
        matches!(
            self,
            Self::PhysicalTest | Self::IndependentLaboratoryTest | Self::RiskAnalysis
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    Candidate,
    AdmittedForResearch,
    AdmittedForEvaluation,
    Blocked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerMethod {
    LocalizedExternalHeating,
    InternalHeating,
    NailPenetration,
    Laser,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestResult {
    Pass,
    Fail,
    TriggerNotAchieved,
    NoPropagationWithinWindow,
    RiskAnalysisSubmitted,
    Invalid,
    Incomplete,
    NotMeasured,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EquivalenceDomain {
    CellSourceTerms,
    Topology,
    ThermalPathways,
    Protection,
    Enclosure,
    Exposure,
}

impl EquivalenceDomain {
    pub const ALL: [Self; 6] = [
        Self::CellSourceTerms,
        Self::Topology,
        Self::ThermalPathways,
        Self::Protection,
        Self::Enclosure,
        Self::Exposure,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquivalenceStatus {
    SameWithEvidence,
    Different,
    Unknown,
    NotApplicableWithRationale,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainFingerprints {
    pub cell_source_terms: String,
    pub topology: String,
    pub thermal_pathways: String,
    pub protection: String,
    pub enclosure: String,
    pub exposure: String,
}

impl DomainFingerprints {
    fn get(&self, domain: EquivalenceDomain) -> &str {
        match domain {
            EquivalenceDomain::CellSourceTerms => &self.cell_source_terms,
            EquivalenceDomain::Topology => &self.topology,
            EquivalenceDomain::ThermalPathways => &self.thermal_pathways,
            EquivalenceDomain::Protection => &self.protection,
            EquivalenceDomain::Enclosure => &self.enclosure,
            EquivalenceDomain::Exposure => &self.exposure,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArticleDescriptor {
    pub article_id: String,
    pub product_revision: String,
    pub application: ApplicationDomain,
    pub level: TestLevel,
    pub chemistry_family: String,
    pub cell_format: String,
    pub nominal_cell_capacity_wh: u32,
    pub state_of_charge_permille: u16,
    pub age_cycles: Option<u32>,
    pub fingerprints: DomainFingerprints,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub source_digest: String,
    pub evidence_class: EvidenceClass,
    pub admission: Admission,
    pub claim_scope: String,
    pub authority_or_lab_ref: String,
    pub metrology_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestRecord {
    pub test_id: String,
    pub article: ArticleDescriptor,
    pub standard_id: String,
    pub standard_revision: String,
    pub protocol_revision: String,
    pub trigger_method: TriggerMethod,
    pub trigger_achieved: bool,
    pub post_trigger_observation_minutes: u32,
    pub propagation_observed: Option<bool>,
    pub fire_observed: Option<bool>,
    pub explosion_observed: Option<bool>,
    pub smoke_into_occupant_boundary: Option<bool>,
    pub warning_time_seconds: Option<u32>,
    pub result: TestResult,
    pub evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquivalenceAssessment {
    pub domain: EquivalenceDomain,
    pub status: EquivalenceStatus,
    pub evidence_id: Option<String>,
    pub rationale: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    pub claim_id: String,
    pub source_test_id: String,
    pub target_article: ArticleDescriptor,
    pub equivalence: Vec<EquivalenceAssessment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub campaign_id: String,
    pub evidence: Vec<EvidenceRecord>,
    pub tests: Vec<TestRecord>,
    pub claims: Vec<Claim>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimDisposition {
    CandidateForHumanReview,
    BlockedRequiresQualifiedReview,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimReport {
    pub claim_id: String,
    pub disposition: ClaimDisposition,
    pub blockers: Vec<String>,
    pub source_test_id: String,
    pub source_result: TestResult,
    pub target_level: TestLevel,
    pub target_application: ApplicationDomain,
    pub source_reported_pass: bool,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditReport {
    pub campaign_id: String,
    pub evidence_count: usize,
    pub test_count: usize,
    pub claim_count: usize,
    pub physical_test_count: usize,
    pub risk_analysis_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub trigger_not_achieved_count: usize,
    pub no_propagation_window_count: usize,
    pub invalid_or_incomplete_count: usize,
    pub unknown_or_not_measured_count: usize,
    pub candidate_for_review_count: usize,
    pub blocked_claim_count: usize,
    pub claims: Vec<ClaimReport>,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditError(pub String);

impl std::fmt::Display for AuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AuditError {}

fn require_nonempty(value: &str, label: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!("{label} must not be empty")));
    }
    Ok(())
}

fn validate_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), AuditError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(AuditError(format!("duplicate {label}: {value}")));
        }
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), AuditError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(AuditError(format!(
            "{label} must be a lowercase 64-character SHA-256 digest"
        )));
    }
    Ok(())
}

fn validate_article(article: &ArticleDescriptor) -> Result<(), AuditError> {
    for (value, label) in [
        (article.article_id.as_str(), "article ID"),
        (article.product_revision.as_str(), "product revision"),
        (article.chemistry_family.as_str(), "chemistry family"),
        (article.cell_format.as_str(), "cell format"),
    ] {
        require_nonempty(value, label)?;
    }
    if article.nominal_cell_capacity_wh == 0 {
        return Err(AuditError("nominal cell capacity must be nonzero".into()));
    }
    if article.state_of_charge_permille > 1000 {
        return Err(AuditError(
            "state of charge must be in the range 0..=1000 permille".into(),
        ));
    }
    for domain in EquivalenceDomain::ALL {
        validate_digest(article.fingerprints.get(domain), "article domain fingerprint")?;
    }
    Ok(())
}

fn evidence_registry(
    records: &[EvidenceRecord],
) -> Result<BTreeMap<&str, &EvidenceRecord>, AuditError> {
    validate_unique(
        records.iter().map(|record| record.evidence_id.as_str()),
        "evidence ID",
    )?;
    if records.is_empty() {
        return Err(AuditError("evidence registry must not be empty".into()));
    }

    let mut map = BTreeMap::new();
    for record in records {
        validate_digest(&record.source_digest, "evidence source digest")?;
        require_nonempty(&record.claim_scope, "evidence claim scope")?;
        require_nonempty(&record.authority_or_lab_ref, "authority/laboratory reference")?;
        if record.evidence_class.is_physical()
            && record.metrology_ref.as_deref().is_none_or(str::is_empty)
        {
            return Err(AuditError(format!(
                "physical evidence {} requires a metrology reference",
                record.evidence_id
            )));
        }
        map.insert(record.evidence_id.as_str(), record);
    }
    Ok(map)
}

fn admitted_evidence<'a>(
    registry: &'a BTreeMap<&str, &EvidenceRecord>,
    evidence_id: &str,
    owner: &str,
) -> Result<&'a EvidenceRecord, AuditError> {
    let record = registry.get(evidence_id).copied().ok_or_else(|| {
        AuditError(format!(
            "{owner} references unknown evidence ID {evidence_id}"
        ))
    })?;
    if record.admission != Admission::AdmittedForEvaluation {
        return Err(AuditError(format!(
            "{owner} references evidence {evidence_id} that is not admitted for evaluation"
        )));
    }
    Ok(record)
}

fn validate_test(
    test: &TestRecord,
    registry: &BTreeMap<&str, &EvidenceRecord>,
) -> Result<&EvidenceRecord, AuditError> {
    require_nonempty(&test.test_id, "test ID")?;
    require_nonempty(&test.standard_id, "standard ID")?;
    require_nonempty(&test.standard_revision, "standard revision")?;
    require_nonempty(&test.protocol_revision, "protocol revision")?;
    validate_article(&test.article)?;
    let evidence = admitted_evidence(registry, &test.evidence_id, &format!("test {}", test.test_id))?;

    match test.result {
        TestResult::Pass | TestResult::Fail => {
            if !evidence.evidence_class.is_physical() {
                return Err(AuditError(format!(
                    "physical test result for {} requires physical-test evidence",
                    test.test_id
                )));
            }
            if !test.trigger_achieved {
                return Err(AuditError(format!(
                    "test {} cannot report PASS/FAIL when the trigger was not achieved",
                    test.test_id
                )));
            }
        }
        TestResult::NoPropagationWithinWindow => {
            if !evidence.evidence_class.is_physical() {
                return Err(AuditError(format!(
                    "no-propagation result for {} requires physical-test evidence",
                    test.test_id
                )));
            }
            if !test.trigger_achieved
                || test.propagation_observed != Some(false)
                || test.post_trigger_observation_minutes == 0
            {
                return Err(AuditError(format!(
                    "no-propagation result for {} requires achieved trigger, observed window and explicit no-propagation outcome",
                    test.test_id
                )));
            }
        }
        TestResult::TriggerNotAchieved => {
            if test.trigger_achieved {
                return Err(AuditError(format!(
                    "test {} is marked trigger-not-achieved but trigger_achieved is true",
                    test.test_id
                )));
            }
        }
        TestResult::RiskAnalysisSubmitted => {
            if evidence.evidence_class != EvidenceClass::RiskAnalysis {
                return Err(AuditError(format!(
                    "risk-analysis record {} requires risk-analysis evidence class",
                    test.test_id
                )));
            }
        }
        TestResult::Invalid
        | TestResult::Incomplete
        | TestResult::NotMeasured
        | TestResult::Unknown => {}
    }
    Ok(evidence)
}

fn validate_equivalence(
    comparisons: &[EquivalenceAssessment],
    registry: &BTreeMap<&str, &EvidenceRecord>,
    claim_id: &str,
) -> Result<BTreeMap<EquivalenceDomain, &EquivalenceAssessment>, AuditError> {
    if comparisons.len() != EquivalenceDomain::ALL.len() {
        return Err(AuditError(format!(
            "claim {claim_id} must declare all six equivalence domains"
        )));
    }
    let mut map = BTreeMap::new();
    for comparison in comparisons {
        require_nonempty(&comparison.rationale, "equivalence rationale")?;
        if map.insert(comparison.domain, comparison).is_some() {
            return Err(AuditError(format!(
                "claim {claim_id} repeats equivalence domain {:?}",
                comparison.domain
            )));
        }
        match comparison.status {
            EquivalenceStatus::SameWithEvidence => {
                let evidence_id = comparison.evidence_id.as_deref().ok_or_else(|| {
                    AuditError(format!(
                        "claim {claim_id} SAME_WITH_EVIDENCE for {:?} requires evidence",
                        comparison.domain
                    ))
                })?;
                let evidence = admitted_evidence(
                    registry,
                    evidence_id,
                    &format!("claim {claim_id} equivalence {:?}", comparison.domain),
                )?;
                if !evidence.evidence_class.is_admissible_review_source() {
                    return Err(AuditError(format!(
                        "claim {claim_id} equivalence evidence {evidence_id} is not an admissible review source"
                    )));
                }
            }
            EquivalenceStatus::Different
            | EquivalenceStatus::Unknown
            | EquivalenceStatus::NotApplicableWithRationale => {
                if comparison.evidence_id.as_deref().is_some_and(str::is_empty) {
                    return Err(AuditError(format!(
                        "claim {claim_id} has an empty equivalence evidence ID"
                    )));
                }
            }
        }
    }
    for domain in EquivalenceDomain::ALL {
        if !map.contains_key(&domain) {
            return Err(AuditError(format!(
                "claim {claim_id} is missing equivalence domain {domain:?}"
            )));
        }
    }
    Ok(map)
}

fn domain_blocker(comparison: &EquivalenceAssessment) -> Option<String> {
    match comparison.status {
        EquivalenceStatus::SameWithEvidence
        | EquivalenceStatus::NotApplicableWithRationale => None,
        EquivalenceStatus::Different => Some(format!(
            "{:?} differs; prior evidence cannot be inherited automatically",
            comparison.domain
        )),
        EquivalenceStatus::Unknown => Some(format!(
            "{:?} equivalence is unknown; prior evidence cannot be inherited automatically",
            comparison.domain
        )),
    }
}

/// Audit a finite thermal-propagation evidence manifest.
/// A positive disposition is only a candidate for competent human/authority review.
pub fn audit_manifest(manifest: &Manifest) -> Result<AuditReport, AuditError> {
    require_nonempty(&manifest.campaign_id, "campaign ID")?;
    if manifest.tests.is_empty() || manifest.claims.is_empty() {
        return Err(AuditError(
            "campaign requires at least one test and one evidence-transfer claim".into(),
        ));
    }
    let registry = evidence_registry(&manifest.evidence)?;
    validate_unique(
        manifest.tests.iter().map(|test| test.test_id.as_str()),
        "test ID",
    )?;
    validate_unique(
        manifest.claims.iter().map(|claim| claim.claim_id.as_str()),
        "claim ID",
    )?;

    let mut tests_by_id = BTreeMap::new();
    let mut physical_test_count = 0;
    let mut risk_analysis_count = 0;
    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut trigger_not_achieved_count = 0;
    let mut no_propagation_window_count = 0;
    let mut invalid_or_incomplete_count = 0;
    let mut unknown_or_not_measured_count = 0;

    for test in &manifest.tests {
        let evidence = validate_test(test, &registry)?;
        match evidence.evidence_class {
            EvidenceClass::PhysicalTest | EvidenceClass::IndependentLaboratoryTest => {
                physical_test_count += 1;
            }
            EvidenceClass::RiskAnalysis => risk_analysis_count += 1,
            EvidenceClass::Simulation | EvidenceClass::Literature | EvidenceClass::Synthetic => {}
        }
        match test.result {
            TestResult::Pass => pass_count += 1,
            TestResult::Fail => fail_count += 1,
            TestResult::TriggerNotAchieved => trigger_not_achieved_count += 1,
            TestResult::NoPropagationWithinWindow => no_propagation_window_count += 1,
            TestResult::Invalid | TestResult::Incomplete => invalid_or_incomplete_count += 1,
            TestResult::NotMeasured | TestResult::Unknown => unknown_or_not_measured_count += 1,
            TestResult::RiskAnalysisSubmitted => {}
        }
        tests_by_id.insert(test.test_id.as_str(), test);
    }

    let mut claim_reports = Vec::new();
    let mut candidate_for_review_count = 0;
    let mut blocked_claim_count = 0;

    for claim in &manifest.claims {
        require_nonempty(&claim.claim_id, "claim ID")?;
        validate_article(&claim.target_article)?;
        let comparisons = validate_equivalence(&claim.equivalence, &registry, &claim.claim_id)?;
        let test = tests_by_id
            .get(claim.source_test_id.as_str())
            .copied()
            .ok_or_else(|| {
                AuditError(format!(
                    "claim {} references unknown test {}",
                    claim.claim_id, claim.source_test_id
                ))
            })?;
        let source_evidence = admitted_evidence(
            &registry,
            &test.evidence_id,
            &format!("claim {} source test", claim.claim_id),
        )?;

        let mut blockers = Vec::new();
        if claim.target_article.application != test.article.application {
            blockers.push(
                "application-domain mismatch: evidence cannot cross EV traction, stationary storage or industrial domains automatically".into(),
            );
        }
        if claim.target_article.level != test.article.level {
            blockers.push(format!(
                "test level {:?} does not directly support target level {:?}",
                test.article.level, claim.target_article.level
            ));
        }
        if !source_evidence.evidence_class.is_admissible_review_source() {
            blockers.push("source evidence class cannot support a safety evidence review".into());
        }
        match test.result {
            TestResult::Pass | TestResult::NoPropagationWithinWindow | TestResult::RiskAnalysisSubmitted => {}
            TestResult::Fail => blockers.push("source test reports FAIL".into()),
            TestResult::TriggerNotAchieved => {
                blockers.push("source trigger was not achieved".into())
            }
            TestResult::Invalid => blockers.push("source test is INVALID".into()),
            TestResult::Incomplete => blockers.push("source test is INCOMPLETE".into()),
            TestResult::NotMeasured => blockers.push("source result is NOT_MEASURED".into()),
            TestResult::Unknown => blockers.push("source result is UNKNOWN".into()),
        }

        for domain in EquivalenceDomain::ALL {
            let comparison = comparisons.get(&domain).copied().ok_or_else(|| {
                AuditError(format!(
                    "claim {} lost equivalence domain {domain:?}",
                    claim.claim_id
                ))
            })?;
            if let Some(blocker) = domain_blocker(comparison) {
                blockers.push(blocker);
            }
        }

        let disposition = if blockers.is_empty() {
            candidate_for_review_count += 1;
            ClaimDisposition::CandidateForHumanReview
        } else {
            blocked_claim_count += 1;
            ClaimDisposition::BlockedRequiresQualifiedReview
        };
        claim_reports.push(ClaimReport {
            claim_id: claim.claim_id.clone(),
            disposition,
            blockers,
            source_test_id: test.test_id.clone(),
            source_result: test.result,
            target_level: claim.target_article.level,
            target_application: claim.target_article.application,
            source_reported_pass: test.result == TestResult::Pass,
            claim_ceiling: "Candidate for competent human/authority review only. Not a safety determination, certification, compliance decision or product release.".into(),
        });
    }

    claim_reports.sort_by(|left, right| left.claim_id.cmp(&right.claim_id));
    Ok(AuditReport {
        campaign_id: manifest.campaign_id.clone(),
        evidence_count: manifest.evidence.len(),
        test_count: manifest.tests.len(),
        claim_count: manifest.claims.len(),
        physical_test_count,
        risk_analysis_count,
        pass_count,
        fail_count,
        trigger_not_achieved_count,
        no_propagation_window_count,
        invalid_or_incomplete_count,
        unknown_or_not_measured_count,
        candidate_for_review_count,
        blocked_claim_count,
        claims: claim_reports,
        claim_ceiling: "Evidence identity, scope, completeness, status and transfer gates only. No thermal-runaway prediction, safety certification or compliance claim.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn evidence(
        id: &str,
        character: char,
        class: EvidenceClass,
        metrology: Option<&str>,
    ) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: id.into(),
            source_digest: digest(character),
            evidence_class: class,
            admission: Admission::AdmittedForEvaluation,
            claim_scope: format!("scope:{id}"),
            authority_or_lab_ref: format!("lab-or-authority:{id}"),
            metrology_ref: metrology.map(str::to_string),
        }
    }

    fn article(
        id: &str,
        revision: &str,
        application: ApplicationDomain,
        level: TestLevel,
        chars: [char; 6],
    ) -> ArticleDescriptor {
        ArticleDescriptor {
            article_id: id.into(),
            product_revision: revision.into(),
            application,
            level,
            chemistry_family: "NMC".into(),
            cell_format: "pouch".into(),
            nominal_cell_capacity_wh: 250,
            state_of_charge_permille: 1000,
            age_cycles: Some(50),
            fingerprints: DomainFingerprints {
                cell_source_terms: digest(chars[0]),
                topology: digest(chars[1]),
                thermal_pathways: digest(chars[2]),
                protection: digest(chars[3]),
                enclosure: digest(chars[4]),
                exposure: digest(chars[5]),
            },
        }
    }

    fn same_comparisons() -> Vec<EquivalenceAssessment> {
        EquivalenceDomain::ALL
            .into_iter()
            .map(|domain| EquivalenceAssessment {
                domain,
                status: EquivalenceStatus::SameWithEvidence,
                evidence_id: Some("E-COMPARISON".into()),
                rationale: "same revision and domain fingerprint; comparison evidence attached".into(),
            })
            .collect()
    }

    fn physical_test(
        test_id: &str,
        article: ArticleDescriptor,
        evidence_id: &str,
        result: TestResult,
        trigger_achieved: bool,
    ) -> TestRecord {
        TestRecord {
            test_id: test_id.into(),
            article,
            standard_id: "UN-R100-2026".into(),
            standard_revision: "05-series".into(),
            protocol_revision: "protocol-r1".into(),
            trigger_method: TriggerMethod::LocalizedExternalHeating,
            trigger_achieved,
            post_trigger_observation_minutes: 120,
            propagation_observed: Some(false),
            fire_observed: Some(false),
            explosion_observed: Some(false),
            smoke_into_occupant_boundary: Some(false),
            warning_time_seconds: Some(15),
            result,
            evidence_id: evidence_id.into(),
        }
    }

    fn fixture() -> Manifest {
        let ev_article = article(
            "EV-REESS-A",
            "r1",
            ApplicationDomain::EvTraction,
            TestLevel::ReessPack,
            ['1', '2', '3', '4', '5', '6'],
        );
        let cell_article = article(
            "EV-CELL-A",
            "r1",
            ApplicationDomain::EvTraction,
            TestLevel::Cell,
            ['1', '2', '3', '4', '5', '6'],
        );
        let bess_article = article(
            "BESS-A",
            "r1",
            ApplicationDomain::StationaryStorage,
            TestLevel::StationaryInstallation,
            ['7', '8', '9', 'a', 'b', 'c'],
        );
        let risk_article = article(
            "EV-REESS-RISK",
            "r1",
            ApplicationDomain::EvTraction,
            TestLevel::ReessPack,
            ['1', '2', '3', '4', '5', '6'],
        );
        let mut risk_test = physical_test(
            "RISK-001",
            risk_article,
            "E-RISK",
            TestResult::RiskAnalysisSubmitted,
            false,
        );
        risk_test.standard_id = "UN-R100-2026".into();

        let tests = vec![
            physical_test(
                "EV-REESS-001",
                ev_article.clone(),
                "E-REESS",
                TestResult::NoPropagationWithinWindow,
                true,
            ),
            physical_test(
                "EV-CELL-001",
                cell_article,
                "E-CELL",
                TestResult::Pass,
                true,
            ),
            physical_test(
                "BESS-INSTALL-001",
                bess_article,
                "E-BESS",
                TestResult::Pass,
                true,
            ),
            risk_test,
        ];

        let same = same_comparisons();
        let mut different_topology = same.clone();
        different_topology
            .iter_mut()
            .find(|item| item.domain == EquivalenceDomain::Topology)
            .expect("topology exists")
            .status = EquivalenceStatus::Different;
        let mut unknown_exposure = same.clone();
        unknown_exposure
            .iter_mut()
            .find(|item| item.domain == EquivalenceDomain::Exposure)
            .expect("exposure exists")
            .status = EquivalenceStatus::Unknown;

        let ev_target = ev_article;
        let mut changed_target = article(
            "EV-REESS-A",
            "r2",
            ApplicationDomain::EvTraction,
            TestLevel::ReessPack,
            ['1', '2', '3', '4', '5', '6'],
        );
        changed_target.product_revision = "r2".into();

        Manifest {
            campaign_id: "TRP-EVIDENCE-CAMPAIGN-001".into(),
            evidence: vec![
                evidence("E-REESS", 'd', EvidenceClass::IndependentLaboratoryTest, Some("MET-REESS")),
                evidence("E-CELL", 'e', EvidenceClass::PhysicalTest, Some("MET-CELL")),
                evidence("E-BESS", 'f', EvidenceClass::PhysicalTest, Some("MET-BESS")),
                evidence("E-RISK", 'a', EvidenceClass::RiskAnalysis, None),
                evidence("E-COMPARISON", 'b', EvidenceClass::IndependentLaboratoryTest, Some("MET-COMP")),
            ],
            tests,
            claims: vec![
                Claim {
                    claim_id: "CLAIM-EXACT-REESS".into(),
                    source_test_id: "EV-REESS-001".into(),
                    target_article: ev_target,
                    equivalence: same.clone(),
                },
                Claim {
                    claim_id: "CLAIM-CELL-TO-VEHICLE".into(),
                    source_test_id: "EV-CELL-001".into(),
                    target_article: article(
                        "EV-REESS-VEHICLE",
                        "r1",
                        ApplicationDomain::EvTraction,
                        TestLevel::Vehicle,
                        ['1', '2', '3', '4', '5', '6'],
                    ),
                    equivalence: same.clone(),
                },
                Claim {
                    claim_id: "CLAIM-BESS-TO-EV".into(),
                    source_test_id: "BESS-INSTALL-001".into(),
                    target_article: article(
                        "EV-REESS-B",
                        "r1",
                        ApplicationDomain::EvTraction,
                        TestLevel::ReessPack,
                        ['7', '8', '9', 'a', 'b', 'c'],
                    ),
                    equivalence: same.clone(),
                },
                Claim {
                    claim_id: "CLAIM-CHANGED-TOPOLOGY".into(),
                    source_test_id: "EV-REESS-001".into(),
                    target_article: changed_target,
                    equivalence: different_topology,
                },
                Claim {
                    claim_id: "CLAIM-UNKNOWN-EXPOSURE".into(),
                    source_test_id: "EV-REESS-001".into(),
                    target_article: article(
                        "EV-REESS-C",
                        "r1",
                        ApplicationDomain::EvTraction,
                        TestLevel::ReessPack,
                        ['1', '2', '3', '4', '5', '6'],
                    ),
                    equivalence: unknown_exposure,
                },
                Claim {
                    claim_id: "CLAIM-RISK-ANALYSIS".into(),
                    source_test_id: "RISK-001".into(),
                    target_article: risk_article,
                    equivalence: same,
                },
            ],
        }
    }

    #[test]
    fn exact_same_level_same_application_is_only_candidate_for_review() {
        let report = audit_manifest(&fixture()).expect("fixture should audit");
        let claim = report
            .claims
            .iter()
            .find(|claim| claim.claim_id == "CLAIM-EXACT-REESS")
            .expect("claim exists");
        assert_eq!(claim.disposition, ClaimDisposition::CandidateForHumanReview);
        assert!(claim.source_reported_pass == false);
        assert!(claim.claim_ceiling.contains("Not a safety determination"));
    }

    #[test]
    fn cell_evidence_cannot_directly_support_vehicle_claim() {
        let report = audit_manifest(&fixture()).expect("manifest should audit");
        let claim = report
            .claims
            .iter()
            .find(|claim| claim.claim_id == "CLAIM-CELL-TO-VEHICLE")
            .expect("claim exists");
        assert_eq!(claim.disposition, ClaimDisposition::BlockedRequiresQualifiedReview);
        assert!(claim.blockers.iter().any(|item| item.contains("test level")));
    }

    #[test]
    fn stationary_storage_evidence_cannot_support_ev_traction_claim() {
        let report = audit_manifest(&fixture()).expect("manifest should audit");
        let claim = report
            .claims
            .iter()
            .find(|claim| claim.claim_id == "CLAIM-BESS-TO-EV")
            .expect("claim exists");
        assert_eq!(claim.disposition, ClaimDisposition::BlockedRequiresQualifiedReview);
        assert!(claim.blockers.iter().any(|item| item.contains("application-domain")));
    }

    #[test]
    fn changed_topology_blocks_automatic_transfer() {
        let report = audit_manifest(&fixture()).expect("manifest should audit");
        let claim = report
            .claims
            .iter()
            .find(|claim| claim.claim_id == "CLAIM-CHANGED-TOPOLOGY")
            .expect("claim exists");
        assert_eq!(claim.disposition, ClaimDisposition::BlockedRequiresQualifiedReview);
        assert!(claim.blockers.iter().any(|item| item.contains("Topology differs")));
    }

    #[test]
    fn unknown_exposure_blocks_automatic_transfer() {
        let report = audit_manifest(&fixture()).expect("manifest should audit");
        let claim = report
            .claims
            .iter()
            .find(|claim| claim.claim_id == "CLAIM-UNKNOWN-EXPOSURE")
            .expect("claim exists");
        assert_eq!(claim.disposition, ClaimDisposition::BlockedRequiresQualifiedReview);
        assert!(claim.blockers.iter().any(|item| item.contains("Exposure equivalence is unknown")));
    }

    #[test]
    fn no_propagation_requires_trigger_and_observation_window() {
        let mut value = fixture();
        value.tests[0].propagation_observed = Some(true);
        assert!(audit_manifest(&value).is_err());
    }

    #[test]
    fn trigger_not_achieved_remains_distinct_from_pass() {
        let mut value = fixture();
        value.tests[1].result = TestResult::TriggerNotAchieved;
        value.tests[1].trigger_achieved = false;
        let report = audit_manifest(&value).expect("trigger failure is a valid recorded outcome");
        assert_eq!(report.trigger_not_achieved_count, 1);
        let claim = report.claims.iter().find(|c| c.claim_id == "CLAIM-CELL-TO-VEHICLE").unwrap();
        assert!(claim.blockers.iter().any(|item| item.contains("trigger was not achieved")));
    }

    #[test]
    fn risk_analysis_is_not_mislabeled_as_physical_test() {
        let value = fixture();
        let report = audit_manifest(&value).expect("risk analysis should be preserved");
        assert_eq!(report.risk_analysis_count, 1);
        assert_eq!(report.physical_test_count, 3);
    }

    #[test]
    fn physical_pass_requires_physical_evidence_and_achieved_trigger() {
        let mut value = fixture();
        value.evidence[0].evidence_class = EvidenceClass::Simulation;
        assert!(audit_manifest(&value).is_err());
        let mut value = fixture();
        value.tests[0].trigger_achieved = false;
        value.tests[0].result = TestResult::Pass;
        assert!(audit_manifest(&value).is_err());
    }

    #[test]
    fn physical_evidence_requires_metrology_reference() {
        let mut value = fixture();
        value.evidence[0].metrology_ref = None;
        assert!(audit_manifest(&value).is_err());
    }

    #[test]
    fn all_six_domains_are_required_exactly_once() {
        let mut value = fixture();
        value.claims[0].equivalence.pop();
        assert!(audit_manifest(&value).is_err());
        let mut value = fixture();
        value.claims[0].equivalence[1].domain = value.claims[0].equivalence[0].domain;
        assert!(audit_manifest(&value).is_err());
    }

    #[test]
    fn same_with_evidence_requires_admitted_evidence() {
        let mut value = fixture();
        value.evidence[4].admission = Admission::AdmittedForResearch;
        assert!(audit_manifest(&value).is_err());
    }

    #[test]
    fn input_permutation_does_not_change_report() {
        let value = fixture();
        let expected = audit_manifest(&value).expect("fixture should audit");
        let mut permuted = value;
        permuted.evidence.reverse();
        permuted.tests.reverse();
        permuted.claims.reverse();
        let actual = audit_manifest(&permuted).expect("permuted fixture should audit");
        assert_eq!(expected, actual);
    }

    #[test]
    fn invalid_digest_is_rejected() {
        let mut value = fixture();
        value.evidence[0].source_digest = "bad".into();
        assert!(audit_manifest(&value).is_err());
    }

    #[test]
    fn missing_test_reference_is_rejected() {
        let mut value = fixture();
        value.claims[0].source_test_id = "MISSING".into();
        assert!(audit_manifest(&value).is_err());
    }
}
