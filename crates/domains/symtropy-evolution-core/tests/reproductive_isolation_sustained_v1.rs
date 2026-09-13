include!("reproductive_isolation_evidence_v1.rs");

#[test]
fn barriers_concentrated_in_one_generation_do_not_support_sustained_isolation() {
    let ctx = context(157);
    let auth = authorities(177);

    let opportunities_a = vec![
        opportunity(&auth, "single-a-1", 3, ctx, 140),
        opportunity(&auth, "single-a-2", 3, ctx, 141),
    ];
    let opportunities_b = vec![
        opportunity(&auth, "single-b-1", 3, ctx, 142),
        opportunity(&auth, "single-b-2", 3, ctx, 143),
    ];
    let a = IsolationContactCase {
        design: design_with(&auth, ctx, opportunities_a.clone()),
        opportunities: opportunities_a,
    };
    let b = IsolationContactCase {
        design: design_with(&auth, ctx, opportunities_b.clone()),
        opportunities: opportunities_b,
    };

    let current_a_design = current_contact_design(&a, &auth, ctx);
    let current_b_design = current_contact_design(&b, &auth, ctx);
    let design = declare_isolation_design(&current_a_design, &current_b_design, 36);
    let current_design = current_isolation_design(
        &design,
        &current_a_design,
        &current_b_design,
        36,
    );

    let study_a = capture_contact_study(&a, &auth, ctx, IsolationContactKind::Barrier, 41);
    let study_b = capture_contact_study(&b, &auth, ctx, IsolationContactKind::Barrier, 61);
    let current_a = current_contact_study(
        &a,
        &study_a,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        41,
    );
    let current_b = current_contact_study(
        &b,
        &study_b,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        61,
    );

    let evidence = ReproductiveIsolationEvidence::capture(
        &current_design,
        vec![
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: &current_a,
            },
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                study: &current_b,
            },
        ],
    )
    .unwrap();

    assert_eq!(evidence.barrier_profile.observed_contact_opportunities, 4);
    assert_eq!(evidence.barrier_profile.barrier_supporting_studies, 0);
    assert_eq!(evidence.status, ReproductiveIsolationStatus::NotSupported);
}
