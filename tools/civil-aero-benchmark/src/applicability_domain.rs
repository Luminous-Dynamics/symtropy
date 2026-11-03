//! Claim-scoped aerodynamic applicability domains and selective abstention.
//!
//! This module evaluates a frozen union of explicit support regions. It deliberately does not
//! infer coverage from a convex hull, nearest-neighbour score, or a global OOD classifier.
//! "Supported" means only that the request lies in a declared region for the exact model,
//! quantity, geometry family, flow regime and feature units. It does not prove physical accuracy.

use std::collections::{BTreeMap, BTreeSet};

use crate::integrity::sha256_hex;

pub const APPLICABILITY_DOMAIN_SCHEMA: &str = "aero-applicability-domain/0.1.0";
pub const APPLICABILITY_DOMAIN_EVIDENCE_LEVEL: &str =
    "deterministic_exact_identity_and_explicit_axis_aligned_support_region_membership_not_physical_validation_or_generalization";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicabilityFeatureSpec {
    pub name: String,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicabilityFeatureBound {
    pub name: String,
    pub unit: String,
    pub lower_inclusive: f64,
    pub upper_inclusive: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicabilitySupportClass {
    Dense,
    Sparse,
    BoundaryOrRegimeTransition,
}

impl ApplicabilitySupportClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dense => "dense",
            Self::Sparse => "sparse",
            Self::BoundaryOrRegimeTransition => "boundary_or_regime_transition",
        }
    }

    fn status(self) -> ApplicabilityStatus {
        match self {
            Self::Dense => ApplicabilityStatus::SupportedWithinDeclaredDomain,
            Self::Sparse => ApplicabilityStatus::SupportedButSparse,
            Self::BoundaryOrRegimeTransition => {
                ApplicabilityStatus::BoundaryOrRegimeTransition
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicabilityEvidenceRef {
    pub evidence_id: String,
    pub evidence_kind: String,
    pub source_ref: String,
    pub license_applicability: String,
    pub artifact_sha256: String,
    pub scope_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicabilityDomainRegion {
    pub region_id: String,
    pub geometry_family: String,
    pub flow_regime: String,
    pub support: ApplicabilitySupportClass,
    pub rationale: String,
    /// Exact source/evidence records supporting this declared region. Digests bind bytes; they
    /// do not authenticate source truth or independently prove the region is physically valid.
    pub supporting_evidence_ids: Vec<String>,
    /// Exactly one inclusive interval is required for every feature in the manifest schema.
    pub bounds: Vec<ApplicabilityFeatureBound>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicabilityDomainManifest {
    pub model_id: String,
    pub quantity: String,
    pub quantity_unit: String,
    pub solver_identity_sha256: String,
    pub output_definition_sha256: String,
    pub feature_schema: Vec<ApplicabilityFeatureSpec>,
    /// Evidence source references and artifact/scope digests for the admitted validation basis.
    pub evidence_refs: Vec<ApplicabilityEvidenceRef>,
    /// Regions are a union, not a convex hull. Unsupported gaps stay unsupported.
    pub regions: Vec<ApplicabilityDomainRegion>,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicabilityFeatureValue {
    pub name: String,
    pub unit: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicabilityQuery {
    pub model_id: String,
    pub quantity: String,
    pub geometry_family: String,
    pub flow_regime: String,
    pub solver_identity_sha256: String,
    pub output_definition_sha256: String,
    pub features: Vec<ApplicabilityFeatureValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicabilityStatus {
    SupportedWithinDeclaredDomain,
    SupportedButSparse,
    BoundaryOrRegimeTransition,
    OutOfDomain,
    UnknownDueToMissingOrUnqualifiedInputs,
}

impl ApplicabilityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SupportedWithinDeclaredDomain => "supported_within_declared_domain",
            Self::SupportedButSparse => "supported_but_sparse",
            Self::BoundaryOrRegimeTransition => "boundary_or_regime_transition",
            Self::OutOfDomain => "out_of_domain",
            Self::UnknownDueToMissingOrUnqualifiedInputs => {
                "unknown_due_to_missing_or_unqualified_inputs"
            }
        }
    }

    /// True only for a declared dense/sparse support region. Boundary regions remain review-only.
    pub fn is_inside_declared_support(self) -> bool {
        matches!(
            self,
            Self::SupportedWithinDeclaredDomain
                | Self::SupportedButSparse
                | Self::BoundaryOrRegimeTransition
        )
    }

    /// Boundary/regime-transition points must not be promoted without a separate review policy.
    pub fn may_support_claim_admission_without_boundary_review(self) -> bool {
        matches!(
            self,
            Self::SupportedWithinDeclaredDomain | Self::SupportedButSparse
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicabilityAssessment {
    pub schema: String,
    pub model_id: String,
    pub quantity: String,
    pub manifest_sha256: String,
    pub query_sha256: String,
    pub status: ApplicabilityStatus,
    pub matched_region_ids: Vec<String>,
    pub reason: String,
    pub evidence_level: String,
    pub assessment_sha256: String,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value == value.to_ascii_lowercase()
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn append_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
}

fn canonical_region_scope_bytes(
    manifest: &ApplicabilityDomainManifest,
    region: &ApplicabilityDomainRegion,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, "aero-applicability-region-scope/0.1.0");
    append_string(&mut bytes, &manifest.model_id);
    append_string(&mut bytes, &manifest.quantity);
    append_string(&mut bytes, &manifest.quantity_unit);
    append_string(&mut bytes, &manifest.solver_identity_sha256);
    append_string(&mut bytes, &manifest.output_definition_sha256);
    append_string(&mut bytes, &region.geometry_family);
    append_string(&mut bytes, &region.flow_regime);
    let mut bounds = region.bounds.iter().collect::<Vec<_>>();
    bounds.sort_by(|left, right| left.name.cmp(&right.name));
    append_u64(&mut bytes, bounds.len() as u64);
    for bound in bounds {
        append_string(&mut bytes, &bound.name);
        append_string(&mut bytes, &bound.unit);
        append_f64(&mut bytes, bound.lower_inclusive);
        append_f64(&mut bytes, bound.upper_inclusive);
    }
    bytes
}

fn region_scope_sha256(
    manifest: &ApplicabilityDomainManifest,
    region: &ApplicabilityDomainRegion,
) -> Result<String, String> {
    sha256_hex(&canonical_region_scope_bytes(manifest, region))
}

fn require_nonempty(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(())
}

fn validate_manifest_payload(manifest: &ApplicabilityDomainManifest) -> Result<(), String> {
    for (name, value) in [
        ("model_id", manifest.model_id.as_str()),
        ("quantity", manifest.quantity.as_str()),
        ("quantity_unit", manifest.quantity_unit.as_str()),
    ] {
        require_nonempty(value, name)?;
    }
    if !is_sha256(&manifest.solver_identity_sha256)
        || !is_sha256(&manifest.output_definition_sha256)
    {
        return Err("solver and output-definition identities must be lowercase SHA-256 digests"
            .to_owned());
    }
    if manifest.feature_schema.is_empty() {
        return Err("applicability feature schema must not be empty".to_owned());
    }
    if manifest.regions.is_empty() {
        return Err("applicability manifest must declare at least one support region".to_owned());
    }

    let mut schema = BTreeMap::new();
    for feature in &manifest.feature_schema {
        require_nonempty(&feature.name, "feature name")?;
        require_nonempty(&feature.unit, "feature unit")?;
        if schema.insert(feature.name.as_str(), feature.unit.as_str()).is_some() {
            return Err(format!("duplicate applicability feature {}", feature.name));
        }
    }

    if manifest.evidence_refs.is_empty() {
        return Err("applicability manifest must reference at least one evidence artifact".to_owned());
    }
    let mut evidence_by_id = BTreeMap::new();
    for evidence in &manifest.evidence_refs {
        for (name, value) in [
            ("evidence_id", evidence.evidence_id.as_str()),
            ("evidence_kind", evidence.evidence_kind.as_str()),
            ("source_ref", evidence.source_ref.as_str()),
            ("license_applicability", evidence.license_applicability.as_str()),
        ] {
            require_nonempty(value, name)?;
        }
        if !is_sha256(&evidence.artifact_sha256) || !is_sha256(&evidence.scope_sha256) {
            return Err(format!(
                "evidence {} artifact and scope identities must be lowercase SHA-256 digests",
                evidence.evidence_id
            ));
        }
        if evidence_by_id.insert(evidence.evidence_id.as_str(), evidence).is_some() {
            return Err(format!("duplicate applicability evidence ID {}", evidence.evidence_id));
        }
    }

    let mut used_evidence = BTreeSet::new();
    let mut region_ids = BTreeSet::new();
    for region in &manifest.regions {
        require_nonempty(&region.region_id, "region_id")?;
        require_nonempty(&region.geometry_family, "region geometry_family")?;
        require_nonempty(&region.flow_regime, "region flow_regime")?;
        require_nonempty(&region.rationale, "region rationale")?;
        if region.supporting_evidence_ids.is_empty() {
            return Err(format!(
                "region {} must reference at least one supporting evidence artifact",
                region.region_id
            ));
        }
        let mut region_evidence_ids = BTreeSet::new();
        for evidence_id in &region.supporting_evidence_ids {
            if !evidence_by_id.contains_key(evidence_id.as_str()) {
                return Err(format!(
                    "region {} references missing evidence ID {}",
                    region.region_id, evidence_id
                ));
            }
            if !region_evidence_ids.insert(evidence_id.as_str()) {
                return Err(format!(
                    "region {} duplicates evidence ID {}",
                    region.region_id, evidence_id
                ));
            }
            let evidence = evidence_by_id
                .get(evidence_id.as_str())
                .expect("evidence existence checked above");
            if evidence.scope_sha256 != region_scope_sha256(manifest, region)? {
                return Err(format!(
                    "region {} evidence {} is scoped to a different model/quantity/geometry/regime/feature envelope",
                    region.region_id, evidence_id
                ));
            }
            used_evidence.insert(evidence_id.as_str());
        }
        if !region_ids.insert(region.region_id.as_str()) {
            return Err(format!("duplicate applicability region {}", region.region_id));
        }
        if region.bounds.len() != schema.len() {
            return Err(format!(
                "region {} must define exactly {} feature bounds",
                region.region_id,
                schema.len()
            ));
        }
        let mut seen_bounds = BTreeSet::new();
        for bound in &region.bounds {
            let Some(expected_unit) = schema.get(bound.name.as_str()) else {
                return Err(format!(
                    "region {} contains undeclared feature {}",
                    region.region_id, bound.name
                ));
            };
            if !seen_bounds.insert(bound.name.as_str()) {
                return Err(format!(
                    "region {} duplicates feature bound {}",
                    region.region_id, bound.name
                ));
            }
            if bound.unit != *expected_unit {
                return Err(format!(
                    "region {} feature {} uses unit {}, expected {}",
                    region.region_id, bound.name, bound.unit, expected_unit
                ));
            }
            if !bound.lower_inclusive.is_finite()
                || !bound.upper_inclusive.is_finite()
                || bound.lower_inclusive > bound.upper_inclusive
            {
                return Err(format!(
                    "region {} feature {} has invalid finite bounds",
                    region.region_id, bound.name
                ));
            }
        }
        if seen_bounds.len() != schema.len() {
            return Err(format!(
                "region {} omits one or more declared feature bounds",
                region.region_id
            ));
        }
    }
    if used_evidence.len() != evidence_by_id.len() {
        return Err("applicability manifest contains an evidence reference unused by every region".to_owned());
    }
    Ok(())
}

fn canonical_manifest_bytes(manifest: &ApplicabilityDomainManifest) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, APPLICABILITY_DOMAIN_SCHEMA);
    append_string(&mut bytes, &manifest.model_id);
    append_string(&mut bytes, &manifest.quantity);
    append_string(&mut bytes, &manifest.quantity_unit);
    append_string(&mut bytes, &manifest.solver_identity_sha256);
    append_string(&mut bytes, &manifest.output_definition_sha256);

    let mut features = manifest.feature_schema.iter().collect::<Vec<_>>();
    features.sort_by(|left, right| left.name.cmp(&right.name));
    append_u64(&mut bytes, features.len() as u64);
    for feature in features {
        append_string(&mut bytes, &feature.name);
        append_string(&mut bytes, &feature.unit);
    }

    let mut evidence_refs = manifest.evidence_refs.iter().collect::<Vec<_>>();
    evidence_refs.sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
    append_u64(&mut bytes, evidence_refs.len() as u64);
    for evidence in evidence_refs {
        append_string(&mut bytes, &evidence.evidence_id);
        append_string(&mut bytes, &evidence.evidence_kind);
        append_string(&mut bytes, &evidence.source_ref);
        append_string(&mut bytes, &evidence.license_applicability);
        append_string(&mut bytes, &evidence.artifact_sha256);
        append_string(&mut bytes, &evidence.scope_sha256);
    }

    let mut regions = manifest.regions.iter().collect::<Vec<_>>();
    regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    append_u64(&mut bytes, regions.len() as u64);
    for region in regions {
        append_string(&mut bytes, &region.region_id);
        append_string(&mut bytes, &region.geometry_family);
        append_string(&mut bytes, &region.flow_regime);
        append_string(&mut bytes, region.support.as_str());
        append_string(&mut bytes, &region.rationale);
        let mut evidence_ids = region.supporting_evidence_ids.iter().collect::<Vec<_>>();
        evidence_ids.sort();
        append_u64(&mut bytes, evidence_ids.len() as u64);
        for evidence_id in evidence_ids {
            append_string(&mut bytes, evidence_id);
        }
        let mut bounds = region.bounds.iter().collect::<Vec<_>>();
        bounds.sort_by(|left, right| left.name.cmp(&right.name));
        append_u64(&mut bytes, bounds.len() as u64);
        for bound in bounds {
            append_string(&mut bytes, &bound.name);
            append_string(&mut bytes, &bound.unit);
            append_f64(&mut bytes, bound.lower_inclusive);
            append_f64(&mut bytes, bound.upper_inclusive);
        }
    }
    bytes
}

/// Validate and seal a manifest. Canonicalization is independent of caller row ordering.
pub fn seal_applicability_domain_manifest(
    mut manifest: ApplicabilityDomainManifest,
) -> Result<ApplicabilityDomainManifest, String> {
    validate_manifest_payload(&manifest)?;
    manifest.manifest_sha256 = sha256_hex(&canonical_manifest_bytes(&manifest))?;
    Ok(manifest)
}

pub fn verify_applicability_domain_manifest(
    manifest: &ApplicabilityDomainManifest,
) -> Result<(), String> {
    validate_manifest_payload(manifest)?;
    if !is_sha256(&manifest.manifest_sha256) {
        return Err("applicability manifest digest is malformed".to_owned());
    }
    let actual = sha256_hex(&canonical_manifest_bytes(manifest))?;
    if actual != manifest.manifest_sha256 {
        return Err("applicability manifest canonical digest mismatch".to_owned());
    }
    Ok(())
}

fn canonical_query_bytes(query: &ApplicabilityQuery) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, "aero-applicability-query/0.1.0");
    append_string(&mut bytes, &query.model_id);
    append_string(&mut bytes, &query.quantity);
    append_string(&mut bytes, &query.geometry_family);
    append_string(&mut bytes, &query.flow_regime);
    append_string(&mut bytes, &query.solver_identity_sha256);
    append_string(&mut bytes, &query.output_definition_sha256);
    let mut features = query.features.iter().collect::<Vec<_>>();
    features.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.unit.cmp(&right.unit))
            .then_with(|| left.value.total_cmp(&right.value))
    });
    append_u64(&mut bytes, features.len() as u64);
    for feature in features {
        append_string(&mut bytes, &feature.name);
        append_string(&mut bytes, &feature.unit);
        append_f64(&mut bytes, feature.value);
    }
    bytes
}

fn assessment_bytes(assessment: &ApplicabilityAssessment) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, &assessment.schema);
    append_string(&mut bytes, &assessment.model_id);
    append_string(&mut bytes, &assessment.quantity);
    append_string(&mut bytes, &assessment.manifest_sha256);
    append_string(&mut bytes, &assessment.query_sha256);
    append_string(&mut bytes, assessment.status.as_str());
    append_u64(&mut bytes, assessment.matched_region_ids.len() as u64);
    for region_id in &assessment.matched_region_ids {
        append_string(&mut bytes, region_id);
    }
    append_string(&mut bytes, &assessment.reason);
    append_string(&mut bytes, &assessment.evidence_level);
    bytes
}

fn finish_assessment(
    manifest: &ApplicabilityDomainManifest,
    query: &ApplicabilityQuery,
    status: ApplicabilityStatus,
    mut matched_region_ids: Vec<String>,
    reason: impl Into<String>,
) -> Result<ApplicabilityAssessment, String> {
    matched_region_ids.sort();
    let mut assessment = ApplicabilityAssessment {
        schema: APPLICABILITY_DOMAIN_SCHEMA.to_owned(),
        model_id: manifest.model_id.clone(),
        quantity: manifest.quantity.clone(),
        manifest_sha256: manifest.manifest_sha256.clone(),
        query_sha256: sha256_hex(&canonical_query_bytes(query))?,
        status,
        matched_region_ids,
        reason: reason.into(),
        evidence_level: APPLICABILITY_DOMAIN_EVIDENCE_LEVEL.to_owned(),
        assessment_sha256: String::new(),
    };
    assessment.assessment_sha256 = sha256_hex(&assessment_bytes(&assessment))?;
    Ok(assessment)
}

fn unknown(
    manifest: &ApplicabilityDomainManifest,
    query: &ApplicabilityQuery,
    reason: impl Into<String>,
) -> Result<ApplicabilityAssessment, String> {
    finish_assessment(
        manifest,
        query,
        ApplicabilityStatus::UnknownDueToMissingOrUnqualifiedInputs,
        Vec::new(),
        reason,
    )
}

fn region_contains(
    region: &ApplicabilityDomainRegion,
    values: &BTreeMap<&str, f64>,
) -> bool {
    region.bounds.iter().all(|bound| {
        values.get(bound.name.as_str()).is_some_and(|value| {
            *value >= bound.lower_inclusive && *value <= bound.upper_inclusive
        })
    })
}

/// Evaluate exact model/quantity/geometry/regime identity plus union-of-regions support.
///
/// Multiple matching regions are treated as ambiguous and return Unknown rather than selecting
/// the most favourable support class. Unknown inputs are never imputed or unit-converted.
pub fn assess_applicability_domain(
    manifest: &ApplicabilityDomainManifest,
    query: &ApplicabilityQuery,
) -> Result<ApplicabilityAssessment, String> {
    verify_applicability_domain_manifest(manifest)?;

    for (name, value) in [
        ("query model_id", query.model_id.as_str()),
        ("query quantity", query.quantity.as_str()),
        ("query geometry_family", query.geometry_family.as_str()),
        ("query flow_regime", query.flow_regime.as_str()),
    ] {
        if value.trim().is_empty() {
            return unknown(manifest, query, format!("{name} is missing"));
        }
    }
    if query.model_id != manifest.model_id || query.quantity != manifest.quantity {
        return unknown(
            manifest,
            query,
            "query model or quantity differs from the applicability manifest",
        );
    }
    if query.solver_identity_sha256 != manifest.solver_identity_sha256
        || query.output_definition_sha256 != manifest.output_definition_sha256
    {
        return unknown(
            manifest,
            query,
            "solver or output-definition identity differs from the applicability manifest",
        );
    }

    let mut values = BTreeMap::new();
    for feature in &query.features {
        if feature.name.trim().is_empty() || feature.unit.trim().is_empty() {
            return unknown(manifest, query, "query contains a missing feature name or unit");
        }
        if !feature.value.is_finite() {
            return unknown(
                manifest,
                query,
                format!("query feature {} is non-finite", feature.name),
            );
        }
        if values.insert(feature.name.as_str(), feature.value).is_some() {
            return unknown(
                manifest,
                query,
                format!("query duplicates feature {}", feature.name),
            );
        }
    }

    let expected = manifest
        .feature_schema
        .iter()
        .map(|feature| (feature.name.as_str(), feature.unit.as_str()))
        .collect::<BTreeMap<_, _>>();
    if values.len() != expected.len()
        || expected.keys().any(|name| !values.contains_key(name))
    {
        return unknown(
            manifest,
            query,
            "query feature set is incomplete or contains undeclared features",
        );
    }
    for feature in &query.features {
        if expected.get(feature.name.as_str()).copied() != Some(feature.unit.as_str()) {
            return unknown(
                manifest,
                query,
                format!(
                    "query feature {} has unit {}, expected {}",
                    feature.name,
                    feature.unit,
                    expected.get(feature.name.as_str()).copied().unwrap_or("undeclared")
                ),
            );
        }
    }

    let matching = manifest
        .regions
        .iter()
        .filter(|region| {
            region.geometry_family == query.geometry_family
                && region.flow_regime == query.flow_regime
                && region_contains(region, &values)
        })
        .collect::<Vec<_>>();

    if matching.len() > 1 {
        return finish_assessment(
            manifest,
            query,
            ApplicabilityStatus::UnknownDueToMissingOrUnqualifiedInputs,
            matching.iter().map(|region| region.region_id.clone()).collect(),
            "query matches overlapping support regions; policy is ambiguous",
        );
    }
    let Some(region) = matching.first() else {
        return finish_assessment(
            manifest,
            query,
            ApplicabilityStatus::OutOfDomain,
            Vec::new(),
            "query is outside every explicitly declared support region for this geometry family and flow regime",
        );
    };

    finish_assessment(
        manifest,
        query,
        region.support.status(),
        vec![region.region_id.clone()],
        format!(
            "query lies within declared region {}; support class is {}",
            region.region_id,
            region.support.as_str()
        ),
    )
}

pub fn verify_applicability_assessment(
    manifest: &ApplicabilityDomainManifest,
    query: &ApplicabilityQuery,
    assessment: &ApplicabilityAssessment,
) -> Result<(), String> {
    if assessment.schema != APPLICABILITY_DOMAIN_SCHEMA
        || assessment.evidence_level != APPLICABILITY_DOMAIN_EVIDENCE_LEVEL
    {
        return Err("unsupported applicability assessment schema or evidence ceiling".to_owned());
    }
    let replayed = assess_applicability_domain(manifest, query)?;
    if replayed != *assessment {
        return Err("replayed applicability assessment differs from supplied report".to_owned());
    }
    if sha256_hex(&assessment_bytes(assessment))? != assessment.assessment_sha256 {
        return Err("applicability assessment canonical digest mismatch".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn bound(name: &str, unit: &str, lower: f64, upper: f64) -> ApplicabilityFeatureBound {
        ApplicabilityFeatureBound {
            name: name.to_owned(),
            unit: unit.to_owned(),
            lower_inclusive: lower,
            upper_inclusive: upper,
        }
    }

    fn region(
        id: &str,
        lower_mach: f64,
        upper_mach: f64,
        support: ApplicabilitySupportClass,
    ) -> ApplicabilityDomainRegion {
        ApplicabilityDomainRegion {
            region_id: id.to_owned(),
            geometry_family: "OAT15A".to_owned(),
            flow_regime: "transonic_steady".to_owned(),
            support,
            rationale: format!("synthetic fixture region {id}"),
            supporting_evidence_ids: vec![format!("synthetic-validation-{id}")],
            bounds: vec![
                bound("mach", "1", lower_mach, upper_mach),
                bound("reynolds", "1", 2.0e6, 4.0e6),
            ],
        }
    }

    fn manifest() -> ApplicabilityDomainManifest {
        let mut manifest = ApplicabilityDomainManifest {
            model_id: "su2-v8.6.0-sa".to_owned(),
            quantity: "CD".to_owned(),
            quantity_unit: "1".to_owned(),
            solver_identity_sha256: digest('a'),
            output_definition_sha256: digest('b'),
            feature_schema: vec![
                ApplicabilityFeatureSpec {
                    name: "mach".to_owned(),
                    unit: "1".to_owned(),
                },
                ApplicabilityFeatureSpec {
                    name: "reynolds".to_owned(),
                    unit: "1".to_owned(),
                },
            ],
            evidence_refs: Vec::new(),
            regions: vec![
                region("dense-low", 0.70, 0.74, ApplicabilitySupportClass::Dense),
                region(
                    "boundary-band",
                    0.75,
                    0.79,
                    ApplicabilitySupportClass::BoundaryOrRegimeTransition,
                ),
                region("sparse-high", 0.80, 0.84, ApplicabilitySupportClass::Sparse),
            ],
            manifest_sha256: String::new(),
        };
        manifest.evidence_refs = manifest
            .regions
            .iter()
            .map(|region| ApplicabilityEvidenceRef {
                evidence_id: region.supporting_evidence_ids[0].clone(),
                evidence_kind: "validation_observation_fixture".to_owned(),
                source_ref: format!("synthetic://aero-applicability/{}", region.region_id),
                license_applicability: "synthetic-ci-fixture-only".to_owned(),
                artifact_sha256: digest('c'),
                scope_sha256: region_scope_sha256(&manifest, region)
                    .expect("compute exact region scope"),
            })
            .collect();
        seal_applicability_domain_manifest(manifest)
            .expect("seal valid applicability manifest")
    }

    fn query(mach: f64) -> ApplicabilityQuery {
        ApplicabilityQuery {
            model_id: "su2-v8.6.0-sa".to_owned(),
            quantity: "CD".to_owned(),
            geometry_family: "OAT15A".to_owned(),
            flow_regime: "transonic_steady".to_owned(),
            solver_identity_sha256: digest('a'),
            output_definition_sha256: digest('b'),
            features: vec![
                ApplicabilityFeatureValue {
                    name: "mach".to_owned(),
                    unit: "1".to_owned(),
                    value: mach,
                },
                ApplicabilityFeatureValue {
                    name: "reynolds".to_owned(),
                    unit: "1".to_owned(),
                    value: 3.0e6,
                },
            ],
        }
    }

    #[test]
    fn explicit_region_union_does_not_fill_the_gap_between_clusters() {
        // 0.745 is inside the convex hull [0.70, 0.84] but outside every declared region.
        let result = assess_applicability_domain(&manifest(), &query(0.745))
            .expect("evaluate out-of-domain point");
        assert_eq!(result.status, ApplicabilityStatus::OutOfDomain);
        assert!(result.matched_region_ids.is_empty());
        assert!(result.reason.contains("outside every"));
    }

    #[test]
    fn support_class_is_preserved_instead_of_collapsing_dense_sparse_and_boundary() {
        let m = manifest();
        let dense = assess_applicability_domain(&m, &query(0.72)).unwrap();
        let boundary = assess_applicability_domain(&m, &query(0.77)).unwrap();
        let sparse = assess_applicability_domain(&m, &query(0.82)).unwrap();
        assert_eq!(
            dense.status,
            ApplicabilityStatus::SupportedWithinDeclaredDomain
        );
        assert_eq!(
            boundary.status,
            ApplicabilityStatus::BoundaryOrRegimeTransition
        );
        assert_eq!(sparse.status, ApplicabilityStatus::SupportedButSparse);
        assert!(dense.status.may_support_claim_admission_without_boundary_review());
        assert!(!boundary.status.may_support_claim_admission_without_boundary_review());
        assert!(boundary.status.is_inside_declared_support());
        assert!(sparse.status.may_support_claim_admission_without_boundary_review());
    }

    #[test]
    fn missing_features_wrong_units_and_model_drift_abstain() {
        let m = manifest();

        let mut missing = query(0.72);
        missing.features.pop();
        assert_eq!(
            assess_applicability_domain(&m, &missing).unwrap().status,
            ApplicabilityStatus::UnknownDueToMissingOrUnqualifiedInputs
        );

        let mut wrong_unit = query(0.72);
        wrong_unit.features[0].unit = "percent".to_owned();
        assert_eq!(
            assess_applicability_domain(&m, &wrong_unit).unwrap().status,
            ApplicabilityStatus::UnknownDueToMissingOrUnqualifiedInputs
        );

        let mut changed_solver = query(0.72);
        changed_solver.solver_identity_sha256 = digest('c');
        assert_eq!(
            assess_applicability_domain(&m, &changed_solver).unwrap().status,
            ApplicabilityStatus::UnknownDueToMissingOrUnqualifiedInputs
        );
    }

    #[test]
    fn overlapping_regions_are_never_resolved_by_best_case_precedence() {
        let mut m = manifest();
        let overlap = region(
            "overlap",
            0.71,
            0.73,
            ApplicabilitySupportClass::Sparse,
        );
        let overlap_evidence_id = overlap.supporting_evidence_ids[0].clone();
        let overlap_scope = region_scope_sha256(&m, &overlap).unwrap();
        m.evidence_refs.push(ApplicabilityEvidenceRef {
            evidence_id: overlap_evidence_id,
            evidence_kind: "validation_observation_fixture".to_owned(),
            source_ref: "synthetic://aero-applicability/overlap".to_owned(),
            license_applicability: "synthetic-ci-fixture-only".to_owned(),
            artifact_sha256: digest('e'),
            scope_sha256: overlap_scope,
        });
        m.regions.push(overlap);
        m = seal_applicability_domain_manifest(m).unwrap();
        let result = assess_applicability_domain(&m, &query(0.72)).unwrap();
        assert_eq!(
            result.status,
            ApplicabilityStatus::UnknownDueToMissingOrUnqualifiedInputs
        );
        assert_eq!(result.matched_region_ids, vec!["dense-low", "overlap"]);
    }

    #[test]
    fn canonical_manifest_and_query_digests_ignore_row_order() {
        let first = manifest();
        let mut reordered = first.clone();
        reordered.feature_schema.reverse();
        reordered.regions.reverse();
        for region in &mut reordered.regions {
            region.bounds.reverse();
        }
        reordered.manifest_sha256.clear();
        let reordered = seal_applicability_domain_manifest(reordered).unwrap();
        assert_eq!(first.manifest_sha256, reordered.manifest_sha256);

        let first_assessment = assess_applicability_domain(&first, &query(0.72)).unwrap();
        let mut reversed_query = query(0.72);
        reversed_query.features.reverse();
        let second_assessment =
            assess_applicability_domain(&first, &reversed_query).unwrap();
        assert_eq!(first_assessment.query_sha256, second_assessment.query_sha256);
        assert_eq!(
            first_assessment.assessment_sha256,
            second_assessment.assessment_sha256
        );
    }

    #[test]
    fn changed_model_or_operating_point_changes_assessment_identity() {
        let m = manifest();
        let first = assess_applicability_domain(&m, &query(0.72)).unwrap();
        let changed = assess_applicability_domain(&m, &query(0.721)).unwrap();
        assert_ne!(first.query_sha256, changed.query_sha256);
        assert_ne!(first.assessment_sha256, changed.assessment_sha256);
    }

    #[test]
    fn nonfinite_query_and_tampered_manifest_fail_closed() {
        let m = manifest();
        let mut nonfinite = query(0.72);
        nonfinite.features[0].value = f64::NAN;
        assert_eq!(
            assess_applicability_domain(&m, &nonfinite).unwrap().status,
            ApplicabilityStatus::UnknownDueToMissingOrUnqualifiedInputs
        );

        let mut tampered = m.clone();
        tampered.evidence_refs[0].license_applicability =
            "silently-changed-license-scope".to_owned();
        assert!(verify_applicability_domain_manifest(&tampered)
            .expect_err("tampered manifest must reject")
            .contains("digest mismatch"));
    }

    #[test]
    fn every_region_requires_an_exactly_scoped_evidence_reference() {
        let mut wrong_scope = manifest();
        wrong_scope.evidence_refs[0].scope_sha256 = digest('e');
        assert!(seal_applicability_domain_manifest(wrong_scope)
            .expect_err("wrong evidence scope must reject")
            .contains("scoped to a different"));

        let mut missing_reference = manifest();
        missing_reference.regions[0].supporting_evidence_ids[0] =
            "not-present".to_owned();
        assert!(seal_applicability_domain_manifest(missing_reference)
            .expect_err("missing evidence reference must reject")
            .contains("references missing evidence ID"));

        let mut orphan_reference = manifest();
        orphan_reference.evidence_refs.push(ApplicabilityEvidenceRef {
            evidence_id: "orphan".to_owned(),
            evidence_kind: "validation_observation_fixture".to_owned(),
            source_ref: "synthetic://orphan".to_owned(),
            license_applicability: "synthetic-ci-fixture-only".to_owned(),
            artifact_sha256: digest('e'),
            scope_sha256: digest('f'),
        });
        assert!(seal_applicability_domain_manifest(orphan_reference)
            .expect_err("orphan evidence reference must reject")
            .contains("unused by every region"));
    }

    #[test]
    fn geometry_family_and_flow_regime_are_exact_categorical_gates() {
        let m = manifest();
        let mut changed_geometry = query(0.72);
        changed_geometry.geometry_family = "different-airfoil-family".to_owned();
        assert_eq!(
            assess_applicability_domain(&m, &changed_geometry)
                .unwrap()
                .status,
            ApplicabilityStatus::OutOfDomain
        );

        let mut changed_regime = query(0.72);
        changed_regime.flow_regime = "buffet_unsteady".to_owned();
        assert_eq!(
            assess_applicability_domain(&m, &changed_regime)
                .unwrap()
                .status,
            ApplicabilityStatus::OutOfDomain
        );
    }

    #[test]
    fn assessment_replays_from_exact_manifest_and_query() {
        let m = manifest();
        let q = query(0.72);
        let assessment = assess_applicability_domain(&m, &q).unwrap();
        verify_applicability_assessment(&m, &q, &assessment)
            .expect("assessment should replay exactly");
        let mut altered = assessment.clone();
        altered.status = ApplicabilityStatus::OutOfDomain;
        assert!(verify_applicability_assessment(&m, &q, &altered).is_err());
    }
}
