// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test-only canonical Q2 authority universe used by cross-layer hostile fixtures.
//!
//! The fixture is intentionally tiny: one closure-backed information kind, one
//! coarse -> reference transition, one exact retained R0 start, one terminal Q2
//! observation, and one uninterrupted execution segment per lane. It exercises
//! production constructors and currentness checks without introducing any test-only
//! production authority path.

use std::collections::BTreeMap;

use crate::applicability_policy_manifest::ManifestBoundTransitionApplicabilityPolicy;
use crate::candidate_evidence_obligations::CandidateCertificationRequestSet;
use crate::information::{
    CapabilityEvidence, ClosureAcceptance, ClosureDomainToken, ClosureModelVersion,
    EcologicalAuthorityLevel, EcologicalInformation, ErrorPpm, EvidenceLineageToken,
    ProcessInformationProfile, ProcessInformationRequirement, ProcessKey,
    QualifiedClosureEvidence, RepresentationCapabilities, RepresentationKey,
};
use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
use crate::information_registry::{
    ClosureEvidenceStatus, InformationPolicyRegistry, InformationPolicyRegistryBuilder,
    InformationPolicyRegistryKey, RegisteredClosureEvidence,
};
use crate::information_transition::{
    InformationTransitionDefinition, InformationTransitionKey, InformationTransitionRegistry,
    InformationTransitionRegistryBuilder, InformationTransitionRegistryKey,
    PromotionProvenance, RetainedAuthorityKey, TransitionCapabilityClaim,
};
use crate::population::{
    CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand, PopulationState,
};
use crate::transition_applicability::{
    TransitionApplicability, TransitionApplicabilityPolicy, TransitionApplicabilityPolicyBuilder,
    TransitionApplicabilityPolicyKey,
};
use crate::transition_candidates::TransitionCandidateSearchLimits;
use crate::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

use super::closure_usage_authority::{
    ClosureUsagePolicyDefinition, ClosureUsagePolicyKey, ClosureUsagePolicyRegistry,
    ClosureUsagePolicyRegistryBuilder, ClosureUsagePolicyRegistryKey, ClosureValidationAnchorId,
    ClosureValidationAnchorRevision, MaximumClosureUseTicks,
};
use super::retained_authority::{
    RetainedAuthorityRecord, RetainedAuthorityRegistry, RetainedAuthorityRegistryBuilder,
    RetainedAuthorityRegistryKey, RetainedAuthorityStatus, RetainedContentManifest,
    RetainedResolvedCandidateRequest, RetainedStoreRevision,
};
use super::shadow_common_start::RetainedShadowStartCertificate;
use super::shadow_current_executable_continuity::{
    CurrentExecutableShadowContinuityCertificate, CurrentExecutableShadowContinuityError,
};
use super::shadow_execution_continuity::{
    PairedShadowContinuityCertificate, ShadowContinuityRunIdentity,
    ShadowContinuityStateIdentity, ShadowExecutionSegment, ShadowExecutionSegmentId,
    ShadowExecutionSegmentManifest, ShadowLaneContinuityTranscript,
};
use super::shadow_execution_lineage::{
    ShadowReferenceExecutionCertificate, ShadowReferenceExecutionRegistry,
    ShadowReferenceExecutionRegistryBuilder, ShadowReferenceRetainedStartBinding,
    ShadowReferenceRunId, ShadowReferenceRunObservedState, ShadowReferenceRunRecord,
    ShadowReferenceRunRevision, ShadowReferenceRunStatus, ShadowReferenceRunnerKey,
    ShadowReferenceRunnerProfileVersion, ShadowReferenceRunnerQualification,
    ShadowReferenceRunnerRegistryKey, ShadowReferenceRunnerStatus,
};
use super::shadow_observable_authority::{
    ObservableBoundShadowValidationCertificate, ShadowObservableAuthorityRegistry,
    ShadowObservableAuthorityRegistryBuilder, ShadowObservableExecutionCapsuleFingerprint,
    ShadowObservableExtractorKey, ShadowObservableExtractorQualification,
    ShadowObservableImplementationFingerprint, ShadowObservableQualificationProfileKey,
    ShadowObservableQualificationStatus, ShadowObservableRegistryKey,
    ShadowObservationContentManifest, ShadowObservationKey, ShadowObservationPair,
    ShadowObservationRecord, ShadowObservationRevision, ShadowObservationSourceIdentity,
};
use super::shadow_paired_execution::{
    PairedShadowExecutionCertificate, PairedShadowExecutionRegistry,
    PairedShadowExecutionRegistryBuilder, ShadowCoarseRunId, ShadowCoarseRunObservedState,
    ShadowCoarseRunRecord, ShadowCoarseRunRevision, ShadowCoarseRunStatus,
    ShadowCoarseRunnerKey, ShadowCoarseRunnerProfileVersion, ShadowCoarseRunnerQualification,
    ShadowCoarseRunnerRegistryKey, ShadowCoarseRunnerStatus,
};
use super::shadow_runner_qualification::{
    QualificationEvidenceDigest, QualificationFixtureDigest, QualificationPredicateDigest,
    QualificationProducerIdentityDigest, QualificationProviderKey, QualificationSubjectDigest,
    QualificationTranscriptDigest, QualificationVerifierPolicyKey,
    ReviewedShadowRunnerQualificationReceipt, ReviewedShadowRunnerQualificationStatus,
    ShadowRunnerBinding, ShadowRunnerQualificationPolicy, ShadowRunnerQualificationPolicyKey,
    ShadowRunnerQualificationPolicyStatus, ShadowRunnerQualificationReceiptKey,
    ShadowRunnerQualificationReceiptRevision, ShadowRunnerQualificationRegistry,
    ShadowRunnerQualificationRegistryBuilder, ShadowRunnerQualificationRegistryKey,
    ShadowRunnerSemanticContractVersion, ShadowStateManifestGrammarFingerprint,
    VerifiedQualificationProvenance,
};
use super::shadow_runner_semantic_provenance::{
    ShadowRunnerSemanticRegistry, ShadowRunnerSemanticRegistryBuilder,
    ShadowRunnerSemanticRegistryKey, VerifiedRunnerSemanticClaim,
    VerifiedRunnerSemanticClaimStatus, VerifierBoundExecutableShadowRunnerPairAuthority,
};
use super::shadow_validation::{
    ShadowEvidenceKey, ShadowEvidenceRevision, ShadowEvidenceTier,
    ShadowExecutionCapsuleFingerprint, ShadowExecutionManifest, ShadowFixedScalar,
    ShadowImplementationFingerprint, ShadowMetricEvaluatorVersion, ShadowScalarSample,
    ShadowScalarTrace, ShadowScalarUnitKey, ShadowScenarioFingerprint,
    ShadowValidationEvidenceRecord, ShadowValidationProfile, ShadowValidationProfileKey,
    ShadowValidationRegistry, ShadowValidationRegistryBuilder, ShadowValidationRegistryKey,
    ShadowValidationWindow,
};
use super::spatiotemporal_information::{
    CanonicalTick, IntegrationSemantics, MaximumStateAgeTicks, MaximumUpdateIntervalTicks,
    ProcessSpatiotemporalRequirement, RepresentationSpatiotemporalCapability,
    SpatialResolutionUnits, SpatiotemporalPolicyRegistry, SpatiotemporalPolicyRegistryBuilder,
    SpatiotemporalPolicyRegistryKey, SpatiotemporalStateContext,
};
use super::transition_domain::{
    TransitionDomainAuthorityScope, TransitionDomainEvaluationSubject,
    TransitionDomainSnapshotId, TransitionDomainSourceRevision,
};
use super::typed_closure_process_acceptance::{
    TypedClosureProcessAcceptance, TypedClosureProcessAcceptanceRegistry,
    TypedClosureProcessAcceptanceRegistryBuilder, TypedClosureProcessAcceptanceRegistryKey,
};
use super::typed_closure_qualification::{
    ClosureAggregationSemantics, ClosureErrorMetric, ClosureEvaluationHorizonTicks,
    ClosureObservableKey, ClosureQualificationImplementationFingerprint,
    ClosureQualificationProfileKey, RelativeZeroReferencePolicy, TypedClosureQualification,
    TypedClosureQualificationRegistry, TypedClosureQualificationRegistryBuilder,
    TypedClosureQualificationRegistryKey,
};

const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(9_000, 1);
const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(9_001, 1);
const APPLICABILITY: TransitionApplicabilityPolicyKey =
    TransitionApplicabilityPolicyKey::new(9_002, 1);
const PROCESS: ProcessKey = ProcessKey::new(9_010, 1);
const COARSE: RepresentationKey = RepresentationKey::new(9_020, 1);
const REFERENCE: RepresentationKey = RepresentationKey::new(9_021, 1);
const EDGE: InformationTransitionKey = InformationTransitionKey::new(9_030, 1);
const RETAINED: RetainedAuthorityKey = RetainedAuthorityKey(9_040, 1);
const LINEAGE: EvidenceLineageToken = EvidenceLineageToken(9_050);
const OBSERVABLE: ClosureObservableKey = ClosureObservableKey::new(9_060, 1);
const USAGE_POLICY: ClosureUsagePolicyKey = ClosureUsagePolicyKey::new(9_070, 1);
const Q2_EVIDENCE: ShadowEvidenceKey = ShadowEvidenceKey::new(9_080, 1);
const Q2_REVISION: ShadowEvidenceRevision = ShadowEvidenceRevision(1);
const Q2_PROFILE: ShadowValidationProfileKey = ShadowValidationProfileKey::new(9_081, 1);
const UNIT: ShadowScalarUnitKey = ShadowScalarUnitKey::new(9_082, 1);
const T0: CanonicalTick = CanonicalTick(10);
const T1: CanonicalTick = CanonicalTick(11);
const SCOPE: TransitionDomainAuthorityScope = TransitionDomainAuthorityScope::new(9_090, 1);
const SNAPSHOT: TransitionDomainSnapshotId = TransitionDomainSnapshotId(9_091);
const REFERENCE_RUN: ShadowReferenceRunId = ShadowReferenceRunId(9_100);
const COARSE_RUN: ShadowCoarseRunId = ShadowCoarseRunId(9_101);
const REFERENCE_RUNNER: ShadowReferenceRunnerKey = ShadowReferenceRunnerKey::new(9_110, 1);
const COARSE_RUNNER: ShadowCoarseRunnerKey = ShadowCoarseRunnerKey::new(9_111, 1);

fn closure() -> QualifiedClosureEvidence {
    QualifiedClosureEvidence::new(
        ClosureModelVersion(4),
        ClosureDomainToken(55),
        ErrorPpm::new(1_000).unwrap(),
        LINEAGE,
    )
}

fn population() -> PopulationState {
    PopulationState::new(
        10,
        10_000,
        CountDistribution::new(BTreeMap::from([(PopulationAgeBand::Mature, 10)])).unwrap(),
        CountDistribution::new(BTreeMap::from([(PopulationConditionBand::Stable, 10)])).unwrap(),
        CountDistribution::new(BTreeMap::from([(PopulationCell::new(0, 0, 0), 10)])).unwrap(),
    )
    .unwrap()
}

fn information_policy() -> InformationPolicyRegistry {
    let closure = closure();
    let acceptance = ClosureAcceptance::new(
        ErrorPpm::new(1_000).unwrap(),
        Some(ClosureModelVersion(4)),
        Some(ClosureDomainToken(55)),
    );
    let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
    builder
        .register_process(ProcessInformationProfile::new(
            PROCESS,
            EcologicalAuthorityLevel::Coarse,
            [ProcessInformationRequirement::closure_allowed(
                EcologicalInformation::AgeDistribution,
                acceptance,
            )],
        ))
        .unwrap();
    builder
        .register_representation(RepresentationCapabilities::new(
            COARSE,
            EcologicalAuthorityLevel::Coarse,
            [(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::QualifiedClosure(closure),
            )],
        ))
        .unwrap();
    builder
        .register_representation(RepresentationCapabilities::new(
            REFERENCE,
            EcologicalAuthorityLevel::Coarse,
            [(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::Exact,
            )],
        ))
        .unwrap();
    builder
        .register_closure_evidence(RegisteredClosureEvidence::new(
            closure,
            ClosureEvidenceStatus::Qualified,
        ))
        .unwrap();
    builder.seal()
}

fn transition_graph(policy: &InformationPolicyRegistry) -> InformationTransitionRegistry {
    let edge = InformationTransitionDefinition::new(
        EDGE,
        COARSE,
        REFERENCE,
        [(
            TransitionCapabilityClaim::new(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::Exact,
            ),
            PromotionProvenance::RetainedExact { authority: RETAINED },
        )],
        [],
    )
    .unwrap();
    let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
    builder.register_transition(edge).unwrap();
    builder.seal(policy).unwrap()
}

fn applicability_policy(graph: &InformationTransitionRegistry) -> TransitionApplicabilityPolicy {
    let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY);
    builder
        .register(EDGE, TransitionApplicability::Universal)
        .unwrap();
    builder.seal(graph).unwrap()
}

fn typed_closures(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
) -> TypedClosureQualificationRegistry {
    let qualification = TypedClosureQualification::new(
        closure(),
        OBSERVABLE,
        ClosureErrorMetric::MaximumRelativePpm {
            zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
        },
        ErrorPpm::new(1_000).unwrap(),
        ClosureEvaluationHorizonTicks::new(1).unwrap(),
        ClosureAggregationSemantics::TerminalState,
        ClosureQualificationProfileKey::new(9_120, 1),
        ClosureQualificationImplementationFingerprint::new(vec![1]).unwrap(),
    )
    .unwrap();
    let mut builder =
        TypedClosureQualificationRegistryBuilder::new(TypedClosureQualificationRegistryKey::new(
            9_121, 1,
        ));
    builder.register(qualification).unwrap();
    builder.seal(policy).unwrap()
}

fn acceptances(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
) -> TypedClosureProcessAcceptanceRegistry {
    let mut builder = TypedClosureProcessAcceptanceRegistryBuilder::new(
        TypedClosureProcessAcceptanceRegistryKey::new(9_130, 1),
    );
    builder
        .register(TypedClosureProcessAcceptance::new(
            PROCESS,
            EcologicalInformation::AgeDistribution,
            OBSERVABLE,
            ClosureErrorMetric::MaximumRelativePpm {
                zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
            },
            ErrorPpm::new(1_000).unwrap(),
            ClosureEvaluationHorizonTicks::new(1).unwrap(),
            ClosureAggregationSemantics::TerminalState,
            Some(ClosureModelVersion(4)),
            Some(ClosureDomainToken(55)),
        ))
        .unwrap();
    builder.seal(policy).unwrap()
}

fn spatiotemporal(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
) -> SpatiotemporalPolicyRegistry {
    let spatial = SpatialResolutionUnits::new(10).unwrap();
    let update = MaximumUpdateIntervalTicks::new(1).unwrap();
    let mut builder = SpatiotemporalPolicyRegistryBuilder::new(SpatiotemporalPolicyRegistryKey::new(
        9_140, 1,
    ));
    builder
        .register_process_requirement(
            ProcessSpatiotemporalRequirement::new(
                PROCESS,
                EcologicalInformation::AgeDistribution,
                spatial,
                MaximumStateAgeTicks(0),
                update,
                IntegrationSemantics::Instantaneous,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    for representation in [COARSE, REFERENCE] {
        builder
            .register_representation_capability(
                RepresentationSpatiotemporalCapability::new(
                    representation,
                    EcologicalInformation::AgeDistribution,
                    spatial,
                    update,
                    IntegrationSemantics::Instantaneous,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
    }
    builder.seal(policy).unwrap()
}

fn usage(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
    closures: &TypedClosureQualificationRegistry,
    acceptances: &TypedClosureProcessAcceptanceRegistry,
    spatiotemporal: &SpatiotemporalPolicyRegistry,
) -> ClosureUsagePolicyRegistry {
    let mut builder = ClosureUsagePolicyRegistryBuilder::new(ClosureUsagePolicyRegistryKey::new(
        9_150, 1,
    ));
    builder
        .register(ClosureUsagePolicyDefinition::new(
            USAGE_POLICY,
            PROCESS,
            EcologicalInformation::AgeDistribution,
            LINEAGE,
            COARSE,
            MaximumClosureUseTicks::new(1).unwrap(),
        ))
        .unwrap();
    builder
        .seal(policy, closures, acceptances, spatiotemporal)
        .unwrap()
}

fn state_context(representation: RepresentationKey) -> SpatiotemporalStateContext {
    SpatiotemporalStateContext::from_records(
        representation,
        [(EcologicalInformation::AgeDistribution, T1)],
    )
    .unwrap()
}

fn shadow_profile() -> ShadowValidationProfile {
    ShadowValidationProfile::new(
        Q2_PROFILE,
        USAGE_POLICY,
        REFERENCE,
        UNIT,
        ShadowMetricEvaluatorVersion(1),
        ShadowImplementationFingerprint::new(vec![31]).unwrap(),
        ShadowImplementationFingerprint::new(vec![32]).unwrap(),
        Some(ShadowExecutionCapsuleFingerprint::new(vec![33]).unwrap()),
    )
}

fn shadow_evidence(start: &PopulationState, current: &PopulationState) -> ShadowValidationEvidenceRecord {
    ShadowValidationEvidenceRecord::completed(
        Q2_EVIDENCE,
        Q2_REVISION,
        ClosureValidationAnchorId(9_160),
        ClosureValidationAnchorRevision(1),
        Q2_PROFILE,
        ShadowEvidenceTier::Q2Differential,
        ShadowValidationWindow::new(T0, T1).unwrap(),
        start,
        current,
        state_context(COARSE),
        state_context(REFERENCE),
        ShadowScenarioFingerprint::new(vec![34]).unwrap(),
        ShadowExecutionManifest::new(vec![35]).unwrap(),
        EvidenceLineageToken(9_161),
        ShadowScalarTrace::new(
            UNIT,
            [ShadowScalarSample::new(
                T1,
                ShadowFixedScalar(100),
                ShadowFixedScalar(100),
            )],
        )
        .unwrap(),
    )
    .unwrap()
}

fn shadow_registry(
    key: ShadowValidationRegistryKey,
    profile: &ShadowValidationProfile,
    evidence: &ShadowValidationEvidenceRecord,
    usage: &ClosureUsagePolicyRegistry,
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
    closures: &TypedClosureQualificationRegistry,
    acceptances: &TypedClosureProcessAcceptanceRegistry,
    spatiotemporal: &SpatiotemporalPolicyRegistry,
) -> ShadowValidationRegistry {
    let mut builder = ShadowValidationRegistryBuilder::new(key);
    builder.register_profile(profile.clone()).unwrap();
    builder.register_evidence(evidence.clone()).unwrap();
    builder
        .seal(usage, policy, closures, acceptances, spatiotemporal)
        .unwrap()
}

fn observable_registry(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
) -> ShadowObservableAuthorityRegistry {
    let coarse_extractor = ShadowObservableExtractorKey::new(9_170, 1);
    let reference_extractor = ShadowObservableExtractorKey::new(9_171, 1);
    let profile = ShadowObservableQualificationProfileKey::new(9_172, 1);
    let mut builder = ShadowObservableAuthorityRegistryBuilder::new(ShadowObservableRegistryKey::new(
        9_173, 1,
    ));
    builder
        .register_qualification(ShadowObservableExtractorQualification::new(
            coarse_extractor,
            OBSERVABLE,
            COARSE,
            UNIT,
            ShadowObservableImplementationFingerprint::new(vec![41]).unwrap(),
            profile,
            EvidenceLineageToken(9_174),
            Some(ShadowObservableExecutionCapsuleFingerprint::new(vec![42]).unwrap()),
            ShadowObservableQualificationStatus::Qualified,
        ))
        .unwrap();
    builder
        .register_qualification(ShadowObservableExtractorQualification::new(
            reference_extractor,
            OBSERVABLE,
            REFERENCE,
            UNIT,
            ShadowObservableImplementationFingerprint::new(vec![43]).unwrap(),
            profile,
            EvidenceLineageToken(9_175),
            Some(ShadowObservableExecutionCapsuleFingerprint::new(vec![44]).unwrap()),
            ShadowObservableQualificationStatus::Qualified,
        ))
        .unwrap();
    builder
        .register_observation(ShadowObservationRecord::completed(
            ShadowObservationKey::new(9_176, 1),
            ShadowObservationRevision(1),
            coarse_extractor,
            T1,
            ShadowObservationSourceIdentity::new(
                COARSE,
                SCOPE,
                SNAPSHOT,
                TransitionDomainSourceRevision(T1.0),
                ShadowObservationContentManifest::new(vec![45]).unwrap(),
            ),
            ShadowFixedScalar(100),
            EvidenceLineageToken(9_177),
        ))
        .unwrap();
    builder
        .register_observation(ShadowObservationRecord::completed(
            ShadowObservationKey::new(9_178, 1),
            ShadowObservationRevision(1),
            reference_extractor,
            T1,
            ShadowObservationSourceIdentity::new(
                REFERENCE,
                SCOPE,
                SNAPSHOT,
                TransitionDomainSourceRevision(T1.0),
                ShadowObservationContentManifest::new(vec![46]).unwrap(),
            ),
            ShadowFixedScalar(100),
            EvidenceLineageToken(9_179),
        ))
        .unwrap();
    builder.seal(policy).unwrap()
}

fn retained_record() -> RetainedAuthorityRecord {
    RetainedAuthorityRecord::new(
        RETAINED,
        REFERENCE,
        SCOPE,
        SNAPSHOT,
        RetainedStoreRevision(1),
        [TransitionCapabilityClaim::new(
            EcologicalInformation::AgeDistribution,
            CapabilityEvidence::Exact,
        )],
        RetainedContentManifest::new(vec![51]).unwrap(),
        RetainedAuthorityStatus::Retained,
    )
    .unwrap()
}

fn retained_registry(
    key: RetainedAuthorityRegistryKey,
    record: &RetainedAuthorityRecord,
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
) -> RetainedAuthorityRegistry {
    let mut builder = RetainedAuthorityRegistryBuilder::new(key);
    builder.register(record.clone()).unwrap();
    builder.seal(policy).unwrap()
}

fn reference_runner(profile: &ShadowValidationProfile) -> ShadowReferenceRunnerQualification {
    ShadowReferenceRunnerQualification::new(
        REFERENCE_RUNNER,
        ShadowReferenceRunnerProfileVersion(1),
        REFERENCE,
        profile.reference_implementation().clone(),
        profile.execution_capsule().cloned(),
        EvidenceLineageToken(9_180),
        ShadowReferenceRunnerStatus::Qualified,
    )
}

fn coarse_runner(profile: &ShadowValidationProfile) -> ShadowCoarseRunnerQualification {
    ShadowCoarseRunnerQualification::new(
        COARSE_RUNNER,
        ShadowCoarseRunnerProfileVersion(1),
        COARSE,
        profile.coarse_implementation().clone(),
        profile.execution_capsule().cloned(),
        EvidenceLineageToken(9_181),
        ShadowCoarseRunnerStatus::Qualified,
    )
}

fn provenance(subject: u8, evidence: u8) -> VerifiedQualificationProvenance {
    VerifiedQualificationProvenance::new(
        QualificationProviderKey::new(9_200, 1),
        QualificationVerifierPolicyKey::new(9_201, 1),
        QualificationProducerIdentityDigest::new(vec![61]).unwrap(),
        QualificationSubjectDigest::new(vec![subject]).unwrap(),
        QualificationPredicateDigest::new(vec![62]).unwrap(),
        QualificationEvidenceDigest::new(vec![evidence]).unwrap(),
        EvidenceLineageToken(9_202 + u128::from(evidence)),
    )
}

fn qualification_policy() -> ShadowRunnerQualificationPolicy {
    ShadowRunnerQualificationPolicy::new(
        ShadowRunnerQualificationPolicyKey::new(9_203, 1),
        QualificationProviderKey::new(9_200, 1),
        QualificationVerifierPolicyKey::new(9_201, 1),
        QualificationProducerIdentityDigest::new(vec![61]).unwrap(),
        ShadowRunnerSemanticContractVersion(1),
        QualificationPredicateDigest::new(vec![62]).unwrap(),
        QualificationFixtureDigest::new(vec![63]).unwrap(),
        ShadowStateManifestGrammarFingerprint::new(vec![64]).unwrap(),
        ShadowRunnerQualificationPolicyStatus::Current,
    )
}

fn reference_receipt(
    runner: &ShadowReferenceRunnerQualification,
) -> ReviewedShadowRunnerQualificationReceipt {
    ReviewedShadowRunnerQualificationReceipt::new(
        ShadowRunnerQualificationReceiptKey::new(9_204, 1),
        ShadowRunnerQualificationReceiptRevision(1),
        ShadowRunnerQualificationPolicyKey::new(9_203, 1),
        ShadowRunnerBinding::Reference {
            runner: runner.runner(),
            profile: runner.profile(),
            representation: runner.reference_representation(),
        },
        runner.implementation().clone(),
        runner.execution_capsule().cloned(),
        runner.qualification_evidence(),
        ShadowRunnerSemanticContractVersion(1),
        QualificationFixtureDigest::new(vec![63]).unwrap(),
        QualificationTranscriptDigest::new(vec![65]).unwrap(),
        ShadowStateManifestGrammarFingerprint::new(vec![64]).unwrap(),
        provenance(66, 67),
        ReviewedShadowRunnerQualificationStatus::Current,
    )
}

fn coarse_receipt(
    runner: &ShadowCoarseRunnerQualification,
) -> ReviewedShadowRunnerQualificationReceipt {
    ReviewedShadowRunnerQualificationReceipt::new(
        ShadowRunnerQualificationReceiptKey::new(9_205, 1),
        ShadowRunnerQualificationReceiptRevision(1),
        ShadowRunnerQualificationPolicyKey::new(9_203, 1),
        ShadowRunnerBinding::Coarse {
            runner: runner.runner(),
            profile: runner.profile(),
            representation: runner.representation(),
        },
        runner.implementation().clone(),
        runner.execution_capsule().cloned(),
        runner.qualification_evidence(),
        ShadowRunnerSemanticContractVersion(1),
        QualificationFixtureDigest::new(vec![63]).unwrap(),
        QualificationTranscriptDigest::new(vec![68]).unwrap(),
        ShadowStateManifestGrammarFingerprint::new(vec![64]).unwrap(),
        provenance(69, 70),
        ReviewedShadowRunnerQualificationStatus::Current,
    )
}

fn qualification_registry(
    key: ShadowRunnerQualificationRegistryKey,
    reference: &ReviewedShadowRunnerQualificationReceipt,
    coarse: &ReviewedShadowRunnerQualificationReceipt,
) -> ShadowRunnerQualificationRegistry {
    let mut builder = ShadowRunnerQualificationRegistryBuilder::new(key);
    builder.register_policy(qualification_policy()).unwrap();
    builder.register_receipt(reference.clone()).unwrap();
    builder.register_receipt(coarse.clone()).unwrap();
    builder.seal().unwrap()
}

fn semantic_claim(
    receipt: &ReviewedShadowRunnerQualificationReceipt,
    evidence: u128,
) -> VerifiedRunnerSemanticClaim {
    VerifiedRunnerSemanticClaim::new(
        receipt.key(),
        receipt.revision(),
        receipt.provenance().clone(),
        receipt.semantic_contract(),
        receipt.fixture().clone(),
        receipt.transcript().clone(),
        receipt.state_manifest_grammar().clone(),
        EvidenceLineageToken(evidence),
        VerifiedRunnerSemanticClaimStatus::Current,
    )
}

fn semantic_registry(
    key: ShadowRunnerSemanticRegistryKey,
    reference: &ReviewedShadowRunnerQualificationReceipt,
    coarse: &ReviewedShadowRunnerQualificationReceipt,
) -> ShadowRunnerSemanticRegistry {
    let mut builder = ShadowRunnerSemanticRegistryBuilder::new(key);
    builder
        .register_claim(semantic_claim(reference, 9_210))
        .unwrap();
    builder.register_claim(semantic_claim(coarse, 9_211)).unwrap();
    builder.seal()
}

pub(crate) struct Q2AuthorityFixture {
    information_policy: InformationPolicyRegistry,
    transition_graph: InformationTransitionRegistry,
    applicability_policy: TransitionApplicabilityPolicy,
    start_population: PopulationState,
    current_coarse_population: PopulationState,
    requests: CandidateCertificationRequestSet,
    retained: RetainedAuthorityRegistry,
    retained_record: RetainedAuthorityRecord,
    observable: ShadowObservableAuthorityRegistry,
    shadow: ShadowValidationRegistry,
    shadow_profile: ShadowValidationProfile,
    shadow_evidence: ShadowValidationEvidenceRecord,
    usage: ClosureUsagePolicyRegistry,
    closures: TypedClosureQualificationRegistry,
    acceptances: TypedClosureProcessAcceptanceRegistry,
    spatiotemporal: SpatiotemporalPolicyRegistry,
    reference_registry: ShadowReferenceExecutionRegistry,
    reference_runner: ShadowReferenceRunnerQualification,
    reference_run: ShadowReferenceRunRecord,
    paired_registry: PairedShadowExecutionRegistry,
    coarse_runner: ShadowCoarseRunnerQualification,
    coarse_run: ShadowCoarseRunRecord,
    paired: PairedShadowExecutionCertificate,
    continuity: PairedShadowContinuityCertificate,
    qualification_registry: ShadowRunnerQualificationRegistry,
    semantic_registry: ShadowRunnerSemanticRegistry,
    executable: VerifierBoundExecutableShadowRunnerPairAuthority,
    reference_receipt: ReviewedShadowRunnerQualificationReceipt,
    coarse_receipt: ReviewedShadowRunnerQualificationReceipt,
}

impl Q2AuthorityFixture {
    pub(crate) fn new() -> Self {
        let start_population = population();
        let current_coarse_population = start_population.clone();
        let information_policy = information_policy();
        let transition_graph = transition_graph(&information_policy);
        let applicability_policy = applicability_policy(&transition_graph);

        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&information_policy);
        let bound_graph =
            ManifestBoundInformationTransitionRegistry::new(&transition_graph, &bound_policy)
                .unwrap();
        let bound_applicability =
            ManifestBoundTransitionApplicabilityPolicy::new(&applicability_policy, &bound_graph)
                .unwrap();

        let structural = transition_graph
            .discover_candidates(
                &information_policy,
                COARSE,
                REFERENCE,
                TransitionCandidateSearchLimits::new(2, 4).unwrap(),
            )
            .unwrap();
        let supplied = applicability_policy
            .annotate_candidate_set(&structural)
            .unwrap();
        let requests = CandidateCertificationRequestSet::prepare(&bound_applicability, &supplied)
            .unwrap();

        let closures = typed_closures(&bound_policy);
        let acceptances = acceptances(&bound_policy);
        let spatiotemporal = spatiotemporal(&bound_policy);
        let usage = usage(&bound_policy, &closures, &acceptances, &spatiotemporal);

        let shadow_profile = shadow_profile();
        let shadow_evidence = shadow_evidence(&start_population, &current_coarse_population);
        let shadow = shadow_registry(
            ShadowValidationRegistryKey::new(9_220, 1),
            &shadow_profile,
            &shadow_evidence,
            &usage,
            &bound_policy,
            &closures,
            &acceptances,
            &spatiotemporal,
        );
        let shadow_certificate = shadow
            .certify(
                Q2_EVIDENCE,
                &usage,
                &bound_policy,
                &closures,
                &acceptances,
                &spatiotemporal,
                &current_coarse_population,
            )
            .unwrap();

        let observable = observable_registry(&bound_policy);
        let observable_bound = observable
            .certify_shadow_trace(
                &shadow_certificate,
                [ShadowObservationPair::new(
                    T1,
                    ShadowObservationKey::new(9_176, 1),
                    ShadowObservationKey::new(9_178, 1),
                )],
                &shadow,
                &usage,
                &bound_policy,
                &closures,
                &acceptances,
                &spatiotemporal,
                &current_coarse_population,
            )
            .unwrap();

        let source_subject = TransitionDomainEvaluationSubject::from_population(
            SCOPE,
            COARSE,
            SNAPSHOT,
            TransitionDomainSourceRevision(T0.0),
            &start_population,
        )
        .unwrap();
        let retained_record = retained_record();
        let retained = retained_registry(
            RetainedAuthorityRegistryKey::new(9_221, 1),
            &retained_record,
            &bound_policy,
        );
        let retained_resolution = RetainedResolvedCandidateRequest::resolve(
            &requests,
            0,
            &retained,
            &bound_policy,
            &bound_applicability,
            &source_subject,
            &start_population,
        )
        .unwrap();
        let retained_start = RetainedShadowStartCertificate::certify(
            &requests,
            &retained_resolution,
            &retained,
            &bound_applicability,
            &observable_bound,
            &observable,
            &shadow,
            &usage,
            &bound_policy,
            &closures,
            &acceptances,
            &spatiotemporal,
            &source_subject,
            &start_population,
            &current_coarse_population,
        )
        .unwrap();

        let reference_runner = reference_runner(&shadow_profile);
        let reference_observation = observable
            .resolve_observation(&bound_policy, ShadowObservationKey::new(9_178, 1))
            .unwrap();
        let reference_run = ShadowReferenceRunRecord::new(
            REFERENCE_RUN,
            ShadowReferenceRunRevision(1),
            reference_runner.runner(),
            Q2_EVIDENCE,
            Q2_REVISION,
            ShadowValidationWindow::new(T0, T1).unwrap(),
            shadow_evidence.scenario().clone(),
            shadow_evidence.execution_manifest().clone(),
            ShadowReferenceRetainedStartBinding::new(
                retained_record.authority(),
                retained_record.revision(),
                retained_record.content_manifest().clone(),
            ),
            [(T1, ShadowReferenceRunObservedState::new(
                reference_observation.observation().key(),
                reference_observation.observation().revision(),
                reference_observation.observation().source().clone(),
            ))],
            EvidenceLineageToken(9_222),
            ShadowReferenceRunStatus::Completed,
        )
        .unwrap();
        let mut reference_builder = ShadowReferenceExecutionRegistryBuilder::new(
            ShadowReferenceRunnerRegistryKey::new(9_223, 1),
        );
        reference_builder
            .register_runner(reference_runner.clone())
            .unwrap();
        reference_builder.register_run(reference_run.clone()).unwrap();
        let reference_registry = reference_builder
            .seal(&shadow, &observable, &retained)
            .unwrap();
        let reference_certificate: ShadowReferenceExecutionCertificate = reference_registry
            .certify_retained_run(
                REFERENCE_RUN,
                &retained_start,
                &requests,
                &retained,
                &bound_applicability,
                &observable,
                &shadow,
                &usage,
                &bound_policy,
                &closures,
                &acceptances,
                &spatiotemporal,
                &start_population,
                &current_coarse_population,
            )
            .unwrap();

        let coarse_runner = coarse_runner(&shadow_profile);
        let coarse_observation = observable
            .resolve_observation(&bound_policy, ShadowObservationKey::new(9_176, 1))
            .unwrap();
        let coarse_run = ShadowCoarseRunRecord::new(
            COARSE_RUN,
            ShadowCoarseRunRevision(1),
            coarse_runner.runner(),
            Q2_EVIDENCE,
            Q2_REVISION,
            ShadowValidationWindow::new(T0, T1).unwrap(),
            shadow_evidence.scenario().clone(),
            shadow_evidence.execution_manifest().clone(),
            source_subject,
            [(T1, ShadowCoarseRunObservedState::new(
                coarse_observation.observation().key(),
                coarse_observation.observation().revision(),
                coarse_observation.observation().source().clone(),
            ))],
            EvidenceLineageToken(9_224),
            ShadowCoarseRunStatus::Completed,
        )
        .unwrap();
        let mut paired_builder =
            PairedShadowExecutionRegistryBuilder::new(ShadowCoarseRunnerRegistryKey::new(
                9_225, 1,
            ));
        paired_builder.register_runner(coarse_runner.clone()).unwrap();
        paired_builder.register_run(coarse_run.clone()).unwrap();
        let paired_registry = paired_builder
            .seal(&shadow, &observable, &usage, &reference_registry)
            .unwrap();
        let paired = paired_registry
            .certify_pair(
                COARSE_RUN,
                &reference_certificate,
                &reference_registry,
                &requests,
                &retained,
                &bound_applicability,
                &observable,
                &shadow,
                &usage,
                &bound_policy,
                &closures,
                &acceptances,
                &spatiotemporal,
                &start_population,
                &current_coarse_population,
            )
            .unwrap();

        let coarse_start = ShadowContinuityStateIdentity::from_coarse_subject(coarse_run.start());
        let coarse_end = ShadowContinuityStateIdentity::from_observation_source(
            coarse_observation.observation().source(),
        );
        let coarse_transcript = ShadowLaneContinuityTranscript::new(
            ShadowContinuityRunIdentity::Coarse {
                id: coarse_run.id(),
                revision: coarse_run.revision(),
            },
            Q2_EVIDENCE,
            Q2_REVISION,
            ShadowValidationWindow::new(T0, T1).unwrap(),
            [ShadowExecutionSegment::new(
                ShadowExecutionSegmentId(9_226),
                T0,
                T1,
                coarse_start,
                coarse_end,
                ShadowExecutionSegmentManifest::new(vec![71]).unwrap(),
            )
            .unwrap()],
        )
        .unwrap();
        let reference_start = ShadowContinuityStateIdentity::from_retained_record(&retained_record);
        let reference_end = ShadowContinuityStateIdentity::from_observation_source(
            reference_observation.observation().source(),
        );
        let reference_transcript = ShadowLaneContinuityTranscript::new(
            ShadowContinuityRunIdentity::Reference {
                id: reference_run.id(),
                revision: reference_run.revision(),
            },
            Q2_EVIDENCE,
            Q2_REVISION,
            ShadowValidationWindow::new(T0, T1).unwrap(),
            [ShadowExecutionSegment::new(
                ShadowExecutionSegmentId(9_227),
                T0,
                T1,
                reference_start,
                reference_end,
                ShadowExecutionSegmentManifest::new(vec![72]).unwrap(),
            )
            .unwrap()],
        )
        .unwrap();
        let continuity = PairedShadowContinuityCertificate::certify(
            &paired,
            &coarse_transcript,
            &reference_transcript,
        )
        .unwrap();

        let reference_receipt = reference_receipt(&reference_runner);
        let coarse_receipt = coarse_receipt(&coarse_runner);
        let qualification_registry = qualification_registry(
            ShadowRunnerQualificationRegistryKey::new(9_228, 1),
            &reference_receipt,
            &coarse_receipt,
        );
        let executable_pair = qualification_registry
            .resolve_pair(&reference_runner, &coarse_runner)
            .unwrap();
        let semantic_registry = semantic_registry(
            ShadowRunnerSemanticRegistryKey::new(9_229, 1),
            &reference_receipt,
            &coarse_receipt,
        );
        let executable = semantic_registry.resolve_pair(&executable_pair).unwrap();

        Self {
            information_policy,
            transition_graph,
            applicability_policy,
            start_population,
            current_coarse_population,
            requests,
            retained,
            retained_record,
            observable,
            shadow,
            shadow_profile,
            shadow_evidence,
            usage,
            closures,
            acceptances,
            spatiotemporal,
            reference_registry,
            reference_runner,
            reference_run,
            paired_registry,
            coarse_runner,
            coarse_run,
            paired,
            continuity,
            qualification_registry,
            semantic_registry,
            executable,
            reference_receipt,
            coarse_receipt,
        }
    }

    pub(crate) fn paired(&self) -> &PairedShadowExecutionCertificate {
        &self.paired
    }

    pub(crate) fn continuity(&self) -> &PairedShadowContinuityCertificate {
        &self.continuity
    }

    pub(crate) fn certify_current(
        &self,
    ) -> Result<CurrentExecutableShadowContinuityCertificate, CurrentExecutableShadowContinuityError>
    {
        self.certify_current_with(
            &self.paired_registry,
            &self.reference_registry,
            &self.retained,
            &self.shadow,
            &self.qualification_registry,
            &self.semantic_registry,
        )
    }

    pub(crate) fn certify_current_with(
        &self,
        paired_registry: &PairedShadowExecutionRegistry,
        reference_registry: &ShadowReferenceExecutionRegistry,
        retained: &RetainedAuthorityRegistry,
        shadow: &ShadowValidationRegistry,
        qualifications: &ShadowRunnerQualificationRegistry,
        semantics: &ShadowRunnerSemanticRegistry,
    ) -> Result<CurrentExecutableShadowContinuityCertificate, CurrentExecutableShadowContinuityError>
    {
        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&self.information_policy);
        let bound_graph = ManifestBoundInformationTransitionRegistry::new(
            &self.transition_graph,
            &bound_policy,
        )
        .unwrap();
        let bound_applicability = ManifestBoundTransitionApplicabilityPolicy::new(
            &self.applicability_policy,
            &bound_graph,
        )
        .unwrap();
        CurrentExecutableShadowContinuityCertificate::certify_current(
            &self.continuity,
            &self.executable,
            paired_registry,
            reference_registry,
            &self.requests,
            retained,
            &bound_applicability,
            &self.observable,
            shadow,
            &self.usage,
            &bound_policy,
            &self.closures,
            &self.acceptances,
            &self.spatiotemporal,
            &self.start_population,
            &self.current_coarse_population,
            qualifications,
            semantics,
        )
    }

    pub(crate) fn replacement_paired_registry(&self) -> PairedShadowExecutionRegistry {
        let mut builder = PairedShadowExecutionRegistryBuilder::new(
            ShadowCoarseRunnerRegistryKey::new(9_325, 1),
        );
        builder.register_runner(self.coarse_runner.clone()).unwrap();
        builder.register_run(self.coarse_run.clone()).unwrap();
        builder
            .seal(
                &self.shadow,
                &self.observable,
                &self.usage,
                &self.reference_registry,
            )
            .unwrap()
    }

    pub(crate) fn replacement_reference_registry(&self) -> ShadowReferenceExecutionRegistry {
        let mut builder = ShadowReferenceExecutionRegistryBuilder::new(
            ShadowReferenceRunnerRegistryKey::new(9_323, 1),
        );
        builder
            .register_runner(self.reference_runner.clone())
            .unwrap();
        builder.register_run(self.reference_run.clone()).unwrap();
        builder
            .seal(&self.shadow, &self.observable, &self.retained)
            .unwrap()
    }

    pub(crate) fn replacement_retained_registry(&self) -> RetainedAuthorityRegistry {
        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&self.information_policy);
        retained_registry(
            RetainedAuthorityRegistryKey::new(9_321, 1),
            &self.retained_record,
            &bound_policy,
        )
    }

    pub(crate) fn replacement_shadow_registry(&self) -> ShadowValidationRegistry {
        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&self.information_policy);
        shadow_registry(
            ShadowValidationRegistryKey::new(9_320, 1),
            &self.shadow_profile,
            &self.shadow_evidence,
            &self.usage,
            &bound_policy,
            &self.closures,
            &self.acceptances,
            &self.spatiotemporal,
        )
    }

    pub(crate) fn replacement_qualification_registry(&self) -> ShadowRunnerQualificationRegistry {
        qualification_registry(
            ShadowRunnerQualificationRegistryKey::new(9_328, 1),
            &self.reference_receipt,
            &self.coarse_receipt,
        )
    }

    pub(crate) fn replacement_semantic_registry(&self) -> ShadowRunnerSemanticRegistry {
        semantic_registry(
            ShadowRunnerSemanticRegistryKey::new(9_329, 1),
            &self.reference_receipt,
            &self.coarse_receipt,
        )
    }

    pub(crate) fn wrong_coarse_run_transcript(&self) -> ShadowLaneContinuityTranscript {
        ShadowLaneContinuityTranscript::new(
            ShadowContinuityRunIdentity::Coarse {
                id: ShadowCoarseRunId(99_999),
                revision: self.coarse_run.revision(),
            },
            self.continuity.coarse().evidence(),
            self.continuity.coarse().evidence_revision(),
            self.continuity.coarse().window(),
            self.continuity.coarse().segments().cloned(),
        )
        .unwrap()
    }

    pub(crate) fn wrong_coarse_evidence_transcript(&self) -> ShadowLaneContinuityTranscript {
        ShadowLaneContinuityTranscript::new(
            self.continuity.coarse().run(),
            ShadowEvidenceKey::new(99_998, 1),
            self.continuity.coarse().evidence_revision(),
            self.continuity.coarse().window(),
            self.continuity.coarse().segments().cloned(),
        )
        .unwrap()
    }
}
