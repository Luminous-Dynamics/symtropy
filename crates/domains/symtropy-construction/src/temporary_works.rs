// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Plan-level contracts for temporary construction works.
//!
//! Temporary works such as braces, shoring, jigs, scaffolds, formwork, bypasses,
//! and temporary utilities are real workpieces installed and removed through the
//! normal fabrication plan. This module adds semantic purpose and validates the
//! required sequencing. It does **not** own strength, stability, load capacity,
//! functional fitness, or physical existence.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};
use symtropy_fabrication::{FabricationPlan, FabricationPlanId, PlanStepId, WorkpieceId};
use symtropy_game_state::StableId;

use crate::{ConstructionSite, ConstructionSiteId};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TemporaryWorkId(StableId);

impl TemporaryWorkId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for TemporaryWorkId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Semantic family only. Physical and engineering behavior comes from the
/// underlying workpieces, interfaces, joints, matter, and engineering evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporaryWorkKind {
    Brace,
    Shoring,
    Jig,
    Fixture,
    Scaffold,
    Formwork,
    Bypass,
    TemporaryPower,
    TemporaryCooling,
    TemporaryPumping,
    Other,
}

/// Immutable annotation over one exact fabrication-plan revision.
///
/// `installation_step_id` must causally precede every protected step. When a
/// removal step exists, every protected step must causally precede removal.
/// Temporary workpieces must participate in both installation and removal so
/// the semantic record cannot float free of real fabrication identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporaryWorkContract {
    pub id: TemporaryWorkId,
    pub plan_id: FabricationPlanId,
    pub plan_revision: u64,
    pub kind: TemporaryWorkKind,
    /// Domain-specific purpose such as `purpose:brace:pipe-alignment`.
    pub purpose_id: StableId,
    temporary_workpieces: Vec<WorkpieceId>,
    pub installation_step_id: PlanStepId,
    protected_step_ids: Vec<PlanStepId>,
    pub removal_step_id: Option<PlanStepId>,
}

impl TemporaryWorkContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TemporaryWorkId,
        plan: &FabricationPlan,
        kind: TemporaryWorkKind,
        purpose_id: StableId,
        mut temporary_workpieces: Vec<WorkpieceId>,
        installation_step_id: PlanStepId,
        mut protected_step_ids: Vec<PlanStepId>,
        removal_step_id: Option<PlanStepId>,
    ) -> Result<Self, TemporaryWorkError> {
        if temporary_workpieces.is_empty() {
            return Err(TemporaryWorkError::TemporaryWorkpieceRequired(id));
        }
        temporary_workpieces.sort();
        reject_duplicate_workpieces(&temporary_workpieces)?;

        if protected_step_ids.is_empty() {
            return Err(TemporaryWorkError::ProtectedStepRequired(id));
        }
        protected_step_ids.sort();
        reject_duplicate_steps(&protected_step_ids)?;

        let installation = plan
            .step(&installation_step_id)
            .ok_or_else(|| TemporaryWorkError::UnknownStep(installation_step_id.clone()))?;
        require_step_workpieces(
            &installation_step_id,
            installation.workpieces(),
            &temporary_workpieces,
        )?;

        if protected_step_ids.contains(&installation_step_id) {
            return Err(TemporaryWorkError::InstallationAlsoProtected(
                installation_step_id,
            ));
        }

        for protected in &protected_step_ids {
            if plan.step(protected).is_none() {
                return Err(TemporaryWorkError::UnknownStep(protected.clone()));
            }
            if !reaches(plan, &installation_step_id, protected) {
                return Err(TemporaryWorkError::InstallationDoesNotPrecede {
                    installation: installation_step_id.clone(),
                    protected: protected.clone(),
                });
            }
        }

        if let Some(removal_step_id) = &removal_step_id {
            if removal_step_id == &installation_step_id {
                return Err(TemporaryWorkError::RemovalEqualsInstallation(
                    removal_step_id.clone(),
                ));
            }
            if protected_step_ids.contains(removal_step_id) {
                return Err(TemporaryWorkError::RemovalAlsoProtected(
                    removal_step_id.clone(),
                ));
            }
            let removal = plan
                .step(removal_step_id)
                .ok_or_else(|| TemporaryWorkError::UnknownStep(removal_step_id.clone()))?;
            require_step_workpieces(
                removal_step_id,
                removal.workpieces(),
                &temporary_workpieces,
            )?;

            for protected in &protected_step_ids {
                if !reaches(plan, protected, removal_step_id) {
                    return Err(TemporaryWorkError::ProtectedDoesNotPrecedeRemoval {
                        protected: protected.clone(),
                        removal: removal_step_id.clone(),
                    });
                }
            }
        }

        Ok(Self {
            id,
            plan_id: plan.id.clone(),
            plan_revision: plan.revision,
            kind,
            purpose_id,
            temporary_workpieces,
            installation_step_id,
            protected_step_ids,
            removal_step_id,
        })
    }

    pub fn temporary_workpieces(&self) -> &[WorkpieceId] {
        &self.temporary_workpieces
    }

    pub fn protected_step_ids(&self) -> &[PlanStepId] {
        &self.protected_step_ids
    }

    pub fn validate_plan(&self, plan: &FabricationPlan) -> Result<(), TemporaryWorkError> {
        if self.plan_id != plan.id || self.plan_revision != plan.revision {
            return Err(TemporaryWorkError::PlanMismatch {
                expected_id: self.plan_id.clone(),
                expected_revision: self.plan_revision,
                actual_id: plan.id.clone(),
                actual_revision: plan.revision,
            });
        }
        Self::new(
            self.id.clone(),
            plan,
            self.kind,
            self.purpose_id.clone(),
            self.temporary_workpieces.clone(),
            self.installation_step_id.clone(),
            self.protected_step_ids.clone(),
            self.removal_step_id.clone(),
        )?;
        Ok(())
    }

    /// Derives sequencing state only from C1's evidence-backed completed steps.
    /// `InstallationRecorded` means the plan's installation process completed;
    /// it is explicitly **not** a claim that engineering capacity is adequate.
    pub fn sequencing_state(
        &self,
        site: &ConstructionSite,
        plan: &FabricationPlan,
    ) -> Result<TemporaryWorkSequencingState, TemporaryWorkError> {
        self.validate_context(site, plan)?;
        let completed = site.completed_step_ids().into_iter().collect::<BTreeSet<_>>();
        let install_completed = completed.contains(&self.installation_step_id);
        let removal_completed = self
            .removal_step_id
            .as_ref()
            .is_some_and(|step| completed.contains(step));

        if removal_completed && !install_completed {
            return Err(TemporaryWorkError::ImpossibleSiteState {
                contract_id: self.id.clone(),
                reason: "removal_recorded_before_installation".into(),
            });
        }
        if self
            .protected_step_ids
            .iter()
            .any(|step| completed.contains(step))
            && !install_completed
        {
            return Err(TemporaryWorkError::ImpossibleSiteState {
                contract_id: self.id.clone(),
                reason: "protected_work_recorded_before_installation".into(),
            });
        }
        if removal_completed
            && self
                .protected_step_ids
                .iter()
                .any(|step| !completed.contains(step))
        {
            return Err(TemporaryWorkError::ImpossibleSiteState {
                contract_id: self.id.clone(),
                reason: "removal_recorded_before_all_protected_work".into(),
            });
        }

        Ok(if removal_completed {
            TemporaryWorkSequencingState::RemovalRecorded
        } else if install_completed {
            TemporaryWorkSequencingState::InstallationRecorded
        } else {
            TemporaryWorkSequencingState::AwaitingInstallation
        })
    }

    /// Answers only whether sequencing evidence says this temporary work has
    /// been installed and not yet removed for the requested protected step.
    /// Structural or functional adequacy must be established elsewhere.
    pub fn installation_recorded_for_step(
        &self,
        site: &ConstructionSite,
        plan: &FabricationPlan,
        step_id: &PlanStepId,
    ) -> Result<bool, TemporaryWorkError> {
        if !self.protected_step_ids.contains(step_id) {
            return Err(TemporaryWorkError::StepNotProtected {
                contract_id: self.id.clone(),
                step_id: step_id.clone(),
            });
        }
        Ok(matches!(
            self.sequencing_state(site, plan)?,
            TemporaryWorkSequencingState::InstallationRecorded
        ))
    }

    fn validate_context(
        &self,
        site: &ConstructionSite,
        plan: &FabricationPlan,
    ) -> Result<(), TemporaryWorkError> {
        self.validate_plan(plan)?;
        if site.plan_id != self.plan_id || site.plan_revision != self.plan_revision {
            return Err(TemporaryWorkError::SitePlanMismatch {
                site_id: site.id.clone(),
                expected_id: self.plan_id.clone(),
                expected_revision: self.plan_revision,
                actual_id: site.plan_id.clone(),
                actual_revision: site.plan_revision,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporaryWorkSequencingState {
    AwaitingInstallation,
    InstallationRecorded,
    RemovalRecorded,
}

fn require_step_workpieces(
    step_id: &PlanStepId,
    actual: &[WorkpieceId],
    required: &[WorkpieceId],
) -> Result<(), TemporaryWorkError> {
    for workpiece in required {
        if !actual.contains(workpiece) {
            return Err(TemporaryWorkError::TemporaryWorkpieceMissingFromStep {
                step_id: step_id.clone(),
                workpiece_id: workpiece.clone(),
            });
        }
    }
    Ok(())
}

fn reject_duplicate_workpieces(workpieces: &[WorkpieceId]) -> Result<(), TemporaryWorkError> {
    for pair in workpieces.windows(2) {
        if pair[0] == pair[1] {
            return Err(TemporaryWorkError::DuplicateTemporaryWorkpiece(
                pair[0].clone(),
            ));
        }
    }
    Ok(())
}

fn reject_duplicate_steps(steps: &[PlanStepId]) -> Result<(), TemporaryWorkError> {
    for pair in steps.windows(2) {
        if pair[0] == pair[1] {
            return Err(TemporaryWorkError::DuplicateProtectedStep(pair[0].clone()));
        }
    }
    Ok(())
}

/// Transitive plan-order test. F10 remains the plan authority; this module only
/// verifies that the immutable DAG already encodes the temporary-work ordering.
fn reaches(plan: &FabricationPlan, from: &PlanStepId, to: &PlanStepId) -> bool {
    if from == to {
        return true;
    }
    let mut visited = BTreeSet::new();
    let mut frontier = vec![from.clone()];

    while let Some(current) = frontier.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        for dependency in plan
            .dependencies()
            .iter()
            .filter(|dependency| dependency.prerequisite == current)
        {
            if &dependency.dependent == to {
                return true;
            }
            if !visited.contains(&dependency.dependent) {
                frontier.push(dependency.dependent.clone());
            }
        }
    }
    false
}

#[derive(Debug)]
pub enum TemporaryWorkError {
    TemporaryWorkpieceRequired(TemporaryWorkId),
    ProtectedStepRequired(TemporaryWorkId),
    DuplicateTemporaryWorkpiece(WorkpieceId),
    DuplicateProtectedStep(PlanStepId),
    UnknownStep(PlanStepId),
    TemporaryWorkpieceMissingFromStep {
        step_id: PlanStepId,
        workpiece_id: WorkpieceId,
    },
    InstallationAlsoProtected(PlanStepId),
    RemovalEqualsInstallation(PlanStepId),
    RemovalAlsoProtected(PlanStepId),
    InstallationDoesNotPrecede {
        installation: PlanStepId,
        protected: PlanStepId,
    },
    ProtectedDoesNotPrecedeRemoval {
        protected: PlanStepId,
        removal: PlanStepId,
    },
    PlanMismatch {
        expected_id: FabricationPlanId,
        expected_revision: u64,
        actual_id: FabricationPlanId,
        actual_revision: u64,
    },
    SitePlanMismatch {
        site_id: ConstructionSiteId,
        expected_id: FabricationPlanId,
        expected_revision: u64,
        actual_id: FabricationPlanId,
        actual_revision: u64,
    },
    ImpossibleSiteState {
        contract_id: TemporaryWorkId,
        reason: String,
    },
    StepNotProtected {
        contract_id: TemporaryWorkId,
        step_id: PlanStepId,
    },
}

impl fmt::Display for TemporaryWorkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TemporaryWorkpieceRequired(id) => {
                write!(formatter, "temporary work {id} requires at least one workpiece")
            }
            Self::ProtectedStepRequired(id) => {
                write!(formatter, "temporary work {id} must protect at least one plan step")
            }
            Self::DuplicateTemporaryWorkpiece(id) => {
                write!(formatter, "temporary work repeats workpiece {id}")
            }
            Self::DuplicateProtectedStep(id) => {
                write!(formatter, "temporary work repeats protected step {id}")
            }
            Self::UnknownStep(id) => write!(formatter, "temporary work references unknown plan step {id}"),
            Self::TemporaryWorkpieceMissingFromStep {
                step_id,
                workpiece_id,
            } => write!(
                formatter,
                "temporary workpiece {workpiece_id} is not an input of plan step {step_id}"
            ),
            Self::InstallationAlsoProtected(id) => write!(
                formatter,
                "temporary-work installation step {id} cannot also be protected work"
            ),
            Self::RemovalEqualsInstallation(id) => write!(
                formatter,
                "temporary-work removal step {id} cannot equal installation"
            ),
            Self::RemovalAlsoProtected(id) => write!(
                formatter,
                "temporary-work removal step {id} cannot also be protected work"
            ),
            Self::InstallationDoesNotPrecede {
                installation,
                protected,
            } => write!(
                formatter,
                "temporary-work installation {installation} does not causally precede protected step {protected}"
            ),
            Self::ProtectedDoesNotPrecedeRemoval { protected, removal } => write!(
                formatter,
                "protected step {protected} does not causally precede temporary-work removal {removal}"
            ),
            Self::PlanMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "temporary work binds plan {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::SitePlanMismatch {
                site_id,
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "site {site_id} should bind temporary-work plan {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::ImpossibleSiteState {
                contract_id,
                reason,
            } => write!(
                formatter,
                "temporary work {contract_id} observed impossible sequencing state: {reason}"
            ),
            Self::StepNotProtected {
                contract_id,
                step_id,
            } => write!(
                formatter,
                "plan step {step_id} is not protected by temporary work {contract_id}"
            ),
        }
    }
}

impl Error for TemporaryWorkError {}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_fabrication::{FabricationPlan, PlanDependency, PlanStep, ProcessSpecId};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn workpiece(name: &str) -> WorkpieceId {
        WorkpieceId::new(id(&format!("workpiece:{name}")))
    }

    fn step(name: &str, workpieces: Vec<WorkpieceId>) -> PlanStep {
        PlanStep::new(
            PlanStepId::new(id(&format!("step:{name}"))),
            ProcessSpecId::new(id(&format!("process-spec:{name}"))),
            1,
            workpieces,
            Vec::new(),
            Vec::new(),
        )
        .unwrap()
    }

    fn dependency(before: &str, after: &str) -> PlanDependency {
        PlanDependency::new(
            PlanStepId::new(id(&format!("step:{before}"))),
            PlanStepId::new(id(&format!("step:{after}"))),
        )
        .unwrap()
    }

    fn brace_plan() -> (FabricationPlan, WorkpieceId, WorkpieceId) {
        let brace = workpiece("brace");
        let conduit = workpiece("conduit");
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:braced-pipe-repair")),
            3,
            vec![
                step("install-brace", vec![brace.clone(), conduit.clone()]),
                step("cut", vec![conduit.clone()]),
                step("fit", vec![conduit.clone()]),
                step("seal", vec![conduit.clone()]),
                step("remove-brace", vec![brace.clone(), conduit.clone()]),
            ],
            vec![
                dependency("install-brace", "cut"),
                dependency("cut", "fit"),
                dependency("fit", "seal"),
                dependency("seal", "remove-brace"),
            ],
        )
        .unwrap();
        (plan, brace, conduit)
    }

    #[test]
    fn valid_brace_contract_uses_existing_plan_causality() {
        let (plan, brace, _conduit) = brace_plan();
        let contract = TemporaryWorkContract::new(
            TemporaryWorkId::new(id("temporary-work:brace")),
            &plan,
            TemporaryWorkKind::Brace,
            id("purpose:brace:pipe-alignment"),
            vec![brace],
            PlanStepId::new(id("step:install-brace")),
            vec![
                PlanStepId::new(id("step:cut")),
                PlanStepId::new(id("step:fit")),
                PlanStepId::new(id("step:seal")),
            ],
            Some(PlanStepId::new(id("step:remove-brace"))),
        )
        .unwrap();

        assert_eq!(contract.kind, TemporaryWorkKind::Brace);
        assert_eq!(contract.protected_step_ids().len(), 3);
        contract.validate_plan(&plan).unwrap();
    }

    #[test]
    fn installation_must_causally_precede_every_protected_step() {
        let brace = workpiece("brace");
        let conduit = workpiece("conduit");
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:bad-install-order")),
            1,
            vec![
                step("install-brace", vec![brace.clone()]),
                step("cut", vec![conduit]),
            ],
            Vec::new(),
        )
        .unwrap();

        let result = TemporaryWorkContract::new(
            TemporaryWorkId::new(id("temporary-work:brace")),
            &plan,
            TemporaryWorkKind::Brace,
            id("purpose:brace"),
            vec![brace],
            PlanStepId::new(id("step:install-brace")),
            vec![PlanStepId::new(id("step:cut"))],
            None,
        );
        assert!(matches!(
            result,
            Err(TemporaryWorkError::InstallationDoesNotPrecede { .. })
        ));
    }

    #[test]
    fn removal_must_follow_all_protected_work() {
        let brace = workpiece("brace");
        let conduit = workpiece("conduit");
        let plan = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:bad-remove-order")),
            1,
            vec![
                step("install-brace", vec![brace.clone()]),
                step("cut", vec![conduit]),
                step("remove-brace", vec![brace.clone()]),
            ],
            vec![dependency("install-brace", "cut")],
        )
        .unwrap();

        let result = TemporaryWorkContract::new(
            TemporaryWorkId::new(id("temporary-work:brace")),
            &plan,
            TemporaryWorkKind::Brace,
            id("purpose:brace"),
            vec![brace],
            PlanStepId::new(id("step:install-brace")),
            vec![PlanStepId::new(id("step:cut"))],
            Some(PlanStepId::new(id("step:remove-brace"))),
        );
        assert!(matches!(
            result,
            Err(TemporaryWorkError::ProtectedDoesNotPrecedeRemoval { .. })
        ));
    }

    #[test]
    fn temporary_workpiece_must_participate_in_install_and_removal() {
        let (plan, brace, _conduit) = brace_plan();
        let unrelated = workpiece("unrelated-scaffold");
        let result = TemporaryWorkContract::new(
            TemporaryWorkId::new(id("temporary-work:brace")),
            &plan,
            TemporaryWorkKind::Brace,
            id("purpose:brace"),
            vec![brace, unrelated],
            PlanStepId::new(id("step:install-brace")),
            vec![PlanStepId::new(id("step:cut"))],
            Some(PlanStepId::new(id("step:remove-brace"))),
        );
        assert!(matches!(
            result,
            Err(TemporaryWorkError::TemporaryWorkpieceMissingFromStep { .. })
        ));
    }

    #[test]
    fn serialization_carries_no_structural_or_functional_oracle() {
        let (plan, brace, _conduit) = brace_plan();
        let contract = TemporaryWorkContract::new(
            TemporaryWorkId::new(id("temporary-work:brace")),
            &plan,
            TemporaryWorkKind::Brace,
            id("purpose:brace:pipe-alignment"),
            vec![brace],
            PlanStepId::new(id("step:install-brace")),
            vec![PlanStepId::new(id("step:cut"))],
            Some(PlanStepId::new(id("step:remove-brace"))),
        )
        .unwrap();

        let value = serde_json::to_value(contract).unwrap();
        for forbidden in [
            "strength",
            "load_capacity",
            "stability",
            "integrity",
            "safe",
            "quality",
            "score",
            "technical_ready",
            "authorized",
        ] {
            assert!(value.get(forbidden).is_none(), "unexpected field {forbidden}");
        }
    }
}
