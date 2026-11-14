// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic passage-admissibility filtering for subterranean mobility.
//!
//! This crate checks a declared geometric/evidence envelope. It is not a
//! physical safety model, rescue certification, geotechnical assessment, or
//! permission to enter a real cave, mine, tunnel, or confined space.

pub mod egress;
pub mod trace;

use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraversalMode {
    PersonOnly,
    SelfEgress,
    AssistedWalking,
    CasualtyOnLitter,
    AidEmpty,
    AidLoaded,
    RobotScout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassageDecisionKind {
    /// All declared hard constraints were satisfied by fresh supplied evidence.
    /// This is not a claim that the passage is safe in the physical world.
    AdmissibleWithinDeclaredEnvelope,
    /// Fresh evidence proves at least one declared hard constraint is violated.
    RejectOutsideEnvelope,
    /// Required evidence is missing or stale; admissibility is not established.
    UnknownInsufficientOrStaleEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ReasonCode {
    MissingClearWidth,
    StaleClearWidth,
    ClearWidthInsufficient,
    MissingClearHeight,
    StaleClearHeight,
    ClearHeightInsufficient,
    MissingTurnRadius,
    StaleTurnRadius,
    TurnRadiusInsufficient,
    MissingGrade,
    StaleGrade,
    AscentTooSteep,
    DescentTooSteep,
    MissingStepUp,
    StaleStepUp,
    StepUpTooHigh,
    MissingStepDown,
    StaleStepDown,
    StepDownTooHigh,
    MissingTraction,
    StaleTraction,
    TractionInsufficient,
    MissingHazardState,
    StaleHazardState,
    BlockingHazard,
    MissingQualifiedSupport,
    StaleQualifiedSupport,
    SupportNotQualified,
}

/// Scalar measurement plus a symmetric uncertainty bound. Units are
/// field-specific and explicit in the consuming field name. Numeric values are
/// integers to avoid NaN/rounding ambiguity in this reference kernel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Measurement {
    pub value: i64,
    pub uncertainty: u64,
    pub observed_at_ms: u64,
    pub max_age_ms: u64,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HazardState {
    NoBlockingHazardObserved,
    BlockingHazard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportState {
    QualifiedByResponsibleAuthority,
    NotQualified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimedEvidence<T> {
    pub value: T,
    pub observed_at_ms: u64,
    pub max_age_ms: u64,
    pub evidence_id: String,
}

/// One directed passage segment. Reverse travel is permitted only if the
/// segment explicitly declares bidirectional traversal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Passage {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub bidirectional: bool,
    pub clear_width_mm: Option<Measurement>,
    pub clear_height_mm: Option<Measurement>,
    pub minimum_turn_radius_mm: Option<Measurement>,
    /// Signed: positive is uphill from from_node to to_node.
    pub grade_permille: Option<Measurement>,
    /// Maximum obstacle height when travelling from from_node to to_node.
    pub maximum_step_up_mm: Option<Measurement>,
    /// Maximum obstacle height when travelling from from_node to to_node.
    pub maximum_step_down_mm: Option<Measurement>,
    /// 0..=1000 evidence scale; interpretation must be declared by the data source.
    pub traction_permille: Option<Measurement>,
    pub hazard_state: Option<TimedEvidence<HazardState>>,
    pub vertical_transition: bool,
    /// Required only for profiles that declare qualified support necessary.
    pub vertical_support: Option<TimedEvidence<SupportState>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraversalProfile {
    pub id: String,
    pub mode: TraversalMode,
    /// Occupied width/height including chosen PPE, load and aid. Safety margins
    /// and body/device shape envelope must already be included upstream.
    pub required_width_mm: u32,
    pub required_height_mm: u32,
    /// Minimum clear radius required by this person/device/load configuration.
    pub required_turn_radius_mm: u32,
    pub maximum_ascent_grade_permille: u32,
    pub maximum_descent_grade_permille: u32,
    pub maximum_step_up_mm: u32,
    pub maximum_step_down_mm: u32,
    pub minimum_traction_permille: u16,
    /// Policy from the responsible rescue/engineering authority.
    pub requires_qualified_support_for_vertical_transition: bool,
}

impl TraversalProfile {
    fn validate(&self) -> Result<(), &'static str> {
        if self.id.trim().is_empty() {
            return Err("profile id must not be empty");
        }
        if self.required_width_mm == 0 || self.required_height_mm == 0 {
            return Err("required width and height must be positive");
        }
        if self.minimum_traction_permille > 1000 {
            return Err("minimum traction must be within 0..=1000 permille");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PassageDecision {
    pub passage_id: String,
    pub direction: TravelDirection,
    pub decision: PassageDecisionKind,
    pub reasons: Vec<ReasonCode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TravelDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reading {
    Missing,
    Stale,
    Fresh { lower: i64, upper: i64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TimedRead<T> {
    Missing,
    Stale,
    Fresh(T),
}

fn read_measurement(
    observation: Option<&Measurement>,
    now_ms: u64,
    allow_negative: bool,
) -> Result<Reading, &'static str> {
    let Some(observation) = observation else {
        return Ok(Reading::Missing);
    };
    if observation.evidence_id.trim().is_empty() {
        return Err("measurement evidence id must not be empty");
    }
    if observation.observed_at_ms > now_ms {
        return Err("measurement timestamp is in the future");
    }
    if observation.uncertainty > i64::MAX as u64 {
        return Err("measurement uncertainty is too large");
    }
    if !allow_negative && observation.value < 0 {
        return Err("measurement value must not be negative");
    }
    if now_ms.saturating_sub(observation.observed_at_ms) > observation.max_age_ms {
        return Ok(Reading::Stale);
    }
    let uncertainty = observation.uncertainty as i64;
    Ok(Reading::Fresh {
        lower: observation.value.saturating_sub(uncertainty),
        upper: observation.value.saturating_add(uncertainty),
    })
}

fn read_timed<T: Copy>(
    observation: Option<&TimedEvidence<T>>,
    now_ms: u64,
) -> Result<TimedRead<T>, &'static str> {
    let Some(observation) = observation else {
        return Ok(TimedRead::Missing);
    };
    if observation.evidence_id.trim().is_empty() {
        return Err("timed evidence id must not be empty");
    }
    if observation.observed_at_ms > now_ms {
        return Err("timed evidence timestamp is in the future");
    }
    if now_ms.saturating_sub(observation.observed_at_ms) > observation.max_age_ms {
        return Ok(TimedRead::Stale);
    }
    Ok(TimedRead::Fresh(observation.value))
}

fn add_reason(reasons: &mut Vec<ReasonCode>, reason: ReasonCode) {
    reasons.push(reason);
}

fn evaluate_measurement(
    observation: Option<&Measurement>,
    now_ms: u64,
    allow_negative: bool,
    missing_reason: ReasonCode,
    stale_reason: ReasonCode,
    violation: impl FnOnce(i64, i64) -> Option<ReasonCode>,
    reasons: &mut Vec<ReasonCode>,
) -> Result<(), &'static str> {
    match read_measurement(observation, now_ms, allow_negative)? {
        Reading::Missing => add_reason(reasons, missing_reason),
        Reading::Stale => add_reason(reasons, stale_reason),
        Reading::Fresh { lower, upper } => {
            if let Some(reason) = violation(lower, upper) {
                add_reason(reasons, reason);
            }
        }
    }
    Ok(())
}

fn evaluate_timed_state<T: Copy + Eq>(
    observation: Option<&TimedEvidence<T>>,
    now_ms: u64,
    missing_reason: ReasonCode,
    stale_reason: ReasonCode,
    violation_value: T,
    violation_reason: ReasonCode,
    reasons: &mut Vec<ReasonCode>,
) -> Result<(), &'static str> {
    match read_timed(observation, now_ms)? {
        TimedRead::Missing => add_reason(reasons, missing_reason),
        TimedRead::Stale => add_reason(reasons, stale_reason),
        TimedRead::Fresh(value) if value == violation_value => {
            add_reason(reasons, violation_reason)
        }
        TimedRead::Fresh(_) => {}
    }
    Ok(())
}

/// Evaluate one passage in one direction against one declared profile.
///
/// Known hard violations take precedence over missing evidence in the overall
/// classification, but all applicable reasons are retained. Route search accepts
/// only AdmissibleWithinDeclaredEnvelope and never treats Unknown as pass.
pub fn evaluate_passage(
    passage: &Passage,
    profile: &TraversalProfile,
    now_ms: u64,
    direction: TravelDirection,
) -> Result<PassageDecision, &'static str> {
    profile.validate()?;
    if passage.id.trim().is_empty() {
        return Err("passage id must not be empty");
    }
    if passage.from_node.trim().is_empty() || passage.to_node.trim().is_empty() {
        return Err("passage endpoints must not be empty");
    }
    if passage.from_node == passage.to_node {
        return Err("self-loop passage is not accepted");
    }
    if direction == TravelDirection::Reverse && !passage.bidirectional {
        return Err("reverse traversal is not declared for this passage");
    }

    let mut reasons = Vec::new();

    evaluate_measurement(
        passage.clear_width_mm.as_ref(),
        now_ms,
        false,
        ReasonCode::MissingClearWidth,
        ReasonCode::StaleClearWidth,
        |lower, _| {
            (lower < i64::from(profile.required_width_mm))
                .then_some(ReasonCode::ClearWidthInsufficient)
        },
        &mut reasons,
    )?;
    evaluate_measurement(
        passage.clear_height_mm.as_ref(),
        now_ms,
        false,
        ReasonCode::MissingClearHeight,
        ReasonCode::StaleClearHeight,
        |lower, _| {
            (lower < i64::from(profile.required_height_mm))
                .then_some(ReasonCode::ClearHeightInsufficient)
        },
        &mut reasons,
    )?;
    evaluate_measurement(
        passage.minimum_turn_radius_mm.as_ref(),
        now_ms,
        false,
        ReasonCode::MissingTurnRadius,
        ReasonCode::StaleTurnRadius,
        |lower, _| {
            (lower < i64::from(profile.required_turn_radius_mm))
                .then_some(ReasonCode::TurnRadiusInsufficient)
        },
        &mut reasons,
    )?;

    evaluate_measurement(
        passage.grade_permille.as_ref(),
        now_ms,
        true,
        ReasonCode::MissingGrade,
        ReasonCode::StaleGrade,
        |lower, upper| {
            let (ascent_upper, descent_upper) = if direction == TravelDirection::Forward {
                (upper, lower.saturating_neg())
            } else {
                // Reversing signed grade [lower, upper] gives [-upper, -lower].
                // Maximum uphill grade is -lower; maximum downhill magnitude is upper.
                (lower.saturating_neg(), upper)
            };
            if ascent_upper > i64::from(profile.maximum_ascent_grade_permille) {
                Some(ReasonCode::AscentTooSteep)
            } else if descent_upper > i64::from(profile.maximum_descent_grade_permille) {
                Some(ReasonCode::DescentTooSteep)
            } else {
                None
            }
        },
        &mut reasons,
    )?;

    // Step-up/down measurements describe from_node -> to_node. Reversing the
    // segment swaps which physical obstacle is an up-step versus down-step.
    let (step_up, step_down) = if direction == TravelDirection::Forward {
        (
            passage.maximum_step_up_mm.as_ref(),
            passage.maximum_step_down_mm.as_ref(),
        )
    } else {
        (
            passage.maximum_step_down_mm.as_ref(),
            passage.maximum_step_up_mm.as_ref(),
        )
    };
    evaluate_measurement(
        step_up,
        now_ms,
        false,
        ReasonCode::MissingStepUp,
        ReasonCode::StaleStepUp,
        |_, upper| {
            (upper > i64::from(profile.maximum_step_up_mm)).then_some(ReasonCode::StepUpTooHigh)
        },
        &mut reasons,
    )?;
    evaluate_measurement(
        step_down,
        now_ms,
        false,
        ReasonCode::MissingStepDown,
        ReasonCode::StaleStepDown,
        |_, upper| {
            (upper > i64::from(profile.maximum_step_down_mm)).then_some(ReasonCode::StepDownTooHigh)
        },
        &mut reasons,
    )?;

    evaluate_measurement(
        passage.traction_permille.as_ref(),
        now_ms,
        false,
        ReasonCode::MissingTraction,
        ReasonCode::StaleTraction,
        |lower, upper| {
            if upper > 1000 || lower < i64::from(profile.minimum_traction_permille) {
                Some(ReasonCode::TractionInsufficient)
            } else {
                None
            }
        },
        &mut reasons,
    )?;

    evaluate_timed_state(
        passage.hazard_state.as_ref(),
        now_ms,
        ReasonCode::MissingHazardState,
        ReasonCode::StaleHazardState,
        HazardState::BlockingHazard,
        ReasonCode::BlockingHazard,
        &mut reasons,
    )?;

    if passage.vertical_transition && profile.requires_qualified_support_for_vertical_transition {
        evaluate_timed_state(
            passage.vertical_support.as_ref(),
            now_ms,
            ReasonCode::MissingQualifiedSupport,
            ReasonCode::StaleQualifiedSupport,
            SupportState::NotQualified,
            ReasonCode::SupportNotQualified,
            &mut reasons,
        )?;
        // Absence/staleness is Unknown. The only admissible fresh support state
        // is the explicit positive qualification from the responsible authority.
        match read_timed(passage.vertical_support.as_ref(), now_ms)? {
            TimedRead::Fresh(SupportState::QualifiedByResponsibleAuthority) => {}
            TimedRead::Fresh(SupportState::NotQualified) => {
                if !reasons.contains(&ReasonCode::SupportNotQualified) {
                    add_reason(&mut reasons, ReasonCode::SupportNotQualified);
                }
            }
            TimedRead::Missing | TimedRead::Stale => {}
        }
    }

    reasons.sort();
    reasons.dedup();

    let has_rejection = reasons.iter().any(|reason| {
        matches!(
            reason,
            ReasonCode::ClearWidthInsufficient
                | ReasonCode::ClearHeightInsufficient
                | ReasonCode::TurnRadiusInsufficient
                | ReasonCode::AscentTooSteep
                | ReasonCode::DescentTooSteep
                | ReasonCode::StepUpTooHigh
                | ReasonCode::StepDownTooHigh
                | ReasonCode::TractionInsufficient
                | ReasonCode::BlockingHazard
                | ReasonCode::SupportNotQualified
        )
    });
    let has_unknown = reasons.iter().any(|reason| {
        matches!(
            reason,
            ReasonCode::MissingClearWidth
                | ReasonCode::StaleClearWidth
                | ReasonCode::MissingClearHeight
                | ReasonCode::StaleClearHeight
                | ReasonCode::MissingTurnRadius
                | ReasonCode::StaleTurnRadius
                | ReasonCode::MissingGrade
                | ReasonCode::StaleGrade
                | ReasonCode::MissingStepUp
                | ReasonCode::StaleStepUp
                | ReasonCode::MissingStepDown
                | ReasonCode::StaleStepDown
                | ReasonCode::MissingTraction
                | ReasonCode::StaleTraction
                | ReasonCode::MissingHazardState
                | ReasonCode::StaleHazardState
                | ReasonCode::MissingQualifiedSupport
                | ReasonCode::StaleQualifiedSupport
        )
    });

    let decision = if has_rejection {
        PassageDecisionKind::RejectOutsideEnvelope
    } else if has_unknown {
        PassageDecisionKind::UnknownInsufficientOrStaleEvidence
    } else {
        PassageDecisionKind::AdmissibleWithinDeclaredEnvelope
    };

    Ok(PassageDecision {
        passage_id: passage.id.clone(),
        direction,
        decision,
        reasons,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteTraversal {
    pub passage_id: String,
    pub from_node: String,
    pub to_node: String,
    pub direction: TravelDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissibleRoute {
    pub nodes: Vec<String>,
    pub traversals: Vec<RouteTraversal>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteSearchDisposition {
    AdmissibleRouteFound,
    NoAdmissibleRouteInDeclaredGraph,
    SearchLimitExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteSearchReport {
    pub disposition: RouteSearchDisposition,
    pub route: Option<AdmissibleRoute>,
    pub expanded_states: usize,
    pub rejected_edge_directions: usize,
    pub unknown_edge_directions: usize,
}

#[derive(Clone, Debug)]
struct DirectedEdge {
    passage_index: usize,
    from: String,
    to: String,
    direction: TravelDirection,
}

/// Find a lowest-hop admissible route using only edges whose required evidence
/// is fresh and whose declared hard constraints pass. Stable lexical ordering
/// makes tie-breaking reproducible. If a cap truncates search, the result is
/// SearchLimitExceeded, not a false claim that no route exists.
pub fn find_admissible_route(
    passages: &[Passage],
    profile: &TraversalProfile,
    source: &str,
    destination: &str,
    now_ms: u64,
    max_hops: usize,
    max_expansions: usize,
) -> Result<RouteSearchReport, &'static str> {
    profile.validate()?;
    if source.trim().is_empty() || destination.trim().is_empty() {
        return Err("route endpoints must not be empty");
    }
    if max_hops == 0 || max_expansions == 0 {
        return Err("route search caps must be positive");
    }
    if source == destination {
        return Ok(RouteSearchReport {
            disposition: RouteSearchDisposition::AdmissibleRouteFound,
            route: Some(AdmissibleRoute {
                nodes: vec![source.to_owned()],
                traversals: Vec::new(),
            }),
            expanded_states: 0,
            rejected_edge_directions: 0,
            unknown_edge_directions: 0,
        });
    }

    let mut passage_ids = BTreeSet::new();
    let mut directed_edges = Vec::new();
    for (index, passage) in passages.iter().enumerate() {
        if passage.id.trim().is_empty() {
            return Err("passage id must not be empty");
        }
        if !passage_ids.insert(passage.id.as_str()) {
            return Err("passage ids must be unique");
        }
        if passage.from_node.trim().is_empty() || passage.to_node.trim().is_empty() {
            return Err("passage endpoints must not be empty");
        }
        if passage.from_node == passage.to_node {
            return Err("self-loop passage is not accepted");
        }
        directed_edges.push(DirectedEdge {
            passage_index: index,
            from: passage.from_node.clone(),
            to: passage.to_node.clone(),
            direction: TravelDirection::Forward,
        });
        if passage.bidirectional {
            directed_edges.push(DirectedEdge {
                passage_index: index,
                from: passage.to_node.clone(),
                to: passage.from_node.clone(),
                direction: TravelDirection::Reverse,
            });
        }
    }
    directed_edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| {
                passages[a.passage_index]
                    .id
                    .cmp(&passages[b.passage_index].id)
            })
    });

    let mut admissible = Vec::new();
    let mut rejected_edge_directions = 0usize;
    let mut unknown_edge_directions = 0usize;
    for edge in directed_edges {
        let decision = evaluate_passage(
            &passages[edge.passage_index],
            profile,
            now_ms,
            edge.direction,
        )?;
        match decision.decision {
            PassageDecisionKind::AdmissibleWithinDeclaredEnvelope => admissible.push(edge),
            PassageDecisionKind::RejectOutsideEnvelope => rejected_edge_directions += 1,
            PassageDecisionKind::UnknownInsufficientOrStaleEvidence => unknown_edge_directions += 1,
        }
    }

    #[derive(Clone)]
    struct State {
        node: String,
        nodes: Vec<String>,
        traversals: Vec<RouteTraversal>,
    }

    let mut queue = VecDeque::new();
    queue.push_back(State {
        node: source.to_owned(),
        nodes: vec![source.to_owned()],
        traversals: Vec::new(),
    });
    let mut expanded_states = 0usize;
    let mut truncated_by_hops = false;

    while let Some(state) = queue.pop_front() {
        if expanded_states >= max_expansions {
            return Ok(RouteSearchReport {
                disposition: RouteSearchDisposition::SearchLimitExceeded,
                route: None,
                expanded_states,
                rejected_edge_directions,
                unknown_edge_directions,
            });
        }
        expanded_states += 1;

        if state.node == destination {
            return Ok(RouteSearchReport {
                disposition: RouteSearchDisposition::AdmissibleRouteFound,
                route: Some(AdmissibleRoute {
                    nodes: state.nodes,
                    traversals: state.traversals,
                }),
                expanded_states,
                rejected_edge_directions,
                unknown_edge_directions,
            });
        }

        if state.traversals.len() >= max_hops {
            truncated_by_hops = true;
            continue;
        }

        for edge in admissible.iter().filter(|edge| edge.from == state.node) {
            if state.nodes.iter().any(|node| node == &edge.to) {
                continue;
            }
            let mut nodes = state.nodes.clone();
            nodes.push(edge.to.clone());
            let mut traversals = state.traversals.clone();
            traversals.push(RouteTraversal {
                passage_id: passages[edge.passage_index].id.clone(),
                from_node: edge.from.clone(),
                to_node: edge.to.clone(),
                direction: edge.direction,
            });
            // Bound pending work as well as popped expansions: high-degree graphs
            // must not allocate an unbounded frontier before the expansion cap trips.
            if queue.len() >= max_expansions {
                return Ok(RouteSearchReport {
                    disposition: RouteSearchDisposition::SearchLimitExceeded,
                    route: None,
                    expanded_states,
                    rejected_edge_directions,
                    unknown_edge_directions,
                });
            }
            queue.push_back(State {
                node: edge.to.clone(),
                nodes,
                traversals,
            });
        }
    }

    Ok(RouteSearchReport {
        disposition: if truncated_by_hops {
            RouteSearchDisposition::SearchLimitExceeded
        } else {
            RouteSearchDisposition::NoAdmissibleRouteInDeclaredGraph
        },
        route: None,
        expanded_states,
        rejected_edge_directions,
        unknown_edge_directions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(value: i64, uncertainty: u64, id: &str) -> Measurement {
        Measurement {
            value,
            uncertainty,
            observed_at_ms: 100,
            max_age_ms: 100,
            evidence_id: id.to_owned(),
        }
    }

    fn timed<T>(value: T, id: &str) -> TimedEvidence<T> {
        TimedEvidence {
            value,
            observed_at_ms: 100,
            max_age_ms: 100,
            evidence_id: id.to_owned(),
        }
    }

    fn profile() -> TraversalProfile {
        TraversalProfile {
            id: "person-ppe-kit-a".to_owned(),
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
        }
    }

    fn passage(id: &str, from: &str, to: &str) -> Passage {
        Passage {
            id: id.to_owned(),
            from_node: from.to_owned(),
            to_node: to.to_owned(),
            bidirectional: true,
            clear_width_mm: Some(m(650, 10, "width-1")),
            clear_height_mm: Some(m(900, 10, "height-1")),
            minimum_turn_radius_mm: Some(m(1000, 20, "turn-1")),
            grade_permille: Some(m(100, 10, "grade-1")),
            maximum_step_up_mm: Some(m(50, 5, "step-up-1")),
            maximum_step_down_mm: Some(m(60, 5, "step-down-1")),
            traction_permille: Some(m(600, 20, "traction-1")),
            hazard_state: Some(timed(HazardState::NoBlockingHazardObserved, "hazard-1")),
            vertical_transition: false,
            vertical_support: None,
        }
    }

    #[test]
    fn fresh_evidence_inside_conservative_bounds_is_admissible() {
        let result = evaluate_passage(
            &passage("p", "s", "d"),
            &profile(),
            150,
            TravelDirection::Forward,
        )
        .expect("valid inputs");
        assert_eq!(
            result.decision,
            PassageDecisionKind::AdmissibleWithinDeclaredEnvelope
        );
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn uncertainty_is_applied_conservatively_to_clearance() {
        let mut p = passage("p", "s", "d");
        p.clear_width_mm = Some(m(510, 20, "width-boundary"));
        let result =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(result.decision, PassageDecisionKind::RejectOutsideEnvelope);
        assert!(result.reasons.contains(&ReasonCode::ClearWidthInsufficient));
    }

    #[test]
    fn missing_required_evidence_is_unknown_not_admissible() {
        let mut p = passage("p", "s", "d");
        p.traction_permille = None;
        let result =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(
            result.decision,
            PassageDecisionKind::UnknownInsufficientOrStaleEvidence
        );
        assert!(result.reasons.contains(&ReasonCode::MissingTraction));
    }

    #[test]
    fn stale_required_evidence_is_unknown_not_admissible() {
        let mut p = passage("p", "s", "d");
        let mut stale = m(900, 0, "old-height");
        stale.observed_at_ms = 0;
        stale.max_age_ms = 10;
        p.clear_height_mm = Some(stale);
        let result =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(
            result.decision,
            PassageDecisionKind::UnknownInsufficientOrStaleEvidence
        );
        assert!(result.reasons.contains(&ReasonCode::StaleClearHeight));
    }

    #[test]
    fn known_blocking_hazard_rejects_even_if_dimensions_fit() {
        let mut p = passage("p", "s", "d");
        p.hazard_state = Some(timed(HazardState::BlockingHazard, "inrush"));
        let result =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(result.decision, PassageDecisionKind::RejectOutsideEnvelope);
        assert!(result.reasons.contains(&ReasonCode::BlockingHazard));
    }

    #[test]
    fn directional_grade_changes_when_traversing_in_reverse() {
        let mut p = passage("p", "s", "d");
        p.grade_permille = Some(m(300, 0, "steep-grade"));
        let forward =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        let reverse =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Reverse).expect("valid inputs");
        assert_eq!(forward.decision, PassageDecisionKind::RejectOutsideEnvelope);
        assert_eq!(
            reverse.decision,
            PassageDecisionKind::AdmissibleWithinDeclaredEnvelope
        );
        assert!(forward.reasons.contains(&ReasonCode::AscentTooSteep));
    }

    #[test]
    fn reversing_a_downhill_segment_checks_the_uphill_limit() {
        let mut p = passage("p", "s", "d");
        p.grade_permille = Some(m(-280, 0, "downhill-grade"));
        let forward =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        let reverse =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Reverse).expect("valid inputs");
        assert_eq!(
            forward.decision,
            PassageDecisionKind::AdmissibleWithinDeclaredEnvelope
        );
        assert_eq!(reverse.decision, PassageDecisionKind::RejectOutsideEnvelope);
        assert!(reverse.reasons.contains(&ReasonCode::AscentTooSteep));
    }

    #[test]
    fn reverse_traversal_swaps_step_up_and_step_down_limits() {
        let mut p = passage("p", "s", "d");
        p.maximum_step_up_mm = Some(m(130, 0, "up-step"));
        p.maximum_step_down_mm = Some(m(40, 0, "down-step"));
        let forward =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        let reverse =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Reverse).expect("valid inputs");
        assert!(forward.reasons.contains(&ReasonCode::StepUpTooHigh));
        assert!(!reverse.reasons.contains(&ReasonCode::StepUpTooHigh));
    }

    #[test]
    fn vertical_transition_requires_explicit_qualified_support_when_profile_demands_it() {
        let mut p = passage("p", "s", "d");
        p.vertical_transition = true;
        p.vertical_support = None;
        let unknown =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(
            unknown.decision,
            PassageDecisionKind::UnknownInsufficientOrStaleEvidence
        );
        assert!(
            unknown
                .reasons
                .contains(&ReasonCode::MissingQualifiedSupport)
        );

        p.vertical_support = Some(timed(SupportState::NotQualified, "support-check"));
        let rejected =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(
            rejected.decision,
            PassageDecisionKind::RejectOutsideEnvelope
        );
        assert!(rejected.reasons.contains(&ReasonCode::SupportNotQualified));

        p.vertical_support = Some(timed(
            SupportState::QualifiedByResponsibleAuthority,
            "support-check-2",
        ));
        let admissible =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(
            admissible.decision,
            PassageDecisionKind::AdmissibleWithinDeclaredEnvelope
        );
    }

    #[test]
    fn future_timestamps_empty_evidence_ids_and_invalid_scales_fail_closed() {
        let mut p = passage("p", "s", "d");
        let mut future = m(650, 0, "future");
        future.observed_at_ms = 151;
        p.clear_width_mm = Some(future);
        assert!(evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).is_err());

        let mut p = passage("p", "s", "d");
        p.clear_width_mm = Some(m(650, 0, " "));
        assert!(evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).is_err());

        let mut p = passage("p", "s", "d");
        p.traction_permille = Some(m(1200, 0, "bad-scale"));
        let result = evaluate_passage(&p, &profile(), 150, TravelDirection::Forward)
            .expect("structural inputs valid");
        assert_eq!(result.decision, PassageDecisionKind::RejectOutsideEnvelope);
        assert!(result.reasons.contains(&ReasonCode::TractionInsufficient));
    }

    #[test]
    fn reverse_traversal_is_not_allowed_unless_declared() {
        let mut p = passage("p", "s", "d");
        p.bidirectional = false;
        assert!(evaluate_passage(&p, &profile(), 150, TravelDirection::Reverse).is_err());
    }

    #[test]
    fn profile_change_for_ppe_or_load_can_invalidate_a_passage() {
        let p = passage("p", "s", "d");
        let mut wider = profile();
        wider.required_width_mm = 650;
        let result =
            evaluate_passage(&p, &wider, 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(result.decision, PassageDecisionKind::RejectOutsideEnvelope);
    }

    #[test]
    fn known_rejection_is_not_hidden_by_other_missing_evidence() {
        let mut p = passage("p", "s", "d");
        p.clear_width_mm = Some(m(300, 0, "narrow"));
        p.traction_permille = None;
        let result =
            evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).expect("valid inputs");
        assert_eq!(result.decision, PassageDecisionKind::RejectOutsideEnvelope);
        assert!(result.reasons.contains(&ReasonCode::ClearWidthInsufficient));
        assert!(result.reasons.contains(&ReasonCode::MissingTraction));
    }

    #[test]
    fn graph_search_excludes_unknown_edges_and_finds_an_admissible_alternative() {
        let mut unknown = passage("s-a", "s", "a");
        unknown.clear_width_mm = None;
        let passages = vec![
            unknown,
            passage("a-d", "a", "d"),
            passage("s-b", "s", "b"),
            passage("b-d", "b", "d"),
        ];
        let report = find_admissible_route(&passages, &profile(), "s", "d", 150, 8, 100)
            .expect("valid graph");
        assert_eq!(
            report.disposition,
            RouteSearchDisposition::AdmissibleRouteFound
        );
        assert_eq!(report.route.as_ref().unwrap().nodes, vec!["s", "b", "d"]);
        assert!(report.unknown_edge_directions > 0);
    }

    #[test]
    fn graph_reports_no_admissible_route_when_required_evidence_is_unknown() {
        let mut p = passage("s-d", "s", "d");
        p.hazard_state = None;
        let report =
            find_admissible_route(&[p], &profile(), "s", "d", 150, 4, 20).expect("valid graph");
        assert_eq!(
            report.disposition,
            RouteSearchDisposition::NoAdmissibleRouteInDeclaredGraph
        );
        assert!(report.route.is_none());
        assert!(report.unknown_edge_directions > 0);
    }

    #[test]
    fn route_search_cap_does_not_masquerade_as_no_route() {
        let passages = vec![
            passage("s-a", "s", "a"),
            passage("a-d", "a", "d"),
            passage("s-b", "s", "b"),
            passage("b-d", "b", "d"),
        ];
        let report =
            find_admissible_route(&passages, &profile(), "s", "d", 150, 8, 1).expect("valid graph");
        assert_eq!(
            report.disposition,
            RouteSearchDisposition::SearchLimitExceeded
        );
        assert!(report.route.is_none());
    }

    #[test]
    fn high_degree_frontier_is_bounded_before_allocating_all_candidates() {
        let passages = vec![
            passage("s-a", "s", "a"),
            passage("s-b", "s", "b"),
            passage("s-c", "s", "c"),
            passage("a-d", "a", "d"),
            passage("b-d", "b", "d"),
            passage("c-d", "c", "d"),
        ];
        let report =
            find_admissible_route(&passages, &profile(), "s", "d", 150, 4, 2).expect("valid graph");
        assert_eq!(
            report.disposition,
            RouteSearchDisposition::SearchLimitExceeded
        );
        assert!(report.route.is_none());
    }

    #[test]
    fn hop_limit_exhaustion_is_not_reported_as_a_complete_no_route_search() {
        let passages = vec![
            passage("s-a", "s", "a"),
            passage("a-b", "a", "b"),
            passage("b-d", "b", "d"),
        ];
        let report = find_admissible_route(&passages, &profile(), "s", "d", 150, 1, 100)
            .expect("valid graph");
        assert_eq!(
            report.disposition,
            RouteSearchDisposition::SearchLimitExceeded
        );
    }

    #[test]
    fn duplicate_passage_ids_are_rejected() {
        let passages = vec![passage("same", "s", "a"), passage("same", "a", "d")];
        assert!(find_admissible_route(&passages, &profile(), "s", "d", 150, 4, 20).is_err());
    }

    #[test]
    fn equal_length_route_ties_are_deterministic() {
        let passages = vec![
            passage("s-b", "s", "b"),
            passage("b-d", "b", "d"),
            passage("s-a", "s", "a"),
            passage("a-d", "a", "d"),
        ];
        let first = find_admissible_route(&passages, &profile(), "s", "d", 150, 4, 100)
            .expect("valid graph");
        let mut reversed_input = passages.clone();
        reversed_input.reverse();
        let second = find_admissible_route(&reversed_input, &profile(), "s", "d", 150, 4, 100)
            .expect("valid graph");
        assert_eq!(first, second);
        assert_eq!(first.route.unwrap().nodes, vec!["s", "a", "d"]);
    }

    #[test]
    fn known_invalid_measurement_values_are_not_treated_as_unknown() {
        let mut p = passage("p", "s", "d");
        p.clear_width_mm = Some(m(-10, 0, "negative-width"));
        assert!(evaluate_passage(&p, &profile(), 150, TravelDirection::Forward).is_err());
    }
}
