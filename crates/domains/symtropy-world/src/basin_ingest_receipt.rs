// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence that an owning Basin domain evaluated exact environmental inputs.
//!
//! This receipt never performs an ingest. It can only bind evidence produced
//! around a transformation/evaluation performed elsewhere by an owning domain.

use std::{error::Error, fmt};

use sha2::{Digest, Sha256};
use symtropy_sim_contracts::{
    AuthorityId, ContractError, DigestAlgorithm, ObservationEvidence, ReferenceFrameId, ScopeId,
    SimInstant, TypedDigest32, MAX_CAUSAL_PARENTS,
};

use crate::{
    BASIN_STATE_DIGEST_DOMAIN, BASIN_STATE_SCHEMA_VERSION, EnvironmentalEvidenceBundle,
};

pub const BASIN_ENVIRONMENT_INGEST_RECEIPT_SCHEMA_VERSION: u32 = 1;
pub const BASIN_ENVIRONMENT_INGEST_RECEIPT_DOMAIN: &str =
    "symtropy.basin.environment-ingest.receipt.v1";
pub const BASIN_ENVIRONMENT_POLICY_DOMAIN_PREFIX: &str = "symtropy.basin.environment-policy.";
pub const MAX_ENVIRONMENT_OBSERVATIONS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnvironmentalObservationRole {
    Terrain,
    Hydrology,
    Climate,
    Ecology,
}

impl EnvironmentalObservationRole {
    const fn stable_code(self) -> u8 {
        match self {
            Self::Terrain => 0,
            Self::Hydrology => 1,
            Self::Climate => 2,
            Self::Ecology => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasinEnvironmentalObservation {
    pub role: EnvironmentalObservationRole,
    pub evidence: ObservationEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasinIngestEffect {
    StateChanged,
    StateUnchanged,
}

/// Provenance for one completed/evaluated environmental ingest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasinEnvironmentalIngestReceipt {
    pub schema_version: u32,
    pub basin_authority: AuthorityId,
    pub scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub at: SimInstant,
    /// Canonical semantic order is Terrain → Hydrology → Climate → Ecology,
    /// with absent domains omitted. Role tags are identity-bearing so the same
    /// evidence cannot be silently reinterpreted as a different domain input.
    pub source_observations: Vec<BasinEnvironmentalObservation>,
    pub prior_basin_state: TypedDigest32,
    pub transformation_policy: TypedDigest32,
    pub resulting_basin_state: TypedDigest32,
    pub causal_parents: Vec<TypedDigest32>,
}

impl BasinEnvironmentalIngestReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        basin_authority: AuthorityId,
        bundle: &EnvironmentalEvidenceBundle,
        prior_basin_state: TypedDigest32,
        transformation_policy: TypedDigest32,
        resulting_basin_state: TypedDigest32,
        causal_parents: Vec<TypedDigest32>,
    ) -> Result<Self, BasinEnvironmentalIngestError> {
        let mut source_observations = Vec::with_capacity(bundle.observation_count());
        if let Some(evidence) = &bundle.terrain {
            source_observations.push(BasinEnvironmentalObservation {
                role: EnvironmentalObservationRole::Terrain,
                evidence: evidence.clone(),
            });
        }
        if let Some(evidence) = &bundle.hydrology {
            source_observations.push(BasinEnvironmentalObservation {
                role: EnvironmentalObservationRole::Hydrology,
                evidence: evidence.clone(),
            });
        }
        if let Some(evidence) = &bundle.climate {
            source_observations.push(BasinEnvironmentalObservation {
                role: EnvironmentalObservationRole::Climate,
                evidence: evidence.clone(),
            });
        }
        if let Some(evidence) = &bundle.ecology {
            source_observations.push(BasinEnvironmentalObservation {
                role: EnvironmentalObservationRole::Ecology,
                evidence: evidence.clone(),
            });
        }

        let receipt = Self {
            schema_version: BASIN_ENVIRONMENT_INGEST_RECEIPT_SCHEMA_VERSION,
            basin_authority,
            scope: bundle.scope.clone(),
            reference_frame: bundle.reference_frame.clone(),
            at: bundle.observed_at,
            source_observations,
            prior_basin_state,
            transformation_policy,
            resulting_basin_state,
            causal_parents,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn effect(&self) -> BasinIngestEffect {
        if self
            .prior_basin_state
            .same_typed_value(&self.resulting_basin_state)
        {
            BasinIngestEffect::StateUnchanged
        } else {
            BasinIngestEffect::StateChanged
        }
    }

    pub fn validate(&self) -> Result<(), BasinEnvironmentalIngestError> {
        if self.schema_version != BASIN_ENVIRONMENT_INGEST_RECEIPT_SCHEMA_VERSION {
            return Err(BasinEnvironmentalIngestError::UnsupportedSchema {
                expected: BASIN_ENVIRONMENT_INGEST_RECEIPT_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        if self.source_observations.is_empty() {
            return Err(BasinEnvironmentalIngestError::NoObservations);
        }
        if self.source_observations.len() > MAX_ENVIRONMENT_OBSERVATIONS {
            return Err(BasinEnvironmentalIngestError::TooManyObservations {
                maximum: MAX_ENVIRONMENT_OBSERVATIONS,
                actual: self.source_observations.len(),
            });
        }
        if self.causal_parents.len() > MAX_CAUSAL_PARENTS {
            return Err(BasinEnvironmentalIngestError::TooManyCausalParents {
                maximum: MAX_CAUSAL_PARENTS,
                actual: self.causal_parents.len(),
            });
        }

        validate_basin_state_digest("prior", &self.prior_basin_state)?;
        validate_basin_state_digest("resulting", &self.resulting_basin_state)?;
        self.transformation_policy
            .validate()
            .map_err(BasinEnvironmentalIngestError::Contract)?;
        if !self
            .transformation_policy
            .domain
            .starts_with(BASIN_ENVIRONMENT_POLICY_DOMAIN_PREFIX)
        {
            return Err(BasinEnvironmentalIngestError::InvalidPolicyDomain(
                self.transformation_policy.domain.clone(),
            ));
        }

        let mut previous_role = None;
        for source in &self.source_observations {
            if let Some(previous) = previous_role {
                if source.role <= previous {
                    return Err(BasinEnvironmentalIngestError::NonCanonicalObservationOrder {
                        previous,
                        actual: source.role,
                    });
                }
            }
            previous_role = Some(source.role);

            let observation = &source.evidence;
            observation
                .validate()
                .map_err(BasinEnvironmentalIngestError::Contract)?;
            if observation.scope != self.scope {
                return Err(BasinEnvironmentalIngestError::ScopeMismatch {
                    expected: self.scope.clone(),
                    actual: observation.scope.clone(),
                });
            }
            if observation.reference_frame != self.reference_frame {
                return Err(BasinEnvironmentalIngestError::ReferenceFrameMismatch {
                    expected: self.reference_frame.clone(),
                    actual: observation.reference_frame.clone(),
                });
            }
            if observation.observed_at != self.at {
                return Err(BasinEnvironmentalIngestError::ObservationTimeMismatch {
                    expected: self.at,
                    actual: observation.observed_at,
                });
            }
        }

        for parent in &self.causal_parents {
            parent
                .validate()
                .map_err(BasinEnvironmentalIngestError::Contract)?;
        }

        Ok(())
    }

    /// Serializer-independent receipt identity.
    pub fn digest(&self) -> Result<TypedDigest32, BasinEnvironmentalIngestError> {
        self.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.basin.environment-ingest.receipt.v1\0");
        hash_u32(&mut hasher, self.schema_version);
        hash_string(&mut hasher, self.basin_authority.as_str());
        hash_string(&mut hasher, self.scope.as_str());
        hash_string(&mut hasher, self.reference_frame.as_str());
        hash_i64(&mut hasher, self.at.seconds_from_genesis);
        hash_u32(&mut hasher, self.at.nanos);

        hash_u64(
            &mut hasher,
            u64::try_from(self.source_observations.len()).map_err(|_| {
                BasinEnvironmentalIngestError::LengthOverflow("source-observations")
            })?,
        );
        for source in &self.source_observations {
            hash_u8(&mut hasher, source.role.stable_code());
            let evidence_digest = source
                .evidence
                .digest()
                .map_err(BasinEnvironmentalIngestError::Contract)?;
            hash_typed_digest(&mut hasher, &evidence_digest);
        }

        hash_typed_digest(&mut hasher, &self.prior_basin_state);
        hash_typed_digest(&mut hasher, &self.transformation_policy);
        hash_typed_digest(&mut hasher, &self.resulting_basin_state);

        hash_u64(
            &mut hasher,
            u64::try_from(self.causal_parents.len())
                .map_err(|_| BasinEnvironmentalIngestError::LengthOverflow("causal-parents"))?,
        );
        for parent in &self.causal_parents {
            hash_typed_digest(&mut hasher, parent);
        }

        TypedDigest32::new(
            BASIN_ENVIRONMENT_INGEST_RECEIPT_DOMAIN,
            DigestAlgorithm::Sha256,
            BASIN_ENVIRONMENT_INGEST_RECEIPT_SCHEMA_VERSION,
            hasher.finalize().into(),
        )
        .map_err(BasinEnvironmentalIngestError::Contract)
    }
}

fn validate_basin_state_digest(
    role: &'static str,
    digest: &TypedDigest32,
) -> Result<(), BasinEnvironmentalIngestError> {
    digest
        .validate()
        .map_err(BasinEnvironmentalIngestError::Contract)?;
    if digest.domain != BASIN_STATE_DIGEST_DOMAIN
        || digest.schema_version != BASIN_STATE_SCHEMA_VERSION
        || digest.algorithm != DigestAlgorithm::Sha256
    {
        return Err(BasinEnvironmentalIngestError::InvalidBasinStateDigest {
            role,
            domain: digest.domain.clone(),
            schema_version: digest.schema_version,
            algorithm: digest.algorithm.clone(),
        });
    }
    Ok(())
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    // All identity/domain strings entering a valid receipt are contract-bounded
    // well below u64::MAX, so this conversion cannot truncate in practice.
    hash_u64(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn hash_u8(hasher: &mut Sha256, value: u8) {
    hasher.update([value]);
}

fn hash_u32(hasher: &mut Sha256, value: u32) {
    hasher.update(value.to_le_bytes());
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn hash_i64(hasher: &mut Sha256, value: i64) {
    hasher.update(value.to_le_bytes());
}

fn hash_typed_digest(hasher: &mut Sha256, digest: &TypedDigest32) {
    hash_string(hasher, &digest.domain);
    match &digest.algorithm {
        DigestAlgorithm::Sha256 => hasher.update([0]),
        DigestAlgorithm::Other(name) => {
            hasher.update([255]);
            hash_string(hasher, name);
        }
    }
    hash_u32(hasher, digest.schema_version);
    hasher.update(digest.value);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BasinEnvironmentalIngestError {
    Contract(ContractError),
    UnsupportedSchema {
        expected: u32,
        actual: u32,
    },
    NoObservations,
    TooManyObservations {
        maximum: usize,
        actual: usize,
    },
    TooManyCausalParents {
        maximum: usize,
        actual: usize,
    },
    NonCanonicalObservationOrder {
        previous: EnvironmentalObservationRole,
        actual: EnvironmentalObservationRole,
    },
    ScopeMismatch {
        expected: ScopeId,
        actual: ScopeId,
    },
    ReferenceFrameMismatch {
        expected: ReferenceFrameId,
        actual: ReferenceFrameId,
    },
    ObservationTimeMismatch {
        expected: SimInstant,
        actual: SimInstant,
    },
    InvalidBasinStateDigest {
        role: &'static str,
        domain: String,
        schema_version: u32,
        algorithm: DigestAlgorithm,
    },
    InvalidPolicyDomain(String),
    LengthOverflow(&'static str),
}

impl fmt::Display for BasinEnvironmentalIngestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "environment ingest contract error: {error}"),
            Self::UnsupportedSchema { expected, actual } => write!(
                formatter,
                "unsupported Basin environment ingest receipt schema {actual}; expected {expected}"
            ),
            Self::NoObservations => write!(formatter, "environment ingest receipt has no source observations"),
            Self::TooManyObservations { maximum, actual } => write!(
                formatter,
                "environment ingest receipt has {actual} observations; maximum is {maximum}"
            ),
            Self::TooManyCausalParents { maximum, actual } => write!(
                formatter,
                "environment ingest receipt has {actual} causal parents; maximum is {maximum}"
            ),
            Self::NonCanonicalObservationOrder { previous, actual } => write!(
                formatter,
                "environment observation roles are not unique canonical order: {previous:?} then {actual:?}"
            ),
            Self::ScopeMismatch { expected, actual } => write!(
                formatter,
                "environment observation scope {actual} does not match receipt scope {expected}"
            ),
            Self::ReferenceFrameMismatch { expected, actual } => write!(
                formatter,
                "environment observation reference frame {actual} does not match receipt frame {expected}"
            ),
            Self::ObservationTimeMismatch { expected, actual } => write!(
                formatter,
                "environment observation time {actual:?} does not match receipt time {expected:?}"
            ),
            Self::InvalidBasinStateDigest {
                role,
                domain,
                schema_version,
                algorithm,
            } => write!(
                formatter,
                "invalid {role} Basin state digest: domain={domain}, schema={schema_version}, algorithm={algorithm:?}"
            ),
            Self::InvalidPolicyDomain(domain) => write!(
                formatter,
                "Basin environment policy digest domain {domain:?} must start with {BASIN_ENVIRONMENT_POLICY_DOMAIN_PREFIX:?}"
            ),
            Self::LengthOverflow(kind) => write!(formatter, "{kind} length does not fit canonical u64 encoding"),
        }
    }
}

impl Error for BasinEnvironmentalIngestError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BasinCausalStateIdentity, BodyCellIdentity, BodyId, DerivedDomainView, GridSystem,
        HexCellId, HydrologyCellSummary, PlanetCellAuthorityView, TerrainCellSummary,
    };
    use symtropy_basin::BasinWorld;
    use symtropy_sim_contracts::RepresentationId;

    fn cell() -> PlanetCellAuthorityView {
        PlanetCellAuthorityView {
            identity: BodyCellIdentity {
                id: HexCellId::new(
                    BodyId::earth(),
                    GridSystem::BodyIcosahedral,
                    7,
                    "cell-7",
                ),
                center_lat_deg: 0.0,
                center_lon_deg: 0.0,
                area_m2: 100.0,
            },
            terrain: None,
            hydrology: None,
            climate: None,
            ecology: None,
        }
    }

    fn bundle(state_suffix: &[u8]) -> EnvironmentalEvidenceBundle {
        let at = SimInstant::new(100, 0).unwrap();
        let scope = cell().identity.scope_id().unwrap();
        let terrain = DerivedDomainView::new(
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            scope.clone(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("terrain.voxel.v2").unwrap(),
            at,
            TypedDigest32::sha256("terrain.state.v2", 2, state_suffix).unwrap(),
            TerrainCellSummary {
                elevation_m: 42.0,
                slope: 0.1,
            },
        )
        .unwrap();
        let hydrology = DerivedDomainView::new(
            AuthorityId::parse("hydrology.authority.v1").unwrap(),
            scope,
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("hydrology.local-flow.v1").unwrap(),
            at,
            TypedDigest32::sha256("hydrology.state.v1", 1, b"water").unwrap(),
            HydrologyCellSummary {
                surface_water_m: 0.2,
                groundwater_m: 1.0,
                flow_accumulation: 3.0,
                salinity: 0.01,
            },
        )
        .unwrap();
        let cell = cell()
            .with_terrain(terrain)
            .unwrap()
            .with_hydrology(hydrology)
            .unwrap();
        EnvironmentalEvidenceBundle::exact_from_cell(&cell).unwrap()
    }

    fn policy(bytes: &[u8]) -> TypedDigest32 {
        TypedDigest32::sha256(
            "symtropy.basin.environment-policy.living-watershed.v1",
            1,
            bytes,
        )
        .unwrap()
    }

    fn receipt(bundle: &EnvironmentalEvidenceBundle) -> BasinEnvironmentalIngestReceipt {
        let basin = BasinWorld::old_waterworks(8, 5);
        let state = basin.causal_state_digest().unwrap();
        BasinEnvironmentalIngestReceipt::new(
            AuthorityId::parse("basin.authority.v1").unwrap(),
            bundle,
            state.clone(),
            policy(b"policy-a"),
            state,
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn unchanged_state_is_a_valid_evaluated_ingest() {
        let receipt = receipt(&bundle(b"terrain-a"));
        assert_eq!(receipt.effect(), BasinIngestEffect::StateUnchanged);
        assert_eq!(receipt.digest().unwrap(), receipt.digest().unwrap());
    }

    #[test]
    fn changed_state_is_reported_without_receipt_performing_mutation() {
        let bundle = bundle(b"terrain-a");
        let before = BasinWorld::old_waterworks(8, 5);
        let prior = before.causal_state_digest().unwrap();
        let mut after = before.clone();
        after.step();
        let resulting = after.causal_state_digest().unwrap();

        let receipt = BasinEnvironmentalIngestReceipt::new(
            AuthorityId::parse("basin.authority.v1").unwrap(),
            &bundle,
            prior,
            policy(b"policy-a"),
            resulting,
            vec![],
        )
        .unwrap();
        assert_eq!(receipt.effect(), BasinIngestEffect::StateChanged);
    }

    #[test]
    fn source_observation_is_receipt_identity_significant() {
        let a = receipt(&bundle(b"terrain-a"));
        let b = receipt(&bundle(b"terrain-b"));
        assert_ne!(a.digest().unwrap(), b.digest().unwrap());
    }

    #[test]
    fn source_roles_are_unique_and_canonical() {
        let mut receipt = receipt(&bundle(b"terrain-a"));
        assert_eq!(receipt.source_observations[0].role, EnvironmentalObservationRole::Terrain);
        assert_eq!(receipt.source_observations[1].role, EnvironmentalObservationRole::Hydrology);
        receipt.source_observations.swap(0, 1);
        assert!(matches!(
            receipt.validate(),
            Err(BasinEnvironmentalIngestError::NonCanonicalObservationOrder { .. })
        ));
    }

    #[test]
    fn policy_is_receipt_identity_significant() {
        let bundle = bundle(b"terrain-a");
        let mut a = receipt(&bundle);
        let mut b = a.clone();
        a.transformation_policy = policy(b"policy-a");
        b.transformation_policy = policy(b"policy-b");
        assert_ne!(a.digest().unwrap(), b.digest().unwrap());
    }

    #[test]
    fn wrong_basin_state_domain_is_rejected() {
        let bundle = bundle(b"terrain-a");
        let mut receipt = receipt(&bundle);
        receipt.prior_basin_state =
            TypedDigest32::sha256("symtropy.basin.metrics.v1", 1, b"metrics").unwrap();
        assert!(matches!(
            receipt.validate(),
            Err(BasinEnvironmentalIngestError::InvalidBasinStateDigest { role: "prior", .. })
        ));
    }

    #[test]
    fn wrong_policy_domain_is_rejected() {
        let bundle = bundle(b"terrain-a");
        let mut receipt = receipt(&bundle);
        receipt.transformation_policy =
            TypedDigest32::sha256("generic.policy.v1", 1, b"policy").unwrap();
        assert!(matches!(
            receipt.validate(),
            Err(BasinEnvironmentalIngestError::InvalidPolicyDomain(_))
        ));
    }

    #[test]
    fn causal_parent_order_is_identity_significant() {
        let bundle = bundle(b"terrain-a");
        let mut a = receipt(&bundle);
        let p1 = TypedDigest32::sha256("event.v1", 1, b"a").unwrap();
        let p2 = TypedDigest32::sha256("event.v1", 1, b"b").unwrap();
        a.causal_parents = vec![p1.clone(), p2.clone()];
        let mut b = a.clone();
        b.causal_parents = vec![p2, p1];
        assert_ne!(a.digest().unwrap(), b.digest().unwrap());
    }
}
