use subterranean_mobility_benchmark::{
    HazardState, Measurement, Passage, PassageDecisionKind, SupportState, TimedEvidence,
    TravelDirection, TraversalMode, TraversalProfile, evaluate_passage,
};

fn measurement(value: i64, uncertainty: u64, evidence_id: &str) -> Measurement {
    Measurement {
        value,
        uncertainty,
        observed_at_ms: 100,
        max_age_ms: 100,
        evidence_id: evidence_id.to_owned(),
    }
}

fn main() {
    let profile = TraversalProfile {
        id: "demo-self-egress-profile".to_owned(),
        mode: TraversalMode::SelfEgress,
        required_width_mm: 500,
        required_height_mm: 700,
        required_turn_radius_mm: 500,
        maximum_ascent_grade_permille: 250,
        maximum_descent_grade_permille: 300,
        maximum_step_up_mm: 100,
        maximum_step_down_mm: 120,
        minimum_traction_permille: 400,
        requires_qualified_support_for_vertical_transition: true,
    };
    let passage = Passage {
        id: "demo-passage".to_owned(),
        from_node: "junction".to_owned(),
        to_node: "refuge".to_owned(),
        bidirectional: true,
        clear_width_mm: Some(measurement(650, 10, "demo-width")),
        clear_height_mm: Some(measurement(900, 10, "demo-height")),
        minimum_turn_radius_mm: Some(measurement(1000, 20, "demo-turn")),
        grade_permille: Some(measurement(100, 10, "demo-grade")),
        maximum_step_up_mm: Some(measurement(50, 5, "demo-step-up")),
        maximum_step_down_mm: Some(measurement(60, 5, "demo-step-down")),
        traction_permille: Some(measurement(600, 20, "demo-traction")),
        hazard_state: Some(TimedEvidence {
            value: HazardState::NoBlockingHazardObserved,
            observed_at_ms: 100,
            max_age_ms: 100,
            evidence_id: "demo-hazard-observation".to_owned(),
        }),
        vertical_transition: false,
        vertical_support: Some(TimedEvidence {
            value: SupportState::NotQualified,
            observed_at_ms: 100,
            max_age_ms: 100,
            evidence_id: "not-required-for-this-flat-passage".to_owned(),
        }),
    };

    match evaluate_passage(&passage, &profile, 150, TravelDirection::Forward) {
        Ok(decision) => {
            println!(
                "passage={} decision={:?}",
                decision.passage_id, decision.decision
            );
            println!("reasons={:?}", decision.reasons);
            if decision.decision != PassageDecisionKind::AdmissibleWithinDeclaredEnvelope {
                std::process::exit(2);
            }
            println!("NOTE: this is synthetic envelope evidence, not a real-world safety claim.");
        }
        Err(error) => {
            eprintln!("invalid benchmark input: {error}");
            std::process::exit(1);
        }
    }
}
