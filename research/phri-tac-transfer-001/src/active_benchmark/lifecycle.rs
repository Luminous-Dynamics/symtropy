//! Synthetic lifecycle-aware tactile calibration applicability contract.
//!
//! A calibration belongs to a physical sensor article in a particular
//! electronics, firmware, skin/material, mounting and geometry state. This
//! module models append-only lifecycle events and checks whether a calibration
//! passport still applies to the current snapshot.
//!
//! Receipt IDs are references only. This module does not verify signatures,
//! provenance, reference independence, uncertainty, physical material state or
//! metrological traceability. Those remain the responsibility of their
//! independent evidence verifiers. No hardware or actuator authority exists.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalIdentity {
    pub sensor_article_id: String,
    pub electronics_revision: String,
    pub firmware_revision: String,
    pub skin_article_id: String,
    pub skin_revision: String,
    pub material_lot_id: String,
    pub mounting_revision: String,
    pub geometry_digest: String,
}

impl PhysicalIdentity {
    fn missing_fields(&self) -> Vec<&'static str> {
        let fields = [
            ("sensor_article_id", self.sensor_article_id.as_str()),
            ("electronics_revision", self.electronics_revision.as_str()),
            ("firmware_revision", self.firmware_revision.as_str()),
            ("skin_article_id", self.skin_article_id.as_str()),
            ("skin_revision", self.skin_revision.as_str()),
            ("material_lot_id", self.material_lot_id.as_str()),
            ("mounting_revision", self.mounting_revision.as_str()),
            ("geometry_digest", self.geometry_digest.as_str()),
        ];
        fields
            .into_iter()
            .filter_map(|(name, value)| value.trim().is_empty().then_some(name))
            .collect()
    }

    fn differences(&self, other: &Self) -> Vec<IdentityField> {
        let mut differences = Vec::new();
        if self.sensor_article_id != other.sensor_article_id {
            differences.push(IdentityField::SensorArticle);
        }
        if self.electronics_revision != other.electronics_revision {
            differences.push(IdentityField::ElectronicsRevision);
        }
        if self.firmware_revision != other.firmware_revision {
            differences.push(IdentityField::FirmwareRevision);
        }
        if self.skin_article_id != other.skin_article_id {
            differences.push(IdentityField::SkinArticle);
        }
        if self.skin_revision != other.skin_revision {
            differences.push(IdentityField::SkinRevision);
        }
        if self.material_lot_id != other.material_lot_id {
            differences.push(IdentityField::MaterialLot);
        }
        if self.mounting_revision != other.mounting_revision {
            differences.push(IdentityField::MountingRevision);
        }
        if self.geometry_digest != other.geometry_digest {
            differences.push(IdentityField::GeometryDigest);
        }
        differences
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityField {
    SensorArticle,
    ElectronicsRevision,
    FirmwareRevision,
    SkinArticle,
    SkinRevision,
    MaterialLot,
    MountingRevision,
    GeometryDigest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceCondition {
    QualifiedClean,
    ConditionUnknown,
    ContaminationSuspected,
    CleaningUnverified,
    Damaged,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LifecycleSnapshot {
    pub identity: PhysicalIdentity,
    /// Monotonically increases whenever calibration-relevant state changes.
    pub lifecycle_generation: u64,
    /// Absolute cumulative contact-cycle counter for this physical article.
    pub total_contact_cycles: u64,
    pub temperature_c: f64,
    pub surface_condition: SurfaceCondition,
    /// Receipt reference for the last independently inspected/cleaned surface state.
    pub surface_condition_receipt_id: Option<String>,
    pub last_event_sequence: u64,
    applied_event_ids: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LifecycleEvent {
    pub event_id: String,
    /// Strictly increasing article-local event sequence number.
    pub sequence: u64,
    pub change: LifecycleChange,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LifecycleChange {
    SkinReplaced {
        skin_article_id: String,
        skin_revision: String,
        material_lot_id: String,
    },
    Remounted {
        mounting_revision: String,
        geometry_digest: String,
    },
    ElectronicsRevisionChanged {
        electronics_revision: String,
    },
    FirmwareUpdated {
        firmware_revision: String,
    },
    GeometryChanged {
        geometry_digest: String,
    },
    ContaminationDetected,
    CleaningCompleted {
        protocol_digest: String,
        /// Optional receipt from an independent condition verifier. This module
        /// stores the reference but does not validate the receipt.
        condition_receipt_id: Option<String>,
    },
    SurfaceInspectionAccepted {
        /// Receipt from the independent inspection/condition verifier.
        condition_receipt_id: String,
    },
    DamageDetected,
    ContactCyclesAdvanced {
        total_contact_cycles: u64,
    },
    TemperatureObserved {
        temperature_c: f64,
    },
}

impl LifecycleSnapshot {
    pub fn new(identity: PhysicalIdentity, temperature_c: f64) -> Result<Self, String> {
        if !identity.missing_fields().is_empty() {
            return Err("physical identity contains an empty required field".to_owned());
        }
        if !temperature_c.is_finite() {
            return Err("initial temperature must be finite".to_owned());
        }
        Ok(Self {
            identity,
            lifecycle_generation: 0,
            total_contact_cycles: 0,
            temperature_c,
            surface_condition: SurfaceCondition::ConditionUnknown,
            surface_condition_receipt_id: None,
            last_event_sequence: 0,
            applied_event_ids: BTreeSet::new(),
        })
    }

    /// Apply one ordered lifecycle event, returning a new snapshot. Invalid
    /// events leave the original snapshot unchanged.
    pub fn apply(&self, event: &LifecycleEvent) -> Result<Self, String> {
        if event.event_id.trim().is_empty() {
            return Err("lifecycle event ID must not be empty".to_owned());
        }
        if self.applied_event_ids.contains(&event.event_id) {
            return Err("lifecycle event ID has already been applied".to_owned());
        }
        let expected_sequence = self
            .last_event_sequence
            .checked_add(1)
            .ok_or_else(|| "lifecycle event sequence overflow".to_owned())?;
        if event.sequence != expected_sequence {
            return Err("lifecycle event sequence must be contiguous and increasing".to_owned());
        }

        let mut next = self.clone();
        match &event.change {
            LifecycleChange::SkinReplaced {
                skin_article_id,
                skin_revision,
                material_lot_id,
            } => {
                require_nonempty(skin_article_id, "skin article ID")?;
                require_nonempty(skin_revision, "skin revision")?;
                require_nonempty(material_lot_id, "material lot ID")?;
                if self.identity.skin_article_id == *skin_article_id
                    && self.identity.skin_revision == *skin_revision
                    && self.identity.material_lot_id == *material_lot_id
                {
                    return Err(
                        "skin replacement must change article, revision or material lot".to_owned(),
                    );
                }
                next.identity.skin_article_id = skin_article_id.clone();
                next.identity.skin_revision = skin_revision.clone();
                next.identity.material_lot_id = material_lot_id.clone();
                next.surface_condition = SurfaceCondition::ConditionUnknown;
                next.surface_condition_receipt_id = None;
                next.bump_generation()?;
            }
            LifecycleChange::Remounted {
                mounting_revision,
                geometry_digest,
            } => {
                require_nonempty(mounting_revision, "mounting revision")?;
                require_nonempty(geometry_digest, "geometry digest")?;
                if self.identity.mounting_revision == *mounting_revision
                    && self.identity.geometry_digest == *geometry_digest
                {
                    return Err(
                        "remount must change mounting revision or geometry digest".to_owned()
                    );
                }
                next.identity.mounting_revision = mounting_revision.clone();
                next.identity.geometry_digest = geometry_digest.clone();
                next.surface_condition = SurfaceCondition::ConditionUnknown;
                next.bump_generation()?;
            }
            LifecycleChange::ElectronicsRevisionChanged {
                electronics_revision,
            } => {
                require_nonempty(electronics_revision, "electronics revision")?;
                if self.identity.electronics_revision == *electronics_revision {
                    return Err("electronics revision event must change the revision".to_owned());
                }
                next.identity.electronics_revision = electronics_revision.clone();
                next.surface_condition = SurfaceCondition::ConditionUnknown;
                next.bump_generation()?;
            }
            LifecycleChange::FirmwareUpdated { firmware_revision } => {
                require_nonempty(firmware_revision, "firmware revision")?;
                if self.identity.firmware_revision == *firmware_revision {
                    return Err("firmware update must change the revision".to_owned());
                }
                next.identity.firmware_revision = firmware_revision.clone();
                next.bump_generation()?;
            }
            LifecycleChange::GeometryChanged { geometry_digest } => {
                require_nonempty(geometry_digest, "geometry digest")?;
                if self.identity.geometry_digest == *geometry_digest {
                    return Err("geometry change must change its digest".to_owned());
                }
                next.identity.geometry_digest = geometry_digest.clone();
                next.surface_condition = SurfaceCondition::ConditionUnknown;
                next.bump_generation()?;
            }
            LifecycleChange::ContaminationDetected => {
                next.surface_condition = SurfaceCondition::ContaminationSuspected;
                next.surface_condition_receipt_id = None;
                next.bump_generation()?;
            }
            LifecycleChange::CleaningCompleted {
                protocol_digest,
                condition_receipt_id,
            } => {
                require_nonempty(protocol_digest, "cleaning protocol digest")?;
                if let Some(receipt_id) = condition_receipt_id {
                    require_nonempty(receipt_id, "surface condition receipt ID")?;
                    next.surface_condition = SurfaceCondition::QualifiedClean;
                    next.surface_condition_receipt_id = Some(receipt_id.clone());
                } else {
                    next.surface_condition = SurfaceCondition::CleaningUnverified;
                    next.surface_condition_receipt_id = None;
                }
                // Even cleaning supported by an external condition receipt
                // changes lifecycle generation. Old calibration is never
                // silently reinstated.
                next.bump_generation()?;
            }
            LifecycleChange::SurfaceInspectionAccepted { condition_receipt_id } => {
                require_nonempty(condition_receipt_id, "surface condition receipt ID")?;
                next.surface_condition = SurfaceCondition::QualifiedClean;
                next.surface_condition_receipt_id = Some(condition_receipt_id.clone());
                next.bump_generation()?;
            }
            LifecycleChange::DamageDetected => {
                next.surface_condition = SurfaceCondition::Damaged;
                next.surface_condition_receipt_id = None;
                next.bump_generation()?;
            }
            LifecycleChange::ContactCyclesAdvanced {
                total_contact_cycles,
            } => {
                if *total_contact_cycles < self.total_contact_cycles {
                    return Err("cumulative contact-cycle counter cannot move backwards".to_owned());
                }
                next.total_contact_cycles = *total_contact_cycles;
            }
            LifecycleChange::TemperatureObserved { temperature_c } => {
                if !temperature_c.is_finite() {
                    return Err("observed temperature must be finite".to_owned());
                }
                next.temperature_c = *temperature_c;
            }
        }

        next.last_event_sequence = event.sequence;
        next.applied_event_ids.insert(event.event_id.clone());
        Ok(next)
    }

    fn bump_generation(&mut self) -> Result<(), String> {
        self.lifecycle_generation = self
            .lifecycle_generation
            .checked_add(1)
            .ok_or_else(|| "lifecycle generation overflow".to_owned())?;
        Ok(())
    }
}

fn require_nonempty(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{name} must not be empty"))
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationPassport {
    pub calibration_id: String,
    pub physical_identity: PhysicalIdentity,
    pub lifecycle_generation: u64,
    pub calibration_contact_cycles: u64,
    pub maximum_additional_contact_cycles: u64,
    pub minimum_temperature_c: f64,
    pub maximum_temperature_c: f64,
    /// IDs point to separate evidence artifacts. Presence is checked here;
    /// those artifacts must be independently retrieved and verified.
    pub qualification_receipt_id: String,
    pub reference_chain_receipt_id: String,
    pub observability_receipt_id: String,
    pub model_adequacy_receipt_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssessmentDisposition {
    Applicable,
    Invalidated,
    MaintenanceRequired,
    OutOfEnvelope,
    UnqualifiedEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssessmentReason {
    MissingCalibrationId,
    MissingIdentityField(&'static str),
    MissingEvidenceReceipt(&'static str),
    IdentityMismatch(IdentityField),
    LifecycleGenerationMismatch {
        calibrated_generation: u64,
        current_generation: u64,
    },
    SurfaceConditionNotQualified(SurfaceCondition),
    ContactCycleCounterRollback {
        calibration_cycles: u64,
        current_cycles: u64,
    },
    ContactCycleEnvelopeExceeded {
        used_cycles: u64,
        allowed_cycles: u64,
    },
    InvalidTemperatureEnvelope,
    TemperatureBelowEnvelope {
        current_c: f64,
        minimum_c: f64,
    },
    TemperatureAboveEnvelope {
        current_c: f64,
        maximum_c: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationAssessment {
    pub calibration_id: String,
    pub disposition: AssessmentDisposition,
    pub reasons: Vec<AssessmentReason>,
    pub contact_cycles_since_calibration: Option<u64>,
}

impl CalibrationPassport {
    pub fn assess(&self, current: &LifecycleSnapshot) -> CalibrationAssessment {
        let mut reasons = Vec::new();

        if self.calibration_id.trim().is_empty() {
            reasons.push(AssessmentReason::MissingCalibrationId);
        }
        for field in self.physical_identity.missing_fields() {
            reasons.push(AssessmentReason::MissingIdentityField(field));
        }
        for (name, receipt) in [
            ("qualification", self.qualification_receipt_id.as_str()),
            ("reference_chain", self.reference_chain_receipt_id.as_str()),
            ("observability", self.observability_receipt_id.as_str()),
            ("model_adequacy", self.model_adequacy_receipt_id.as_str()),
        ] {
            if receipt.trim().is_empty() {
                reasons.push(AssessmentReason::MissingEvidenceReceipt(name));
            }
        }

        for field in self.physical_identity.differences(&current.identity) {
            reasons.push(AssessmentReason::IdentityMismatch(field));
        }
        if self.lifecycle_generation != current.lifecycle_generation {
            reasons.push(AssessmentReason::LifecycleGenerationMismatch {
                calibrated_generation: self.lifecycle_generation,
                current_generation: current.lifecycle_generation,
            });
        }
        if current.surface_condition != SurfaceCondition::QualifiedClean {
            reasons.push(AssessmentReason::SurfaceConditionNotQualified(
                current.surface_condition,
            ));
        } else if current
            .surface_condition_receipt_id
            .as_deref()
            .is_none_or(|receipt_id| receipt_id.trim().is_empty())
        {
            reasons.push(AssessmentReason::MissingEvidenceReceipt("surface_condition"));
        }

        let contact_cycles_since_calibration = current
            .total_contact_cycles
            .checked_sub(self.calibration_contact_cycles);
        match contact_cycles_since_calibration {
            None => reasons.push(AssessmentReason::ContactCycleCounterRollback {
                calibration_cycles: self.calibration_contact_cycles,
                current_cycles: current.total_contact_cycles,
            }),
            Some(used_cycles) if used_cycles > self.maximum_additional_contact_cycles => {
                reasons.push(AssessmentReason::ContactCycleEnvelopeExceeded {
                    used_cycles,
                    allowed_cycles: self.maximum_additional_contact_cycles,
                });
            }
            Some(_) => {}
        }

        if !self.minimum_temperature_c.is_finite()
            || !self.maximum_temperature_c.is_finite()
            || self.minimum_temperature_c > self.maximum_temperature_c
        {
            reasons.push(AssessmentReason::InvalidTemperatureEnvelope);
        } else if current.temperature_c < self.minimum_temperature_c {
            reasons.push(AssessmentReason::TemperatureBelowEnvelope {
                current_c: current.temperature_c,
                minimum_c: self.minimum_temperature_c,
            });
        } else if current.temperature_c > self.maximum_temperature_c {
            reasons.push(AssessmentReason::TemperatureAboveEnvelope {
                current_c: current.temperature_c,
                maximum_c: self.maximum_temperature_c,
            });
        }

        let has_unqualified_evidence = reasons.iter().any(|reason| {
            matches!(
                reason,
                AssessmentReason::MissingCalibrationId
                    | AssessmentReason::MissingIdentityField(_)
                    | AssessmentReason::MissingEvidenceReceipt(_)
                    | AssessmentReason::InvalidTemperatureEnvelope
                    | AssessmentReason::ContactCycleCounterRollback { .. }
            )
        });
        let has_identity_invalidation = reasons.iter().any(|reason| {
            matches!(
                reason,
                AssessmentReason::IdentityMismatch(_)
                    | AssessmentReason::LifecycleGenerationMismatch { .. }
            )
        });
        let has_maintenance_condition = reasons
            .iter()
            .any(|reason| matches!(reason, AssessmentReason::SurfaceConditionNotQualified(_)));
        let has_envelope_failure = reasons.iter().any(|reason| {
            matches!(
                reason,
                AssessmentReason::ContactCycleEnvelopeExceeded { .. }
                    | AssessmentReason::TemperatureBelowEnvelope { .. }
                    | AssessmentReason::TemperatureAboveEnvelope { .. }
            )
        });

        let disposition = if has_unqualified_evidence {
            AssessmentDisposition::UnqualifiedEvidence
        } else if has_identity_invalidation {
            AssessmentDisposition::Invalidated
        } else if has_maintenance_condition {
            AssessmentDisposition::MaintenanceRequired
        } else if has_envelope_failure {
            AssessmentDisposition::OutOfEnvelope
        } else {
            AssessmentDisposition::Applicable
        };

        CalibrationAssessment {
            calibration_id: self.calibration_id.clone(),
            disposition,
            reasons,
            contact_cycles_since_calibration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> PhysicalIdentity {
        PhysicalIdentity {
            sensor_article_id: "sensor-article-01".to_owned(),
            electronics_revision: "pcb-r1".to_owned(),
            firmware_revision: "fw-r1".to_owned(),
            skin_article_id: "skin-article-01".to_owned(),
            skin_revision: "skin-r1".to_owned(),
            material_lot_id: "silicone-lot-01".to_owned(),
            mounting_revision: "mount-r1".to_owned(),
            geometry_digest: "sha256:geometry-r1".to_owned(),
        }
    }

    fn snapshot() -> LifecycleSnapshot {
        LifecycleSnapshot::new(identity(), 23.0).expect("snapshot")
    }

    fn qualified_snapshot() -> LifecycleSnapshot {
        snapshot()
            .apply(&event(
                "event-inspection-01",
                1,
                LifecycleChange::SurfaceInspectionAccepted {
                    condition_receipt_id: "receipt:surface-condition-01".to_owned(),
                },
            ))
            .expect("independent surface inspection receipt")
    }

    fn passport(snapshot: &LifecycleSnapshot) -> CalibrationPassport {
        CalibrationPassport {
            calibration_id: "calibration-01".to_owned(),
            physical_identity: snapshot.identity.clone(),
            lifecycle_generation: snapshot.lifecycle_generation,
            calibration_contact_cycles: snapshot.total_contact_cycles,
            maximum_additional_contact_cycles: 10_000,
            minimum_temperature_c: 10.0,
            maximum_temperature_c: 40.0,
            qualification_receipt_id: "receipt:qualification".to_owned(),
            reference_chain_receipt_id: "receipt:reference".to_owned(),
            observability_receipt_id: "receipt:observability".to_owned(),
            model_adequacy_receipt_id: "receipt:model-adequacy".to_owned(),
        }
    }

    fn event(event_id: &str, sequence: u64, change: LifecycleChange) -> LifecycleEvent {
        LifecycleEvent {
            event_id: event_id.to_owned(),
            sequence,
            change,
        }
    }

    #[test]
    fn unchanged_physical_state_with_complete_evidence_remains_applicable() {
        let current = qualified_snapshot();
        let assessment = passport(&current).assess(&current);
        assert_eq!(assessment.disposition, AssessmentDisposition::Applicable);
        assert!(assessment.reasons.is_empty());
        assert_eq!(assessment.contact_cycles_since_calibration, Some(0));
    }

    #[test]
    fn replacing_the_skin_invalidates_the_previous_calibration() {
        let original = qualified_snapshot();
        let calibration = passport(&original);
        let current = original
            .apply(&event(
                "event-skin-replace-01",
                2,
                LifecycleChange::SkinReplaced {
                    skin_article_id: "skin-article-02".to_owned(),
                    skin_revision: "skin-r2".to_owned(),
                    material_lot_id: "silicone-lot-02".to_owned(),
                },
            ))
            .expect("replacement");
        let assessment = calibration.assess(&current);
        assert_eq!(assessment.disposition, AssessmentDisposition::Invalidated);
        assert!(
            assessment
                .reasons
                .contains(&AssessmentReason::IdentityMismatch(
                    IdentityField::SkinArticle
                ))
        );
        assert!(
            assessment
                .reasons
                .contains(&AssessmentReason::LifecycleGenerationMismatch {
                    calibrated_generation: 0,
                    current_generation: 1
                })
        );
    }

    #[test]
    fn remount_or_geometry_change_invalidates_previous_calibration() {
        let original = snapshot();
        let calibration = passport(&original);
        let current = original
            .apply(&event(
                "event-remount-01",
                2,
                LifecycleChange::Remounted {
                    mounting_revision: "mount-r2".to_owned(),
                    geometry_digest: "sha256:geometry-r2".to_owned(),
                },
            ))
            .expect("remount");
        let assessment = calibration.assess(&current);
        assert_eq!(assessment.disposition, AssessmentDisposition::Invalidated);
        assert!(
            assessment
                .reasons
                .contains(&AssessmentReason::IdentityMismatch(
                    IdentityField::MountingRevision
                ))
        );
    }

    #[test]
    fn firmware_and_electronics_changes_invalidate_previous_calibration() {
        let original = snapshot();
        let calibration = passport(&original);
        let current = original
            .apply(&event(
                "event-firmware-01",
                2,
                LifecycleChange::FirmwareUpdated {
                    firmware_revision: "fw-r2".to_owned(),
                },
            ))
            .expect("firmware update");
        assert_eq!(
            calibration.assess(&current).disposition,
            AssessmentDisposition::Invalidated
        );

        let second = current
            .apply(&event(
                "event-electronics-01",
                3,
                LifecycleChange::ElectronicsRevisionChanged {
                    electronics_revision: "pcb-r2".to_owned(),
                },
            ))
            .expect("electronics update");
        assert_eq!(
            calibration.assess(&second).disposition,
            AssessmentDisposition::Invalidated
        );
    }

    #[test]
    fn verified_cleaning_does_not_reinstate_a_superseded_calibration() {
        let original = snapshot();
        let calibration = passport(&original);
        let contaminated = original
            .apply(&event(
                "event-contamination-01",
                2,
                LifecycleChange::ContaminationDetected,
            ))
            .expect("contamination");
        let cleaned = contaminated
            .apply(&event(
                "event-cleaning-01",
                3,
                LifecycleChange::CleaningCompleted {
                    protocol_digest: "sha256:cleaning-protocol".to_owned(),
                    condition_receipt_id: Some("receipt:cleaning-condition-01".to_owned()),
                },
            ))
            .expect("cleaning");
        assert_eq!(cleaned.surface_condition, SurfaceCondition::QualifiedClean);
        assert_eq!(
            calibration.assess(&cleaned).disposition,
            AssessmentDisposition::Invalidated
        );
        assert_eq!(cleaned.lifecycle_generation, 3);
    }

    #[test]
    fn contamination_and_unverified_cleaning_require_maintenance() {
        let original = snapshot();
        let contaminated = original
            .apply(&event(
                "event-contamination-01",
                1,
                LifecycleChange::ContaminationDetected,
            ))
            .expect("contamination");
        let current_passport = passport(&contaminated);
        assert_eq!(
            current_passport.assess(&contaminated).disposition,
            AssessmentDisposition::MaintenanceRequired
        );

        let cleaned = contaminated
            .apply(&event(
                "event-cleaning-01",
                2,
                LifecycleChange::CleaningCompleted {
                    protocol_digest: "sha256:cleaning-protocol".to_owned(),
                    condition_receipt_id: None,
                },
            ))
            .expect("unverified cleaning");
        assert_eq!(
            passport(&cleaned).assess(&cleaned).disposition,
            AssessmentDisposition::MaintenanceRequired
        );
    }

    #[test]
    fn damage_requires_maintenance_even_when_identity_is_unchanged() {
        let current = snapshot()
            .apply(&event(
                "event-damage-01",
                2,
                LifecycleChange::DamageDetected,
            ))
            .expect("damage");
        assert_eq!(
            passport(&current).assess(&current).disposition,
            AssessmentDisposition::MaintenanceRequired
        );
    }

    #[test]
    fn contact_cycle_budget_and_temperature_envelope_are_enforced() {
        let original = snapshot();
        let calibration = passport(&original);
        let cycles = original
            .apply(&event(
                "event-cycles-01",
                2,
                LifecycleChange::ContactCyclesAdvanced {
                    total_contact_cycles: 10_001,
                },
            ))
            .expect("cycle advance");
        assert_eq!(
            calibration.assess(&cycles).disposition,
            AssessmentDisposition::OutOfEnvelope
        );

        let hot = original
            .apply(&event(
                "event-temperature-01",
                1,
                LifecycleChange::TemperatureObserved {
                    temperature_c: 45.0,
                },
            ))
            .expect("temperature update");
        assert_eq!(
            calibration.assess(&hot).disposition,
            AssessmentDisposition::OutOfEnvelope
        );
    }

    #[test]
    fn missing_receipt_reference_never_qualifies_a_calibration() {
        let current = qualified_snapshot();
        let mut incomplete = passport(&current);
        incomplete.reference_chain_receipt_id.clear();
        let assessment = incomplete.assess(&current);
        assert_eq!(
            assessment.disposition,
            AssessmentDisposition::UnqualifiedEvidence
        );
        assert!(
            assessment
                .reasons
                .contains(&AssessmentReason::MissingEvidenceReceipt("reference_chain"))
        );
    }

    #[test]
    fn duplicate_or_out_of_order_lifecycle_events_are_rejected() {
        let original = snapshot();
        let first = event(
            "event-temp-01",
            1,
            LifecycleChange::TemperatureObserved {
                temperature_c: 24.0,
            },
        );
        let current = original.apply(&first).expect("first event");
        assert!(current.apply(&first).is_err());
        assert!(
            current
                .apply(&event(
                    "event-temp-02",
                    1,
                    LifecycleChange::TemperatureObserved {
                        temperature_c: 25.0
                    },
                ))
                .is_err()
        );
        assert_eq!(original.last_event_sequence, 0);
        assert_eq!(current.last_event_sequence, 1);
    }

    #[test]
    fn missing_lifecycle_event_sequence_is_rejected() {
        let original = snapshot();
        let gap = event(
            "event-gap-02",
            2,
            LifecycleChange::TemperatureObserved { temperature_c: 25.0 },
        );
        assert!(original.apply(&gap).is_err());
        assert_eq!(original.last_event_sequence, 0);
    }

    #[test]
    fn cycle_counter_cannot_move_backwards_and_failed_events_are_atomic() {
        let original = snapshot()
            .apply(&event(
                "event-cycles-01",
                1,
                LifecycleChange::ContactCyclesAdvanced {
                    total_contact_cycles: 500,
                },
            ))
            .expect("first cycle update");
        assert!(
            original
                .apply(&event(
                    "event-cycles-02",
                    2,
                    LifecycleChange::ContactCyclesAdvanced {
                        total_contact_cycles: 499,
                    },
                ))
                .is_err()
        );
        assert_eq!(original.total_contact_cycles, 500);
        assert_eq!(original.last_event_sequence, 1);
    }

    #[test]
    fn empty_physical_identity_and_nonfinite_conditions_fail_closed() {
        let mut missing_identity = identity();
        missing_identity.skin_article_id.clear();
        assert!(LifecycleSnapshot::new(missing_identity, 23.0).is_err());
        assert!(LifecycleSnapshot::new(identity(), f64::NAN).is_err());

        let current = qualified_snapshot();
        let mut invalid_passport = passport(&current);
        invalid_passport.minimum_temperature_c = 42.0;
        invalid_passport.maximum_temperature_c = 20.0;
        assert_eq!(
            invalid_passport.assess(&current).disposition,
            AssessmentDisposition::UnqualifiedEvidence
        );
    }
}
