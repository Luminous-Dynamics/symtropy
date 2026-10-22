// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Persistent semantic joints between compatible interfaces.
//!
//! A joint explains what connection was intentionally made and by which
//! evidence. It intentionally carries no independent strength/health scalar;
//! structural integrity remains owned by the physical authority beneath this
//! crate.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{
    Interface, InterfaceCompatibility, InterfaceFamily, InterfaceId, InterfaceMismatch,
};

/// Stable identity of one fabricated connection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JointId(StableId);

impl JointId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for JointId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Semantic joining method. Physical solvers may realize these methods very
/// differently; this enum does not prescribe a solver implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JointKind {
    Fastened,
    Welded,
    Brazed,
    Soldered,
    Bonded,
    Clamped,
    PressFit,
    Threaded,
    Bearing,
    SealedCoupling,
    ElectricalTermination,
}

impl JointKind {
    /// Prevents semantically impossible joints (for example an electrical
    /// termination across a structural rail) without pretending to evaluate
    /// engineering capacity or structural strength.
    pub const fn supports_family(self, family: InterfaceFamily) -> bool {
        use InterfaceFamily::{DataControl, Electrical, Fluid, Mechanical, Structural, Thermal};
        match self {
            Self::Fastened | Self::Welded | Self::Bonded | Self::Clamped | Self::PressFit => {
                matches!(family, Mechanical | Structural | Fluid | Thermal)
            }
            Self::Threaded => matches!(family, Mechanical | Structural | Fluid),
            Self::Brazed => {
                matches!(family, Mechanical | Structural | Fluid | Electrical | Thermal)
            }
            Self::Soldered => matches!(family, Fluid | Electrical | DataControl),
            Self::Bearing => matches!(family, Mechanical),
            Self::SealedCoupling => matches!(family, Fluid),
            Self::ElectricalTermination => matches!(family, Electrical | DataControl),
        }
    }
}

/// Opaque evidence that another authority accepted a process or physical
/// transition. Keeping this reference opaque prevents the joint from becoming a
/// duplicate process log or structural store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JointEvidenceRef {
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub revision: u64,
    pub digest: String,
}

impl JointEvidenceRef {
    pub fn new(
        authority_id: StableId,
        evidence_id: StableId,
        revision: u64,
        digest: impl Into<String>,
    ) -> Result<Self, JointError> {
        let digest = digest.into();
        if digest.is_empty() || digest.len() > 256 {
            return Err(JointError::InvalidEvidenceDigest(digest));
        }
        Ok(Self {
            authority_id,
            evidence_id,
            revision,
            digest,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JointLifecycle {
    Active,
    Released,
}

/// Semantic record of an intentional connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Joint {
    pub id: JointId,
    pub kind: JointKind,
    /// Canonically ordered interface pair for deterministic persistence.
    pub interfaces: [InterfaceId; 2],
    /// Evidence for the operation that established the connection.
    pub establishment_evidence: JointEvidenceRef,
    /// Evidence for a later controlled release/dismantling operation.
    pub release_evidence: Option<JointEvidenceRef>,
    pub lifecycle: JointLifecycle,
}

impl Joint {
    /// Creates a semantic joint only after the exposed interfaces are proven
    /// compatible and the joining method is meaningful for that family. No
    /// physical strength is invented here.
    pub fn establish(
        id: JointId,
        kind: JointKind,
        left: &Interface,
        right: &Interface,
        evidence: JointEvidenceRef,
    ) -> Result<Self, JointError> {
        match left.compatibility_with(right) {
            InterfaceCompatibility::Compatible => {}
            InterfaceCompatibility::Incompatible(reason) => {
                return Err(JointError::IncompatibleInterfaces(reason));
            }
        }

        let family = left.kind.family();
        if !kind.supports_family(family) {
            return Err(JointError::UnsupportedJointKind { kind, family });
        }

        let mut interfaces = [left.id.clone(), right.id.clone()];
        if interfaces[1] < interfaces[0] {
            interfaces.swap(0, 1);
        }

        Ok(Self {
            id,
            kind,
            interfaces,
            establishment_evidence: evidence,
            release_evidence: None,
            lifecycle: JointLifecycle::Active,
        })
    }

    /// Records controlled dismantling/separation evidence. Whether the physical
    /// connection has actually separated must be established by the supplying
    /// authority before it emits this evidence.
    pub fn release(&mut self, evidence: JointEvidenceRef) -> Result<(), JointError> {
        if self.lifecycle == JointLifecycle::Released {
            return Err(JointError::AlreadyReleased(self.id.clone()));
        }
        self.release_evidence = Some(evidence);
        self.lifecycle = JointLifecycle::Released;
        Ok(())
    }

    pub const fn is_active(&self) -> bool {
        matches!(self.lifecycle, JointLifecycle::Active)
    }
}

#[derive(Debug)]
pub enum JointError {
    InvalidEvidenceDigest(String),
    IncompatibleInterfaces(InterfaceMismatch),
    UnsupportedJointKind {
        kind: JointKind,
        family: InterfaceFamily,
    },
    AlreadyReleased(JointId),
}

impl fmt::Display for JointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEvidenceDigest(digest) => write!(
                formatter,
                "joint evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::IncompatibleInterfaces(reason) => {
                write!(formatter, "cannot establish joint between incompatible interfaces: {reason:?}")
            }
            Self::UnsupportedJointKind { kind, family } => {
                write!(formatter, "joint kind {kind:?} is not meaningful for {family:?} interfaces")
            }
            Self::AlreadyReleased(id) => write!(formatter, "joint {id} is already released"),
        }
    }
}

impl Error for JointError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InterfaceKind, MatingGeometry, WorkpieceId};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn interface(interface_id: &str, workpiece_id: &str, size: u64) -> Interface {
        Interface {
            id: InterfaceId::new(id(interface_id)),
            workpiece_id: WorkpieceId::new(id(workpiece_id)),
            kind: InterfaceKind::Structural {
                geometry: MatingGeometry {
                    profile_id: id("profile:structural:rail-a"),
                    nominal_size_um: size,
                    tolerance_um: 100,
                },
                load_transfer_profile: id("load:fixed"),
            },
        }
    }

    fn evidence(name: &str, revision: u64) -> JointEvidenceRef {
        JointEvidenceRef::new(
            id("authority:fabrication-process"),
            id(name),
            revision,
            format!("digest:{name}:{revision}"),
        )
        .unwrap()
    }

    #[test]
    fn joint_requires_compatible_interfaces() {
        let left = interface("interface:left", "workpiece:left", 40_000);
        let right = interface("interface:right", "workpiece:right", 42_000);
        let result = Joint::establish(
            JointId::new(id("joint:test")),
            JointKind::Fastened,
            &left,
            &right,
            evidence("evidence:join", 1),
        );
        assert!(matches!(result, Err(JointError::IncompatibleInterfaces(_))));
    }

    #[test]
    fn specialized_joint_kind_must_match_interface_family() {
        let left = interface("interface:left", "workpiece:left", 40_000);
        let right = interface("interface:right", "workpiece:right", 40_000);
        let result = Joint::establish(
            JointId::new(id("joint:test")),
            JointKind::ElectricalTermination,
            &left,
            &right,
            evidence("evidence:join", 1),
        );
        assert!(matches!(
            result,
            Err(JointError::UnsupportedJointKind {
                kind: JointKind::ElectricalTermination,
                family: InterfaceFamily::Structural,
            })
        ));
    }

    #[test]
    fn two_distinct_interfaces_on_same_workpiece_can_form_a_seam() {
        let left = interface("interface:left", "workpiece:sheet", 40_000);
        let right = interface("interface:right", "workpiece:sheet", 40_000);
        let joint = Joint::establish(
            JointId::new(id("joint:seam")),
            JointKind::Welded,
            &left,
            &right,
            evidence("evidence:seam", 1),
        )
        .unwrap();
        assert!(joint.is_active());
    }

    #[test]
    fn interface_order_does_not_change_persistent_joint_pair() {
        let left = interface("interface:a", "workpiece:a", 40_000);
        let right = interface("interface:b", "workpiece:b", 40_000);
        let a = Joint::establish(
            JointId::new(id("joint:a")),
            JointKind::Fastened,
            &left,
            &right,
            evidence("evidence:a", 1),
        )
        .unwrap();
        let b = Joint::establish(
            JointId::new(id("joint:b")),
            JointKind::Fastened,
            &right,
            &left,
            evidence("evidence:b", 1),
        )
        .unwrap();
        assert_eq!(a.interfaces, b.interfaces);
    }

    #[test]
    fn release_preserves_establishment_and_dismantling_evidence() {
        let left = interface("interface:a", "workpiece:a", 40_000);
        let right = interface("interface:b", "workpiece:b", 40_000);
        let establish = evidence("evidence:establish", 3);
        let release = evidence("evidence:release", 9);
        let mut joint = Joint::establish(
            JointId::new(id("joint:a")),
            JointKind::Welded,
            &left,
            &right,
            establish.clone(),
        )
        .unwrap();
        joint.release(release.clone()).unwrap();

        assert_eq!(joint.lifecycle, JointLifecycle::Released);
        assert_eq!(joint.establishment_evidence, establish);
        assert_eq!(joint.release_evidence.as_ref(), Some(&release));
    }

    #[test]
    fn joint_has_no_local_strength_or_health_field() {
        let left = interface("interface:a", "workpiece:a", 40_000);
        let right = interface("interface:b", "workpiece:b", 40_000);
        let joint = Joint::establish(
            JointId::new(id("joint:a")),
            JointKind::Fastened,
            &left,
            &right,
            evidence("evidence:a", 1),
        )
        .unwrap();
        let serialized = serde_json::to_value(joint).unwrap();
        assert!(serialized.get("strength").is_none());
        assert!(serialized.get("health").is_none());
        assert!(serialized.get("integrity").is_none());
    }
}
