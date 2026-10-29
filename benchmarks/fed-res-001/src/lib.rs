//! Consumer-side regression checks for FED-RES-001.
//!
//! Mycelix owns federation identity, evidence, policy and authority semantics.
//! This module does not revalidate those semantics. It consumes the disposition
//! produced by the upstream Mycelix adapter and checks whether the simulation
//! boundary uses that result safely. Synthetic inputs here are not signed
//! receipts, and this is not a production authorization service.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorKind {
    HumanOrchestrator,
    RecoveryAdapter,
    Model,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionScope {
    LocalServiceRecovery,
    CrossOrganizationPlan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpstreamDecision {
    Permit,
    Deny,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkStatus {
    Connected,
    Partitioned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    /// Store an observation without claiming it is verified.
    RecordObservation,
    /// Emit a non-authoritative analytical proposal.
    ProposeAnalysis,
    /// Apply an already-authorized local service recovery action.
    ApplyLocalRecovery,
    /// Publish a recovery plan that claims cross-organization coordination.
    PublishCrossOrganizationPlan,
}

/// In an integrated build this is populated only after the Mycelix adapter
/// verifies and interprets the exact upstream receipt/profile. The synthetic
/// kernel does not verify signatures, legal authority, evidence truth, currentness,
/// or source independence itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamDisposition {
    pub receipt_ref: String,
    pub profile_ref: String,
    pub permitted_scope: ActionScope,
    pub decision: UpstreamDecision,
}

impl UpstreamDisposition {
    fn is_well_referenced(&self) -> bool {
        !self.receipt_ref.trim().is_empty() && !self.profile_ref.trim().is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub event_id: u64,
    /// Stable idempotency key for a consequential effect.
    pub idempotency_key: String,
    pub actor: ActorKind,
    pub requested_scope: Option<ActionScope>,
    /// Output of the upstream Mycelix adapter, if the effect requires it.
    pub upstream: Option<UpstreamDisposition>,
    pub link_status: LinkStatus,
    pub effect: Effect,
    /// True only if the effect actually occurred.
    pub effect_applied: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Violation {
    MissingUpstreamDisposition { event_id: u64 },
    MissingUpstreamReferences { event_id: u64 },
    UpstreamDeniedOrIndeterminate { event_id: u64 },
    ScopeMismatch { event_id: u64 },
    ModelAppliedConsequentialEffect { event_id: u64 },
    CrossOrganizationEffectDuringPartition { event_id: u64 },
    MissingIdempotencyKey { event_id: u64 },
    DuplicateAppliedEffect {
        first_event_id: u64,
        repeated_event_id: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanViolation {
    EmptyPlanId,
    ResourceBudgetExceeded {
        required: u64,
        available: u64,
    },
    MinimumServiceFloorNotMet {
        predicted_capacity: u64,
        minimum_capacity: u64,
    },
}

/// A tiny operational feasibility contract; richer scenarios belong in
/// Symtropy BANK-RES-SIM-001 and existing Mycelix logistics benchmarks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryPlan {
    pub plan_id: String,
    pub resource_units_required: u64,
    pub resource_units_available: u64,
    pub predicted_service_capacity: u64,
    pub minimum_service_capacity: u64,
}

pub fn verify_recovery_plan(plan: &RecoveryPlan) -> Vec<PlanViolation> {
    let mut violations = Vec::new();

    if plan.plan_id.trim().is_empty() {
        violations.push(PlanViolation::EmptyPlanId);
    }

    if plan.resource_units_required > plan.resource_units_available {
        violations.push(PlanViolation::ResourceBudgetExceeded {
            required: plan.resource_units_required,
            available: plan.resource_units_available,
        });
    }

    if plan.predicted_service_capacity < plan.minimum_service_capacity {
        violations.push(PlanViolation::MinimumServiceFloorNotMet {
            predicted_capacity: plan.predicted_service_capacity,
            minimum_capacity: plan.minimum_service_capacity,
        });
    }

    violations
}

fn requires_upstream(effect: Effect) -> bool {
    matches!(
        effect,
        Effect::ApplyLocalRecovery | Effect::PublishCrossOrganizationPlan
    )
}

fn required_scope(effect: Effect) -> Option<ActionScope> {
    match effect {
        Effect::ApplyLocalRecovery => Some(ActionScope::LocalServiceRecovery),
        Effect::PublishCrossOrganizationPlan => Some(ActionScope::CrossOrganizationPlan),
        Effect::RecordObservation | Effect::ProposeAnalysis => None,
    }
}

/// Check consumer-side use of upstream decisions and operational boundaries.
///
/// This function deliberately does not decide whether Mycelix should permit an
/// effect. It requires an upstream Permit for the exact effect scope, disallows
/// direct model execution, preserves local operation during partitions, and
/// detects duplicate applied effects.
pub fn verify_trace(events: &[TraceEvent]) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut applied_effects: BTreeMap<String, u64> = BTreeMap::new();

    for event in events {
        if requires_upstream(event.effect) {
            if event.actor == ActorKind::Model {
                violations.push(Violation::ModelAppliedConsequentialEffect {
                    event_id: event.event_id,
                });
            }

            let Some(upstream) = event.upstream.as_ref() else {
                violations.push(Violation::MissingUpstreamDisposition {
                    event_id: event.event_id,
                });
                continue;
            };

            if !upstream.is_well_referenced() {
                violations.push(Violation::MissingUpstreamReferences {
                    event_id: event.event_id,
                });
            }

            if upstream.decision != UpstreamDecision::Permit {
                violations.push(Violation::UpstreamDeniedOrIndeterminate {
                    event_id: event.event_id,
                });
            }

            if required_scope(event.effect) != Some(upstream.permitted_scope)
                || event.requested_scope != Some(upstream.permitted_scope)
            {
                violations.push(Violation::ScopeMismatch {
                    event_id: event.event_id,
                });
            }
        }

        // A federation partition cannot by itself veto valid local recovery.
        // It can, however, make a cross-organization publication unavailable.
        if event.effect == Effect::PublishCrossOrganizationPlan
            && event.effect_applied
            && event.link_status != LinkStatus::Connected
        {
            violations.push(Violation::CrossOrganizationEffectDuringPartition {
                event_id: event.event_id,
            });
        }

        if requires_upstream(event.effect) && event.effect_applied {
            if event.idempotency_key.trim().is_empty() {
                violations.push(Violation::MissingIdempotencyKey {
                    event_id: event.event_id,
                });
            } else if let Some(first_event_id) =
                applied_effects.get(event.idempotency_key.as_str())
            {
                violations.push(Violation::DuplicateAppliedEffect {
                    first_event_id: *first_event_id,
                    repeated_event_id: event.event_id,
                });
            } else {
                applied_effects.insert(event.idempotency_key.clone(), event.event_id);
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permit(scope: ActionScope) -> UpstreamDisposition {
        UpstreamDisposition {
            receipt_ref: "fixture://mycelix/receipt-1".to_owned(),
            profile_ref: "fixture://mycelix/profile-v1".to_owned(),
            permitted_scope: scope,
            decision: UpstreamDecision::Permit,
        }
    }

    fn event(effect: Effect, scope: Option<ActionScope>) -> TraceEvent {
        TraceEvent {
            event_id: 1,
            idempotency_key: "effect-1".to_owned(),
            actor: ActorKind::RecoveryAdapter,
            requested_scope: scope,
            upstream: scope.map(permit),
            link_status: LinkStatus::Connected,
            effect,
            effect_applied: true,
        }
    }

    fn has(violations: &[Violation], expected: &Violation) -> bool {
        violations.iter().any(|violation| violation == expected)
    }

    #[test]
    fn authorized_local_recovery_continues_during_partition() {
        let mut local = event(
            Effect::ApplyLocalRecovery,
            Some(ActionScope::LocalServiceRecovery),
        );
        local.link_status = LinkStatus::Partitioned;

        assert!(verify_trace(&[local]).is_empty());
    }

    #[test]
    fn cross_organization_publication_is_blocked_during_partition() {
        let mut shared = event(
            Effect::PublishCrossOrganizationPlan,
            Some(ActionScope::CrossOrganizationPlan),
        );
        shared.link_status = LinkStatus::Partitioned;

        assert!(has(
            &verify_trace(&[shared]),
            &Violation::CrossOrganizationEffectDuringPartition { event_id: 1 }
        ));
    }

    #[test]
    fn model_cannot_apply_authorized_effect_directly() {
        let mut model = event(
            Effect::ApplyLocalRecovery,
            Some(ActionScope::LocalServiceRecovery),
        );
        model.actor = ActorKind::Model;

        assert!(has(
            &verify_trace(&[model]),
            &Violation::ModelAppliedConsequentialEffect { event_id: 1 }
        ));
    }

    #[test]
    fn consequential_effect_requires_upstream_disposition() {
        let mut missing = event(
            Effect::ApplyLocalRecovery,
            Some(ActionScope::LocalServiceRecovery),
        );
        missing.upstream = None;

        assert!(has(
            &verify_trace(&[missing]),
            &Violation::MissingUpstreamDisposition { event_id: 1 }
        ));
    }

    #[test]
    fn denied_or_indeterminate_upstream_disposition_is_not_applied() {
        let mut denied = event(
            Effect::ApplyLocalRecovery,
            Some(ActionScope::LocalServiceRecovery),
        );
        denied.upstream.as_mut().unwrap().decision = UpstreamDecision::Indeterminate;

        assert!(has(
            &verify_trace(&[denied]),
            &Violation::UpstreamDeniedOrIndeterminate { event_id: 1 }
        ));
    }

    #[test]
    fn upstream_scope_cannot_be_reused_for_a_different_effect() {
        let mut mismatch = event(
            Effect::PublishCrossOrganizationPlan,
            Some(ActionScope::CrossOrganizationPlan),
        );
        mismatch.upstream.as_mut().unwrap().permitted_scope =
            ActionScope::LocalServiceRecovery;

        assert!(has(
            &verify_trace(&[mismatch]),
            &Violation::ScopeMismatch { event_id: 1 }
        ));
    }

    #[test]
    fn duplicate_consequential_effect_is_detected() {
        let first = event(
            Effect::ApplyLocalRecovery,
            Some(ActionScope::LocalServiceRecovery),
        );
        let mut replay = first.clone();
        replay.event_id = 2;

        assert!(has(
            &verify_trace(&[first, replay]),
            &Violation::DuplicateAppliedEffect {
                first_event_id: 1,
                repeated_event_id: 2,
            }
        ));
    }

    #[test]
    fn missing_receipt_reference_fails_consumer_gate() {
        let mut missing_ref = event(
            Effect::ApplyLocalRecovery,
            Some(ActionScope::LocalServiceRecovery),
        );
        missing_ref.upstream.as_mut().unwrap().receipt_ref.clear();

        assert!(has(
            &verify_trace(&[missing_ref]),
            &Violation::MissingUpstreamReferences { event_id: 1 }
        ));
    }

    #[test]
    fn recovery_plan_rejects_resource_overcommit() {
        let plan = RecoveryPlan {
            plan_id: "plan-1".to_owned(),
            resource_units_required: 11,
            resource_units_available: 10,
            predicted_service_capacity: 80,
            minimum_service_capacity: 60,
        };

        assert!(verify_recovery_plan(&plan).contains(
            &PlanViolation::ResourceBudgetExceeded {
                required: 11,
                available: 10,
            }
        ));
    }

    #[test]
    fn recovery_plan_rejects_below_floor_and_empty_id() {
        let plan = RecoveryPlan {
            plan_id: " ".to_owned(),
            resource_units_required: 4,
            resource_units_available: 4,
            predicted_service_capacity: 40,
            minimum_service_capacity: 60,
        };

        let violations = verify_recovery_plan(&plan);
        assert!(violations.contains(&PlanViolation::EmptyPlanId));
        assert!(violations.contains(
            &PlanViolation::MinimumServiceFloorNotMet {
                predicted_capacity: 40,
                minimum_capacity: 60,
            }
        ));
    }

    #[test]
    fn observation_and_analysis_do_not_require_an_authority_receipt() {
        let observation = TraceEvent {
            event_id: 7,
            idempotency_key: String::new(),
            actor: ActorKind::Model,
            requested_scope: None,
            upstream: None,
            link_status: LinkStatus::Partitioned,
            effect: Effect::RecordObservation,
            effect_applied: false,
        };
        let analysis = TraceEvent {
            event_id: 8,
            effect: Effect::ProposeAnalysis,
            ..observation.clone()
        };

        assert!(verify_trace(&[observation, analysis]).is_empty());
    }
}
