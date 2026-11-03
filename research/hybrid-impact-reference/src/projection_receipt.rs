//! Loss-aware projection receipt for canonical hybrid-physics models.
//!
//! This module carries projection metadata only. It does not define a second
//! physics model, parse an upstream ScientificModel, authenticate an external
//! verifier receipt, or prove semantic equivalence. Those checks belong to an
//! independent consumer/verifier that re-reads the source and target artifacts.

/// Schema identity for this consumer-side projection receipt.
pub const HYBRID_PROJECTION_RECEIPT_SCHEMA: &str = "SYMTROPY_HYBRID_PROJECTION_RECEIPT.v0.1";
pub const SOURCE_SCIENTIFIC_MODEL_SCHEMA: &str = "SCIENTIFIC_MODEL.v0.2";
pub const SOURCE_HYBRID_DYNAMICS_SCHEMA: &str = "HYBRID_DYNAMICS.v0.1";

/// Loss treatment for one source semantic identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionTreatment {
    /// The source meaning is retained without semantic change.
    Preserved,
    /// The source meaning is expressed in a different form with an explicit
    /// equivalence-evidence reference.
    Transformed,
    /// The source meaning is approximated with an explicit evidence/bounds
    /// reference. This is not equivalent to preservation.
    Approximated,
    /// The meaning is retained outside the target model and must be resolved
    /// through an external artifact reference.
    Externalized,
    /// The target cannot represent this source semantic.
    Unsupported,
    /// A target mapping is proposed but equivalence has not been established.
    Unproven,
}

impl ProjectionTreatment {
    fn tag(self) -> &'static str {
        match self {
            Self::Preserved => "PRESERVED",
            Self::Transformed => "TRANSFORMED",
            Self::Approximated => "APPROXIMATED",
            Self::Externalized => "EXTERNALIZED",
            Self::Unsupported => "UNSUPPORTED",
            Self::Unproven => "UNPROVEN",
        }
    }
}

/// One ledger row. Exactly one row must exist for every source semantic ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionLedgerEntry {
    pub source_semantic_id: String,
    /// Target semantic identifier, absent when semantics are externalized or
    /// unsupported. Approximate and unproven mappings may carry a target ID.
    pub target_semantic_id: Option<String>,
    pub treatment: ProjectionTreatment,
    /// Required for TRANSFORMED (equivalence evidence), APPROXIMATED (bounds /
    /// error evidence), and EXTERNALIZED (resolvable external artifact).
    pub evidence_reference: Option<String>,
    /// Human-readable scope note. This is descriptive, never proof.
    pub note: String,
}

/// Identity-bound projection manifest from ScientificModel.v0.2 to Symtropy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridProjectionReceipt {
    pub schema_revision: String,
    pub source_model_schema: String,
    pub source_hybrid_schema: String,
    /// Canonical digest of the complete source ScientificModel.v0.2.
    pub source_model_digest: String,
    /// Canonical digest of its HybridDynamics.v0.1 payload.
    pub source_hybrid_digest: String,
    pub target_schema_revision: String,
    /// Canonical digest of the projected target model.
    pub target_model_digest: String,
    /// Digest of the projection implementation/build artifact.
    pub projection_implementation_digest: String,
    /// Digest of exact mapping options and configuration.
    pub projection_configuration_digest: String,
    /// Complete source semantic inventory, extracted from the source model.
    pub source_semantic_ids: Vec<String>,
    /// Exactly one loss-ledger row for every source semantic identifier.
    pub entries: Vec<ProjectionLedgerEntry>,
    /// Optional independent verification binding. Both fields must be present
    /// together. Presence does not authenticate the receipt.
    pub verifier_id: Option<String>,
    pub verification_receipt_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionReceiptError {
    InvalidSchemaRevision,
    InvalidSourceSchema,
    InvalidTargetSchema,
    InvalidDigest,
    EmptyIdentifier,
    InvalidIdentifier,
    DuplicateSourceSemantic,
    DuplicateLedgerEntry,
    MissingLedgerEntry,
    UnexpectedLedgerEntry,
    EmptyNote,
    InvalidTreatmentFields,
    IncompleteVerifierBinding,
    SourceModelDigestMismatch,
    SourceHybridDigestMismatch,
    SourceInventoryMismatch,
}

impl HybridProjectionReceipt {
    /// Normalize order-insensitive inventories and entries before canonical
    /// serialization. Source IDs and ledger rows are keyed by source ID.
    pub fn canonicalize(&mut self) {
        self.source_semantic_ids.sort();
        self.entries
            .sort_by(|left, right| left.source_semantic_id.cmp(&right.source_semantic_id));
    }

    /// Validate receipt structure and that its ledger covers its declared
    /// source inventory exactly once. This does not authenticate receipts.
    pub fn validate(&self) -> Result<(), ProjectionReceiptError> {
        if self.schema_revision != HYBRID_PROJECTION_RECEIPT_SCHEMA {
            return Err(ProjectionReceiptError::InvalidSchemaRevision);
        }
        if self.source_model_schema != SOURCE_SCIENTIFIC_MODEL_SCHEMA
            || self.source_hybrid_schema != SOURCE_HYBRID_DYNAMICS_SCHEMA
        {
            return Err(ProjectionReceiptError::InvalidSourceSchema);
        }
        if self.target_schema_revision.trim().is_empty() {
            return Err(ProjectionReceiptError::InvalidTargetSchema);
        }
        for digest in [
            self.source_model_digest.as_str(),
            self.source_hybrid_digest.as_str(),
            self.target_model_digest.as_str(),
            self.projection_implementation_digest.as_str(),
            self.projection_configuration_digest.as_str(),
        ] {
            if !is_lower_hex_digest(digest) {
                return Err(ProjectionReceiptError::InvalidDigest);
            }
        }
        if self.verifier_id.is_some() != self.verification_receipt_digest.is_some() {
            return Err(ProjectionReceiptError::IncompleteVerifierBinding);
        }
        if self
            .verifier_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(ProjectionReceiptError::EmptyIdentifier);
        }
        if self
            .verification_receipt_digest
            .as_deref()
            .is_some_and(|value| !is_lower_hex_digest(value))
        {
            return Err(ProjectionReceiptError::InvalidDigest);
        }

        let mut inventory = self.source_semantic_ids.clone();
        inventory.sort();
        for id in &inventory {
            validate_semantic_id(id)?;
        }
        if inventory.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ProjectionReceiptError::DuplicateSourceSemantic);
        }

        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort_by(|left, right| left.source_semantic_id.cmp(&right.source_semantic_id));
        for entry in &entries {
            validate_semantic_id(&entry.source_semantic_id)?;
            if entry.note.trim().is_empty() {
                return Err(ProjectionReceiptError::EmptyNote);
            }
            if entry
                .target_semantic_id
                .as_deref()
                .is_some_and(|id| validate_semantic_id(id).is_err())
            {
                return Err(ProjectionReceiptError::InvalidIdentifier);
            }
            if entry
                .evidence_reference
                .as_deref()
                .is_some_and(|reference| reference.trim().is_empty())
            {
                return Err(ProjectionReceiptError::EmptyIdentifier);
            }

            let has_target = entry.target_semantic_id.is_some();
            let has_evidence = entry.evidence_reference.is_some();
            let treatment_fields_valid = match entry.treatment {
                ProjectionTreatment::Preserved => has_target && !has_evidence,
                ProjectionTreatment::Transformed | ProjectionTreatment::Approximated => {
                    has_target && has_evidence
                }
                ProjectionTreatment::Externalized => !has_target && has_evidence,
                ProjectionTreatment::Unsupported => !has_target && !has_evidence,
                ProjectionTreatment::Unproven => has_target || has_evidence,
            };
            if !treatment_fields_valid {
                return Err(ProjectionReceiptError::InvalidTreatmentFields);
            }
        }

        if entries
            .windows(2)
            .any(|pair| pair[0].source_semantic_id == pair[1].source_semantic_id)
        {
            return Err(ProjectionReceiptError::DuplicateLedgerEntry);
        }

        let entry_ids = entries
            .iter()
            .map(|entry| entry.source_semantic_id.as_str())
            .collect::<Vec<_>>();
        let inventory_ids = inventory.iter().map(String::as_str).collect::<Vec<_>>();

        for id in &inventory_ids {
            if entry_ids.binary_search(id).is_err() {
                return Err(ProjectionReceiptError::MissingLedgerEntry);
            }
        }
        for id in &entry_ids {
            if inventory_ids.binary_search(id).is_err() {
                return Err(ProjectionReceiptError::UnexpectedLedgerEntry);
            }
        }

        Ok(())
    }

    /// Check a receipt against independently extracted source identities.
    /// This prevents a self-declared inventory or stale source digest from
    /// being treated as authoritative.
    pub fn validate_source_binding(
        &self,
        expected_model_digest: &str,
        expected_hybrid_digest: &str,
        expected_semantic_ids: &[String],
    ) -> Result<(), ProjectionReceiptError> {
        self.validate()?;
        if self.source_model_digest != expected_model_digest {
            return Err(ProjectionReceiptError::SourceModelDigestMismatch);
        }
        if self.source_hybrid_digest != expected_hybrid_digest {
            return Err(ProjectionReceiptError::SourceHybridDigestMismatch);
        }

        let mut expected = expected_semantic_ids.to_vec();
        expected.sort();
        if expected.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ProjectionReceiptError::DuplicateSourceSemantic);
        }
        if expected != self.source_semantic_ids {
            return Err(ProjectionReceiptError::SourceInventoryMismatch);
        }
        Ok(())
    }

    /// Deterministic, unambiguous byte representation. This is not a hash;
    /// callers select and record the digest algorithm separately.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut canonical = self.clone();
        canonical.canonicalize();
        let mut output = Vec::new();

        push_string(&mut output, &canonical.schema_revision);
        push_string(&mut output, &canonical.source_model_schema);
        push_string(&mut output, &canonical.source_hybrid_schema);
        push_string(&mut output, &canonical.source_model_digest);
        push_string(&mut output, &canonical.source_hybrid_digest);
        push_string(&mut output, &canonical.target_schema_revision);
        push_string(&mut output, &canonical.target_model_digest);
        push_string(&mut output, &canonical.projection_implementation_digest);
        push_string(&mut output, &canonical.projection_configuration_digest);

        push_len(&mut output, canonical.source_semantic_ids.len());
        for id in &canonical.source_semantic_ids {
            push_string(&mut output, id);
        }

        push_len(&mut output, canonical.entries.len());
        for entry in &canonical.entries {
            push_string(&mut output, &entry.source_semantic_id);
            push_optional_string(&mut output, entry.target_semantic_id.as_deref());
            push_string(&mut output, entry.treatment.tag());
            push_optional_string(&mut output, entry.evidence_reference.as_deref());
            push_string(&mut output, &entry.note);
        }

        push_optional_string(&mut output, canonical.verifier_id.as_deref());
        push_optional_string(
            &mut output,
            canonical.verification_receipt_digest.as_deref(),
        );
        output
    }
}

fn validate_semantic_id(value: &str) -> Result<(), ProjectionReceiptError> {
    if value.trim().is_empty() {
        return Err(ProjectionReceiptError::EmptyIdentifier);
    }
    if value
        .chars()
        .any(|character| character.is_whitespace() || character == '/' || character == '\\')
    {
        return Err(ProjectionReceiptError::InvalidIdentifier);
    }
    Ok(())
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn push_len(output: &mut Vec<u8>, length: usize) {
    output.extend_from_slice(&(length as u64).to_le_bytes());
}

fn push_string(output: &mut Vec<u8>, value: &str) {
    push_len(output, value.len());
    output.extend_from_slice(value.as_bytes());
}

fn push_optional_string(output: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            output.push(1);
            push_string(output, value);
        }
        None => output.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn valid_receipt() -> HybridProjectionReceipt {
        let source_semantic_ids = vec![
            "mode.free-flight".into(),
            "guard.ground-impact".into(),
            "reset.rebound".into(),
            "transition.rebound".into(),
        ];
        let entries = vec![
            ProjectionLedgerEntry {
                source_semantic_id: "mode.free-flight".into(),
                target_semantic_id: Some("symtropy.mode.flight".into()),
                treatment: ProjectionTreatment::Preserved,
                evidence_reference: None,
                note: "Vector field and declared assumptions preserved.".into(),
            },
            ProjectionLedgerEntry {
                source_semantic_id: "guard.ground-impact".into(),
                target_semantic_id: Some("symtropy.guard.ground".into()),
                treatment: ProjectionTreatment::Transformed,
                evidence_reference: Some(digest('a')),
                note: "Guard expression normalized; dimension and orientation equivalence checked."
                    .into(),
            },
            ProjectionLedgerEntry {
                source_semantic_id: "reset.rebound".into(),
                target_semantic_id: Some("symtropy.reset.newton".into()),
                treatment: ProjectionTreatment::Preserved,
                evidence_reference: None,
                note: "Newton reset and strict restitution domain preserved.".into(),
            },
            ProjectionLedgerEntry {
                source_semantic_id: "transition.rebound".into(),
                target_semantic_id: Some("symtropy.transition.rebound".into()),
                treatment: ProjectionTreatment::Unproven,
                evidence_reference: None,
                note: "Target mapping exists but branch-domain equivalence is not yet verified."
                    .into(),
            },
        ];
        HybridProjectionReceipt {
            schema_revision: HYBRID_PROJECTION_RECEIPT_SCHEMA.into(),
            source_model_schema: SOURCE_SCIENTIFIC_MODEL_SCHEMA.into(),
            source_hybrid_schema: SOURCE_HYBRID_DYNAMICS_SCHEMA.into(),
            source_model_digest: digest('b'),
            source_hybrid_digest: digest('c'),
            target_schema_revision: "SYMTROPY_PHYSICS_MODEL.v0.1".into(),
            target_model_digest: digest('d'),
            projection_implementation_digest: digest('e'),
            projection_configuration_digest: digest('f'),
            source_semantic_ids,
            entries,
            verifier_id: None,
            verification_receipt_digest: None,
        }
    }

    #[test]
    fn complete_loss_ledger_validates_against_independent_source_inventory() {
        let receipt = valid_receipt();
        receipt.validate().expect("valid receipt");
        receipt
            .validate_source_binding(&digest('b'), &digest('c'), &receipt.source_semantic_ids)
            .expect("source identities and semantic inventory match");
    }

    #[test]
    fn missing_or_extra_semantic_entries_fail_closed() {
        let mut missing = valid_receipt();
        missing.entries.pop();
        assert_eq!(
            missing.validate(),
            Err(ProjectionReceiptError::MissingLedgerEntry)
        );

        let mut extra = valid_receipt();
        extra.entries.push(ProjectionLedgerEntry {
            source_semantic_id: "reset.unlisted".into(),
            target_semantic_id: None,
            treatment: ProjectionTreatment::Unsupported,
            evidence_reference: None,
            note: "Not included in the independently extracted source inventory.".into(),
        });
        assert_eq!(
            extra.validate(),
            Err(ProjectionReceiptError::UnexpectedLedgerEntry)
        );
    }

    #[test]
    fn duplicate_source_semantics_and_duplicate_ledger_rows_are_rejected() {
        let mut duplicate_source = valid_receipt();
        duplicate_source
            .source_semantic_ids
            .push("mode.free-flight".into());
        assert_eq!(
            duplicate_source.validate(),
            Err(ProjectionReceiptError::DuplicateSourceSemantic)
        );

        let mut duplicate_entry = valid_receipt();
        duplicate_entry
            .entries
            .push(duplicate_entry.entries[0].clone());
        assert_eq!(
            duplicate_entry.validate(),
            Err(ProjectionReceiptError::DuplicateLedgerEntry)
        );
    }

    #[test]
    fn treatment_fields_cannot_claim_lossless_mapping_without_target_identity() {
        let mut receipt = valid_receipt();
        receipt.entries[0].target_semantic_id = None;
        assert_eq!(
            receipt.validate(),
            Err(ProjectionReceiptError::InvalidTreatmentFields)
        );

        let mut unsupported = valid_receipt();
        let entry = &mut unsupported.entries[0];
        entry.target_semantic_id = None;
        entry.treatment = ProjectionTreatment::Unsupported;
        assert!(unsupported.validate().is_ok());
    }

    #[test]
    fn transformed_and_approximate_semantics_require_evidence_references() {
        let mut transformed = valid_receipt();
        transformed.entries[1].evidence_reference = None;
        assert_eq!(
            transformed.validate(),
            Err(ProjectionReceiptError::InvalidTreatmentFields)
        );

        let mut approximate = valid_receipt();
        approximate.entries[1].treatment = ProjectionTreatment::Approximated;
        approximate.entries[1].evidence_reference = None;
        assert_eq!(
            approximate.validate(),
            Err(ProjectionReceiptError::InvalidTreatmentFields)
        );
    }

    #[test]
    fn externalized_semantics_require_an_external_reference_without_target_identity() {
        let mut receipt = valid_receipt();
        receipt.entries[0].target_semantic_id = None;
        receipt.entries[0].treatment = ProjectionTreatment::Externalized;
        assert!(receipt.validate().is_err());

        receipt.entries[0].evidence_reference = Some("artifact://external/contact-law".into());
        assert!(receipt.validate().is_ok());
    }

    #[test]
    fn verification_identity_and_digest_must_be_supplied_together() {
        let mut receipt = valid_receipt();
        receipt.verifier_id = Some("independent-verifier-v1".into());
        assert_eq!(
            receipt.validate(),
            Err(ProjectionReceiptError::IncompleteVerifierBinding)
        );

        receipt.verification_receipt_digest = Some(digest('9'));
        assert!(receipt.validate().is_ok());
    }

    #[test]
    fn stale_source_model_or_hybrid_digest_is_rejected() {
        let receipt = valid_receipt();
        assert_eq!(
            receipt.validate_source_binding(
                &digest('0'),
                &digest('c'),
                &receipt.source_semantic_ids
            ),
            Err(ProjectionReceiptError::SourceModelDigestMismatch)
        );
        assert_eq!(
            receipt.validate_source_binding(
                &digest('b'),
                &digest('0'),
                &receipt.source_semantic_ids
            ),
            Err(ProjectionReceiptError::SourceHybridDigestMismatch)
        );
    }

    #[test]
    fn source_inventory_is_independently_checked_not_self_attested() {
        let receipt = valid_receipt();
        let expected = vec![
            "mode.free-flight".into(),
            "guard.ground-impact".into(),
            "reset.rebound".into(),
            "transition.rebound".into(),
            "reset.settle".into(),
        ];
        assert_eq!(
            receipt.validate_source_binding(&digest('b'), &digest('c'), &expected),
            Err(ProjectionReceiptError::SourceInventoryMismatch)
        );
    }

    #[test]
    fn canonical_bytes_ignore_input_order_but_bind_every_treatment() {
        let receipt = valid_receipt();
        let mut permuted = receipt.clone();
        permuted.source_semantic_ids.reverse();
        permuted.entries.reverse();
        assert_eq!(receipt.canonical_bytes(), permuted.canonical_bytes());

        let mut changed = receipt.clone();
        changed.entries[0].treatment = ProjectionTreatment::Unsupported;
        changed.entries[0].target_semantic_id = None;
        assert_ne!(receipt.canonical_bytes(), changed.canonical_bytes());
    }
}
