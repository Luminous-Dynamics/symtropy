use symtropy_world_start::{
    admit_new_world, CanonicalDigest, GenesisIntentV1, GenesisRequestV1, ProvenanceBindingV1,
    ProvenanceClass, ScenarioTemplateV1, SemanticId, SemanticParameterV1, TypedAuthorityRefV1,
    WorldStartError,
};

fn id(value: &str) -> SemanticId {
    SemanticId::parse(value).expect("valid test id")
}

fn digest(byte: u8) -> CanonicalDigest {
    CanonicalDigest::from_bytes([byte; 32])
}

fn authority(domain: &str, profile: &str, byte: u8) -> TypedAuthorityRefV1 {
    TypedAuthorityRefV1::new(id(domain), 1, id(profile), digest(byte)).expect("authority")
}

fn parameter(key: &str, value: &[u8]) -> SemanticParameterV1 {
    SemanticParameterV1::new(id(key), value.to_vec()).expect("parameter")
}

fn provenance(
    claim: &str,
    class: ProvenanceClass,
    source: TypedAuthorityRefV1,
) -> ProvenanceBindingV1 {
    ProvenanceBindingV1::new(id(claim), class, source)
}

fn template(
    display: &str,
    predecessors: Vec<TypedAuthorityRefV1>,
    provenance: Vec<ProvenanceBindingV1>,
    parameters: Vec<SemanticParameterV1>,
) -> ScenarioTemplateV1 {
    ScenarioTemplateV1::new(
        id("template:pelagos"),
        id("profile:world-start-v1"),
        Some(display.into()),
        predecessors,
        provenance,
        parameters,
    )
    .expect("template")
}

fn request(template: &ScenarioTemplateV1, request_id: &str) -> GenesisRequestV1 {
    GenesisRequestV1::new(
        id(request_id),
        template.semantic_digest(),
        GenesisIntentV1::NewWorld,
        vec![parameter("seed:terrain", b"41")],
    )
    .expect("request")
}

#[test]
fn exact_retry_is_deterministic_but_distinct_new_request_gets_distinct_lineage() {
    let planet = authority("planet", "planet:v1", 1);
    let template = template("Pelagos", vec![planet], vec![], vec![]);
    let first_request = request(&template, "genesis:run-a");
    let first = admit_new_world(&template, &first_request).expect("first admission");
    first
        .revalidate(&template, &first_request)
        .expect("current receipt revalidates");
    let retry = admit_new_world(&template, &first_request).expect("retry");
    assert_eq!(first, retry);

    let second = admit_new_world(&template, &request(&template, "genesis:run-b"))
        .expect("second lineage");
    assert_ne!(first.lineage(), second.lineage());
    assert_eq!(first.template_digest(), second.template_digest());
}

#[test]
fn display_metadata_does_not_change_semantic_identity_or_equality() {
    let predecessor = authority("planet", "planet:v1", 2);
    let left = template("Pelagos", vec![predecessor.clone()], vec![], vec![]);
    let right = template("Πέλαγος", vec![predecessor], vec![], vec![]);
    assert_ne!(left.display_name(), right.display_name());
    assert_eq!(left.semantic_digest(), right.semantic_digest());
    assert_eq!(left, right);
}

#[test]
fn constructor_sealed_values_expose_read_only_semantic_accessors() {
    let source = authority("planet", "planet:v1", 10);
    assert_eq!(source.domain().as_str(), "planet");
    assert_eq!(source.schema_version(), 1);
    assert_eq!(source.profile().as_str(), "planet:v1");
    assert_eq!(source.digest(), digest(10));

    let binding = provenance("claim:ocean", ProvenanceClass::A, source.clone());
    assert_eq!(binding.claim_id().as_str(), "claim:ocean");
    assert_eq!(binding.class(), ProvenanceClass::A);
    assert_eq!(binding.source(), &source);

    let param = parameter("seed:terrain", b"41");
    assert_eq!(param.key().as_str(), "seed:terrain");
    assert_eq!(param.value(), b"41");

    let template = template("Pelagos", vec![source], vec![binding], vec![param]);
    assert_eq!(template.template_id().as_str(), "template:pelagos");
    assert_eq!(template.genesis_profile().as_str(), "profile:world-start-v1");

    let request = request(&template, "genesis:sealed");
    assert_eq!(request.request_id().as_str(), "genesis:sealed");
    assert_eq!(request.template_digest(), template.semantic_digest());
    assert_eq!(request.intent(), GenesisIntentV1::NewWorld);

    let receipt = admit_new_world(&template, &request).expect("receipt");
    assert_eq!(receipt.schema_version(), 1);
    assert_eq!(receipt.template_digest(), template.semantic_digest());
    assert_eq!(receipt.request_digest(), request.semantic_digest());
    assert_ne!(receipt.candidate_digest(), digest(0));
    assert_ne!(receipt.receipt_digest(), digest(0));
}

#[test]
fn set_like_inputs_are_canonical_but_duplicates_fail_closed() {
    let a = authority("planet", "planet:v1", 3);
    let b = authority("history", "history:v1", 4);
    let p1 = provenance("claim:atmosphere", ProvenanceClass::A, a.clone());
    let p2 = provenance("claim:history", ProvenanceClass::D, b.clone());

    let left = template(
        "x",
        vec![a.clone(), b.clone()],
        vec![p1.clone(), p2.clone()],
        vec![parameter("seed:a", b"1"), parameter("seed:b", b"2")],
    );
    let right = template(
        "y",
        vec![b.clone(), a.clone()],
        vec![p2, p1.clone()],
        vec![parameter("seed:b", b"2"), parameter("seed:a", b"1")],
    );
    assert_eq!(left.semantic_digest(), right.semantic_digest());
    assert_eq!(left, right);

    let duplicate_predecessor = ScenarioTemplateV1::new(
        id("template:dup"),
        id("profile:v1"),
        None,
        vec![a.clone(), a.clone()],
        vec![],
        vec![],
    );
    assert!(matches!(
        duplicate_predecessor,
        Err(WorldStartError::DuplicatePredecessor)
    ));

    let duplicate_claim = ScenarioTemplateV1::new(
        id("template:dup-claim"),
        id("profile:v1"),
        None,
        vec![a],
        vec![
            p1,
            provenance("claim:atmosphere", ProvenanceClass::K, b),
        ],
        vec![],
    );
    assert!(matches!(
        duplicate_claim,
        Err(WorldStartError::DuplicateProvenanceClaim(_))
    ));

    let duplicate_parameter = ScenarioTemplateV1::new(
        id("template:dup-param"),
        id("profile:v1"),
        None,
        vec![],
        vec![],
        vec![parameter("seed:a", b"1"), parameter("seed:a", b"1")],
    );
    assert!(matches!(
        duplicate_parameter,
        Err(WorldStartError::DuplicateParameter(_))
    ));
}

#[test]
fn request_parameter_order_is_canonical_and_duplicate_keys_reject() {
    let template = template("world", vec![], vec![], vec![]);
    let left = GenesisRequestV1::new(
        id("genesis:ordered"),
        template.semantic_digest(),
        GenesisIntentV1::NewWorld,
        vec![parameter("seed:a", b"1"), parameter("seed:b", b"2")],
    )
    .expect("left request");
    let right = GenesisRequestV1::new(
        id("genesis:ordered"),
        template.semantic_digest(),
        GenesisIntentV1::NewWorld,
        vec![parameter("seed:b", b"2"), parameter("seed:a", b"1")],
    )
    .expect("right request");
    assert_eq!(left, right);
    assert_eq!(left.semantic_digest(), right.semantic_digest());
    assert_eq!(
        admit_new_world(&template, &left).expect("left receipt"),
        admit_new_world(&template, &right).expect("right receipt")
    );

    let duplicate = GenesisRequestV1::new(
        id("genesis:duplicate-param"),
        template.semantic_digest(),
        GenesisIntentV1::NewWorld,
        vec![parameter("seed:a", b"1"), parameter("seed:a", b"2")],
    );
    assert!(matches!(
        duplicate,
        Err(WorldStartError::DuplicateParameter(_))
    ));
}

#[test]
fn provenance_class_changes_are_semantic_and_not_interchangeable() {
    let source = authority("planet", "planet:v1", 5);
    let authored = template(
        "alien world",
        vec![source.clone()],
        vec![provenance(
            "claim:surface-ocean",
            ProvenanceClass::A,
            source.clone(),
        )],
        vec![],
    );
    let known = template(
        "alien world",
        vec![source.clone()],
        vec![provenance(
            "claim:surface-ocean",
            ProvenanceClass::K,
            source,
        )],
        vec![],
    );
    assert_ne!(authored.semantic_digest(), known.semantic_digest());
}

#[test]
fn changed_predecessor_stales_old_request_and_receipt() {
    let original = template(
        "world",
        vec![authority("planet", "planet:v1", 6)],
        vec![],
        vec![],
    );
    let req = request(&original, "genesis:stale-test");
    let receipt = admit_new_world(&original, &req).expect("original receipt");

    let changed = template(
        "world",
        vec![authority("planet", "planet:v1", 7)],
        vec![],
        vec![],
    );
    assert!(matches!(
        admit_new_world(&changed, &req),
        Err(WorldStartError::TemplateDigestMismatch { .. })
    ));
    assert!(receipt.revalidate(&changed, &req).is_err());
}

#[test]
fn resume_intent_is_not_accepted_as_new_genesis() {
    let template = template("world", vec![], vec![], vec![]);
    let resume = GenesisRequestV1::new(
        id("resume:save-17"),
        template.semantic_digest(),
        GenesisIntentV1::ResumeExisting,
        vec![],
    )
    .expect("well-formed non-genesis request");
    assert!(matches!(
        admit_new_world(&template, &resume),
        Err(WorldStartError::ResumeIntentRejected)
    ));
}

#[test]
fn schema_profile_and_semantic_parameters_change_identity() {
    let predecessor_v1 = authority("planet", "profile:v1", 8);
    let predecessor_v2 = TypedAuthorityRefV1::new(
        id("planet"),
        2,
        id("profile:v1"),
        digest(8),
    )
    .expect("v2 authority");
    let a = template(
        "world",
        vec![predecessor_v1],
        vec![],
        vec![parameter("world-rule", b"a")],
    );
    let b = template(
        "world",
        vec![predecessor_v2],
        vec![],
        vec![parameter("world-rule", b"a")],
    );
    let c = template(
        "world",
        vec![authority("planet", "profile:v1", 8)],
        vec![],
        vec![parameter("world-rule", b"b")],
    );
    let d = ScenarioTemplateV1::new(
        id("template:pelagos"),
        id("profile:world-start-v2"),
        Some("world".into()),
        vec![authority("planet", "profile:v1", 8)],
        vec![],
        vec![parameter("world-rule", b"a")],
    )
    .expect("different genesis profile");
    assert_ne!(a.semantic_digest(), b.semantic_digest());
    assert_ne!(a.semantic_digest(), c.semantic_digest());
    assert_ne!(a.semantic_digest(), d.semantic_digest());
}

#[test]
fn presentation_and_save_slot_concepts_are_absent_from_genesis_authority() {
    let source = authority("planet", "planet:v1", 9);
    let a = template("localized A", vec![source.clone()], vec![], vec![]);
    let b = template("localized B", vec![source], vec![], vec![]);
    let request_a = request(&a, "genesis:presentation-independent");
    let request_b = request(&b, "genesis:presentation-independent");
    assert_eq!(request_a.semantic_digest(), request_b.semantic_digest());
    assert_eq!(
        admit_new_world(&a, &request_a).expect("a"),
        admit_new_world(&b, &request_b).expect("b")
    );
}

#[test]
fn semantic_parameter_bound_rejects_oversized_values() {
    let oversized = SemanticParameterV1::new(id("oversized"), vec![0_u8; 16 * 1024 + 1]);
    assert!(matches!(
        oversized,
        Err(WorldStartError::ParameterValueTooLarge { .. })
    ));
}

#[test]
fn canonical_digest_chain_matches_frozen_golden_vectors() {
    // These constants were derived independently from the documented V1 byte
    // grammar (big-endian u32 lengths/counts, explicit enum tags, raw 32-byte
    // digests, domain separation) rather than from serde or Debug output.
    let source = authority("planet", "planet:v1", 0x11);
    let template = ScenarioTemplateV1::new(
        id("template:golden"),
        id("profile:world-start-v1"),
        Some("nonsemantic display".into()),
        vec![source.clone()],
        vec![provenance(
            "claim:ocean",
            ProvenanceClass::A,
            source,
        )],
        vec![parameter("world-rule", b"hard")],
    )
    .expect("golden template");
    let request = GenesisRequestV1::new(
        id("genesis:golden"),
        template.semantic_digest(),
        GenesisIntentV1::NewWorld,
        vec![parameter("seed:terrain", b"41")],
    )
    .expect("golden request");
    let receipt = admit_new_world(&template, &request).expect("golden receipt");

    assert_eq!(
        template.semantic_digest().to_string(),
        "327475c42723d991f02b48013019a74b37ef2b8c5ff87bbc81a59bcdffa3709b"
    );
    assert_eq!(
        request.semantic_digest().to_string(),
        "bd4c2db75c810d628636f9783a0357c537604cf97b6f0fb40f536c32474ab8b4"
    );
    assert_eq!(
        receipt.lineage().digest().to_string(),
        "dfde39786de43173d9e61fe6fb7de834e4f8046044ec28505ecded70e1a756ac"
    );
    assert_eq!(
        receipt.candidate_digest().to_string(),
        "334004c983dbb2621938cc971d2d800a45c573c9cda36e3a8a6da96f146d185f"
    );
    assert_eq!(
        receipt.receipt_digest().to_string(),
        "be543284028bbdc56868351433d3d01ac15a05b99bbe17d381bd41699f0cd74e"
    );
}
