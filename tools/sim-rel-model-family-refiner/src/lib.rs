//! Counterexample-guided, append-only model-family refinement.
//!
//! A candidate model is admitted only if the original model family is
//! contradicted by the trace, the proposed complete joint kernel validates,
//! and the candidate assigns exact nonzero probability to that full trace.
//! Existing worlds are never removed, reordered or silently rewritten.

use sim_rel_joint_stochastic_oracle::robust::ModelWorld;
use sim_rel_model_family_auditor::{
    FamilyTraceStatus, ModelFamilyAudit, ObservedStep, audit_trace_against_worlds,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelFamilyRevision {
    pub original_world_ids: Vec<String>,
    /// Exact ordered input worlds, retained as provenance rather than ID-only references.
    pub original_world_snapshots: Vec<ModelWorld>,
    pub candidate_world_id: String,
    /// Exact candidate kernel supplied to the append-only revision.
    pub candidate_world_snapshot: ModelWorld,
    pub counterexample_trace: Vec<ObservedStep>,
    pub human_rationale: Option<String>,
    pub candidate_trace_likelihood: sim_rel_joint_stochastic_oracle::Rational,
    pub original_family_audit: ModelFamilyAudit,
    pub expanded_family_audit: ModelFamilyAudit,
}

/// Propose an append-only family revision using one independently supplied candidate.
///
/// This does not infer or promote the candidate as true. It only proves that the
/// proposed candidate is a valid additional explanation for the specific trace
/// that falsified the prior declared family.
pub fn propose_model_family_revision(
    original_worlds: &[ModelWorld],
    candidate: &ModelWorld,
    counterexample_trace: &[ObservedStep],
    human_rationale: Option<String>,
) -> Result<ModelFamilyRevision, String> {
    if let Some(rationale) = &human_rationale {
        if rationale.trim().is_empty() {
            return Err("human rationale must be non-empty when supplied".into());
        }
    }
    if candidate.id.trim().is_empty()
        || original_worlds.iter().any(|world| world.id == candidate.id)
    {
        return Err("candidate world ID must be new, non-empty and unique".into());
    }

    let original_family_audit =
        audit_trace_against_worlds(original_worlds, counterexample_trace)?;
    if !matches!(
        &original_family_audit.status,
        FamilyTraceStatus::OutOfDeclaredFamily { .. }
    ) {
        return Err("model-family revision requires a trace that falsifies every original world".into());
    }

    // The shared validator rejects incompatible semantics and malformed kernels.
    // The candidate is appended; original worlds are neither modified nor reordered.
    let mut expanded_worlds = original_worlds.to_vec();
    expanded_worlds.push(candidate.clone());
    let expanded_family_audit =
        audit_trace_against_worlds(&expanded_worlds, counterexample_trace)?;
    if expanded_family_audit.status != FamilyTraceStatus::WithinDeclaredFamily {
        return Err("candidate did not restore support for the complete counterexample trace".into());
    }

    let candidate_audit = expanded_family_audit
        .per_world
        .iter()
        .find(|audit| audit.world_id == candidate.id)
        .ok_or_else(|| "expanded audit is missing candidate world".to_string())?;
    if !candidate_audit.supports_trace() || candidate_audit.trace_likelihood.is_zero() {
        return Err("candidate must have exact nonzero likelihood for the complete trace".into());
    }

    Ok(ModelFamilyRevision {
        original_world_ids: original_worlds.iter().map(|world| world.id.clone()).collect(),
        original_world_snapshots: original_worlds.to_vec(),
        candidate_world_id: candidate.id.clone(),
        candidate_world_snapshot: candidate.clone(),
        counterexample_trace: counterexample_trace.to_vec(),
        human_rationale,
        candidate_trace_likelihood: candidate_audit.trace_likelihood,
        original_family_audit,
        expanded_family_audit,
    })
}

/// Independently replay and verify an append-only model-family revision receipt.
pub fn verify_model_family_revision(
    original_worlds: &[ModelWorld],
    candidate: &ModelWorld,
    receipt: &ModelFamilyRevision,
) -> Result<bool, String> {
    let original_ids: Vec<String> = original_worlds.iter().map(|world| world.id.clone()).collect();
    if receipt.original_world_ids != original_ids
        || receipt.original_world_snapshots.as_slice() != original_worlds
        || &receipt.candidate_world_snapshot != candidate
        || receipt.candidate_world_id != candidate.id
        || original_worlds.iter().any(|world| world.id == candidate.id)
        || receipt.human_rationale.as_ref().is_some_and(|text| text.trim().is_empty())
    {
        return Ok(false);
    }

    let original_audit =
        audit_trace_against_worlds(original_worlds, &receipt.counterexample_trace)?;
    if !matches!(
        &original_audit.status,
        FamilyTraceStatus::OutOfDeclaredFamily { .. }
    ) || original_audit != receipt.original_family_audit
    {
        return Ok(false);
    }

    let mut expanded_worlds = original_worlds.to_vec();
    expanded_worlds.push(candidate.clone());
    let expanded_audit =
        audit_trace_against_worlds(&expanded_worlds, &receipt.counterexample_trace)?;
    if expanded_audit.status != FamilyTraceStatus::WithinDeclaredFamily
        || expanded_audit != receipt.expanded_family_audit
    {
        return Ok(false);
    }

    let candidate_audit = expanded_audit
        .per_world
        .iter()
        .find(|audit| audit.world_id == candidate.id)
        .ok_or_else(|| "expanded audit is missing candidate world".to_string())?;
    Ok(candidate_audit.supports_trace()
        && !candidate_audit.trace_likelihood.is_zero()
        && candidate_audit.trace_likelihood == receipt.candidate_trace_likelihood)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_rel_joint_stochastic_oracle::{
        JointKernel, JointMass, JointOutcome, KernelRow, Rational,
    };
    use sim_rel_model_family_auditor::{ObservedOutcome, ObservedStep};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn world(id: &str, output: usize) -> ModelWorld {
        let rows = (0..2)
            .map(|hypothesis| KernelRow {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(output),
                }],
            })
            .collect();
        ModelWorld {
            id: id.to_string(),
            kernel: JointKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                action_ids: vec!["read".into()],
                observation_ids: vec!["zero".into(), "one".into()],
                action_costs: vec![1],
                initial_belief: vec![
                    JointMass {
                        hypothesis: 0,
                        physical_state: 0,
                        sensor_state: 0,
                        mass: half(),
                    },
                    JointMass {
                        hypothesis: 1,
                        physical_state: 0,
                        sensor_state: 0,
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    fn counterexample() -> Vec<ObservedStep> {
        vec![ObservedStep {
            action_id: "read".into(),
            outcome: ObservedOutcome::Observation { id: "one".into() },
        }]
    }

    #[test]
    fn candidate_is_appended_only_after_exactly_falsified_family_and_replayable() {
        let original = vec![world("old-a", 0), world("old-b", 0)];
        let candidate = world("candidate", 1);
        let receipt = propose_model_family_revision(
            &original,
            &candidate,
            &counterexample(),
            Some("Independent calibration found reversed detector polarity".into()),
        ).unwrap();

        assert_eq!(receipt.original_world_ids, vec!["old-a".to_string(), "old-b".to_string()]);
        assert_eq!(receipt.candidate_world_id, "candidate");
        assert_eq!(receipt.candidate_trace_likelihood, Rational::one());
        assert!(matches!(
            &receipt.original_family_audit.status,
            FamilyTraceStatus::OutOfDeclaredFamily { first_empty_prefix: 1 }
        ));
        assert_eq!(receipt.expanded_family_audit.supported_world_ids, vec!["candidate".to_string()]);
        assert!(verify_model_family_revision(&original, &candidate, &receipt).unwrap());
    }

    #[test]
    fn zero_likelihood_candidate_is_rejected() {
        let original = vec![world("old", 0)];
        let candidate = world("also-zero", 0);
        assert!(propose_model_family_revision(
            &original,
            &candidate,
            &counterexample(),
            None,
        ).unwrap_err().contains("candidate did not restore support"));
    }

    #[test]
    fn family_that_already_supports_trace_cannot_be_revised_as_falsified() {
        let original = vec![world("already-supports", 1)];
        let candidate = world("candidate", 1);
        assert!(propose_model_family_revision(
            &original,
            &candidate,
            &counterexample(),
            None,
        ).unwrap_err().contains("requires a trace"));
    }

    #[test]
    fn duplicate_id_incompatible_contract_and_blank_rationale_fail_closed() {
        let original = vec![world("old", 0)];
        let duplicate = world("old", 1);
        assert!(propose_model_family_revision(
            &original,
            &duplicate,
            &counterexample(),
            None,
        ).is_err());

        let mut incompatible = world("incompatible", 1);
        incompatible.kernel.action_costs[0] = 7;
        assert!(propose_model_family_revision(
            &original,
            &incompatible,
            &counterexample(),
            None,
        ).is_err());

        assert!(propose_model_family_revision(
            &original,
            &world("candidate", 1),
            &counterexample(),
            Some("  ".into()),
        ).is_err());
    }

    #[test]
    fn tampered_receipt_is_rejected() {
        let original = vec![world("old", 0)];
        let candidate = world("candidate", 1);
        let mut receipt = propose_model_family_revision(
            &original,
            &candidate,
            &counterexample(),
            None,
        ).unwrap();
        receipt.counterexample_trace.clear();
        assert!(!verify_model_family_revision(&original, &candidate, &receipt).unwrap());
    }



    #[test]
    fn changed_candidate_kernel_is_rejected_even_when_id_is_unchanged() {
        let original = vec![world("old", 0)];
        let candidate = world("candidate", 1);
        let receipt = propose_model_family_revision(
            &original,
            &candidate,
            &counterexample(),
            None,
        ).unwrap();

        let mut substituted = candidate.clone();
        substituted.kernel.rows[0].outcomes[0].probability = Rational::new(1, 2).unwrap();
        substituted.kernel.rows[0].outcomes.push(JointOutcome {
            probability: Rational::new(1, 2).unwrap(),
            next_physical_state: 0,
            next_sensor_state: 0,
            observation: Some(0),
        });
        assert!(!verify_model_family_revision(&original, &substituted, &receipt).unwrap());
    }

    #[test]
    fn reordered_or_removed_original_worlds_invalidate_receipt() {
        let original = vec![world("old-a", 0), world("old-b", 0)];
        let candidate = world("candidate", 1);
        let receipt = propose_model_family_revision(
            &original,
            &candidate,
            &counterexample(),
            None,
        ).unwrap();

        let reordered = vec![original[1].clone(), original[0].clone()];
        assert!(!verify_model_family_revision(&reordered, &candidate, &receipt).unwrap());
        let removed = vec![original[0].clone()];
        assert!(!verify_model_family_revision(&removed, &candidate, &receipt).unwrap());
    }

    #[test]
    fn tiny_nonzero_candidate_likelihood_is_preserved_exactly() {
        let original = vec![world("old", 0)];
        let mut candidate = world("rare-candidate", 0);
        let epsilon = Rational::new(1, u128::MAX).unwrap();
        let complement = Rational::one().checked_sub(epsilon).unwrap();
        for row in &mut candidate.kernel.rows {
            row.outcomes = vec![
                JointOutcome {
                    probability: complement,
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(0),
                },
                JointOutcome {
                    probability: epsilon,
                    next_physical_state: 0,
                    next_sensor_state: 0,
                    observation: Some(1),
                },
            ];
        }
        let receipt = propose_model_family_revision(
            &original,
            &candidate,
            &counterexample(),
            None,
        ).unwrap();
        assert_eq!(receipt.candidate_trace_likelihood, epsilon);
        assert!(verify_model_family_revision(&original, &candidate, &receipt).unwrap());
    }
}
