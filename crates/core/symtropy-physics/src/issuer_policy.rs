// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Canonical, untrusted issuer-authorization policy bytes.
//!
//! This module establishes representation only. Decoding a valid policy does
//! not make it trusted and does not authorize any evidence.

use core::fmt;

pub const ISSUER_POLICY_MAGIC_V1: &[u8] = b"symtropy-physics-issuer-policy\0";
pub const ISSUER_POLICY_FORMAT_VERSION_V1: u32 = 1;
pub const MAX_ISSUER_POLICY_ENTRIES_V1: usize = 1024;
pub const MAX_PHYSICAL_AUTHORITIES_PER_ENTRY_V1: usize = 64;
pub const MAX_PROFILE_TAGS_PER_ENTRY_V1: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssuerCredentialStatusV1 {
    Active,
    Disabled,
    RevokedCompromised,
}
impl IssuerCredentialStatusV1 {
    const fn tag(self) -> u8 {
        match self { Self::Active => 1, Self::Disabled => 2, Self::RevokedCompromised => 3 }
    }
    fn from_tag(tag: u8) -> Option<Self> {
        match tag { 1 => Some(Self::Active), 2 => Some(Self::Disabled), 3 => Some(Self::RevokedCompromised), _ => None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssuerClaimProfileV1 { EndpointSample, ConsecutivePresence }
impl IssuerClaimProfileV1 {
    const fn tag(self) -> u32 { match self { Self::EndpointSample => 1, Self::ConsecutivePresence => 2 } }
    fn from_tag(tag: u32) -> Option<Self> { match tag { 1 => Some(Self::EndpointSample), 2 => Some(Self::ConsecutivePresence), _ => None } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssuerSessionProfileV1 { OsCsprng }
impl IssuerSessionProfileV1 {
    const fn tag(self) -> u32 { 1 }
    fn from_tag(tag: u32) -> Option<Self> { (tag == 1).then_some(Self::OsCsprng) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssuerAttestationSuiteV1 { HybridEd25519MlDsa65 }
impl IssuerAttestationSuiteV1 {
    const fn tag(self) -> u32 { 1 }
    fn from_tag(tag: u32) -> Option<Self> { (tag == 1).then_some(Self::HybridEd25519MlDsa65) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssuerCryptoPolicyProfileV1 { HybridPqcAuth }
impl IssuerCryptoPolicyProfileV1 {
    const fn tag(self) -> u32 { 1 }
    fn from_tag(tag: u32) -> Option<Self> { (tag == 1).then_some(Self::HybridPqcAuth) }
}

/// Plain untrusted source entry. Source order has no canonical meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuerAuthorizationEntryDraftV1 {
    pub credential_id: [u8; 32],
    pub status: IssuerCredentialStatusV1,
    pub physical_authority_ids: Vec<u128>,
    pub claim_profiles: Vec<IssuerClaimProfileV1>,
    pub session_profiles: Vec<IssuerSessionProfileV1>,
    pub attestation_suites: Vec<IssuerAttestationSuiteV1>,
    pub crypto_policy_profiles: Vec<IssuerCryptoPolicyProfileV1>,
}

/// Plain untrusted source policy. `policy_revision` is not trusted time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuerAuthorizationPolicyDraftV1 {
    pub policy_revision: u64,
    pub entries: Vec<IssuerAuthorizationEntryDraftV1>,
}

/// An admitted canonical entry. Fields are private so this type cannot be
/// fabricated as a shortcut around canonical decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedIssuerAuthorizationEntryV1 {
    credential_id: [u8; 32],
    status: IssuerCredentialStatusV1,
    physical_authority_ids: Vec<u128>,
    claim_profiles: Vec<IssuerClaimProfileV1>,
    session_profiles: Vec<IssuerSessionProfileV1>,
    attestation_suites: Vec<IssuerAttestationSuiteV1>,
    crypto_policy_profiles: Vec<IssuerCryptoPolicyProfileV1>,
}
impl DecodedIssuerAuthorizationEntryV1 {
    pub const fn credential_id(&self) -> &[u8; 32] { &self.credential_id }
    pub const fn status(&self) -> IssuerCredentialStatusV1 { self.status }
    pub fn physical_authority_ids(&self) -> &[u128] { &self.physical_authority_ids }
    pub fn claim_profiles(&self) -> &[IssuerClaimProfileV1] { &self.claim_profiles }
    pub fn session_profiles(&self) -> &[IssuerSessionProfileV1] { &self.session_profiles }
    pub fn attestation_suites(&self) -> &[IssuerAttestationSuiteV1] { &self.attestation_suites }
    pub fn crypto_policy_profiles(&self) -> &[IssuerCryptoPolicyProfileV1] { &self.crypto_policy_profiles }
}

/// An admitted canonical policy snapshot. This is still an **untrusted** type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedIssuerAuthorizationPolicyV1 {
    policy_revision: u64,
    entries: Vec<DecodedIssuerAuthorizationEntryV1>,
}
impl DecodedIssuerAuthorizationPolicyV1 {
    pub const fn policy_revision(&self) -> u64 { self.policy_revision }
    pub fn entries(&self) -> &[DecodedIssuerAuthorizationEntryV1] { &self.entries }
    pub fn to_canonical_bytes(&self) -> Vec<u8> { encode_decoded_policy(self.policy_revision, &self.entries) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssuerPolicyV1Error {
    BadMagic,
    UnsupportedVersion(u32),
    Truncated,
    TrailingBytes,
    TooManyEntries(usize),
    DuplicateCredentialId,
    UnsortedCredentialIds,
    UnknownStatus(u8),
    EmptyPhysicalAuthorityScope,
    TooManyPhysicalAuthorities(usize),
    ZeroPhysicalAuthorityId,
    DuplicatePhysicalAuthorityId,
    UnsortedPhysicalAuthorityIds,
    EmptyClaimProfileScope,
    TooManyClaimProfiles(usize),
    UnknownClaimProfile(u32),
    DuplicateClaimProfile,
    UnsortedClaimProfiles,
    EmptySessionProfileScope,
    TooManySessionProfiles(usize),
    UnknownSessionProfile(u32),
    DuplicateSessionProfile,
    UnsortedSessionProfiles,
    EmptyAttestationSuiteScope,
    TooManyAttestationSuites(usize),
    UnknownAttestationSuite(u32),
    DuplicateAttestationSuite,
    UnsortedAttestationSuites,
    EmptyCryptoPolicyScope,
    TooManyCryptoPolicies(usize),
    UnknownCryptoPolicy(u32),
    DuplicateCryptoPolicy,
    UnsortedCryptoPolicies,
}
impl fmt::Display for IssuerPolicyV1Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for IssuerPolicyV1Error {}

/// Canonicalize an unordered plain policy draft into exact v1 bytes.
pub fn encode_issuer_authorization_policy_v1(
    draft: &IssuerAuthorizationPolicyDraftV1,
) -> Result<Vec<u8>, IssuerPolicyV1Error> {
    if draft.entries.len() > MAX_ISSUER_POLICY_ENTRIES_V1 {
        return Err(IssuerPolicyV1Error::TooManyEntries(draft.entries.len()));
    }
    let mut entries = Vec::with_capacity(draft.entries.len());
    for entry in &draft.entries { entries.push(canonicalize_entry(entry)?); }
    entries.sort_unstable_by_key(|entry| entry.credential_id);
    if entries.windows(2).any(|p| p[0].credential_id == p[1].credential_id) {
        return Err(IssuerPolicyV1Error::DuplicateCredentialId);
    }
    Ok(encode_decoded_policy(draft.policy_revision, &entries))
}

/// Decode only already-canonical v1 bytes into a plain untrusted record.
pub fn decode_issuer_authorization_policy_v1(
    bytes: &[u8],
) -> Result<DecodedIssuerAuthorizationPolicyV1, IssuerPolicyV1Error> {
    if bytes.len() < ISSUER_POLICY_MAGIC_V1.len() { return Err(IssuerPolicyV1Error::Truncated); }
    if &bytes[..ISSUER_POLICY_MAGIC_V1.len()] != ISSUER_POLICY_MAGIC_V1 { return Err(IssuerPolicyV1Error::BadMagic); }
    let mut c = Cursor::new(bytes, ISSUER_POLICY_MAGIC_V1.len());
    let version = c.read_u32()?;
    if version != ISSUER_POLICY_FORMAT_VERSION_V1 { return Err(IssuerPolicyV1Error::UnsupportedVersion(version)); }
    let policy_revision = c.read_u64()?;
    let entry_count = c.read_u32()? as usize;
    if entry_count > MAX_ISSUER_POLICY_ENTRIES_V1 { return Err(IssuerPolicyV1Error::TooManyEntries(entry_count)); }

    let mut entries = Vec::with_capacity(entry_count);
    let mut previous_credential: Option<[u8; 32]> = None;
    for _ in 0..entry_count {
        let credential_id = c.read_array::<32>()?;
        if let Some(previous) = previous_credential {
            if credential_id == previous { return Err(IssuerPolicyV1Error::DuplicateCredentialId); }
            if credential_id < previous { return Err(IssuerPolicyV1Error::UnsortedCredentialIds); }
        }
        previous_credential = Some(credential_id);

        let status_tag = c.read_u8()?;
        let status = IssuerCredentialStatusV1::from_tag(status_tag).ok_or(IssuerPolicyV1Error::UnknownStatus(status_tag))?;

        let authority_count = c.read_u32()? as usize;
        if authority_count == 0 { return Err(IssuerPolicyV1Error::EmptyPhysicalAuthorityScope); }
        if authority_count > MAX_PHYSICAL_AUTHORITIES_PER_ENTRY_V1 { return Err(IssuerPolicyV1Error::TooManyPhysicalAuthorities(authority_count)); }
        let mut physical_authority_ids = Vec::with_capacity(authority_count);
        let mut previous_authority = None;
        for _ in 0..authority_count {
            let authority = c.read_u128()?;
            if authority == 0 { return Err(IssuerPolicyV1Error::ZeroPhysicalAuthorityId); }
            if let Some(previous) = previous_authority {
                if authority == previous { return Err(IssuerPolicyV1Error::DuplicatePhysicalAuthorityId); }
                if authority < previous { return Err(IssuerPolicyV1Error::UnsortedPhysicalAuthorityIds); }
            }
            previous_authority = Some(authority);
            physical_authority_ids.push(authority);
        }

        entries.push(DecodedIssuerAuthorizationEntryV1 {
            credential_id,
            status,
            physical_authority_ids,
            claim_profiles: decode_tag_scope(&mut c, ScopeKind::Claim, IssuerClaimProfileV1::from_tag)?,
            session_profiles: decode_tag_scope(&mut c, ScopeKind::Session, IssuerSessionProfileV1::from_tag)?,
            attestation_suites: decode_tag_scope(&mut c, ScopeKind::Attestation, IssuerAttestationSuiteV1::from_tag)?,
            crypto_policy_profiles: decode_tag_scope(&mut c, ScopeKind::CryptoPolicy, IssuerCryptoPolicyProfileV1::from_tag)?,
        });
    }
    if c.remaining() != 0 { return Err(IssuerPolicyV1Error::TrailingBytes); }
    Ok(DecodedIssuerAuthorizationPolicyV1 { policy_revision, entries })
}

fn canonicalize_entry(entry: &IssuerAuthorizationEntryDraftV1) -> Result<DecodedIssuerAuthorizationEntryV1, IssuerPolicyV1Error> {
    let mut authorities = entry.physical_authority_ids.clone();
    if authorities.is_empty() { return Err(IssuerPolicyV1Error::EmptyPhysicalAuthorityScope); }
    if authorities.len() > MAX_PHYSICAL_AUTHORITIES_PER_ENTRY_V1 { return Err(IssuerPolicyV1Error::TooManyPhysicalAuthorities(authorities.len())); }
    if authorities.contains(&0) { return Err(IssuerPolicyV1Error::ZeroPhysicalAuthorityId); }
    authorities.sort_unstable();
    if has_duplicate(&authorities) { return Err(IssuerPolicyV1Error::DuplicatePhysicalAuthorityId); }
    Ok(DecodedIssuerAuthorizationEntryV1 {
        credential_id: entry.credential_id,
        status: entry.status,
        physical_authority_ids: authorities,
        claim_profiles: canonicalize_tag_scope(&entry.claim_profiles, ScopeKind::Claim)?,
        session_profiles: canonicalize_tag_scope(&entry.session_profiles, ScopeKind::Session)?,
        attestation_suites: canonicalize_tag_scope(&entry.attestation_suites, ScopeKind::Attestation)?,
        crypto_policy_profiles: canonicalize_tag_scope(&entry.crypto_policy_profiles, ScopeKind::CryptoPolicy)?,
    })
}

fn encode_decoded_policy(revision: u64, entries: &[DecodedIssuerAuthorizationEntryV1]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(ISSUER_POLICY_MAGIC_V1);
    out.extend_from_slice(&ISSUER_POLICY_FORMAT_VERSION_V1.to_be_bytes());
    out.extend_from_slice(&revision.to_be_bytes());
    out.extend_from_slice(&(entries.len() as u32).to_be_bytes());
    for e in entries {
        out.extend_from_slice(&e.credential_id);
        out.push(e.status.tag());
        out.extend_from_slice(&(e.physical_authority_ids.len() as u32).to_be_bytes());
        for authority in &e.physical_authority_ids { out.extend_from_slice(&authority.to_be_bytes()); }
        encode_tag_scope(&mut out, &e.claim_profiles, IssuerClaimProfileV1::tag);
        encode_tag_scope(&mut out, &e.session_profiles, IssuerSessionProfileV1::tag);
        encode_tag_scope(&mut out, &e.attestation_suites, IssuerAttestationSuiteV1::tag);
        encode_tag_scope(&mut out, &e.crypto_policy_profiles, IssuerCryptoPolicyProfileV1::tag);
    }
    out
}

fn encode_tag_scope<T: Copy>(out: &mut Vec<u8>, values: &[T], tag: fn(T) -> u32) {
    out.extend_from_slice(&(values.len() as u32).to_be_bytes());
    for value in values { out.extend_from_slice(&tag(*value).to_be_bytes()); }
}

fn canonicalize_tag_scope<T: Copy + Ord>(values: &[T], kind: ScopeKind) -> Result<Vec<T>, IssuerPolicyV1Error> {
    if values.is_empty() { return Err(kind.empty_error()); }
    if values.len() > MAX_PROFILE_TAGS_PER_ENTRY_V1 { return Err(kind.too_many_error(values.len())); }
    let mut out = values.to_vec();
    out.sort_unstable();
    if has_duplicate(&out) { return Err(kind.duplicate_error()); }
    Ok(out)
}

fn decode_tag_scope<T: Copy>(c: &mut Cursor<'_>, kind: ScopeKind, from_tag: fn(u32) -> Option<T>) -> Result<Vec<T>, IssuerPolicyV1Error> {
    let count = c.read_u32()? as usize;
    if count == 0 { return Err(kind.empty_error()); }
    if count > MAX_PROFILE_TAGS_PER_ENTRY_V1 { return Err(kind.too_many_error(count)); }
    let mut out = Vec::with_capacity(count);
    let mut previous_tag = None;
    for _ in 0..count {
        let tag = c.read_u32()?;
        let value = from_tag(tag).ok_or_else(|| kind.unknown_error(tag))?;
        if let Some(previous) = previous_tag {
            if tag == previous { return Err(kind.duplicate_error()); }
            if tag < previous { return Err(kind.unsorted_error()); }
        }
        previous_tag = Some(tag);
        out.push(value);
    }
    Ok(out)
}

fn has_duplicate<T: PartialEq>(values: &[T]) -> bool { values.windows(2).any(|p| p[0] == p[1]) }

#[derive(Debug, Clone, Copy)]
enum ScopeKind { Claim, Session, Attestation, CryptoPolicy }
impl ScopeKind {
    const fn empty_error(self) -> IssuerPolicyV1Error { match self { Self::Claim => IssuerPolicyV1Error::EmptyClaimProfileScope, Self::Session => IssuerPolicyV1Error::EmptySessionProfileScope, Self::Attestation => IssuerPolicyV1Error::EmptyAttestationSuiteScope, Self::CryptoPolicy => IssuerPolicyV1Error::EmptyCryptoPolicyScope } }
    const fn too_many_error(self, n: usize) -> IssuerPolicyV1Error { match self { Self::Claim => IssuerPolicyV1Error::TooManyClaimProfiles(n), Self::Session => IssuerPolicyV1Error::TooManySessionProfiles(n), Self::Attestation => IssuerPolicyV1Error::TooManyAttestationSuites(n), Self::CryptoPolicy => IssuerPolicyV1Error::TooManyCryptoPolicies(n) } }
    const fn duplicate_error(self) -> IssuerPolicyV1Error { match self { Self::Claim => IssuerPolicyV1Error::DuplicateClaimProfile, Self::Session => IssuerPolicyV1Error::DuplicateSessionProfile, Self::Attestation => IssuerPolicyV1Error::DuplicateAttestationSuite, Self::CryptoPolicy => IssuerPolicyV1Error::DuplicateCryptoPolicy } }
    const fn unsorted_error(self) -> IssuerPolicyV1Error { match self { Self::Claim => IssuerPolicyV1Error::UnsortedClaimProfiles, Self::Session => IssuerPolicyV1Error::UnsortedSessionProfiles, Self::Attestation => IssuerPolicyV1Error::UnsortedAttestationSuites, Self::CryptoPolicy => IssuerPolicyV1Error::UnsortedCryptoPolicies } }
    const fn unknown_error(self, tag: u32) -> IssuerPolicyV1Error { match self { Self::Claim => IssuerPolicyV1Error::UnknownClaimProfile(tag), Self::Session => IssuerPolicyV1Error::UnknownSessionProfile(tag), Self::Attestation => IssuerPolicyV1Error::UnknownAttestationSuite(tag), Self::CryptoPolicy => IssuerPolicyV1Error::UnknownCryptoPolicy(tag) } }
}

struct Cursor<'a> { bytes: &'a [u8], offset: usize }
impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8], offset: usize) -> Self { Self { bytes, offset } }
    fn remaining(&self) -> usize { self.bytes.len().saturating_sub(self.offset) }
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], IssuerPolicyV1Error> {
        let end = self.offset.checked_add(N).ok_or(IssuerPolicyV1Error::Truncated)?;
        let slice = self.bytes.get(self.offset..end).ok_or(IssuerPolicyV1Error::Truncated)?;
        let mut out = [0u8; N]; out.copy_from_slice(slice); self.offset = end; Ok(out)
    }
    fn read_u8(&mut self) -> Result<u8, IssuerPolicyV1Error> { Ok(self.read_array::<1>()?[0]) }
    fn read_u32(&mut self) -> Result<u32, IssuerPolicyV1Error> { Ok(u32::from_be_bytes(self.read_array::<4>()?)) }
    fn read_u64(&mut self) -> Result<u64, IssuerPolicyV1Error> { Ok(u64::from_be_bytes(self.read_array::<8>()?)) }
    fn read_u128(&mut self) -> Result<u128, IssuerPolicyV1Error> { Ok(u128::from_be_bytes(self.read_array::<16>()?)) }
}
