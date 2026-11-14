// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Versioned, evidence-carrying Power Fabric intermediate representation.
//!
//! The IR composes identities, claims, and typed relationships. It is not an
//! electrical solver, installation approval, physical-state discovery system,
//! or safety certification. Wire DTOs deserialize separately and must pass
//! validated constructors before becoming trusted IR values.

use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

const SUPPORTED_SCHEMA_MAJOR: u16 = 0;
const SUPPORTED_SCHEMA_MINOR: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IrSchemaVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

impl IrSchemaVersion {
    pub fn new(major: u16, minor: u16, patch: u16) -> Result<Self, IrError> {
        if major != SUPPORTED_SCHEMA_MAJOR || minor != SUPPORTED_SCHEMA_MINOR {
            return Err(IrError::UnsupportedSchemaVersion { major, minor });
        }
        Ok(Self { major, minor, patch })
    }

    pub const fn current() -> Self {
        Self { major: 0, minor: 2, patch: 0 }
    }

    pub const fn major(self) -> u16 { self.major }
    pub const fn minor(self) -> u16 { self.minor }
    pub const fn patch(self) -> u16 { self.patch }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct NamespacedType {
    namespace: String,
    local_name: String,
}

impl NamespacedType {
    pub fn new(namespace: impl Into<String>, local_name: impl Into<String>) -> Result<Self, IrError> {
        let namespace = namespace.into();
        let local_name = local_name.into();
        require_non_empty("namespace", &namespace)?;
        require_non_empty("local_name", &local_name)?;
        Ok(Self { namespace, local_name })
    }

    pub fn namespace(&self) -> &str { &self.namespace }
    pub fn local_name(&self) -> &str { &self.local_name }
}

/// The stable ID is distinct from manufacturer, serial, CIM mRID, and
/// IEC 81346 reference-designation identifiers. Source IDs remain namespaced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectIdentity {
    stable_id: String,
    external_identifiers: BTreeMap<String, String>,
}

impl ObjectIdentity {
    pub fn new(
        stable_id: impl Into<String>,
        external_identifiers: BTreeMap<String, String>,
    ) -> Result<Self, IrError> {
        let stable_id = stable_id.into();
        require_non_empty("stable_id", &stable_id)?;
        for (namespace, value) in &external_identifiers {
            require_non_empty("external identifier namespace", namespace)?;
            require_non_empty("external identifier value", value)?;
        }
        Ok(Self { stable_id, external_identifiers })
    }

    pub fn stable_id(&self) -> &str { &self.stable_id }
    pub fn external_identifiers(&self) -> &BTreeMap<String, String> { &self.external_identifiers }
}

/// Claim domains are independent epistemic layers, not an automatic maturity ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimDomain {
    DesignIntent,
    Declared,
    Observed,
    Verified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PropertyValue {
    Text { value: String },
    Boolean { value: bool },
    Integer { value: i64 },
    Decimal { lexical: String, unit: Option<String> },
    Identifier { namespace: String, value: String },
}

impl PropertyValue {
    fn validate(&self) -> Result<(), IrError> {
        match self {
            Self::Text { value } => require_non_empty("text property value", value),
            Self::Boolean { .. } | Self::Integer { .. } => Ok(()),
            Self::Decimal { lexical, unit } => {
                if lexical.trim().is_empty()
                    || lexical.parse::<f64>().map_or(true, |value| !value.is_finite())
                {
                    return Err(IrError::InvalidDecimal { lexical: lexical.clone() });
                }
                if let Some(unit) = unit {
                    require_non_empty("decimal unit", unit)?;
                }
                Ok(())
            }
            Self::Identifier { namespace, value } => {
                require_non_empty("identifier namespace", namespace)?;
                require_non_empty("identifier value", value)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum ClaimState {
    Value(PropertyValue),
    Unknown,
    Unavailable,
    NotApplicable,
    Contradictory { conflicting_claim_ids: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClaimProvenance {
    source_id: String,
    evidence_id: Option<String>,
    recorded_at: Option<String>,
}

impl ClaimProvenance {
    pub fn new(
        source_id: impl Into<String>,
        evidence_id: Option<String>,
        recorded_at: Option<String>,
    ) -> Result<Self, IrError> {
        let source_id = source_id.into();
        require_non_empty("provenance source_id", &source_id)?;
        if let Some(id) = &evidence_id { require_non_empty("provenance evidence_id", id)?; }
        if let Some(timestamp) = &recorded_at { require_non_empty("provenance recorded_at", timestamp)?; }
        Ok(Self { source_id, evidence_id, recorded_at })
    }

    pub fn source_id(&self) -> &str { &self.source_id }
    pub fn evidence_id(&self) -> Option<&str> { self.evidence_id.as_deref() }
    pub fn recorded_at(&self) -> Option<&str> { self.recorded_at.as_deref() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PropertyClaim {
    claim_id: String,
    property_id: String,
    domain: ClaimDomain,
    state: ClaimState,
    provenance: ClaimProvenance,
}

impl PropertyClaim {
    pub fn new(
        claim_id: impl Into<String>,
        property_id: impl Into<String>,
        domain: ClaimDomain,
        state: ClaimState,
        provenance: ClaimProvenance,
    ) -> Result<Self, IrError> {
        let claim_id = claim_id.into();
        let property_id = property_id.into();
        require_non_empty("claim_id", &claim_id)?;
        require_non_empty("property_id", &property_id)?;

        match &state {
            ClaimState::Value(value) => value.validate()?,
            ClaimState::Unknown if provenance.evidence_id.is_some() => {
                return Err(IrError::UnexpectedEvidenceForUnknownClaim);
            }
            ClaimState::Unknown | ClaimState::Unavailable | ClaimState::NotApplicable => {}
            ClaimState::Contradictory { conflicting_claim_ids } => {
                let ids: BTreeSet<&str> = conflicting_claim_ids.iter().map(String::as_str).collect();
                if ids.len() < 2
                    || ids.len() != conflicting_claim_ids.len()
                    || ids.contains(claim_id.as_str())
                    || conflicting_claim_ids.iter().any(|id| id.trim().is_empty())
                {
                    return Err(IrError::InvalidContradiction);
                }
            }
        }

        if matches!(domain, ClaimDomain::Observed | ClaimDomain::Verified)
            && provenance.evidence_id.is_none()
        {
            return Err(IrError::EvidenceRequiredForDomain { domain });
        }

        Ok(Self { claim_id, property_id, domain, state, provenance })
    }

    pub fn claim_id(&self) -> &str { &self.claim_id }
    pub fn property_id(&self) -> &str { &self.property_id }
    pub const fn domain(&self) -> ClaimDomain { self.domain }
    pub fn state(&self) -> &ClaimState { &self.state }
    pub fn provenance(&self) -> &ClaimProvenance { &self.provenance }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrObject {
    identity: ObjectIdentity,
    kind: NamespacedType,
    #[serde(serialize_with = "serialize_map_values")]
    claims: BTreeMap<String, PropertyClaim>,
}

impl IrObject {
    pub fn new(
        identity: ObjectIdentity,
        kind: NamespacedType,
        claims: Vec<PropertyClaim>,
    ) -> Result<Self, IrError> {
        let mut by_id = BTreeMap::new();
        for claim in claims {
            let id = claim.claim_id.clone();
            if by_id.insert(id.clone(), claim).is_some() {
                return Err(IrError::DuplicateClaimId { claim_id: id });
            }
        }
        for claim in by_id.values() {
            if let ClaimState::Contradictory { conflicting_claim_ids } = &claim.state {
                for conflicting_id in conflicting_claim_ids {
                    let Some(conflicting) = by_id.get(conflicting_id) else {
                        return Err(IrError::InvalidContradictionReference {
                            claim_id: claim.claim_id.clone(),
                            conflicting_claim_id: conflicting_id.clone(),
                        });
                    };
                    if conflicting.property_id != claim.property_id {
                        return Err(IrError::InvalidContradictionReference {
                            claim_id: claim.claim_id.clone(),
                            conflicting_claim_id: conflicting_id.clone(),
                        });
                    }
                }
            }
        }
        Ok(Self { identity, kind, claims: by_id })
    }

    pub fn identity(&self) -> &ObjectIdentity { &self.identity }
    pub fn kind(&self) -> &NamespacedType { &self.kind }
    pub fn claims(&self) -> &BTreeMap<String, PropertyClaim> { &self.claims }
    pub fn claims_for_property(&self, property_id: &str) -> impl Iterator<Item = &PropertyClaim> {
        self.claims.values().filter(move |claim| claim.property_id == property_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipDirection { Directed, Undirected }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrRelationship {
    relationship_id: String,
    from_object_id: String,
    to_object_id: String,
    kind: NamespacedType,
    direction: RelationshipDirection,
    domain: ClaimDomain,
    provenance: ClaimProvenance,
}

impl IrRelationship {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        relationship_id: impl Into<String>,
        from_object_id: impl Into<String>,
        to_object_id: impl Into<String>,
        kind: NamespacedType,
        direction: RelationshipDirection,
        domain: ClaimDomain,
        provenance: ClaimProvenance,
    ) -> Result<Self, IrError> {
        let relationship_id = relationship_id.into();
        let from_object_id = from_object_id.into();
        let to_object_id = to_object_id.into();
        require_non_empty("relationship_id", &relationship_id)?;
        require_non_empty("from_object_id", &from_object_id)?;
        require_non_empty("to_object_id", &to_object_id)?;
        if from_object_id == to_object_id {
            return Err(IrError::SelfRelationship { relationship_id });
        }
        if matches!(domain, ClaimDomain::Observed | ClaimDomain::Verified)
            && provenance.evidence_id.is_none()
        {
            return Err(IrError::EvidenceRequiredForDomain { domain });
        }
        Ok(Self { relationship_id, from_object_id, to_object_id, kind, direction, domain, provenance })
    }

    pub fn relationship_id(&self) -> &str { &self.relationship_id }
    pub fn from_object_id(&self) -> &str { &self.from_object_id }
    pub fn to_object_id(&self) -> &str { &self.to_object_id }
    pub fn kind(&self) -> &NamespacedType { &self.kind }
    pub const fn direction(&self) -> RelationshipDirection { self.direction }
    pub const fn domain(&self) -> ClaimDomain { self.domain }
    pub fn provenance(&self) -> &ClaimProvenance { &self.provenance }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum ExtensionOwner {
    Document,
    Object { object_id: String },
    Relationship { relationship_id: String },
}

/// Opaque UTF-8 extension text. The IR validates the envelope but does not
/// parse or rewrite the payload; namespace-specific adapters own interpretation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpaqueExtension {
    extension_id: String,
    namespace: String,
    media_type: String,
    owner: ExtensionOwner,
    payload: String,
}

impl OpaqueExtension {
    pub fn new(
        extension_id: impl Into<String>,
        namespace: impl Into<String>,
        media_type: impl Into<String>,
        owner: ExtensionOwner,
        payload: impl Into<String>,
    ) -> Result<Self, IrError> {
        let extension_id = extension_id.into();
        let namespace = namespace.into();
        let media_type = media_type.into();
        let payload = payload.into();
        require_non_empty("extension_id", &extension_id)?;
        require_non_empty("extension namespace", &namespace)?;
        require_non_empty("extension media_type", &media_type)?;
        Ok(Self { extension_id, namespace, media_type, owner, payload })
    }

    pub fn extension_id(&self) -> &str { &self.extension_id }
    pub fn namespace(&self) -> &str { &self.namespace }
    pub fn media_type(&self) -> &str { &self.media_type }
    pub fn owner(&self) -> &ExtensionOwner { &self.owner }
    pub fn payload(&self) -> &str { &self.payload }
}

/// BTreeMaps provide deterministic key iteration. This is not by itself RFC
/// 8785 canonical JSON, a cryptographic digest, or reproducible compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PowerFabricDocument {
    schema_version: IrSchemaVersion,
    document_id: String,
    revision_id: String,
    #[serde(serialize_with = "serialize_map_values")]
    objects: BTreeMap<String, IrObject>,
    #[serde(serialize_with = "serialize_map_values")]
    relationships: BTreeMap<String, IrRelationship>,
    #[serde(serialize_with = "serialize_map_values")]
    extensions: BTreeMap<String, OpaqueExtension>,
}

impl PowerFabricDocument {
    pub fn new(
        schema_version: IrSchemaVersion,
        document_id: impl Into<String>,
        revision_id: impl Into<String>,
        objects: Vec<IrObject>,
        relationships: Vec<IrRelationship>,
        extensions: Vec<OpaqueExtension>,
    ) -> Result<Self, IrError> {
        let document_id = document_id.into();
        let revision_id = revision_id.into();
        require_non_empty("document_id", &document_id)?;
        require_non_empty("revision_id", &revision_id)?;

        let mut object_map = BTreeMap::new();
        let mut claim_ids = BTreeSet::new();
        for object in objects {
            let id = object.identity.stable_id.clone();
            if object_map.insert(id.clone(), object.clone()).is_some() {
                return Err(IrError::DuplicateObjectId { object_id: id });
            }
            for claim_id in object.claims.keys() {
                if !claim_ids.insert(claim_id.clone()) {
                    return Err(IrError::DuplicateClaimId { claim_id: claim_id.clone() });
                }
            }
        }

        let mut relationship_map = BTreeMap::new();
        for relationship in relationships {
            let id = relationship.relationship_id.clone();
            for endpoint in [&relationship.from_object_id, &relationship.to_object_id] {
                if !object_map.contains_key(endpoint.as_str()) {
                    return Err(IrError::DanglingRelationshipEndpoint {
                        relationship_id: id.clone(),
                        object_id: endpoint.clone(),
                    });
                }
            }
            if relationship_map.insert(id.clone(), relationship).is_some() {
                return Err(IrError::DuplicateRelationshipId { relationship_id: id });
            }
        }

        let mut extension_map = BTreeMap::new();
        for extension in extensions {
            let id = extension.extension_id.clone();
            match &extension.owner {
                ExtensionOwner::Document => {}
                ExtensionOwner::Object { object_id } => {
                    if !object_map.contains_key(object_id) {
                        return Err(IrError::UnknownExtensionOwner { owner_id: object_id.clone() });
                    }
                }
                ExtensionOwner::Relationship { relationship_id } => {
                    if !relationship_map.contains_key(relationship_id) {
                        return Err(IrError::UnknownExtensionOwner { owner_id: relationship_id.clone() });
                    }
                }
            }
            if extension_map.insert(id.clone(), extension).is_some() {
                return Err(IrError::DuplicateExtensionId { extension_id: id });
            }
        }

        Ok(Self {
            schema_version, document_id, revision_id,
            objects: object_map, relationships: relationship_map, extensions: extension_map,
        })
    }

    pub const fn schema_version(&self) -> IrSchemaVersion { self.schema_version }
    pub fn document_id(&self) -> &str { &self.document_id }
    pub fn revision_id(&self) -> &str { &self.revision_id }
    pub fn objects(&self) -> &BTreeMap<String, IrObject> { &self.objects }
    pub fn relationships(&self) -> &BTreeMap<String, IrRelationship> { &self.relationships }
    pub fn extensions(&self) -> &BTreeMap<String, OpaqueExtension> { &self.extensions }
}

/// Untrusted wire representation. Deserialize to this DTO, then call
/// into_validated(); trusted IR types intentionally do not implement Deserialize.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireDocument {
    schema_version: WireSchemaVersion,
    document_id: String,
    revision_id: String,
    objects: Vec<WireObject>,
    relationships: Vec<WireRelationship>,
    extensions: Vec<WireExtension>,
}

impl WireDocument {
    pub fn into_validated(self) -> Result<PowerFabricDocument, IrError> {
        let version = IrSchemaVersion::new(
            self.schema_version.major, self.schema_version.minor, self.schema_version.patch,
        )?;
        let mut objects = Vec::with_capacity(self.objects.len());
        for object in self.objects {
            let identity = ObjectIdentity::new(object.stable_id, object.external_identifiers)?;
            let kind = NamespacedType::new(object.kind.namespace, object.kind.local_name)?;
            let mut claims = Vec::with_capacity(object.claims.len());
            for claim in object.claims {
                claims.push(PropertyClaim::new(
                    claim.claim_id,
                    claim.property_id,
                    claim.domain,
                    claim.state.into_validated()?,
                    ClaimProvenance::new(
                        claim.provenance.source_id,
                        claim.provenance.evidence_id,
                        claim.provenance.recorded_at,
                    )?,
                )?);
            }
            objects.push(IrObject::new(identity, kind, claims)?);
        }

        let mut relationships = Vec::with_capacity(self.relationships.len());
        for relationship in self.relationships {
            relationships.push(IrRelationship::new(
                relationship.relationship_id,
                relationship.from_object_id,
                relationship.to_object_id,
                NamespacedType::new(relationship.kind.namespace, relationship.kind.local_name)?,
                relationship.direction,
                relationship.domain,
                ClaimProvenance::new(
                    relationship.provenance.source_id,
                    relationship.provenance.evidence_id,
                    relationship.provenance.recorded_at,
                )?,
            )?);
        }

        let mut extensions = Vec::with_capacity(self.extensions.len());
        for extension in self.extensions {
            let owner = match extension.owner {
                WireExtensionOwner::Document => ExtensionOwner::Document,
                WireExtensionOwner::Object { object_id } => ExtensionOwner::Object { object_id },
                WireExtensionOwner::Relationship { relationship_id } => ExtensionOwner::Relationship { relationship_id },
            };
            extensions.push(OpaqueExtension::new(
                extension.extension_id, extension.namespace, extension.media_type, owner, extension.payload,
            )?);
        }

        PowerFabricDocument::new(
            version, self.document_id, self.revision_id, objects, relationships, extensions,
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSchemaVersion { major: u16, minor: u16, patch: u16 }

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObject {
    stable_id: String,
    external_identifiers: BTreeMap<String, String>,
    kind: WireNamespacedType,
    claims: Vec<WireClaim>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireNamespacedType { namespace: String, local_name: String }

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireClaim {
    claim_id: String,
    property_id: String,
    domain: ClaimDomain,
    state: WireClaimState,
    provenance: WireProvenance,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
enum WireClaimState {
    Value(WirePropertyValue),
    Unknown,
    Unavailable,
    NotApplicable,
    Contradictory { conflicting_claim_ids: Vec<String> },
}

impl WireClaimState {
    fn into_validated(self) -> Result<ClaimState, IrError> {
        match self {
            Self::Value(value) => Ok(ClaimState::Value(value.into_validated()?)),
            Self::Unknown => Ok(ClaimState::Unknown),
            Self::Unavailable => Ok(ClaimState::Unavailable),
            Self::NotApplicable => Ok(ClaimState::NotApplicable),
            Self::Contradictory { conflicting_claim_ids } => Ok(ClaimState::Contradictory { conflicting_claim_ids }),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WirePropertyValue {
    Text { value: String },
    Boolean { value: bool },
    Integer { value: i64 },
    Decimal { lexical: String, unit: Option<String> },
    Identifier { namespace: String, value: String },
}

impl WirePropertyValue {
    fn into_validated(self) -> Result<PropertyValue, IrError> {
        let value = match self {
            Self::Text { value } => PropertyValue::Text { value },
            Self::Boolean { value } => PropertyValue::Boolean { value },
            Self::Integer { value } => PropertyValue::Integer { value },
            Self::Decimal { lexical, unit } => PropertyValue::Decimal { lexical, unit },
            Self::Identifier { namespace, value } => PropertyValue::Identifier { namespace, value },
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireProvenance { source_id: String, evidence_id: Option<String>, recorded_at: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRelationship {
    relationship_id: String,
    from_object_id: String,
    to_object_id: String,
    kind: WireNamespacedType,
    direction: RelationshipDirection,
    domain: ClaimDomain,
    provenance: WireProvenance,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireExtension {
    extension_id: String,
    namespace: String,
    media_type: String,
    owner: WireExtensionOwner,
    payload: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
enum WireExtensionOwner {
    Document,
    Object { object_id: String },
    Relationship { relationship_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrError {
    EmptyField { field: &'static str },
    UnsupportedSchemaVersion { major: u16, minor: u16 },
    InvalidDecimal { lexical: String },
    UnexpectedEvidenceForUnknownClaim,
    EvidenceRequiredForDomain { domain: ClaimDomain },
    InvalidContradiction,
    InvalidContradictionReference { claim_id: String, conflicting_claim_id: String },
    DuplicateObjectId { object_id: String },
    DuplicateRelationshipId { relationship_id: String },
    DuplicateClaimId { claim_id: String },
    DuplicateExtensionId { extension_id: String },
    DanglingRelationshipEndpoint { relationship_id: String, object_id: String },
    UnknownExtensionOwner { owner_id: String },
    SelfRelationship { relationship_id: String },
}

impl core::fmt::Display for IrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for IrError {}

fn serialize_map_values<K, V, S>(
    values: &BTreeMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    K: Ord,
    V: Serialize,
    S: Serializer,
{
    values.values().collect::<Vec<_>>().serialize(serializer)
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), IrError> {
    if value.trim().is_empty() { return Err(IrError::EmptyField { field }); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(local: &str) -> NamespacedType {
        NamespacedType::new("urn:luminous:power-fabric:test", local).unwrap()
    }
    fn identity(id: &str) -> ObjectIdentity {
        ObjectIdentity::new(id, BTreeMap::new()).unwrap()
    }
    fn provenance(domain: ClaimDomain) -> ClaimProvenance {
        let evidence = matches!(domain, ClaimDomain::Observed | ClaimDomain::Verified)
            .then(|| "evidence:test:001".to_owned());
        ClaimProvenance::new("source:test:001", evidence, Some("2026-11-14T00:00:00Z".to_owned())).unwrap()
    }
    fn object(id: &str) -> IrObject {
        IrObject::new(identity(id), kind("test_object"), Vec::new()).unwrap()
    }
    fn relationship(id: &str, from: &str, to: &str) -> IrRelationship {
        IrRelationship::new(id, from, to, kind("association"), RelationshipDirection::Directed,
            ClaimDomain::Declared, provenance(ClaimDomain::Declared)).unwrap()
    }
    fn document(objects: Vec<IrObject>) -> Result<PowerFabricDocument, IrError> {
        PowerFabricDocument::new(IrSchemaVersion::current(), "document:test", "revision:test:1",
            objects, Vec::new(), Vec::new())
    }

    #[test]
    fn object_and_relationship_iteration_is_deterministic() {
        let doc = PowerFabricDocument::new(IrSchemaVersion::current(), "document:test", "revision:test:1",
            vec![object("z"), object("a"), object("m")],
            vec![relationship("rel-z", "z", "a"), relationship("rel-a", "a", "m")],
            Vec::new()).unwrap();
        assert_eq!(doc.objects().keys().map(String::as_str).collect::<Vec<_>>(), vec!["a", "m", "z"]);
        assert_eq!(doc.relationships().keys().map(String::as_str).collect::<Vec<_>>(), vec!["rel-a", "rel-z"]);
    }

    #[test]
    fn duplicate_object_identity_is_rejected() {
        assert!(matches!(document(vec![object("same"), object("same")]), Err(IrError::DuplicateObjectId { .. })));
    }

    #[test]
    fn dangling_relationship_endpoint_is_rejected() {
        assert!(matches!(PowerFabricDocument::new(IrSchemaVersion::current(), "doc", "rev",
            vec![object("source")], vec![relationship("r1", "source", "missing")], Vec::new()),
            Err(IrError::DanglingRelationshipEndpoint { .. })));
    }

    #[test]
    fn unknown_claim_cannot_carry_an_evidence_identifier() {
        let p = ClaimProvenance::new("source:test", Some("evidence:test".into()), None).unwrap();
        assert_eq!(PropertyClaim::new("c1", "voltage", ClaimDomain::Declared, ClaimState::Unknown, p),
            Err(IrError::UnexpectedEvidenceForUnknownClaim));
    }

    #[test]
    fn observed_and_verified_claims_require_evidence() {
        for domain in [ClaimDomain::Observed, ClaimDomain::Verified] {
            let p = ClaimProvenance::new("source:test", None, None).unwrap();
            assert!(matches!(PropertyClaim::new("c1", "voltage", domain,
                ClaimState::Value(PropertyValue::Decimal { lexical: "230".into(), unit: Some("V".into()) }), p),
                Err(IrError::EvidenceRequiredForDomain { .. })));
        }
    }

    #[test]
    fn contradictory_claim_requires_two_distinct_non_self_references() {
        let p = provenance(ClaimDomain::Declared);
        assert_eq!(PropertyClaim::new("c1", "voltage", ClaimDomain::Declared,
            ClaimState::Contradictory { conflicting_claim_ids: vec!["c1".into(), "c2".into()] }, p.clone()),
            Err(IrError::InvalidContradiction));
        assert_eq!(PropertyClaim::new("c3", "voltage", ClaimDomain::Declared,
            ClaimState::Contradictory { conflicting_claim_ids: vec!["c1".into(), "c1".into()] }, p),
            Err(IrError::InvalidContradiction));
    }

    #[test]
    fn decimal_lexical_value_is_preserved_and_non_finite_is_rejected() {
        let valid = PropertyValue::Decimal { lexical: "230.000".into(), unit: Some("V".into()) };
        valid.validate().unwrap();
        assert_eq!(valid, PropertyValue::Decimal { lexical: "230.000".into(), unit: Some("V".into()) });
        assert!(matches!(PropertyValue::Decimal { lexical: "NaN".into(), unit: Some("V".into()) }.validate(),
            Err(IrError::InvalidDecimal { .. })));
    }

    #[test]
    fn claim_domains_remain_separately_addressable_for_one_property() {
        let claims = vec![
            PropertyClaim::new("design-1", "nominal_voltage", ClaimDomain::DesignIntent,
                ClaimState::Value(PropertyValue::Decimal { lexical: "230".into(), unit: Some("V".into()) }),
                provenance(ClaimDomain::DesignIntent)).unwrap(),
            PropertyClaim::new("declared-1", "nominal_voltage", ClaimDomain::Declared,
                ClaimState::Value(PropertyValue::Decimal { lexical: "240".into(), unit: Some("V".into()) }),
                provenance(ClaimDomain::Declared)).unwrap(),
        ];
        let asset = IrObject::new(identity("asset:1"), kind("load"), claims).unwrap();
        assert_eq!(asset.claims_for_property("nominal_voltage").count(), 2);
        assert_eq!(asset.claims()["design-1"].domain(), ClaimDomain::DesignIntent);
        assert_eq!(asset.claims()["declared-1"].domain(), ClaimDomain::Declared);
    }


    #[test]
    fn contradictory_references_must_resolve_to_same_property_on_same_object() {
        let p = provenance(ClaimDomain::Declared);
        let first = PropertyClaim::new(
            "claim:a", "nominal_voltage", ClaimDomain::Declared,
            ClaimState::Value(PropertyValue::Decimal { lexical: "230".into(), unit: Some("V".into()) }),
            p.clone()
        ).unwrap();
        let second = PropertyClaim::new(
            "claim:b", "nominal_voltage", ClaimDomain::Declared,
            ClaimState::Value(PropertyValue::Decimal { lexical: "240".into(), unit: Some("V".into()) }),
            p.clone()
        ).unwrap();
        let contradiction = PropertyClaim::new(
            "claim:c", "nominal_voltage", ClaimDomain::Declared,
            ClaimState::Contradictory { conflicting_claim_ids: vec!["claim:a".into(), "claim:b".into()] },
            p
        ).unwrap();
        assert!(IrObject::new(identity("asset:1"), kind("asset"),
            vec![first, second, contradiction]).is_ok());

        let missing = PropertyClaim::new(
            "claim:d", "nominal_voltage", ClaimDomain::Declared,
            ClaimState::Contradictory { conflicting_claim_ids: vec!["missing:a".into(), "missing:b".into()] },
            provenance(ClaimDomain::Declared)
        ).unwrap();
        assert!(matches!(
            IrObject::new(identity("asset:2"), kind("asset"), vec![missing]),
            Err(IrError::InvalidContradictionReference { .. })
        ));
    }

    #[test]
    fn duplicate_claim_identity_is_rejected_within_object() {
        let p = provenance(ClaimDomain::Declared);
        let a = PropertyClaim::new("same", "a", ClaimDomain::Declared, ClaimState::Value(PropertyValue::Boolean { value: true }), p.clone()).unwrap();
        let b = PropertyClaim::new("same", "b", ClaimDomain::Declared, ClaimState::Value(PropertyValue::Boolean { value: false }), p).unwrap();
        assert!(matches!(IrObject::new(identity("x"), kind("asset"), vec![a, b]), Err(IrError::DuplicateClaimId { .. })));
    }

    #[test]
    fn opaque_extension_payload_is_retained_as_exact_utf8_text() {
        let payload = "<VendorExtension revision=\"7\"><Opaque>keep &amp; exact</Opaque></VendorExtension>";
        let extension = OpaqueExtension::new("ext:1", "urn:vendor:extension", "application/xml",
            ExtensionOwner::Object { object_id: "asset:1".into() }, payload).unwrap();
        let doc = PowerFabricDocument::new(IrSchemaVersion::current(), "doc", "rev",
            vec![object("asset:1")], Vec::new(), vec![extension]).unwrap();
        assert_eq!(doc.extensions()["ext:1"].payload().as_bytes(), payload.as_bytes());
    }

    #[test]
    fn extension_owner_must_exist() {
        let extension = OpaqueExtension::new("ext:1", "urn:vendor:x", "application/json",
            ExtensionOwner::Object { object_id: "missing".into() }, "{}").unwrap();
        assert!(matches!(PowerFabricDocument::new(IrSchemaVersion::current(), "doc", "rev",
            vec![object("present")], Vec::new(), vec![extension]), Err(IrError::UnknownExtensionOwner { .. })));
    }

    #[test]
    fn duplicate_extension_identity_is_rejected() {
        let a = OpaqueExtension::new("same", "urn:v", "text/plain", ExtensionOwner::Document, "a").unwrap();
        let b = OpaqueExtension::new("same", "urn:v", "text/plain", ExtensionOwner::Document, "b").unwrap();
        assert!(matches!(PowerFabricDocument::new(IrSchemaVersion::current(), "doc", "rev",
            vec![object("x")], Vec::new(), vec![a, b]), Err(IrError::DuplicateExtensionId { .. })));
    }

    #[test]
    fn future_schema_major_and_minor_are_rejected() {
        assert_eq!(IrSchemaVersion::new(1, 0, 0), Err(IrError::UnsupportedSchemaVersion { major: 1, minor: 0 }));
        assert_eq!(IrSchemaVersion::new(0, 3, 0), Err(IrError::UnsupportedSchemaVersion { major: 0, minor: 3 }));
    }

    #[test]
    fn wire_ingestion_uses_validated_constructors() {
        let wire = WireDocument {
            schema_version: WireSchemaVersion { major: 0, minor: 2, patch: 0 },
            document_id: "doc:wire".into(), revision_id: "rev:wire:1".into(),
            objects: vec![WireObject {
                stable_id: "asset:wire".into(),
                external_identifiers: BTreeMap::from([("urn:vendor:serial".into(), "SN-001".into())]),
                kind: WireNamespacedType { namespace: "urn:pf:test".into(), local_name: "battery".into() },
                claims: vec![WireClaim {
                    claim_id: "claim:wire".into(), property_id: "capacity".into(), domain: ClaimDomain::Observed,
                    state: WireClaimState::Value(WirePropertyValue::Decimal { lexical: "12.50".into(), unit: Some("kWh".into()) }),
                    provenance: WireProvenance { source_id: "inspection:001".into(), evidence_id: Some("evidence:001".into()),
                        recorded_at: Some("2026-11-14T10:00:00+02:00".into()) },
                }],
            }],
            relationships: Vec::new(),
            extensions: vec![WireExtension {
                extension_id: "ext:wire".into(), namespace: "urn:vendor:extension".into(),
                media_type: "application/xml".into(),
                owner: WireExtensionOwner::Object { object_id: "asset:wire".into() },
                payload: "<VendorExtension>retain</VendorExtension>".into(),
            }],
        };
        let doc = wire.into_validated().unwrap();
        assert_eq!(doc.objects()["asset:wire"].claims()["claim:wire"].domain(), ClaimDomain::Observed);
        assert_eq!(doc.extensions()["ext:wire"].payload(), "<VendorExtension>retain</VendorExtension>");
    }

    #[test]
    fn wire_ingestion_rejects_empty_stable_identity() {
        let wire = WireDocument {
            schema_version: WireSchemaVersion { major: 0, minor: 2, patch: 0 },
            document_id: "doc".into(), revision_id: "rev".into(),
            objects: vec![WireObject { stable_id: " ".into(), external_identifiers: BTreeMap::new(),
                kind: WireNamespacedType { namespace: "urn:pf".into(), local_name: "asset".into() }, claims: Vec::new() }],
            relationships: Vec::new(), extensions: Vec::new(),
        };
        assert_eq!(wire.into_validated().unwrap_err(), IrError::EmptyField { field: "stable_id" });
    }

    #[test]
    fn duplicate_claim_ids_are_rejected_across_objects() {
        let a_claim = PropertyClaim::new("claim:same", "p", ClaimDomain::Declared, ClaimState::Unknown,
            provenance(ClaimDomain::Declared)).unwrap();
        let b_claim = PropertyClaim::new("claim:same", "q", ClaimDomain::Declared, ClaimState::Unavailable,
            provenance(ClaimDomain::Declared)).unwrap();
        let a = IrObject::new(identity("a"), kind("asset"), vec![a_claim]).unwrap();
        let b = IrObject::new(identity("b"), kind("asset"), vec![b_claim]).unwrap();
        assert!(matches!(document(vec![a, b]), Err(IrError::DuplicateClaimId { .. })));
    }

    #[test]
    fn duplicate_relationship_identity_is_rejected() {
        assert!(matches!(PowerFabricDocument::new(IrSchemaVersion::current(), "doc", "rev",
            vec![object("a"), object("b")],
            vec![relationship("r", "a", "b"), relationship("r", "b", "a")], Vec::new()),
            Err(IrError::DuplicateRelationshipId { .. })));
    }
}
