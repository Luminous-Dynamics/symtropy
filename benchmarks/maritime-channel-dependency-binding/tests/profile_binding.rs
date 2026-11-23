use std::collections::BTreeMap;

use maritime_channel_dependency_binding::{
    assess, ClosureProjection, Digest, MaritimeProfile, ProfileDisposition,
    ProjectionCurrentness, ValidationError, CLOSURE_SCHEMA,
};

fn projection_for(profile: &MaritimeProfile) -> ClosureProjection {
    ClosureProjection {
        schema: CLOSURE_SCHEMA.to_owned(),
        graph_schema_version: "lex-net-030-v1".to_owned(),
        source_registry_version: "registry-test".to_owned(),
        snapshot_id: "snapshot-test".to_owned(),
        claim_id: profile.claim_id.clone(),
        subject_digest: profile.subject_digest,
        profile_digest: profile.profile_digest,
        graph_digest: Digest::from_byte(3),
        closure_digest: Digest::from_byte(4),
        as_of_epoch_seconds: profile.as_of_epoch_seconds,
        currentness: ProjectionCurrentness::Current,
        roles: BTreeMap::new(),
    }
}

#[test]
fn release_library_rejects_unreviewed_profile_ids_and_versions() {
    let subject = Digest::from_byte(1);
    let profile_digest = Digest::from_byte(2);

    for (profile_id, profile_version) in [
        ("unreviewed-maritime-profile", "0.1.0"),
        ("maritime-two-channel-v0.1", "0.1.1"),
    ] {
        let mut profile =
            MaritimeProfile::v0_1("claim-release-binding-test", subject, profile_digest, 500);
        profile.profile_id = profile_id.to_owned();
        profile.profile_version = profile_version.to_owned();

        let result = assess(&profile, &projection_for(&profile));
        assert_eq!(result.disposition, ProfileDisposition::ProfileInvalid);
        assert!(
            result
                .validation_errors
                .contains(&ValidationError::UnsupportedProfileBinding),
            "unexpected profile binding was accepted: {profile_id} / {profile_version}"
        );
    }
}

#[test]
fn public_v0_1_constructor_exposes_the_full_frozen_eighteen_obligation_set() {
    let profile = MaritimeProfile::v0_1(
        "claim-frozen-profile-inventory",
        Digest::from_byte(1),
        Digest::from_byte(2),
        500,
    );
    assert_eq!(profile.profile_id, "maritime-two-channel-v0.1");
    assert_eq!(profile.profile_version, "0.1.0");
    assert_eq!(profile.obligations.len(), 18);
}
