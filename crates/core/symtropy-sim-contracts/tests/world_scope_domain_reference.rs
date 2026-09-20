// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! WORLD-SCOPE-DOMAIN-00A executable reference oracle.
//!
//! This file freezes scope-membership/existence algebra only. It does not own
//! hierarchy mutation, content generation, residency, scheduling, networking,
//! rendering, or evolved domain state.
//!
//! The key separation is:
//!
//! `scope exists != scope contents generated != scope resident != scope current`

use std::collections::{BTreeMap, BTreeSet};

use symtropy_sim_contracts::{
    ContractError, DigestAlgorithm, ScopeId, TypedDigest32, WorldInstanceId,
};

const MAX_EXPLICIT_SCOPES: usize = 4_096;
const MAX_GENERATED_SCOPES: u64 = 65_536;
const EXPLICIT_DOMAIN_TAG: &[u8] = b"symtropy.scope-domain.explicit.v1\0";
const GENERATED_DOMAIN_TAG: &[u8] = b"symtropy.scope-domain.generated.v1\0";
const GENERATED_SCOPE_TAG: &[u8] = b"symtropy.scope-domain.generated-scope.v1\0";

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthorizedScope {
    world: WorldInstanceId,
    scope: ScopeId,
    parent: ScopeId,
    domain_identity: TypedDigest32,
    semantic_key: Vec<u8>,
    authorization_proof: TypedDigest32,
}

impl AuthorizedScope {
    fn validate(&self) -> Result<(), RefError> {
        self.world.validate()?;
        self.scope.validate()?;
        self.parent.validate()?;
        self.domain_identity.validate()?;
        self.authorization_proof.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Resolution {
    Authorized(AuthorizedScope),
    QualifiedNoScope,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExplicitEntry {
    scope: ScopeId,
    parent: ScopeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExplicitScopeDomain {
    world: WorldInstanceId,
    entries: Vec<ExplicitEntry>,
    identity: TypedDigest32,
}

impl ExplicitScopeDomain {
    fn new(world: WorldInstanceId, mut entries: Vec<ExplicitEntry>) -> Result<Self, RefError> {
        if entries.len() > MAX_EXPLICIT_SCOPES {
            return Err(RefError::TooManyExplicitScopes);
        }
        world.validate()?;
        entries.sort();

        let mut seen = BTreeSet::new();
        for entry in &entries {
            entry.scope.validate()?;
            entry.parent.validate()?;
            if !seen.insert(entry.scope.clone()) {
                return Err(RefError::DuplicateScope);
            }
        }

        let identity = explicit_domain_identity(&world, &entries)?;
        Ok(Self {
            world,
            entries,
            identity,
        })
    }

    fn resolve(&self, requested: &ScopeId) -> Result<Resolution, RefError> {
        requested.validate()?;
        let Some(entry) = self.entries.iter().find(|entry| &entry.scope == requested) else {
            return Ok(Resolution::QualifiedNoScope);
        };

        let authorization_proof = authorization_proof(
            &self.world,
            &self.identity,
            &entry.scope,
            &entry.parent,
            entry.scope.as_str().as_bytes(),
        )?;

        Ok(Resolution::Authorized(AuthorizedScope {
            world: self.world.clone(),
            scope: entry.scope.clone(),
            parent: entry.parent.clone(),
            domain_identity: self.identity.clone(),
            semantic_key: entry.scope.as_str().as_bytes().to_vec(),
            authorization_proof,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedScopeDomain {
    world: WorldInstanceId,
    parent: ScopeId,
    profile_identity: TypedDigest32,
    seed_identity: TypedDigest32,
    count: u64,
    identity: TypedDigest32,
}

impl GeneratedScopeDomain {
    fn new(
        world: WorldInstanceId,
        parent: ScopeId,
        profile_identity: TypedDigest32,
        seed_identity: TypedDigest32,
        count: u64,
    ) -> Result<Self, RefError> {
        world.validate()?;
        parent.validate()?;
        profile_identity.validate()?;
        seed_identity.validate()?;
        if count > MAX_GENERATED_SCOPES {
            return Err(RefError::GeneratedBoundExceeded);
        }

        let identity = generated_domain_identity(
            &world,
            &parent,
            &profile_identity,
            &seed_identity,
            count,
        )?;

        Ok(Self {
            world,
            parent,
            profile_identity,
            seed_identity,
            count,
            identity,
        })
    }

    fn resolve_index(&self, index: u64) -> Result<Resolution, RefError> {
        if index >= self.count {
            return Ok(Resolution::QualifiedNoScope);
        }

        let semantic_key = index.to_le_bytes().to_vec();
        let scope = derive_generated_scope_id(
            &self.world,
            &self.parent,
            &self.profile_identity,
            &self.seed_identity,
            index,
        )?;
        let authorization_proof = authorization_proof(
            &self.world,
            &self.identity,
            &scope,
            &self.parent,
            &semantic_key,
        )?;

        Ok(Resolution::Authorized(AuthorizedScope {
            world: self.world.clone(),
            scope,
            parent: self.parent.clone(),
            domain_identity: self.identity.clone(),
            semantic_key,
            authorization_proof,
        }))
    }

    fn expansion_preserves_existing_ids(&self, expanded_count: u64) -> Result<bool, RefError> {
        if expanded_count < self.count || expanded_count > MAX_GENERATED_SCOPES {
            return Err(RefError::InvalidExpansion);
        }

        let expanded = Self::new(
            self.world.clone(),
            self.parent.clone(),
            self.profile_identity.clone(),
            self.seed_identity.clone(),
            expanded_count,
        )?;

        for index in 0..self.count {
            let before = authorized(self.resolve_index(index)?)?;
            let after = authorized(expanded.resolve_index(index)?)?;
            if before.scope != after.scope || before.parent != after.parent {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnavailableScopeDomain;

impl UnavailableScopeDomain {
    fn resolve(&self) -> Resolution {
        Resolution::Incomplete
    }
}

fn explicit_domain_identity(
    world: &WorldInstanceId,
    entries: &[ExplicitEntry],
) -> Result<TypedDigest32, RefError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(EXPLICIT_DOMAIN_TAG);
    push_string(&mut bytes, world.as_str());
    bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        push_string(&mut bytes, entry.scope.as_str());
        push_string(&mut bytes, entry.parent.as_str());
    }
    Ok(TypedDigest32::sha256(
        "symtropy.scope-domain.explicit.identity.v1",
        1,
        &bytes,
    )?)
}

fn generated_domain_identity(
    world: &WorldInstanceId,
    parent: &ScopeId,
    profile_identity: &TypedDigest32,
    seed_identity: &TypedDigest32,
    count: u64,
) -> Result<TypedDigest32, RefError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(GENERATED_DOMAIN_TAG);
    push_string(&mut bytes, world.as_str());
    push_string(&mut bytes, parent.as_str());
    push_digest(&mut bytes, profile_identity);
    push_digest(&mut bytes, seed_identity);
    bytes.extend_from_slice(&count.to_le_bytes());
    Ok(TypedDigest32::sha256(
        "symtropy.scope-domain.generated.identity.v1",
        1,
        &bytes,
    )?)
}

fn derive_generated_scope_id(
    world: &WorldInstanceId,
    parent: &ScopeId,
    profile_identity: &TypedDigest32,
    seed_identity: &TypedDigest32,
    index: u64,
) -> Result<ScopeId, RefError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(GENERATED_SCOPE_TAG);
    push_string(&mut bytes, world.as_str());
    push_string(&mut bytes, parent.as_str());
    push_digest(&mut bytes, profile_identity);
    push_digest(&mut bytes, seed_identity);
    bytes.extend_from_slice(&index.to_le_bytes());

    let digest = TypedDigest32::sha256(
        "symtropy.scope-domain.generated-scope.identity.v1",
        1,
        &bytes,
    )?;
    ScopeId::parse(format!("generated:{}", hex(&digest.value))).map_err(RefError::from)
}

fn authorization_proof(
    world: &WorldInstanceId,
    domain_identity: &TypedDigest32,
    scope: &ScopeId,
    parent: &ScopeId,
    semantic_key: &[u8],
) -> Result<TypedDigest32, RefError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"symtropy.scope-domain.authorization.v1\0");
    push_string(&mut bytes, world.as_str());
    push_digest(&mut bytes, domain_identity);
    push_string(&mut bytes, scope.as_str());
    push_string(&mut bytes, parent.as_str());
    bytes.extend_from_slice(&(semantic_key.len() as u32).to_le_bytes());
    bytes.extend_from_slice(semantic_key);
    Ok(TypedDigest32::sha256(
        "symtropy.scope-domain.authorization-proof.v1",
        1,
        &bytes,
    )?)
}

fn ensure_distinct_semantic_subjects(
    authorizations: &[AuthorizedScope],
) -> Result<(), RefError> {
    let mut by_scope: BTreeMap<&ScopeId, &[u8]> = BTreeMap::new();
    for authorization in authorizations {
        authorization.validate()?;
        if let Some(existing_key) = by_scope.insert(&authorization.scope, &authorization.semantic_key)
            && existing_key != authorization.semantic_key.as_slice()
        {
            return Err(RefError::IdentityCollision);
        }
    }
    Ok(())
}

fn authorized(result: Resolution) -> Result<AuthorizedScope, RefError> {
    match result {
        Resolution::Authorized(value) => Ok(value),
        Resolution::QualifiedNoScope => Err(RefError::UnexpectedNoScope),
        Resolution::Incomplete => Err(RefError::UnexpectedIncomplete),
    }
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_digest(bytes: &mut Vec<u8>, digest: &TypedDigest32) {
    push_string(bytes, &digest.domain);
    match &digest.algorithm {
        DigestAlgorithm::Sha256 => bytes.push(0),
        DigestAlgorithm::Other(name) => {
            bytes.push(1);
            push_string(bytes, name);
        }
    }
    bytes.extend_from_slice(&digest.schema_version.to_le_bytes());
    bytes.extend_from_slice(&digest.value);
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    TooManyExplicitScopes,
    DuplicateScope,
    GeneratedBoundExceeded,
    InvalidExpansion,
    IdentityCollision,
    UnexpectedNoScope,
    UnexpectedIncomplete,
}

impl From<ContractError> for RefError {
    fn from(_: ContractError) -> Self {
        Self::Contract
    }
}

fn world(value: &str) -> WorldInstanceId {
    WorldInstanceId::parse(value).unwrap()
}

fn scope(value: &str) -> ScopeId {
    ScopeId::parse(value).unwrap()
}

fn digest(domain: &str, value: &[u8]) -> TypedDigest32 {
    TypedDigest32::sha256(domain, 1, value).unwrap()
}

fn generated(world_id: &str, count: u64) -> GeneratedScopeDomain {
    GeneratedScopeDomain::new(
        world(world_id),
        scope("galaxy:root"),
        digest("scope-domain.profile.v1", b"bounded-indexed-star-field-v1"),
        digest("scope-domain.seed.v1", b"seed-42"),
        count,
    )
    .unwrap()
}

#[test]
fn explicit_set_proves_present_and_absent_exactly() {
    let domain = ExplicitScopeDomain::new(
        world("world:explicit"),
        vec![
            ExplicitEntry {
                scope: scope("system:sol"),
                parent: scope("galaxy:root"),
            },
            ExplicitEntry {
                scope: scope("system:alpha-centauri"),
                parent: scope("galaxy:root"),
            },
        ],
    )
    .unwrap();

    let present = authorized(domain.resolve(&scope("system:sol")).unwrap()).unwrap();
    assert_eq!(present.scope, scope("system:sol"));
    assert_eq!(present.parent, scope("galaxy:root"));
    assert_eq!(
        domain.resolve(&scope("system:missing")).unwrap(),
        Resolution::QualifiedNoScope
    );
}

#[test]
fn explicit_input_order_cannot_change_domain_identity() {
    let a = ExplicitEntry {
        scope: scope("system:a"),
        parent: scope("galaxy:root"),
    };
    let b = ExplicitEntry {
        scope: scope("system:b"),
        parent: scope("galaxy:root"),
    };
    let first = ExplicitScopeDomain::new(world("world:x"), vec![a.clone(), b.clone()]).unwrap();
    let second = ExplicitScopeDomain::new(world("world:x"), vec![b, a]).unwrap();
    assert_eq!(first.identity, second.identity);
}

#[test]
fn generated_identity_is_repeatable_and_query_order_independent() {
    let domain = generated("world:generated", 100);
    let first_a = authorized(domain.resolve_index(7).unwrap()).unwrap();
    let first_b = authorized(domain.resolve_index(19).unwrap()).unwrap();
    let second_b = authorized(domain.resolve_index(19).unwrap()).unwrap();
    let second_a = authorized(domain.resolve_index(7).unwrap()).unwrap();

    assert_eq!(first_a, second_a);
    assert_eq!(first_b, second_b);
    assert_ne!(first_a.scope, first_b.scope);
}

#[test]
fn generated_out_of_domain_index_is_qualified_absence() {
    let domain = generated("world:generated", 10);
    assert!(domain.resolve_index(9).unwrap().is_authorized());
    assert_eq!(domain.resolve_index(10).unwrap(), Resolution::QualifiedNoScope);
}

trait ResolutionExt {
    fn is_authorized(&self) -> bool;
}

impl ResolutionExt for Resolution {
    fn is_authorized(&self) -> bool {
        matches!(self, Self::Authorized(_))
    }
}

#[test]
fn first_access_does_not_mint_or_rename_generated_scope() {
    let domain = generated("world:first-access", 100);
    let before = authorized(domain.resolve_index(42).unwrap()).unwrap();

    for index in [3_u64, 2, 99, 0, 41, 7] {
        let _ = domain.resolve_index(index).unwrap();
    }

    let after = authorized(domain.resolve_index(42).unwrap()).unwrap();
    assert_eq!(before.scope, after.scope);
    assert_eq!(before.authorization_proof, after.authorization_proof);
}

#[test]
fn different_world_profile_seed_or_parent_changes_generated_scope_identity() {
    let base = generated("world:a", 100);
    let base_scope = authorized(base.resolve_index(5).unwrap()).unwrap().scope;

    let other_world = generated("world:b", 100);
    assert_ne!(
        base_scope,
        authorized(other_world.resolve_index(5).unwrap()).unwrap().scope
    );

    let other_profile = GeneratedScopeDomain::new(
        world("world:a"),
        scope("galaxy:root"),
        digest("scope-domain.profile.v1", b"other-profile"),
        digest("scope-domain.seed.v1", b"seed-42"),
        100,
    )
    .unwrap();
    assert_ne!(
        base_scope,
        authorized(other_profile.resolve_index(5).unwrap()).unwrap().scope
    );

    let other_seed = GeneratedScopeDomain::new(
        world("world:a"),
        scope("galaxy:root"),
        digest("scope-domain.profile.v1", b"bounded-indexed-star-field-v1"),
        digest("scope-domain.seed.v1", b"seed-43"),
        100,
    )
    .unwrap();
    assert_ne!(
        base_scope,
        authorized(other_seed.resolve_index(5).unwrap()).unwrap().scope
    );

    let other_parent = GeneratedScopeDomain::new(
        world("world:a"),
        scope("galaxy:other"),
        digest("scope-domain.profile.v1", b"bounded-indexed-star-field-v1"),
        digest("scope-domain.seed.v1", b"seed-42"),
        100,
    )
    .unwrap();
    assert_ne!(
        base_scope,
        authorized(other_parent.resolve_index(5).unwrap()).unwrap().scope
    );
}

#[test]
fn monotonic_expansion_changes_domain_identity_but_preserves_existing_scope_ids() {
    let original = generated("world:expand", 10);
    let expanded = generated("world:expand", 20);

    assert_ne!(original.identity, expanded.identity);
    assert!(original.expansion_preserves_existing_ids(20).unwrap());
    for index in 0..10 {
        let before = authorized(original.resolve_index(index).unwrap()).unwrap();
        let after = authorized(expanded.resolve_index(index).unwrap()).unwrap();
        assert_eq!(before.scope, after.scope);
    }
    assert!(expanded.resolve_index(15).unwrap().is_authorized());
}

#[test]
fn unavailable_provider_is_incomplete_not_absence() {
    assert_eq!(UnavailableScopeDomain.resolve(), Resolution::Incomplete);
}

#[test]
fn identity_collision_seam_fails_closed() {
    let domain = generated("world:collision", 10);
    let first = authorized(domain.resolve_index(1).unwrap()).unwrap();
    let mut second = authorized(domain.resolve_index(2).unwrap()).unwrap();

    second.scope = first.scope.clone();
    assert_eq!(
        ensure_distinct_semantic_subjects(&[first, second]),
        Err(RefError::IdentityCollision)
    );
}

#[test]
fn generated_existence_receipt_contains_no_content_or_evolved_state() {
    let domain = generated("world:evolved", 10);
    let authorization = authorized(domain.resolve_index(3).unwrap()).unwrap();
    authorization.validate().unwrap();

    assert_eq!(authorization.parent, scope("galaxy:root"));
    assert_eq!(authorization.semantic_key, 3_u64.to_le_bytes().to_vec());
    // Structurally, the authorization contains only world/scope/domain membership
    // evidence. There is intentionally no genesis content, current state digest,
    // residency handle, scheduler entry, or continuation snapshot in this type.
}

#[test]
fn generated_query_work_is_bounded_by_one_semantic_key_not_domain_cardinality() {
    let small = generated("world:bounded", 10);
    let large = generated("world:bounded", MAX_GENERATED_SCOPES);

    let small_scope = authorized(small.resolve_index(7).unwrap()).unwrap().scope;
    let large_scope = authorized(large.resolve_index(7).unwrap()).unwrap().scope;
    assert_eq!(small_scope, large_scope);
}
