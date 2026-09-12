use symtropy_analysis_contracts::{
    AnalysisEvidence, AnalysisEvidenceId, AnalysisObservation, AnalysisObservationId,
    AnalysisProfileId, AnalysisProfileRef, AnalysisRequest, AnalysisRequestId, AnalysisValue,
    ExactSemanticRef, NumericalDisposition, ObservableRequest, ObservationClass, RunDisposition,
    SolverIdentity,
};
use symtropy_analysis_qualification::{
    AnalysisQualificationCut, FacetAcceptance, FacetAssessment, FacetDisposition, FacetRule,
    QualificationCutId, QualificationProfile, QualificationProfileId,
};
use symtropy_design::{ContentDigest, DesignId, DesignRevisionRef};
use symtropy_game_state::StableId;

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn digest(value: &str) -> ContentDigest {
    ContentDigest::new(id("blake3"), value).unwrap()
}

fn exact(authority: &str, subject: &str, content: &str) -> ExactSemanticRef {
    ExactSemanticRef::new(id(authority), id(subject), 1, digest(content)).unwrap()
}

fn request() -> AnalysisRequest {
    let subject = DesignRevisionRef::new(
        DesignId::new(id("design:bracket")),
        1,
        digest("design-a"),
    )
    .unwrap();
    let profile = AnalysisProfileRef::new(
        AnalysisProfileId::new(id("analysis-profile:static")),
        1,
        digest("analysis-profile"),
    )
    .unwrap();
    AnalysisRequest::new(
        AnalysisRequestId::new(id("analysis-request:bracket")),
        subject,
        profile,
        Vec::new(),
        vec![ObservableRequest::new(
            id("observable:deflection"),
            id("dimension:length"),
            Some(id("unit:um")),
        )
        .unwrap()],
        Vec::new(),
        Vec::new(),
    )
    .unwrap()
}

fn evidence(request: &AnalysisRequest) -> AnalysisEvidence {
    let observations = vec![AnalysisObservation::new(
        AnalysisObservationId::new(id("observation:deflection")),
        ObservationClass::RequestedResult,
        id("observable:deflection"),
        id("dimension:length"),
        Some(id("unit:um")),
        AnalysisValue::Measurement {
            lower: 390,
            upper: 430,
            resolution: 10,
        },
    )
    .unwrap()];
    AnalysisEvidence::new(
        AnalysisEvidenceId::new(id("analysis-evidence:bracket")),
        request,
        SolverIdentity::new(
            id("provider:solver-lab"),
            id("solver:fem"),
            "1.0.0",
            digest("solver-a"),
        )
        .unwrap(),
        exact("authority:nix", "environment:solver", "environment"),
        exact("authority:model", "model:elastic", "model"),
        exact(
            "authority:applicability",
            "domain:bracket",
            "applicability",
        ),
        RunDisposition::Completed,
        NumericalDisposition::Converged,
        Vec::new(),
        observations,
    )
    .unwrap()
}

fn profile() -> QualificationProfile {
    QualificationProfile::new(
        QualificationProfileId::new(id("qualification-profile:prototype")),
        1,
        vec![
            FacetRule::new(
                id("facet:model-applicability"),
                id("authority:model-qualification"),
                FacetAcceptance::EstablishedOnly,
            )
            .unwrap(),
            FacetRule::new(
                id("facet:numerical"),
                id("authority:numerical-qualification"),
                FacetAcceptance::EstablishedOnly,
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn assessment(
    request: &AnalysisRequest,
    evidence: &AnalysisEvidence,
    facet: &str,
    authority: &str,
) -> FacetAssessment {
    FacetAssessment::new(
        id(facet),
        evidence.exact_ref(request).unwrap(),
        exact(authority, &format!("attestation:{facet}"), facet),
        FacetDisposition::Established,
        Vec::new(),
    )
    .unwrap()
}

fn main() {
    let request = request();
    let evidence = evidence(&request);
    let profile = profile();
    let cut = AnalysisQualificationCut::new(
        QualificationCutId::new(id("qualification-cut:bracket")),
        &request,
        &evidence,
        &profile,
        exact("authority:clock", "evaluation-context:1", "context"),
        vec![
            assessment(
                &request,
                &evidence,
                "facet:numerical",
                "authority:numerical-qualification",
            ),
            assessment(
                &request,
                &evidence,
                "facet:model-applicability",
                "authority:model-qualification",
            ),
        ],
    )
    .unwrap();
    println!(
        "A1_PROFILE_GOLDEN={}",
        profile.content_digest().unwrap().value
    );
    println!(
        "A1_CUT_GOLDEN={}",
        cut.content_digest(&request, &evidence, &profile)
            .unwrap()
            .value
    );
}
