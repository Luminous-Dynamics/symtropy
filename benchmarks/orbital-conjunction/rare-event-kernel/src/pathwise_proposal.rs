// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bind finite importance-sampling atoms to whole-horizon pathwise events.
//!
//! Each finite atom represents one shared trajectory realization across every
//! declared encounter window. Event membership is derived from the existing
//! pathwise horizon reducer's set-union outcome, never by summing per-window
//! probabilities or treating windows as independent.

use std::collections::{BTreeMap, BTreeSet};

use crate::pathwise_event::{
    HorizonContactDisposition, HorizonEventDefinition, PathWindowOutcome, PathwiseHorizonOutcome,
};
use crate::proposal_execution_conformance::{
    FiniteAdaptiveProposalConfig, FiniteProposalAtom, FiniteProposalExecutionVerifier, TICKET_SPACE,
};

/// One finite support atom containing a shared trajectory realization.
#[derive(Debug, Clone, PartialEq)]
pub struct FinitePathwiseTrajectoryAtom {
    /// Stable nonzero identity for this proposal-support atom.
    pub atom_id: [u8; 32],
    /// Per-window classifications for one shared joint trajectory realization.
    pub window_outcomes: Vec<PathWindowOutcome>,
}

/// Independently declared finite trajectory-support inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinitePathwiseSupportManifest {
    /// Opaque source identity for the frozen finite-support manifest.
    pub manifest_digest: [u8; 32],
    /// Exact expected atom IDs in strictly increasing canonical order.
    pub expected_atom_ids: Vec<[u8; 32]>,
}

/// Finite proposal support whose event flags were derived from pathwise unions.
#[derive(Debug, Clone, PartialEq)]
pub struct FinitePathwiseProposalSupport {
    /// Source-bound event-definition identity supplied by the caller.
    pub event_digest: [u8; 32],
    /// Opaque source identity for the finite-support manifest.
    pub support_manifest_digest: [u8; 32],
    /// Event identity from the frozen horizon definition.
    pub event_id: String,
    /// Event-definition revision from the frozen horizon definition.
    pub event_revision: String,
    /// One event atom for each unique shared trajectory realization.
    pub atoms: Vec<FiniteProposalAtom>,
    /// Complete horizon outcomes in the same order as atoms.
    pub horizon_outcomes: Vec<PathwiseHorizonOutcome>,
    /// Count of trajectory atoms classified as any-contact over the horizon.
    pub any_contact_atom_count: usize,
    /// Count of trajectory atoms classified as no-contact over the horizon.
    pub no_contact_atom_count: usize,
}

impl FinitePathwiseProposalSupport {
    /// Require a proposal configuration to use this exact event scope and atom semantics.
    pub fn verify_proposal_config(
        &self,
        config: &FiniteAdaptiveProposalConfig,
    ) -> Result<(), PathwiseProposalError> {
        if config.event_digest != self.event_digest {
            return Err(PathwiseProposalError::EventDigestMismatch);
        }
        if config.support_manifest_digest != self.support_manifest_digest {
            return Err(PathwiseProposalError::SupportManifestMismatch);
        }
        if config.atoms != self.atoms {
            return Err(PathwiseProposalError::ProposalAtomSemanticsMismatch);
        }
        FiniteProposalExecutionVerifier::new(config.clone())
            .map_err(|_| PathwiseProposalError::InvalidProposalConfiguration)?;
        Ok(())
    }
}

/// Errors when deriving or binding finite proposal atoms to pathwise events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathwiseProposalError {
    /// The event-definition digest is all zero.
    InvalidEventDigest,
    /// The supplied event definition has blank event or revision identity.
    InvalidEventDefinitionIdentity,
    /// The trajectory support is empty.
    EmptyTrajectorySupport,
    /// The source support manifest is malformed.
    InvalidSupportManifest,
    /// Observed trajectory atom IDs do not exactly match the frozen support manifest.
    SupportManifestMismatch,
    /// The trajectory support exceeds the finite random-ticket support limit.
    TooManyTrajectoryAtoms,
    /// An atom identity is all zero.
    InvalidAtomIdentity,
    /// Two trajectory atoms reuse one support identity.
    DuplicateAtomIdentity,
    /// Two atoms claim the same trajectory realization identity.
    DuplicateTrajectoryRealization,
    /// The pathwise reducer rejected the caller-supplied per-window outcomes.
    InvalidPathwiseClassification,
    /// The pathwise result is ambiguous and cannot be coerced into a Boolean event.
    AmbiguousPathwiseOutcome,
    /// The event digest in the sampler does not match the derived support.
    EventDigestMismatch,
    /// The sampler atom list differs from the pathwise-derived atom IDs/event flags.
    ProposalAtomSemanticsMismatch,
    /// The supplied finite proposal configuration is otherwise invalid.
    InvalidProposalConfiguration,
}

impl core::fmt::Display for PathwiseProposalError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidEventDigest => "pathwise proposal event digest is invalid",
            Self::InvalidEventDefinitionIdentity => "pathwise event definition identity is invalid",
            Self::EmptyTrajectorySupport => "finite pathwise trajectory support is empty",
            Self::InvalidSupportManifest => "finite pathwise support manifest is invalid",
            Self::SupportManifestMismatch => {
                "observed pathwise trajectory atoms differ from the support manifest"
            }
            Self::TooManyTrajectoryAtoms => {
                "finite pathwise support exceeds the ticket-space limit"
            }
            Self::InvalidAtomIdentity => "pathwise trajectory atom identity is invalid",
            Self::DuplicateAtomIdentity => "pathwise trajectory atom identity is duplicated",
            Self::DuplicateTrajectoryRealization => {
                "trajectory realization is represented more than once"
            }
            Self::InvalidPathwiseClassification => "pathwise horizon classification is invalid",
            Self::AmbiguousPathwiseOutcome => {
                "ambiguous pathwise outcome cannot be a Boolean proposal atom"
            }
            Self::EventDigestMismatch => "pathwise support and proposal event digests differ",
            Self::ProposalAtomSemanticsMismatch => {
                "proposal atom IDs or event flags differ from pathwise support"
            }
            Self::InvalidProposalConfiguration => {
                "finite defensive proposal configuration is invalid"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PathwiseProposalError {}

/// Derive finite proposal event membership from complete shared trajectory realizations.
///
/// Each atom is classified once by the existing horizon reducer. A trajectory
/// is an event atom exactly when at least one encounter window is definitely
/// in contact. A no-contact atom requires the reducer to prove no contact over
/// the complete declared window inventory. Ambiguous paths remain unresolved.
pub fn derive_finite_pathwise_proposal_support(
    event_digest: [u8; 32],
    definition: &HorizonEventDefinition,
    manifest: &FinitePathwiseSupportManifest,
    trajectories: &[FinitePathwiseTrajectoryAtom],
) -> Result<FinitePathwiseProposalSupport, PathwiseProposalError> {
    if is_zero_digest(&event_digest) {
        return Err(PathwiseProposalError::InvalidEventDigest);
    }
    if definition.event_id().trim().is_empty() || definition.revision().trim().is_empty() {
        return Err(PathwiseProposalError::InvalidEventDefinitionIdentity);
    }
    if trajectories.is_empty() {
        return Err(PathwiseProposalError::EmptyTrajectorySupport);
    }
    // Diagnose an observed zero atom as an atom-identity defect, rather than
    // misclassifying the derived expected-ID list as a malformed manifest.
    if trajectories
        .iter()
        .any(|trajectory| is_zero_digest(&trajectory.atom_id))
    {
        return Err(PathwiseProposalError::InvalidAtomIdentity);
    }
    if is_zero_digest(&manifest.manifest_digest)
        || manifest.expected_atom_ids.is_empty()
        || manifest.expected_atom_ids.iter().any(is_zero_digest)
        || manifest
            .expected_atom_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(PathwiseProposalError::InvalidSupportManifest);
    }
    if manifest.expected_atom_ids.len() > TICKET_SPACE as usize {
        return Err(PathwiseProposalError::TooManyTrajectoryAtoms);
    }
    if trajectories.len() > TICKET_SPACE as usize {
        return Err(PathwiseProposalError::TooManyTrajectoryAtoms);
    }

    let mut by_atom_id = BTreeMap::new();
    for trajectory in trajectories {
        if is_zero_digest(&trajectory.atom_id) {
            return Err(PathwiseProposalError::InvalidAtomIdentity);
        }
        if by_atom_id.insert(trajectory.atom_id, trajectory).is_some() {
            return Err(PathwiseProposalError::DuplicateAtomIdentity);
        }
    }
    if by_atom_id.len() != manifest.expected_atom_ids.len()
        || manifest
            .expected_atom_ids
            .iter()
            .any(|atom_id| !by_atom_id.contains_key(atom_id))
    {
        return Err(PathwiseProposalError::SupportManifestMismatch);
    }

    // Canonical manifest order, not caller receipt order, defines proposal atom order.
    let mut realization_ids = BTreeSet::new();
    let mut atoms = Vec::with_capacity(trajectories.len());
    let mut horizon_outcomes = Vec::with_capacity(trajectories.len());
    let mut any_contact_atom_count = 0;
    let mut no_contact_atom_count = 0;

    for atom_id in &manifest.expected_atom_ids {
        let trajectory = by_atom_id
            .get(atom_id)
            .copied()
            .ok_or(PathwiseProposalError::SupportManifestMismatch)?;
        let realization_id = trajectory
            .window_outcomes
            .first()
            .map(|outcome| outcome.realization_id.trim())
            .filter(|id| !id.is_empty())
            .ok_or(PathwiseProposalError::InvalidPathwiseClassification)?;
        if !realization_ids.insert(realization_id.to_owned()) {
            return Err(PathwiseProposalError::DuplicateTrajectoryRealization);
        }

        let horizon = definition
            .classify_realization(&trajectory.window_outcomes)
            .map_err(|_| PathwiseProposalError::InvalidPathwiseClassification)?;
        let is_event = match horizon.disposition {
            HorizonContactDisposition::DefinitelyContact => {
                any_contact_atom_count += 1;
                true
            }
            HorizonContactDisposition::DefinitelyNoContact => {
                no_contact_atom_count += 1;
                false
            }
            HorizonContactDisposition::Ambiguous => {
                return Err(PathwiseProposalError::AmbiguousPathwiseOutcome);
            }
        };

        atoms.push(FiniteProposalAtom {
            atom_id: *atom_id,
            is_event,
        });
        horizon_outcomes.push(horizon);
    }

    Ok(FinitePathwiseProposalSupport {
        event_digest,
        support_manifest_digest: manifest.manifest_digest,
        event_id: definition.event_id().to_owned(),
        event_revision: definition.revision().to_owned(),
        atoms,
        horizon_outcomes,
        any_contact_atom_count,
        no_contact_atom_count,
    })
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathwise_event::{
        EncounterWindow, HorizonEventConfig, PathWindowDisposition, WindowInventoryCoverage,
    };

    const EVENT_DIGEST: [u8; 32] = [1; 32];

    fn window(id: &str, start: f64, end: f64) -> EncounterWindow {
        EncounterWindow {
            window_id: id.to_owned(),
            start_seconds: start,
            end_seconds: end,
        }
    }

    fn definition(
        coverage: WindowInventoryCoverage,
        windows: Vec<EncounterWindow>,
    ) -> HorizonEventDefinition {
        HorizonEventDefinition::new(HorizonEventConfig {
            event_id: "pair-17-horizon".to_owned(),
            revision: "rev-4".to_owned(),
            epoch: "2026-11-25T00:00:00".to_owned(),
            time_scale: "TAI".to_owned(),
            reference_frame: "GCRF".to_owned(),
            horizon_start_seconds: 0.0,
            horizon_end_seconds: 100.0,
            hard_body_radius_m: 5.0,
            window_inventory_coverage: coverage,
            windows,
        })
        .unwrap()
    }

    fn outcome(
        realization: &str,
        window_id: &str,
        disposition: PathWindowDisposition,
    ) -> PathWindowOutcome {
        PathWindowOutcome {
            realization_id: realization.to_owned(),
            window_id: window_id.to_owned(),
            disposition,
        }
    }

    fn contact(realization: &str, window_id: &str, time: f64) -> PathWindowOutcome {
        outcome(
            realization,
            window_id,
            PathWindowDisposition::DefinitelyContact {
                first_contact_time_lower_seconds: time,
                first_contact_time_upper_seconds: time,
            },
        )
    }

    fn no_contact(realization: &str, window_id: &str) -> PathWindowOutcome {
        outcome(
            realization,
            window_id,
            PathWindowDisposition::DefinitelyNoContact,
        )
    }

    fn proposal_config(
        atoms: Vec<FiniteProposalAtom>,
        event_digest: [u8; 32],
        support_manifest_digest: [u8; 32],
    ) -> FiniteAdaptiveProposalConfig {
        let count = u16::try_from(atoms.len()).unwrap();
        assert!(count > 0 && count <= TICKET_SPACE);
        let target_mass = TICKET_SPACE / count;
        let mut masses = vec![target_mass; usize::from(count)];
        let remainder = TICKET_SPACE - target_mass * count;
        masses[0] += remainder;
        FiniteAdaptiveProposalConfig {
            support_manifest_digest,
            event_digest,
            target_measure_digest: [2; 32],
            defensive_component_digest: [3; 32],
            proposal_policy_digest: [4; 32],
            sampler_implementation_digest: [5; 32],
            rng_algorithm_digest: [6; 32],
            rng_seed_commitment_digest: [7; 32],
            branch_stream_digest: [8; 32],
            atom_stream_digest: [9; 32],
            residual_before_event_digest: [10; 32],
            residual_after_event_digest: [11; 32],
            complete_proposal_before_event_digest: [12; 32],
            complete_proposal_after_event_digest: [13; 32],
            defensive_mixture_weight_ticks: 64,
            atoms,
            target_mass_ticks: masses.clone(),
            residual_before_event_ticks: masses.clone(),
            residual_after_event_ticks: masses,
        }
    }

    fn derive_from_observed(
        event_digest: [u8; 32],
        definition: &HorizonEventDefinition,
        trajectories: &[FinitePathwiseTrajectoryAtom],
    ) -> Result<FinitePathwiseProposalSupport, PathwiseProposalError> {
        let mut expected_atom_ids = trajectories
            .iter()
            .map(|trajectory| trajectory.atom_id)
            .collect::<Vec<_>>();
        expected_atom_ids.sort_unstable();
        expected_atom_ids.dedup();
        let manifest = FinitePathwiseSupportManifest {
            manifest_digest: [200; 32],
            expected_atom_ids,
        };
        derive_finite_pathwise_proposal_support(event_digest, definition, &manifest, trajectories)
    }

    #[test]
    fn overlapping_encounter_windows_produce_one_pathwise_event_atom() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![window("early", 10.0, 30.0), window("overlap", 20.0, 40.0)],
        );
        let support = derive_from_observed(
            EVENT_DIGEST,
            &definition,
            &[FinitePathwiseTrajectoryAtom {
                atom_id: [20; 32],
                window_outcomes: vec![
                    contact("trajectory-a", "early", 25.0),
                    contact("trajectory-a", "overlap", 25.0),
                ],
            }],
        )
        .unwrap();

        assert_eq!(support.atoms.len(), 1);
        assert_eq!(support.any_contact_atom_count, 1);
        assert_eq!(support.no_contact_atom_count, 0);
        assert!(support.atoms[0].is_event);
        assert_eq!(
            support.horizon_outcomes[0].contacting_window_ids,
            vec!["early".to_owned(), "overlap".to_owned()]
        );
    }

    #[test]
    fn repeated_encounters_share_one_union_event_not_a_sum_of_window_events() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![
                window("first", 10.0, 20.0),
                window("second", 30.0, 40.0),
                window("third", 50.0, 60.0),
            ],
        );
        let support = derive_from_observed(
            EVENT_DIGEST,
            &definition,
            &[
                FinitePathwiseTrajectoryAtom {
                    atom_id: [21; 32],
                    window_outcomes: vec![
                        contact("trajectory-hit-twice", "first", 15.0),
                        no_contact("trajectory-hit-twice", "second"),
                        contact("trajectory-hit-twice", "third", 55.0),
                    ],
                },
                FinitePathwiseTrajectoryAtom {
                    atom_id: [22; 32],
                    window_outcomes: vec![
                        no_contact("trajectory-never-hits", "first"),
                        no_contact("trajectory-never-hits", "second"),
                        no_contact("trajectory-never-hits", "third"),
                    ],
                },
            ],
        )
        .unwrap();

        assert_eq!(
            support.atoms,
            vec![
                FiniteProposalAtom {
                    atom_id: [21; 32],
                    is_event: true
                },
                FiniteProposalAtom {
                    atom_id: [22; 32],
                    is_event: false
                },
            ]
        );
        assert_eq!(support.any_contact_atom_count, 1);
        assert_eq!(support.no_contact_atom_count, 1);
        assert_eq!(
            support.horizon_outcomes[0].disposition,
            HorizonContactDisposition::DefinitelyContact
        );
    }

    #[test]
    fn incomplete_inventory_no_contact_remains_ambiguous_and_is_not_mapped_to_false() {
        let definition = definition(
            WindowInventoryCoverage::KnownIncomplete,
            vec![window("observed", 10.0, 20.0)],
        );
        let result = derive_from_observed(
            EVENT_DIGEST,
            &definition,
            &[FinitePathwiseTrajectoryAtom {
                atom_id: [23; 32],
                window_outcomes: vec![no_contact("trajectory-partial", "observed")],
            }],
        );
        assert_eq!(result, Err(PathwiseProposalError::AmbiguousPathwiseOutcome));
    }

    #[test]
    fn incomplete_inventory_with_proven_contact_is_an_event_but_has_no_first_contact_claim() {
        let definition = definition(
            WindowInventoryCoverage::KnownIncomplete,
            vec![window("observed", 10.0, 20.0)],
        );
        let support = derive_from_observed(
            EVENT_DIGEST,
            &definition,
            &[FinitePathwiseTrajectoryAtom {
                atom_id: [24; 32],
                window_outcomes: vec![contact("trajectory-partial-hit", "observed", 15.0)],
            }],
        )
        .unwrap();
        assert!(support.atoms[0].is_event);
        assert_eq!(
            support.horizon_outcomes[0].first_contact_time_bounds_seconds,
            None
        );
    }

    #[test]
    fn malformed_window_inventory_and_mismatched_realization_are_rejected() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![window("a", 10.0, 20.0), window("b", 30.0, 40.0)],
        );
        assert_eq!(
            derive_from_observed(
                EVENT_DIGEST,
                &definition,
                &[FinitePathwiseTrajectoryAtom {
                    atom_id: [25; 32],
                    window_outcomes: vec![no_contact("trajectory-x", "a")],
                }],
            ),
            Err(PathwiseProposalError::InvalidPathwiseClassification)
        );
        assert_eq!(
            derive_from_observed(
                EVENT_DIGEST,
                &definition,
                &[FinitePathwiseTrajectoryAtom {
                    atom_id: [26; 32],
                    window_outcomes: vec![
                        no_contact("trajectory-x", "a"),
                        no_contact("trajectory-y", "b"),
                    ],
                }],
            ),
            Err(PathwiseProposalError::InvalidPathwiseClassification)
        );
    }

    #[test]
    fn duplicate_atom_ids_and_duplicate_realizations_are_rejected() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![window("a", 10.0, 20.0)],
        );
        let duplicate_id = [
            FinitePathwiseTrajectoryAtom {
                atom_id: [27; 32],
                window_outcomes: vec![no_contact("trajectory-a", "a")],
            },
            FinitePathwiseTrajectoryAtom {
                atom_id: [27; 32],
                window_outcomes: vec![no_contact("trajectory-b", "a")],
            },
        ];
        assert_eq!(
            derive_from_observed(EVENT_DIGEST, &definition, &duplicate_id),
            Err(PathwiseProposalError::DuplicateAtomIdentity)
        );

        let duplicate_realization = [
            FinitePathwiseTrajectoryAtom {
                atom_id: [28; 32],
                window_outcomes: vec![no_contact("trajectory-same", "a")],
            },
            FinitePathwiseTrajectoryAtom {
                atom_id: [29; 32],
                window_outcomes: vec![no_contact("trajectory-same", "a")],
            },
        ];
        assert_eq!(
            derive_from_observed(EVENT_DIGEST, &definition, &duplicate_realization),
            Err(PathwiseProposalError::DuplicateTrajectoryRealization)
        );
    }

    #[test]
    fn support_must_match_exact_proposal_event_digest_and_atom_semantics() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![window("a", 10.0, 20.0)],
        );
        let support = derive_from_observed(
            EVENT_DIGEST,
            &definition,
            &[FinitePathwiseTrajectoryAtom {
                atom_id: [30; 32],
                window_outcomes: vec![contact("trajectory-hit", "a", 15.0)],
            }],
        )
        .unwrap();
        let mut config = proposal_config(
            support.atoms.clone(),
            EVENT_DIGEST,
            support.support_manifest_digest,
        );
        support.verify_proposal_config(&config).unwrap();

        config.atoms[0].is_event = false;
        assert_eq!(
            support.verify_proposal_config(&config),
            Err(PathwiseProposalError::ProposalAtomSemanticsMismatch)
        );
        config.atoms = support.atoms.clone();
        config.event_digest = [99; 32];
        assert_eq!(
            support.verify_proposal_config(&config),
            Err(PathwiseProposalError::EventDigestMismatch)
        );
        config.event_digest = EVENT_DIGEST;
        config.support_manifest_digest = [98; 32];
        assert_eq!(
            support.verify_proposal_config(&config),
            Err(PathwiseProposalError::SupportManifestMismatch)
        );
    }

    #[test]
    fn empty_support_and_zero_event_digest_fail_closed() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![window("a", 10.0, 20.0)],
        );
        assert_eq!(
            derive_from_observed(EVENT_DIGEST, &definition, &[]),
            Err(PathwiseProposalError::EmptyTrajectorySupport)
        );
        assert_eq!(
            derive_from_observed(
                [0; 32],
                &definition,
                &[FinitePathwiseTrajectoryAtom {
                    atom_id: [31; 32],
                    window_outcomes: vec![no_contact("trajectory-none", "a")],
                }],
            ),
            Err(PathwiseProposalError::InvalidEventDigest)
        );
    }

    #[test]
    fn zero_atom_identity_is_rejected() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![window("a", 10.0, 20.0)],
        );
        assert_eq!(
            derive_from_observed(
                EVENT_DIGEST,
                &definition,
                &[FinitePathwiseTrajectoryAtom {
                    atom_id: [0; 32],
                    window_outcomes: vec![no_contact("trajectory-zero-id", "a")],
                }],
            ),
            Err(PathwiseProposalError::InvalidAtomIdentity)
        );
    }

    #[test]
    fn frozen_support_manifest_detects_omitted_atoms_and_canonicalizes_input_order() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![window("a", 10.0, 20.0)],
        );
        let manifest = FinitePathwiseSupportManifest {
            manifest_digest: [201; 32],
            expected_atom_ids: vec![[40; 32], [41; 32]],
        };
        let only_one = [FinitePathwiseTrajectoryAtom {
            atom_id: [40; 32],
            window_outcomes: vec![no_contact("trajectory-40", "a")],
        }];
        assert_eq!(
            derive_finite_pathwise_proposal_support(
                EVENT_DIGEST,
                &definition,
                &manifest,
                &only_one
            ),
            Err(PathwiseProposalError::SupportManifestMismatch)
        );

        let reversed = [
            FinitePathwiseTrajectoryAtom {
                atom_id: [41; 32],
                window_outcomes: vec![contact("trajectory-41", "a", 15.0)],
            },
            FinitePathwiseTrajectoryAtom {
                atom_id: [40; 32],
                window_outcomes: vec![no_contact("trajectory-40", "a")],
            },
        ];
        let support = derive_finite_pathwise_proposal_support(
            EVENT_DIGEST,
            &definition,
            &manifest,
            &reversed,
        )
        .unwrap();
        assert_eq!(
            support.atoms,
            vec![
                FiniteProposalAtom {
                    atom_id: [40; 32],
                    is_event: false
                },
                FiniteProposalAtom {
                    atom_id: [41; 32],
                    is_event: true
                },
            ]
        );
        assert_eq!(support.support_manifest_digest, [201; 32]);
    }

    #[test]
    fn malformed_or_noncanonical_support_manifest_is_rejected() {
        let definition = definition(
            WindowInventoryCoverage::DeclaredComplete,
            vec![window("a", 10.0, 20.0)],
        );
        let trajectory = [FinitePathwiseTrajectoryAtom {
            atom_id: [42; 32],
            window_outcomes: vec![no_contact("trajectory-42", "a")],
        }];
        let zero_digest = FinitePathwiseSupportManifest {
            manifest_digest: [0; 32],
            expected_atom_ids: vec![[42; 32]],
        };
        assert_eq!(
            derive_finite_pathwise_proposal_support(
                EVENT_DIGEST,
                &definition,
                &zero_digest,
                &trajectory
            ),
            Err(PathwiseProposalError::InvalidSupportManifest)
        );
        let unsorted = FinitePathwiseSupportManifest {
            manifest_digest: [202; 32],
            expected_atom_ids: vec![[43; 32], [42; 32]],
        };
        assert_eq!(
            derive_finite_pathwise_proposal_support(
                EVENT_DIGEST,
                &definition,
                &unsorted,
                &[
                    trajectory[0].clone(),
                    FinitePathwiseTrajectoryAtom {
                        atom_id: [43; 32],
                        window_outcomes: vec![no_contact("trajectory-43", "a")],
                    }
                ]
            ),
            Err(PathwiseProposalError::InvalidSupportManifest)
        );
    }
}
