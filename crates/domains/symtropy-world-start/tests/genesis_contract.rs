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
    let retry = admit_new_world(&template, &first_request).expect("retry");
    assert_eq!(first, retry);

    let second = admit_new_world(&template, &request(&template, "genesis:run-b"))
        .expect("second lineage");
    assert_ne!(first.lineage, second.lineage);
    assert_eq!(first.template_digest, second.template_digest);
}

#[test]
fn display_metadata_does_not_change_semantic_identity_or_equality() {
    let predecessor = authority("planet", "planet:v1", 2);
    let left = template("Pelagos", vec![predecessor.clone()], vec![], vec![]);
    let right = template("Πέλαγος", vec![predecessor], vec![], vec![]);
    assert_eq!(left.semantic_digest(), right.semantic_digest());
    assert_eq!(left, right);
}

#[test]
fn set_like_inputs_are_canonical_but_duplicates_fail_closed() {
    let a = authority("planet", "planet:v1", 3);
    let b = authority("history", "history:v1", 4);
    let p1 = ProvenanceBindingV1 {
        claim_id: id("claim:atmosphere"),
        class: ProvenanceClass::A,
        source: a.clone(),
    };
    let p2 = ProvenanceBindingV1 {
        claim_id: id("claim:history"),
        class: ProvenanceClass::D,
        source: b.clone(),
    };

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
            p1.clone(),
            ProvenanceBindingV1 {
                claim_id: p1.claim_id,
                class: ProvenanceClass::K,
                source: b,
            },
        ],
        vec![],
    );
    assert!(matches!(
        duplicate_claim,
        Err(WorldStartError::DuplicateProvenanceClaim(_))
    ));
}

#[test]
fn provenance_class_changes_are_semantic_and_not_interchangeable() {
    let source = authority("planet", "planet:v1", 5);
    let authored = template(
        "alien world",
        vec![source.clone()],
        vec![ProvenanceBindingV1 {
            claim_id: id("claim:surface-ocean"),
            class: ProvenanceClass::A,
            source: source.clone(),
        }],
        vec![],
    );
    let known = template(
        "alien world",
        vec![source.clone()],
        vec![ProvenanceBindingV1 {
            claim_id: id("claim:surface-ocean"),
            class: ProvenanceClass::K,
            source,
        }],
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
    assert_ne!(a.semantic_digest(), b.semantic_digest());
    assert_ne!(a.semantic_digest(), c.semantic_digest());
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
