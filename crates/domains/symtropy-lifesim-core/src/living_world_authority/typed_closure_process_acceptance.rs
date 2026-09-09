// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Process-side acceptance of typed ecological closure qualifications.
//!
//! Authenticating a typed closure lineage does not prove that a canonical
//! process accepts it. This layer binds one process × information pair to one
//! explicit observable/metric/bound/horizon/aggregation contract, then matches
//! a current typed qualification only when the exact information-policy and
//! spatiotemporal sufficiency gates also pass.
//!
//! This is not yet an R2 candidate-admission certificate. Active process-set
//! authority and final #314 candidate certification remain separate gates.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::information::{
    CapabilityEvidence, ClosureDomainToken, ClosureModelVersion, EcologicalInformation, ErrorPpm,
    EvidenceLineageToken, EvidenceRequirement, ProcessKey, RepresentationKey,
};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::{InformationRegistryError, RegisteredSufficiencyReport};

use super::spatiotemporal_information::{
    CanonicalTick, SpatiotemporalFailure, SpatiotemporalPolicyAuthorityStamp,
    SpatiotemporalPolicyError, SpatiotemporalPolicyRegistry, SpatiotemporalStateContext,
    SpatiotemporalSufficiencyReport,
};
use super::typed_closure_qualification::{
    ClosureAggregationSemantics, ClosureErrorMetric, ClosureEvaluationHorizonTicks,
    ClosureObservableKey, ResolvedTypedClosureQualification, TypedClosureQualificationAuthorityStamp,
    TypedClosureQualificationError, TypedClosureQualificationRegistry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedClosureProcessAcceptanceRegistryKey {
    id: u128,
    version: u32,
}

impl TypedClosureProcessAcceptanceRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

/// Typed closure contract accepted by one canonical process for one information kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedClosureProcessAcceptance {
    process: ProcessKey,
    information: EcologicalInformation,
    observable: ClosureObservableKey,
    metric: ClosureErrorMetric,
    maximum_error_ppm: ErrorPpm,
    required_evidence_horizon: ClosureEvaluationHorizonTicks,
    aggregation: ClosureAggregationSemantics,
    required_model: Option<ClosureModelVersion>,
    required_domain: Option<ClosureDomainToken>,
}

impl TypedClosureProcessAcceptance {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        process: ProcessKey,
        information: EcologicalInformation,
        observable: ClosureObservableKey,
        metric: ClosureErrorMetric,
        maximum_error_ppm: ErrorPpm,
        required_evidence_horizon: ClosureEvaluationHorizonTicks,
        aggregation: ClosureAggregationSemantics,
        required_model: Option<ClosureModelVersion>,
        required_domain: Option<ClosureDomainToken>,
    ) -> Self {
        Self {
            process,
            information,
            observable,
            metric,
            maximum_error_ppm,
            required_evidence_horizon,
            aggregation,
            required_model,
            required_domain,
        }
    }

    pub const fn process(self) -> ProcessKey {
        self.process
    }

    pub const fn information(self) -> EcologicalInformation {
        self.information
    }

    pub const fn observable(self) -> ClosureObservableKey {
        self.observable
    }

    pub const fn metric(self) -> ClosureErrorMetric {
        self.metric
    }

    pub const fn maximum_error_ppm(self) -> ErrorPpm {
        self.maximum_error_ppm
    }

    pub const fn required_evidence_horizon(self) -> ClosureEvaluationHorizonTicks {
        self.required_evidence_horizon
    }

    pub const fn aggregation(self) -> ClosureAggregationSemantics {
        self.aggregation
    }

    pub const fn required_model(self) -> Option<ClosureModelVersion> {
        self.required_model
    }

    pub const fn required_domain(self) -> Option<ClosureDomainToken> {
        self.required_domain
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureProcessAcceptanceRegistryBuilder {
    key: TypedClosureProcessAcceptanceRegistryKey,
    acceptances: BTreeMap<(ProcessKey, EcologicalInformation), TypedClosureProcessAcceptance>,
}

impl TypedClosureProcessAcceptanceRegistryBuilder {
    pub const fn new(key: TypedClosureProcessAcceptanceRegistryKey) -> Self {
        Self {
            key,
            acceptances: BTreeMap::new(),
        }
    }

    /// V0 deliberately allows exactly one typed acceptance contract per
    /// process × information pair. Multiple alternative scientific contracts
    /// should enter under a later explicit selection/versioning layer rather
    /// than depending on insertion order.
    pub fn register(
        &mut self,
        acceptance: TypedClosureProcessAcceptance,
    ) -> Result<(), TypedClosureProcessAcceptanceError> {
        use std::collections::btree_map::Entry;

        let key = (acceptance.process(), acceptance.information());
        match self.acceptances.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(acceptance);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &acceptance => Ok(()),
            Entry::Occupied(_) => Err(
                TypedClosureProcessAcceptanceError::ConflictingAcceptanceRegistration {
                    process: key.0,
                    information: key.1,
                },
            ),
        }
    }

    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<TypedClosureProcessAcceptanceRegistry, TypedClosureProcessAcceptanceError> {
        policy
            .authority_stamp()
            .validate_registry(policy.registry())
            .map_err(TypedClosureProcessAcceptanceError::PolicyIdentity)?;

        for acceptance in self.acceptances.values().copied() {
            validate_acceptance_against_legacy_process(policy, acceptance)?;
        }

        let authority = TypedClosureProcessAcceptanceAuthorityStamp {
            key: self.key,
            information_policy_authority: policy.authority_stamp().clone(),
            acceptances: self.acceptances,
        };
        Ok(TypedClosureProcessAcceptanceRegistry { authority })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureProcessAcceptanceAuthorityStamp {
    key: TypedClosureProcessAcceptanceRegistryKey,
    information_policy_authority: InformationPolicyAuthorityStamp,
    acceptances: BTreeMap<(ProcessKey, EcologicalInformation), TypedClosureProcessAcceptance>,
}

impl TypedClosureProcessAcceptanceAuthorityStamp {
    pub const fn key(&self) -> TypedClosureProcessAcceptanceRegistryKey {
        self.key
    }

    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.information_policy_authority
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureProcessAcceptanceRegistry {
    authority: TypedClosureProcessAcceptanceAuthorityStamp,
}

impl TypedClosureProcessAcceptanceRegistry {
    pub const fn authority_stamp(&self) -> &TypedClosureProcessAcceptanceAuthorityStamp {
        &self.authority
    }

    pub fn validate_current(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), TypedClosureProcessAcceptanceError> {
        self.authority
            .information_policy_authority
            .validate_registry(policy.registry())
            .map_err(TypedClosureProcessAcceptanceError::PolicyIdentity)?;
        if policy.authority_stamp() != &self.authority.information_policy_authority {
            return Err(TypedClosureProcessAcceptanceError::PolicyAuthorityMismatch);
        }
        for acceptance in self.authority.acceptances.values().copied() {
            validate_acceptance_against_legacy_process(policy, acceptance)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn match_use(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        typed_closures: &TypedClosureQualificationRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        process: ProcessKey,
        information: EcologicalInformation,
        lineage: EvidenceLineageToken,
        representation: RepresentationKey,
        current_tick: CanonicalTick,
        state: &SpatiotemporalStateContext,
    ) -> Result<ResolvedTypedClosureUse, TypedClosureProcessAcceptanceError> {
        self.validate_current(policy)?;
        typed_closures
            .validate_current(policy)
            .map_err(TypedClosureProcessAcceptanceError::ClosureQualification)?;
        spatiotemporal
            .validate_current(policy)
            .map_err(TypedClosureProcessAcceptanceError::SpatiotemporalPolicy)?;

        let acceptance = self
            .authority
            .acceptances
            .get(&(process, information))
            .copied()
            .ok_or(TypedClosureProcessAcceptanceError::NoTypedAcceptance {
                process,
                information,
            })?;

        let qualification = typed_closures
            .resolve(policy, lineage)
            .map_err(TypedClosureProcessAcceptanceError::ClosureQualification)?;
        validate_typed_match(acceptance, &qualification)?;
        validate_representation_carries_lineage(policy, representation, information, &qualification)?;

        let information_report = policy
            .registry()
            .evaluate_registered([process], representation)
            .map_err(TypedClosureProcessAcceptanceError::InformationRegistry)?;
        if !information_report.is_sufficient() {
            return Err(TypedClosureProcessAcceptanceError::InformationInsufficient);
        }

        let spatiotemporal_report = spatiotemporal
            .evaluate(policy, [process], representation, current_tick, state)
            .map_err(TypedClosureProcessAcceptanceError::SpatiotemporalPolicy)?;
        if !spatiotemporal_report.is_sufficient() {
            return Err(TypedClosureProcessAcceptanceError::SpatiotemporalInsufficient {
                failures: spatiotemporal_report.failures().to_vec(),
            });
        }

        Ok(ResolvedTypedClosureUse {
            acceptance_authority: self.authority.clone(),
            closure_authority: typed_closures.authority_stamp().clone(),
            spatiotemporal_authority: spatiotemporal.authority_stamp().clone(),
            process,
            information,
            lineage,
            representation,
            current_tick,
            state: state.clone(),
            acceptance,
            qualification,
            information_report,
            spatiotemporal_report,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTypedClosureUse {
    acceptance_authority: TypedClosureProcessAcceptanceAuthorityStamp,
    closure_authority: TypedClosureQualificationAuthorityStamp,
    spatiotemporal_authority: SpatiotemporalPolicyAuthorityStamp,
    process: ProcessKey,
    information: EcologicalInformation,
    lineage: EvidenceLineageToken,
    representation: RepresentationKey,
    current_tick: CanonicalTick,
    state: SpatiotemporalStateContext,
    acceptance: TypedClosureProcessAcceptance,
    qualification: ResolvedTypedClosureQualification,
    information_report: RegisteredSufficiencyReport,
    spatiotemporal_report: SpatiotemporalSufficiencyReport,
}

impl ResolvedTypedClosureUse {
    pub const fn process(&self) -> ProcessKey {
        self.process
    }

    pub const fn information(&self) -> EcologicalInformation {
        self.information
    }

    pub const fn lineage(&self) -> EvidenceLineageToken {
        self.lineage
    }

    pub const fn representation(&self) -> RepresentationKey {
        self.representation
    }

    pub const fn acceptance(&self) -> TypedClosureProcessAcceptance {
        self.acceptance
    }

    pub const fn qualification(&self) -> &ResolvedTypedClosureQualification {
        &self.qualification
    }

    pub const fn information_report(&self) -> &RegisteredSufficiencyReport {
        &self.information_report
    }

    pub const fn spatiotemporal_report(&self) -> &SpatiotemporalSufficiencyReport {
        &self.spatiotemporal_report
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        registry: &TypedClosureProcessAcceptanceRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        typed_closures: &TypedClosureQualificationRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        state: &SpatiotemporalStateContext,
    ) -> Result<(), TypedClosureProcessAcceptanceError> {
        if registry.authority_stamp() != &self.acceptance_authority {
            return Err(TypedClosureProcessAcceptanceError::AcceptanceAuthorityChanged);
        }
        if typed_closures.authority_stamp() != &self.closure_authority {
            return Err(TypedClosureProcessAcceptanceError::ClosureAuthorityChanged);
        }
        if spatiotemporal.authority_stamp() != &self.spatiotemporal_authority {
            return Err(TypedClosureProcessAcceptanceError::SpatiotemporalAuthorityChanged);
        }
        if state != &self.state {
            return Err(TypedClosureProcessAcceptanceError::StateContextChanged);
        }

        let current = registry.match_use(
            policy,
            typed_closures,
            spatiotemporal,
            self.process,
            self.information,
            self.lineage,
            self.representation,
            self.current_tick,
            state,
        )?;
        if current != *self {
            return Err(TypedClosureProcessAcceptanceError::ResolvedUseStale);
        }
        Ok(())
    }
}

fn validate_acceptance_against_legacy_process(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
    acceptance: TypedClosureProcessAcceptance,
) -> Result<(), TypedClosureProcessAcceptanceError> {
    let process = policy
        .registry()
        .resolve_process(acceptance.process())
        .map_err(TypedClosureProcessAcceptanceError::InformationRegistry)?;

    let compatible = process.profile().requirements().iter().any(|requirement| {
        if requirement.information() != acceptance.information() {
            return false;
        }
        match requirement.evidence() {
            EvidenceRequirement::Exact => false,
            EvidenceRequirement::ExactOrQualifiedClosure(legacy) => {
                acceptance.maximum_error_ppm() <= legacy.max_error_ppm()
                    && legacy
                        .required_model()
                        .is_none_or(|required| acceptance.required_model() == Some(required))
                    && legacy
                        .required_domain()
                        .is_none_or(|required| acceptance.required_domain() == Some(required))
            }
        }
    });

    if !compatible {
        return Err(TypedClosureProcessAcceptanceError::TypedAcceptanceBroadensLegacyProcess {
            process: acceptance.process(),
            information: acceptance.information(),
        });
    }
    Ok(())
}

fn validate_typed_match(
    acceptance: TypedClosureProcessAcceptance,
    qualification: &ResolvedTypedClosureQualification,
) -> Result<(), TypedClosureProcessAcceptanceError> {
    let qualified = qualification.qualification();
    if qualified.observable() != acceptance.observable() {
        return Err(TypedClosureProcessAcceptanceError::ObservableMismatch);
    }
    if qualified.metric() != acceptance.metric() {
        return Err(TypedClosureProcessAcceptanceError::MetricMismatch);
    }
    if qualified.max_error_ppm() > acceptance.maximum_error_ppm() {
        return Err(TypedClosureProcessAcceptanceError::ErrorBoundTooLarge {
            accepted: acceptance.maximum_error_ppm(),
            qualified: qualified.max_error_ppm(),
        });
    }
    if qualified.horizon().get() < acceptance.required_evidence_horizon().get() {
        return Err(TypedClosureProcessAcceptanceError::EvidenceHorizonTooShort {
            required: acceptance.required_evidence_horizon(),
            qualified: qualified.horizon(),
        });
    }
    if qualified.aggregation() != acceptance.aggregation() {
        return Err(TypedClosureProcessAcceptanceError::AggregationMismatch);
    }
    if acceptance
        .required_model()
        .is_some_and(|required| qualified.evidence().model_version() != required)
    {
        return Err(TypedClosureProcessAcceptanceError::ModelMismatch);
    }
    if acceptance
        .required_domain()
        .is_some_and(|required| qualified.evidence().domain() != required)
    {
        return Err(TypedClosureProcessAcceptanceError::ClosureDomainMismatch);
    }
    Ok(())
}

fn validate_representation_carries_lineage(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
    representation: RepresentationKey,
    information: EcologicalInformation,
    qualification: &ResolvedTypedClosureQualification,
) -> Result<(), TypedClosureProcessAcceptanceError> {
    let representation = policy
        .registry()
        .resolve_representation(representation)
        .map_err(TypedClosureProcessAcceptanceError::InformationRegistry)?;
    let exact_evidence = qualification.qualification().evidence();
    let carries = representation
        .capabilities()
        .claims()
        .iter()
        .any(|(available, evidence)| {
            available.covers(information)
                && evidence.contains(&CapabilityEvidence::QualifiedClosure(exact_evidence))
        });
    if !carries {
        return Err(TypedClosureProcessAcceptanceError::RepresentationDoesNotCarryClosureLineage {
            representation: representation.capabilities().key(),
            information,
            lineage: qualification.qualification().lineage(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedClosureProcessAcceptanceError {
    PolicyIdentity(InformationPolicyIdentityError),
    PolicyAuthorityMismatch,
    InformationRegistry(InformationRegistryError),
    ClosureQualification(TypedClosureQualificationError),
    SpatiotemporalPolicy(SpatiotemporalPolicyError),
    ConflictingAcceptanceRegistration {
        process: ProcessKey,
        information: EcologicalInformation,
    },
    TypedAcceptanceBroadensLegacyProcess {
        process: ProcessKey,
        information: EcologicalInformation,
    },
    NoTypedAcceptance {
        process: ProcessKey,
        information: EcologicalInformation,
    },
    ObservableMismatch,
    MetricMismatch,
    ErrorBoundTooLarge {
        accepted: ErrorPpm,
        qualified: ErrorPpm,
    },
    EvidenceHorizonTooShort {
        required: ClosureEvaluationHorizonTicks,
        qualified: ClosureEvaluationHorizonTicks,
    },
    AggregationMismatch,
    ModelMismatch,
    ClosureDomainMismatch,
    RepresentationDoesNotCarryClosureLineage {
        representation: RepresentationKey,
        information: EcologicalInformation,
        lineage: EvidenceLineageToken,
    },
    InformationInsufficient,
    SpatiotemporalInsufficient {
        failures: Vec<SpatiotemporalFailure>,
    },
    AcceptanceAuthorityChanged,
    ClosureAuthorityChanged,
    SpatiotemporalAuthorityChanged,
    StateContextChanged,
    ResolvedUseStale,
}

impl fmt::Display for TypedClosureProcessAcceptanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PolicyIdentity(error) => write!(formatter, "information-policy identity error: {error}"),
            Self::PolicyAuthorityMismatch => write!(
                formatter,
                "typed closure process acceptance was sealed under a different exact information-policy corpus"
            ),
            Self::InformationRegistry(error) => write!(formatter, "information registry error: {error}"),
            Self::ClosureQualification(error) => write!(formatter, "typed closure qualification error: {error}"),
            Self::SpatiotemporalPolicy(error) => write!(formatter, "spatiotemporal policy error: {error}"),
            Self::ConflictingAcceptanceRegistration { process, information } => write!(
                formatter,
                "conflicting typed closure acceptance for process {process:?} / {information:?}"
            ),
            Self::TypedAcceptanceBroadensLegacyProcess { process, information } => write!(
                formatter,
                "typed closure acceptance for process {process:?} / {information:?} is not permitted by the frozen legacy process contract"
            ),
            Self::NoTypedAcceptance { process, information } => write!(
                formatter,
                "no typed closure acceptance exists for process {process:?} / {information:?}"
            ),
            Self::ObservableMismatch => write!(formatter, "closure observable does not match process acceptance"),
            Self::MetricMismatch => write!(formatter, "closure error metric does not match process acceptance"),
            Self::ErrorBoundTooLarge { accepted, qualified } => write!(
                formatter,
                "closure error {} ppm exceeds accepted {} ppm",
                qualified.get(),
                accepted.get()
            ),
            Self::EvidenceHorizonTooShort { required, qualified } => write!(
                formatter,
                "closure evidence horizon {} ticks is shorter than required {} ticks",
                qualified.get(),
                required.get()
            ),
            Self::AggregationMismatch => write!(formatter, "closure aggregation semantics do not match process acceptance"),
            Self::ModelMismatch => write!(formatter, "closure model version does not match process acceptance"),
            Self::ClosureDomainMismatch => write!(formatter, "closure applicability domain does not match process acceptance"),
            Self::RepresentationDoesNotCarryClosureLineage { representation, information, lineage } => write!(
                formatter,
                "representation {representation:?} does not carry closure lineage {} for {information:?}",
                lineage.0
            ),
            Self::InformationInsufficient => write!(formatter, "base information/evidence sufficiency failed for closure-backed process use"),
            Self::SpatiotemporalInsufficient { failures } => write!(
                formatter,
                "spatiotemporal sufficiency failed with {} typed failure(s)",
                failures.len()
            ),
            Self::AcceptanceAuthorityChanged => write!(formatter, "typed closure process-acceptance authority changed"),
            Self::ClosureAuthorityChanged => write!(formatter, "typed closure qualification authority changed"),
            Self::SpatiotemporalAuthorityChanged => write!(formatter, "spatiotemporal authority changed"),
            Self::StateContextChanged => write!(formatter, "spatiotemporal state context changed"),
            Self::ResolvedUseStale => write!(formatter, "resolved typed closure process use is stale"),
        }
    }
}

impl Error for TypedClosureProcessAcceptanceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyIdentity(error) => Some(error),
            Self::InformationRegistry(error) => Some(error),
            Self::ClosureQualification(error) => Some(error),
            Self::SpatiotemporalPolicy(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        ClosureAcceptance, ClosureDomainToken, ClosureModelVersion, EcologicalAuthorityLevel,
        ProcessInformationProfile, ProcessInformationRequirement, QualifiedClosureEvidence,
        RepresentationCapabilities,
    };
    use crate::information_registry::{
        ClosureEvidenceStatus, InformationPolicyRegistry, InformationPolicyRegistryBuilder,
        InformationPolicyRegistryKey, RegisteredClosureEvidence,
    };
    use crate::living_world_authority::spatiotemporal_information::{
        IntegrationSemantics, MaximumStateAgeTicks, MaximumUpdateIntervalTicks,
        ProcessSpatiotemporalRequirement, RepresentationSpatiotemporalCapability,
        SpatialResolutionUnits, SpatiotemporalPolicyRegistryBuilder,
        SpatiotemporalPolicyRegistryKey,
    };
    use crate::living_world_authority::typed_closure_qualification::{
        ClosureQualificationImplementationFingerprint, ClosureQualificationProfileKey,
        RelativeZeroReferencePolicy, TypedClosureQualification,
        TypedClosureQualificationRegistryBuilder, TypedClosureQualificationRegistryKey,
    };

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(6_000, 1);
    const PROCESS: ProcessKey = ProcessKey::new(6_010, 1);
    const REPRESENTATION: RepresentationKey = RepresentationKey::new(6_020, 1);
    const LINEAGE: EvidenceLineageToken = EvidenceLineageToken(6_030);
    const OBSERVABLE: ClosureObservableKey = ClosureObservableKey::new(6_040, 1);
    const OTHER_OBSERVABLE: ClosureObservableKey = ClosureObservableKey::new(6_041, 1);
    const ACCEPTANCE_REGISTRY: TypedClosureProcessAcceptanceRegistryKey =
        TypedClosureProcessAcceptanceRegistryKey::new(6_050, 1);
    const CLOSURE_REGISTRY: TypedClosureQualificationRegistryKey =
        TypedClosureQualificationRegistryKey::new(6_060, 1);
    const ST_REGISTRY: SpatiotemporalPolicyRegistryKey = SpatiotemporalPolicyRegistryKey::new(6_070, 1);

    fn evidence() -> QualifiedClosureEvidence {
        QualifiedClosureEvidence::new(
            ClosureModelVersion(1),
            ClosureDomainToken(2),
            ErrorPpm::new(10_000).unwrap(),
            LINEAGE,
        )
    }

    fn policy() -> InformationPolicyRegistry {
        let closure = ClosureAcceptance::new(
            ErrorPpm::new(20_000).unwrap(),
            Some(ClosureModelVersion(1)),
            Some(ClosureDomainToken(2)),
        );
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(
                evidence(),
                ClosureEvidenceStatus::Qualified,
            ))
            .unwrap();
        builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::closure_allowed(
                    EcologicalInformation::OccupancyDistribution,
                    closure,
                )],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                REPRESENTATION,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::OccupancyDistribution,
                    CapabilityEvidence::QualifiedClosure(evidence()),
                )],
            ))
            .unwrap();
        builder.seal()
    }

    fn typed_closures(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        observable: ClosureObservableKey,
        metric: ClosureErrorMetric,
        horizon: u64,
    ) -> TypedClosureQualificationRegistry {
        let mut builder = TypedClosureQualificationRegistryBuilder::new(CLOSURE_REGISTRY);
        builder
            .register(
                TypedClosureQualification::new(
                    evidence(),
                    observable,
                    metric,
                    ErrorPpm::new(10_000).unwrap(),
                    ClosureEvaluationHorizonTicks::new(horizon).unwrap(),
                    ClosureAggregationSemantics::MeanOverHorizon,
                    ClosureQualificationProfileKey::new(6_080, 1),
                    ClosureQualificationImplementationFingerprint::new(b"closure-model".to_vec())
                        .unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
        builder.seal(policy).unwrap()
    }

    fn spatiotemporal(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> SpatiotemporalPolicyRegistry {
        let mut builder = SpatiotemporalPolicyRegistryBuilder::new(ST_REGISTRY);
        builder
            .register_process_requirement(
                ProcessSpatiotemporalRequirement::new(
                    PROCESS,
                    EcologicalInformation::OccupancyDistribution,
                    SpatialResolutionUnits::new(5).unwrap(),
                    MaximumStateAgeTicks(2),
                    MaximumUpdateIntervalTicks::new(1).unwrap(),
                    IntegrationSemantics::Instantaneous,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        builder
            .register_representation_capability(
                RepresentationSpatiotemporalCapability::new(
                    REPRESENTATION,
                    EcologicalInformation::OccupancyDistribution,
                    SpatialResolutionUnits::new(5).unwrap(),
                    MaximumUpdateIntervalTicks::new(1).unwrap(),
                    IntegrationSemantics::Instantaneous,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        builder.seal(policy).unwrap()
    }

    fn acceptance_registry(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        observable: ClosureObservableKey,
        metric: ClosureErrorMetric,
        horizon: u64,
    ) -> TypedClosureProcessAcceptanceRegistry {
        let mut builder = TypedClosureProcessAcceptanceRegistryBuilder::new(ACCEPTANCE_REGISTRY);
        builder
            .register(TypedClosureProcessAcceptance::new(
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                observable,
                metric,
                ErrorPpm::new(15_000).unwrap(),
                ClosureEvaluationHorizonTicks::new(horizon).unwrap(),
                ClosureAggregationSemantics::MeanOverHorizon,
                Some(ClosureModelVersion(1)),
                Some(ClosureDomainToken(2)),
            ))
            .unwrap();
        builder.seal(policy).unwrap()
    }

    fn state(tick: u64) -> SpatiotemporalStateContext {
        SpatiotemporalStateContext::from_records(
            REPRESENTATION,
            [(
                EcologicalInformation::OccupancyDistribution,
                CanonicalTick(tick),
            )],
        )
        .unwrap()
    }

    fn mean_metric() -> ClosureErrorMetric {
        ClosureErrorMetric::MeanRelativePpm {
            zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
        }
    }

    #[test]
    fn matching_typed_contract_and_current_spatiotemporal_state_resolves_use() {
        let raw = policy();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let closures = typed_closures(&exact, OBSERVABLE, mean_metric(), 100);
        let st = spatiotemporal(&exact);
        let acceptance = acceptance_registry(&exact, OBSERVABLE, mean_metric(), 50);
        let state = state(10);

        let resolved = acceptance
            .match_use(
                &exact,
                &closures,
                &st,
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                LINEAGE,
                REPRESENTATION,
                CanonicalTick(10),
                &state,
            )
            .unwrap();
        assert!(resolved.information_report().is_sufficient());
        assert!(resolved.spatiotemporal_report().is_sufficient());
    }

    #[test]
    fn mean_error_evidence_cannot_satisfy_max_error_process_contract() {
        let raw = policy();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let closures = typed_closures(&exact, OBSERVABLE, mean_metric(), 100);
        let st = spatiotemporal(&exact);
        let acceptance = acceptance_registry(
            &exact,
            OBSERVABLE,
            ClosureErrorMetric::MaximumRelativePpm {
                zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
            },
            50,
        );
        assert!(matches!(
            acceptance.match_use(
                &exact,
                &closures,
                &st,
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                LINEAGE,
                REPRESENTATION,
                CanonicalTick(10),
                &state(10),
            ),
            Err(TypedClosureProcessAcceptanceError::MetricMismatch)
        ));
    }

    #[test]
    fn short_horizon_cannot_authorize_longer_process_use() {
        let raw = policy();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let closures = typed_closures(&exact, OBSERVABLE, mean_metric(), 10);
        let st = spatiotemporal(&exact);
        let acceptance = acceptance_registry(&exact, OBSERVABLE, mean_metric(), 1_000);
        assert!(matches!(
            acceptance.match_use(
                &exact,
                &closures,
                &st,
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                LINEAGE,
                REPRESENTATION,
                CanonicalTick(10),
                &state(10),
            ),
            Err(TypedClosureProcessAcceptanceError::EvidenceHorizonTooShort { .. })
        ));
    }

    #[test]
    fn different_observable_with_same_numeric_error_rejects() {
        let raw = policy();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let closures = typed_closures(&exact, OBSERVABLE, mean_metric(), 100);
        let st = spatiotemporal(&exact);
        let acceptance = acceptance_registry(&exact, OTHER_OBSERVABLE, mean_metric(), 50);
        assert!(matches!(
            acceptance.match_use(
                &exact,
                &closures,
                &st,
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                LINEAGE,
                REPRESENTATION,
                CanonicalTick(10),
                &state(10),
            ),
            Err(TypedClosureProcessAcceptanceError::ObservableMismatch)
        ));
    }

    #[test]
    fn stale_spatiotemporal_state_blocks_otherwise_matching_closure() {
        let raw = policy();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let closures = typed_closures(&exact, OBSERVABLE, mean_metric(), 100);
        let st = spatiotemporal(&exact);
        let acceptance = acceptance_registry(&exact, OBSERVABLE, mean_metric(), 50);
        assert!(matches!(
            acceptance.match_use(
                &exact,
                &closures,
                &st,
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                LINEAGE,
                REPRESENTATION,
                CanonicalTick(10),
                &state(7),
            ),
            Err(TypedClosureProcessAcceptanceError::SpatiotemporalInsufficient { .. })
        ));
    }
}
