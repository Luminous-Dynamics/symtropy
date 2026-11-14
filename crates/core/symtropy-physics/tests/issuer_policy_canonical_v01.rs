use symtropy_physics::issuer_policy::{
    decode_issuer_authorization_policy_v1, encode_issuer_authorization_policy_v1,
    IssuerAttestationSuiteV1, IssuerAuthorizationEntryDraftV1,
    IssuerAuthorizationPolicyDraftV1, IssuerClaimProfileV1, IssuerCredentialStatusV1,
    IssuerCryptoPolicyProfileV1, IssuerPolicyV1Error, IssuerSessionProfileV1,
    ISSUER_POLICY_FORMAT_VERSION_V1, ISSUER_POLICY_MAGIC_V1, MAX_ISSUER_POLICY_ENTRIES_V1,
};

const FIXTURE: &str = include_str!("fixtures/issuer_policy_v01_golden.json");

fn fixture_canonical_hex() -> String {
    let marker = "\"canonical_hex\": \"";
    let start = FIXTURE.find(marker).expect("fixture canonical_hex") + marker.len();
    let tail = &FIXTURE[start..];
    let end = tail.find('"').expect("fixture canonical_hex terminator");
    tail[..end].to_string()
}

fn decode_hex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("valid fixture hex"))
        .collect()
}

fn draft() -> IssuerAuthorizationPolicyDraftV1 {
    IssuerAuthorizationPolicyDraftV1 {
        policy_revision: 7,
        entries: vec![
            IssuerAuthorizationEntryDraftV1 {
                credential_id: [0x22; 32],
                status: IssuerCredentialStatusV1::Disabled,
                physical_authority_ids: vec![0x30],
                claim_profiles: vec![IssuerClaimProfileV1::EndpointSample],
                session_profiles: vec![IssuerSessionProfileV1::OsCsprng],
                attestation_suites: vec![IssuerAttestationSuiteV1::HybridEd25519MlDsa65],
                crypto_policy_profiles: vec![IssuerCryptoPolicyProfileV1::HybridPqcAuth],
            },
            IssuerAuthorizationEntryDraftV1 {
                credential_id: [0x11; 32],
                status: IssuerCredentialStatusV1::Active,
                physical_authority_ids: vec![0x20, 0x10],
                claim_profiles: vec![
                    IssuerClaimProfileV1::ConsecutivePresence,
                    IssuerClaimProfileV1::EndpointSample,
                ],
                session_profiles: vec![IssuerSessionProfileV1::OsCsprng],
                attestation_suites: vec![IssuerAttestationSuiteV1::HybridEd25519MlDsa65],
                crypto_policy_profiles: vec![IssuerCryptoPolicyProfileV1::HybridPqcAuth],
            },
        ],
    }
}

#[test]
fn unordered_draft_matches_the_237_byte_language_neutral_fixture() {
    let encoded = encode_issuer_authorization_policy_v1(&draft()).expect("canonicalize fixture");
    let expected = decode_hex(&fixture_canonical_hex());
    assert_eq!(expected.len(), 237);
    assert_eq!(encoded, expected);

    let decoded = decode_issuer_authorization_policy_v1(&encoded).expect("decode canonical fixture");
    assert_eq!(decoded.policy_revision(), 7);
    assert_eq!(decoded.entries().len(), 2);
    assert_eq!(decoded.entries()[0].credential_id(), &[0x11; 32]);
    assert_eq!(decoded.entries()[0].status(), IssuerCredentialStatusV1::Active);
    assert_eq!(decoded.entries()[0].physical_authority_ids(), &[0x10, 0x20]);
    assert_eq!(
        decoded.entries()[0].claim_profiles(),
        &[
            IssuerClaimProfileV1::EndpointSample,
            IssuerClaimProfileV1::ConsecutivePresence,
        ]
    );
    assert_eq!(decoded.to_canonical_bytes(), encoded);
}

#[test]
fn duplicate_credential_and_scope_semantics_fail_closed() {
    let mut duplicate_credential = draft();
    duplicate_credential.entries[1].credential_id = [0x22; 32];
    assert_eq!(
        encode_issuer_authorization_policy_v1(&duplicate_credential),
        Err(IssuerPolicyV1Error::DuplicateCredentialId)
    );

    let mut duplicate_authority = draft();
    duplicate_authority.entries[1].physical_authority_ids = vec![0x10, 0x10];
    assert_eq!(
        encode_issuer_authorization_policy_v1(&duplicate_authority),
        Err(IssuerPolicyV1Error::DuplicatePhysicalAuthorityId)
    );

    let mut duplicate_claim = draft();
    duplicate_claim.entries[1].claim_profiles = vec![
        IssuerClaimProfileV1::EndpointSample,
        IssuerClaimProfileV1::EndpointSample,
    ];
    assert_eq!(
        encode_issuer_authorization_policy_v1(&duplicate_claim),
        Err(IssuerPolicyV1Error::DuplicateClaimProfile)
    );
}

#[test]
fn zero_authority_and_empty_scope_fail_closed() {
    let mut zero_authority = draft();
    zero_authority.entries[0].physical_authority_ids = vec![0];
    assert_eq!(
        encode_issuer_authorization_policy_v1(&zero_authority),
        Err(IssuerPolicyV1Error::ZeroPhysicalAuthorityId)
    );

    let mut empty_claims = draft();
    empty_claims.entries[0].claim_profiles.clear();
    assert_eq!(
        encode_issuer_authorization_policy_v1(&empty_claims),
        Err(IssuerPolicyV1Error::EmptyClaimProfileScope)
    );
}

#[test]
fn wire_decoder_rejects_unknown_status_unsorted_credentials_and_trailing_bytes() {
    let canonical = decode_hex(&fixture_canonical_hex());
    let header_len = ISSUER_POLICY_MAGIC_V1.len() + 4 + 8 + 4;

    let mut unknown_status = canonical.clone();
    unknown_status[header_len + 32] = 0xff;
    assert_eq!(
        decode_issuer_authorization_policy_v1(&unknown_status),
        Err(IssuerPolicyV1Error::UnknownStatus(0xff))
    );

    let first_credential = header_len;
    let second_credential = canonical
        .windows(32)
        .position(|window| window.iter().all(|byte| *byte == 0x22))
        .expect("second credential run");
    let mut unsorted = canonical.clone();
    for i in 0..32 {
        unsorted.swap(first_credential + i, second_credential + i);
    }
    assert_eq!(
        decode_issuer_authorization_policy_v1(&unsorted),
        Err(IssuerPolicyV1Error::UnsortedCredentialIds)
    );

    let mut trailing = canonical;
    trailing.push(0);
    assert_eq!(
        decode_issuer_authorization_policy_v1(&trailing),
        Err(IssuerPolicyV1Error::TrailingBytes)
    );
}

#[test]
fn over_bound_entry_count_rejects_before_entry_allocation_or_body_parse() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(ISSUER_POLICY_MAGIC_V1);
    bytes.extend_from_slice(&ISSUER_POLICY_FORMAT_VERSION_V1.to_be_bytes());
    bytes.extend_from_slice(&0u64.to_be_bytes());
    bytes.extend_from_slice(&((MAX_ISSUER_POLICY_ENTRIES_V1 + 1) as u32).to_be_bytes());
    assert_eq!(
        decode_issuer_authorization_policy_v1(&bytes),
        Err(IssuerPolicyV1Error::TooManyEntries(
            MAX_ISSUER_POLICY_ENTRIES_V1 + 1
        ))
    );
}

#[test]
fn truncation_fails_at_every_prefix_of_the_golden_policy() {
    let canonical = decode_hex(&fixture_canonical_hex());
    for end in 0..canonical.len() {
        assert!(decode_issuer_authorization_policy_v1(&canonical[..end]).is_err());
    }
    assert!(decode_issuer_authorization_policy_v1(&canonical).is_ok());
}
