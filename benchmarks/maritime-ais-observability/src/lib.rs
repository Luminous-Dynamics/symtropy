use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Zone {
    Outside,
    Anchorage,
    Approach,
    Berth,
}

impl Zone {
    fn dwell_ticks(self, profile: &DwellProfile) -> i64 {
        match self {
            Self::Outside => profile.outside_dwell_ticks,
            Self::Anchorage => profile.anchorage_dwell_ticks,
            Self::Approach => profile.approach_dwell_ticks,
            Self::Berth => profile.berth_dwell_ticks,
        }
    }

    fn max_speed_milli_knots(self, profile: &DwellProfile) -> u32 {
        match self {
            Self::Outside => profile.outside_max_speed_milli_knots,
            Self::Anchorage => profile.anchorage_max_speed_milli_knots,
            Self::Approach => profile.approach_max_speed_milli_knots,
            Self::Berth => profile.berth_max_speed_milli_knots,
        }
    }

    fn service_state(self) -> ServiceState {
        match self {
            Self::Outside => ServiceState::Outside,
            Self::Anchorage => ServiceState::Anchorage,
            Self::Approach => ServiceState::Approach,
            Self::Berth => ServiceState::Berth,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceState {
    Outside,
    Anchorage,
    Approach,
    Berth,
    Departure,
    Indeterminate,
}

impl ServiceState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Outside => "OUTSIDE",
            Self::Anchorage => "ANCHORAGE",
            Self::Approach => "APPROACH",
            Self::Berth => "BERTH",
            Self::Departure => "DEPARTURE",
            Self::Indeterminate => "INDETERMINATE",
        }
    }

    fn is_stable_for(self, zone: Zone) -> bool {
        match self {
            Self::Outside => zone == Zone::Outside,
            Self::Anchorage => zone == Zone::Anchorage,
            Self::Approach => zone == Zone::Approach,
            Self::Berth => zone == Zone::Berth,
            Self::Departure => matches!(zone, Zone::Approach | Zone::Anchorage),
            Self::Indeterminate => false,
        }
    }

    fn transition_target(self, zone: Zone) -> Self {
        match (self, zone) {
            (Self::Berth, Zone::Outside | Zone::Approach | Zone::Anchorage) => Self::Departure,
            (Self::Departure, Zone::Outside) => Self::Outside,
            (Self::Departure, Zone::Approach | Zone::Anchorage) => Self::Departure,
            _ => zone.service_state(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    AnchorageZoneEntry,
    ApproachZoneEntry,
    BerthZoneEntry,
    BerthExitDetected,
    PortExitDetected,
    ZoneReacquired,
    ObservabilityGap,
    InvalidPosition,
    SpeedZoneConflict,
}

impl EventKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::AnchorageZoneEntry => "AIS_INFERRED_ANCHORAGE_ZONE_ENTRY",
            Self::ApproachZoneEntry => "AIS_INFERRED_APPROACH_ZONE_ENTRY",
            Self::BerthZoneEntry => "AIS_INFERRED_BERTH_ZONE_ENTRY",
            Self::BerthExitDetected => "AIS_INFERRED_BERTH_ZONE_EXIT",
            Self::PortExitDetected => "AIS_INFERRED_PORT_GEOFENCE_EXIT",
            Self::ZoneReacquired => "AIS_ZONE_REACQUIRED_AFTER_UNCERTAINTY",
            Self::ObservabilityGap => "AIS_OBSERVABILITY_GAP",
            Self::InvalidPosition => "AIS_INVALID_POSITION_FIX",
            Self::SpeedZoneConflict => "AIS_ZONE_SPEED_INCONSISTENCY",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DwellProfile {
    pub version: String,
    pub max_observation_gap_ticks: i64,
    pub max_receive_gap_ticks: i64,
    pub anchorage_dwell_ticks: i64,
    pub approach_dwell_ticks: i64,
    pub berth_dwell_ticks: i64,
    pub departure_dwell_ticks: i64,
    pub outside_dwell_ticks: i64,
    pub anchorage_max_speed_milli_knots: u32,
    pub approach_max_speed_milli_knots: u32,
    pub berth_max_speed_milli_knots: u32,
    pub outside_max_speed_milli_knots: u32,
}

impl Default for DwellProfile {
    fn default() -> Self {
        Self {
            version: "edge-port-fsm-v0.1".to_string(),
            max_observation_gap_ticks: 2,
            max_receive_gap_ticks: 2,
            anchorage_dwell_ticks: 2,
            approach_dwell_ticks: 1,
            berth_dwell_ticks: 3,
            departure_dwell_ticks: 2,
            outside_dwell_ticks: 2,
            anchorage_max_speed_milli_knots: 1_500,
            approach_max_speed_milli_knots: 12_000,
            berth_max_speed_milli_knots: 500,
            outside_max_speed_milli_knots: 50_000,
        }
    }
}

impl DwellProfile {
    fn validate(&self) -> Result<(), String> {
        if self.version.trim().is_empty() {
            return Err("profile version must be non-empty".to_string());
        }
        if self.max_observation_gap_ticks <= 0
            || self.max_receive_gap_ticks <= 0
            || self.anchorage_dwell_ticks <= 0
            || self.approach_dwell_ticks <= 0
            || self.berth_dwell_ticks <= 0
            || self.departure_dwell_ticks <= 0
            || self.outside_dwell_ticks <= 0
        {
            return Err("gap and dwell thresholds must be positive".to_string());
        }
        Ok(())
    }

    fn required_dwell(&self, current: ServiceState, zone: Zone) -> i64 {
        if current == ServiceState::Berth && zone != Zone::Berth {
            self.departure_dwell_ticks
        } else if current == ServiceState::Departure && zone == Zone::Outside {
            self.outside_dwell_ticks
        } else {
            zone.dwell_ticks(self)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AisFix {
    pub call_reference_id: String,
    pub mmsi: String,
    pub message_id: String,
    pub source_id: String,
    pub receiver_id: String,
    pub geofence_digest: String,
    pub spatial_reference_id: String,
    pub tick: i64,
    pub received_tick: i64,
    pub zone: Zone,
    pub speed_milli_knots: u32,
    pub position_valid: bool,
    pub nav_status_raw: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InferredEvent {
    pub kind: EventKind,
    pub lower_tick: i64,
    pub upper_tick: i64,
    pub confirmation_tick: i64,
    pub confirmation_received_tick: i64,
    pub event_gap_ticks: Option<i64>,
    pub receive_gap_ticks: Option<i64>,
    pub supporting_fix_count: usize,
    pub profile_version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ScoredEventKind {
    AnchorageEntry,
    ApproachEntry,
    BerthEntry,
    BerthExit,
    PortExit,
}

impl ScoredEventKind {
    fn from_inferred(kind: EventKind) -> Option<Self> {
        match kind {
            EventKind::AnchorageZoneEntry => Some(Self::AnchorageEntry),
            EventKind::ApproachZoneEntry => Some(Self::ApproachEntry),
            EventKind::BerthZoneEntry => Some(Self::BerthEntry),
            EventKind::BerthExitDetected => Some(Self::BerthExit),
            EventKind::PortExitDetected => Some(Self::PortExit),
            EventKind::ZoneReacquired
            | EventKind::ObservabilityGap
            | EventKind::InvalidPosition
            | EventKind::SpeedZoneConflict => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroundTruthEvent {
    pub call_reference_id: String,
    pub kind: ScoredEventKind,
    pub event_tick: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScoredPrediction {
    pub call_reference_id: String,
    pub kind: ScoredEventKind,
    pub lower_tick: i64,
    pub upper_tick: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventMetrics {
    pub true_positive: usize,
    pub false_positive: usize,
    pub false_negative: usize,
    pub total_interval_distance_ticks: u128,
    pub total_matched_interval_width_ticks: u128,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub mean_interval_distance_ticks: Option<f64>,
    pub mean_matched_interval_width_ticks: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventScorecard {
    pub by_kind: BTreeMap<ScoredEventKind, EventMetrics>,
    pub overall: EventMetrics,
}

#[derive(Clone, Debug, Default)]
struct MutableEventMetrics {
    true_positive: usize,
    false_positive: usize,
    false_negative: usize,
    total_interval_distance: u128,
    total_interval_width: u128,
}

impl MutableEventMetrics {
    fn finalize(self) -> EventMetrics {
        let predicted = self.true_positive + self.false_positive;
        let actual = self.true_positive + self.false_negative;
        EventMetrics {
            true_positive: self.true_positive,
            false_positive: self.false_positive,
            false_negative: self.false_negative,
            total_interval_distance_ticks: self.total_interval_distance,
            total_matched_interval_width_ticks: self.total_interval_width,
            precision: (predicted > 0).then_some(self.true_positive as f64 / predicted as f64),
            recall: (actual > 0).then_some(self.true_positive as f64 / actual as f64),
            mean_interval_distance_ticks: (self.true_positive > 0)
                .then_some(self.total_interval_distance as f64 / self.true_positive as f64),
            mean_matched_interval_width_ticks: (self.true_positive > 0)
                .then_some(self.total_interval_width as f64 / self.true_positive as f64),
        }
    }
}

pub fn predictions_from_report(report: &CallReport) -> Vec<ScoredPrediction> {
    report
        .events
        .iter()
        .filter_map(|event| {
            ScoredEventKind::from_inferred(event.kind).map(|kind| ScoredPrediction {
                call_reference_id: report.call_reference_id.clone(),
                kind,
                lower_tick: event.lower_tick,
                upper_tick: event.upper_tick,
            })
        })
        .collect()
}

fn interval_distance(event_tick: i64, lower_tick: i64, upper_tick: i64) -> Result<u64, String> {
    if lower_tick > upper_tick {
        return Err("predicted event interval lower bound exceeds upper bound".to_string());
    }
    let distance = if event_tick < lower_tick {
        lower_tick
            .checked_sub(event_tick)
            .ok_or_else(|| "event interval distance overflow".to_string())?
    } else if event_tick > upper_tick {
        event_tick
            .checked_sub(upper_tick)
            .ok_or_else(|| "event interval distance overflow".to_string())?
    } else {
        0
    };
    u64::try_from(distance).map_err(|_| "negative event interval distance".to_string())
}

/// Match inferred transition intervals to independently adjudicated event times.
///
/// Matching is one-to-one by source-local call reference and event kind.
/// Predictions outside the tolerance become false positives; unmatched
/// ground-truth events become false negatives. An event inside the predicted
/// interval has zero interval-distance error.
#[derive(Clone, Debug)]
struct ResidualEdge {
    to: usize,
    reverse_index: usize,
    capacity: usize,
    cost: i128,
}

fn add_residual_edge(graph: &mut [Vec<ResidualEdge>], from: usize, to: usize, cost: i128) -> usize {
    let forward_index = graph[from].len();
    let reverse_index = graph[to].len();
    graph[from].push(ResidualEdge {
        to,
        reverse_index,
        capacity: 1,
        cost,
    });
    graph[to].push(ResidualEdge {
        to: from,
        reverse_index: forward_index,
        capacity: 0,
        cost: -cost,
    });
    forward_index
}

fn augment_min_cost_matching(
    graph: &mut [Vec<ResidualEdge>],
    source: usize,
    sink: usize,
) -> Result<(), String> {
    let node_count = graph.len();

    loop {
        let mut distance = vec![i128::MAX; node_count];
        let mut previous: Vec<Option<(usize, usize)>> = vec![None; node_count];
        distance[source] = 0;

        // Bellman-Ford handles negative reverse residual edges. Strict
        // improvement keeps equal-cost path selection deterministic.
        for _ in 0..node_count.saturating_sub(1) {
            let mut changed = false;
            for from in 0..node_count {
                if distance[from] == i128::MAX {
                    continue;
                }
                for (edge_index, edge) in graph[from].iter().enumerate() {
                    if edge.capacity == 0 {
                        continue;
                    }
                    let candidate = distance[from]
                        .checked_add(edge.cost)
                        .ok_or_else(|| "matching path cost overflow".to_string())?;
                    if candidate < distance[edge.to] {
                        distance[edge.to] = candidate;
                        previous[edge.to] = Some((from, edge_index));
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        if previous[sink].is_none() {
            break;
        }

        let mut node = sink;
        while node != source {
            let (from, edge_index) = previous[node]
                .ok_or_else(|| "matching augmenting path is incomplete".to_string())?;
            let reverse_index = graph[from][edge_index].reverse_index;
            if graph[from][edge_index].capacity == 0 {
                return Err("matching residual capacity underflow".to_string());
            }
            graph[from][edge_index].capacity -= 1;
            graph[node][reverse_index].capacity += 1;
            node = from;
        }
    }

    Ok(())
}

pub fn score_events(
    ground_truth: &[GroundTruthEvent],
    predictions: &[ScoredPrediction],
    tolerance_ticks: u64,
) -> Result<EventScorecard, String> {
    if ground_truth
        .iter()
        .any(|event| event.call_reference_id.trim().is_empty())
        || predictions
            .iter()
            .any(|event| event.call_reference_id.trim().is_empty())
    {
        return Err("call reference IDs must be non-empty".to_string());
    }

    for prediction in predictions {
        if prediction.lower_tick > prediction.upper_tick {
            return Err("predicted event interval lower bound exceeds upper bound".to_string());
        }
    }

    let source_node = 0;
    let truth_base = 1;
    let prediction_base = truth_base + ground_truth.len();
    let sink_node = prediction_base + predictions.len();
    let mut graph: Vec<Vec<ResidualEdge>> = (0..=sink_node).map(|_| Vec::new()).collect();

    for truth_index in 0..ground_truth.len() {
        add_residual_edge(&mut graph, source_node, truth_base + truth_index, 0);
    }
    for prediction_index in 0..predictions.len() {
        add_residual_edge(&mut graph, prediction_base + prediction_index, sink_node, 0);
    }

    let mut match_edges = Vec::new();
    for (truth_index, truth) in ground_truth.iter().enumerate() {
        for (prediction_index, prediction) in predictions.iter().enumerate() {
            if truth.call_reference_id != prediction.call_reference_id
                || truth.kind != prediction.kind
            {
                continue;
            }
            let distance = interval_distance(
                truth.event_tick,
                prediction.lower_tick,
                prediction.upper_tick,
            )?;
            if distance <= tolerance_ticks {
                let edge_index = add_residual_edge(
                    &mut graph,
                    truth_base + truth_index,
                    prediction_base + prediction_index,
                    i128::from(distance),
                );
                match_edges.push((truth_index, prediction_index, edge_index, distance));
            }
        }
    }

    // Successive shortest augmenting paths maximize match cardinality first;
    // shortest residual paths minimize total interval distance at that cardinality.
    augment_min_cost_matching(&mut graph, source_node, sink_node)?;

    let mut matched_truth = vec![false; ground_truth.len()];
    let mut matched_prediction = vec![false; predictions.len()];
    let mut by_kind: BTreeMap<ScoredEventKind, MutableEventMetrics> = BTreeMap::new();

    for (truth_index, prediction_index, edge_index, distance) in match_edges {
        let truth_node = truth_base + truth_index;
        if graph[truth_node][edge_index].capacity != 0 {
            continue;
        }
        matched_truth[truth_index] = true;
        matched_prediction[prediction_index] = true;

        let truth = &ground_truth[truth_index];
        let prediction = &predictions[prediction_index];
        let width = prediction
            .upper_tick
            .checked_sub(prediction.lower_tick)
            .ok_or_else(|| "predicted interval width overflow".to_string())?;

        let metrics = by_kind.entry(truth.kind).or_default();
        metrics.true_positive += 1;
        metrics.total_interval_distance = metrics
            .total_interval_distance
            .checked_add(u128::from(distance))
            .ok_or_else(|| "interval distance total overflow".to_string())?;
        metrics.total_interval_width = metrics
            .total_interval_width
            .checked_add(u128::try_from(width).map_err(|_| "negative interval width".to_string())?)
            .ok_or_else(|| "interval width total overflow".to_string())?;
    }

    for (index, truth) in ground_truth.iter().enumerate() {
        if !matched_truth[index] {
            by_kind.entry(truth.kind).or_default().false_negative += 1;
        }
    }
    for (index, prediction) in predictions.iter().enumerate() {
        if !matched_prediction[index] {
            by_kind.entry(prediction.kind).or_default().false_positive += 1;
        }
    }

    let per_kind: BTreeMap<ScoredEventKind, EventMetrics> = by_kind
        .into_iter()
        .map(|(kind, metrics)| (kind, metrics.finalize()))
        .collect();

    let mut overall = MutableEventMetrics::default();
    for metrics in per_kind.values() {
        overall.true_positive += metrics.true_positive;
        overall.false_positive += metrics.false_positive;
        overall.false_negative += metrics.false_negative;
        overall.total_interval_distance = overall
            .total_interval_distance
            .checked_add(metrics.total_interval_distance_ticks)
            .ok_or_else(|| "overall interval distance total overflow".to_string())?;
        overall.total_interval_width = overall
            .total_interval_width
            .checked_add(metrics.total_matched_interval_width_ticks)
            .ok_or_else(|| "overall interval width total overflow".to_string())?;
    }

    Ok(EventScorecard {
        by_kind: per_kind,
        overall: overall.finalize(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallReport {
    pub call_reference_id: String,
    pub mmsi: String,
    pub source_id: String,
    pub receiver_id: String,
    pub geofence_digest: String,
    pub spatial_reference_id: String,
    pub profile: DwellProfile,
    pub profile_version: String,
    pub initial_state: ServiceState,
    pub initial_state_evidence_id: Option<String>,
    pub final_state: ServiceState,
    pub accepted_fix_count: usize,
    pub duplicate_fix_count: usize,
    pub invalid_fix_count: usize,
    pub nav_status_conflict_count: usize,
    pub kinematic_conflict_count: usize,
    pub observability_gap_count: usize,
    pub events: Vec<InferredEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Candidate {
    zone: Zone,
    first_tick: i64,
    last_tick: i64,
    supporting_fix_count: usize,
}

fn valid_mmsi(mmsi: &str) -> bool {
    mmsi.len() == 9 && mmsi.bytes().all(|byte| byte.is_ascii_digit())
}

fn nav_status_conflicts_with_zone(status: Option<&str>, zone: Zone) -> bool {
    let Some(status) = status else {
        return false;
    };

    let normalized = status.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "moored" | "moored using engine" => zone != Zone::Berth,
        "at anchor" | "anchored" => zone != Zone::Anchorage,
        _ => false,
    }
}

fn transition_kind(previous: ServiceState, next: ServiceState, zone: Zone) -> EventKind {
    match (previous, next) {
        (ServiceState::Berth, ServiceState::Departure) => EventKind::BerthExitDetected,
        (ServiceState::Departure, ServiceState::Outside) => EventKind::PortExitDetected,
        (ServiceState::Indeterminate, _) => EventKind::ZoneReacquired,
        (_, ServiceState::Anchorage) => EventKind::AnchorageZoneEntry,
        (_, ServiceState::Approach) => EventKind::ApproachZoneEntry,
        (_, ServiceState::Berth) => EventKind::BerthZoneEntry,
        (_, ServiceState::Outside) if zone == Zone::Outside => EventKind::PortExitDetected,
        _ => EventKind::ZoneReacquired,
    }
}

/// Evaluate one source-local vessel-call trajectory.
///
/// Zone labels and their geometry provenance are upstream inputs. This crate
/// does not calculate polygons, infer DCSA actual timestamps, or authorize action.
pub fn evaluate_call(
    expected_mmsi: &str,
    fixes: &[AisFix],
    profile: &DwellProfile,
) -> Result<CallReport, String> {
    evaluate_call_with_initial_state(
        expected_mmsi,
        ServiceState::Indeterminate,
        None,
        fixes,
        profile,
    )
}

pub fn evaluate_call_with_initial_state(
    expected_mmsi: &str,
    initial_state: ServiceState,
    initial_state_evidence_id: Option<&str>,
    fixes: &[AisFix],
    profile: &DwellProfile,
) -> Result<CallReport, String> {
    profile.validate()?;
    if initial_state != ServiceState::Indeterminate
        && initial_state_evidence_id.is_none_or(|evidence_id| evidence_id.trim().is_empty())
    {
        return Err("a non-indeterminate initial state requires an evidence reference".to_string());
    }
    if !valid_mmsi(expected_mmsi) {
        return Err(format!(
            "invalid MMSI {expected_mmsi:?}: expected nine digits"
        ));
    }
    if fixes.is_empty() {
        return Err("trajectory contains no AIS fixes".to_string());
    }

    let first = &fixes[0];
    if first.mmsi != expected_mmsi {
        return Err("first fix MMSI does not match expected trajectory identity".to_string());
    }
    if first.call_reference_id.trim().is_empty() {
        return Err("call reference ID must be non-empty".to_string());
    }
    if first.source_id.trim().is_empty() {
        return Err("source ID must be non-empty".to_string());
    }
    if first.receiver_id.trim().is_empty() {
        return Err("receiver ID must be non-empty".to_string());
    }
    if first.spatial_reference_id.trim().is_empty() {
        return Err("spatial reference ID must be non-empty".to_string());
    }

    let call_reference_id = first.call_reference_id.clone();
    let source_id = first.source_id.clone();
    let receiver_id = first.receiver_id.clone();
    let spatial_reference_id = first.spatial_reference_id.clone();
    if first.geofence_digest.trim().is_empty() {
        return Err("geofence digest must be non-empty".to_string());
    }
    let geofence_digest = first.geofence_digest.clone();
    let mut seen_messages: BTreeMap<String, AisFix> = BTreeMap::new();
    let mut state = initial_state;
    let mut candidate: Option<Candidate> = None;
    let mut last_tick: Option<i64> = None;
    let mut last_received_tick: Option<i64> = None;
    let mut accepted_fix_count = 0;
    let mut duplicate_fix_count = 0;
    let mut invalid_fix_count = 0;
    let mut nav_status_conflict_count = 0;
    let mut kinematic_conflict_count = 0;
    let mut observability_gap_count = 0;
    let mut events = Vec::new();

    for fix in fixes {
        if fix.call_reference_id != call_reference_id {
            return Err(format!(
                "call reference ID changed from {call_reference_id:?} to {:?}",
                fix.call_reference_id
            ));
        }
        if fix.mmsi != expected_mmsi {
            return Err(format!(
                "trajectory identity changed from {expected_mmsi:?} to {:?}",
                fix.mmsi
            ));
        }
        if fix.source_id != source_id {
            return Err(format!(
                "source identity changed from {source_id:?} to {:?}",
                fix.source_id
            ));
        }
        if fix.geofence_digest != geofence_digest {
            return Err(format!(
                "geofence identity changed from {geofence_digest:?} to {:?}",
                fix.geofence_digest
            ));
        }
        if fix.receiver_id != receiver_id {
            return Err(format!(
                "receiver identity changed from {receiver_id:?} to {:?}",
                fix.receiver_id
            ));
        }
        if fix.spatial_reference_id != spatial_reference_id {
            return Err(format!(
                "spatial reference changed from {spatial_reference_id:?} to {:?}",
                fix.spatial_reference_id
            ));
        }
        if fix.message_id.trim().is_empty() {
            return Err("message ID must be non-empty".to_string());
        }

        match seen_messages.get(&fix.message_id) {
            Some(previous) if previous == fix => {
                duplicate_fix_count += 1;
                continue;
            }
            Some(_) => {
                return Err(format!(
                    "message ID {:?} was reused with conflicting content",
                    fix.message_id
                ));
            }
            None => {
                seen_messages.insert(fix.message_id.clone(), fix.clone());
            }
        }

        let event_gap_ticks = if let Some(previous_tick) = last_tick {
            if fix.tick < previous_tick {
                return Err(format!(
                    "out-of-order AIS event tick {} follows {}",
                    fix.tick, previous_tick
                ));
            }
            Some(
                fix.tick
                    .checked_sub(previous_tick)
                    .ok_or_else(|| "AIS event-tick difference overflow".to_string())?,
            )
        } else {
            None
        };

        let receive_gap_ticks = if let Some(previous_received_tick) = last_received_tick {
            if fix.received_tick < previous_received_tick {
                return Err(format!(
                    "out-of-order receiver tick {} follows {}",
                    fix.received_tick, previous_received_tick
                ));
            }
            Some(
                fix.received_tick
                    .checked_sub(previous_received_tick)
                    .ok_or_else(|| "AIS receiver-tick difference overflow".to_string())?,
            )
        } else {
            None
        };

        let event_gap_exceeded =
            event_gap_ticks.is_some_and(|gap| gap > profile.max_observation_gap_ticks);
        let receive_gap_exceeded =
            receive_gap_ticks.is_some_and(|gap| gap > profile.max_receive_gap_ticks);
        if event_gap_exceeded || receive_gap_exceeded {
            observability_gap_count += 1;
            candidate = None;
            state = ServiceState::Indeterminate;
            events.push(InferredEvent {
                kind: EventKind::ObservabilityGap,
                lower_tick: last_tick.unwrap_or(fix.tick),
                upper_tick: fix.tick,
                confirmation_tick: fix.tick,
                confirmation_received_tick: fix.received_tick,
                event_gap_ticks,
                receive_gap_ticks,
                supporting_fix_count: 0,
                profile_version: profile.version.clone(),
            });
        }

        last_tick = Some(fix.tick);
        last_received_tick = Some(fix.received_tick);
        accepted_fix_count += 1;

        if !fix.position_valid {
            invalid_fix_count += 1;
            candidate = None;
            state = ServiceState::Indeterminate;
            events.push(InferredEvent {
                kind: EventKind::InvalidPosition,
                lower_tick: fix.tick,
                upper_tick: fix.tick,
                confirmation_tick: fix.tick,
                confirmation_received_tick: fix.received_tick,
                event_gap_ticks: None,
                receive_gap_ticks: None,
                supporting_fix_count: 0,
                profile_version: profile.version.clone(),
            });
            continue;
        }

        if nav_status_conflicts_with_zone(fix.nav_status_raw.as_deref(), fix.zone) {
            nav_status_conflict_count += 1;
        }

        let speed_is_plausible = fix.speed_milli_knots <= fix.zone.max_speed_milli_knots(profile);

        if state.is_stable_for(fix.zone) {
            candidate = None;
            if speed_is_plausible {
                continue;
            }

            kinematic_conflict_count += 1;
            state = ServiceState::Indeterminate;
            events.push(InferredEvent {
                kind: EventKind::SpeedZoneConflict,
                lower_tick: fix.tick,
                upper_tick: fix.tick,
                confirmation_tick: fix.tick,
                confirmation_received_tick: fix.received_tick,
                event_gap_ticks: None,
                receive_gap_ticks: None,
                supporting_fix_count: 1,
                profile_version: profile.version.clone(),
            });
            continue;
        }

        if !speed_is_plausible {
            kinematic_conflict_count += 1;
            candidate = None;
            events.push(InferredEvent {
                kind: EventKind::SpeedZoneConflict,
                lower_tick: fix.tick,
                upper_tick: fix.tick,
                confirmation_tick: fix.tick,
                confirmation_received_tick: fix.received_tick,
                event_gap_ticks: None,
                receive_gap_ticks: None,
                supporting_fix_count: 1,
                profile_version: profile.version.clone(),
            });
            continue;
        }

        let candidate_gap = match candidate.as_ref() {
            Some(current) => Some(
                fix.tick
                    .checked_sub(current.last_tick)
                    .ok_or_else(|| "candidate tick difference overflow".to_string())?,
            ),
            None => None,
        };

        match candidate.as_mut() {
            Some(current)
                if current.zone == fix.zone
                    && candidate_gap
                        .is_some_and(|gap| gap <= profile.max_observation_gap_ticks) =>
            {
                current.last_tick = fix.tick;
                current.supporting_fix_count += 1;
            }
            _ => {
                candidate = Some(Candidate {
                    zone: fix.zone,
                    first_tick: fix.tick,
                    last_tick: fix.tick,
                    supporting_fix_count: 1,
                });
            }
        }

        let Some(current) = candidate.as_ref() else {
            continue;
        };
        let required_dwell = profile.required_dwell(state, current.zone);
        let candidate_duration = current
            .last_tick
            .checked_sub(current.first_tick)
            .ok_or_else(|| "candidate dwell duration overflow".to_string())?;
        if candidate_duration < required_dwell || current.supporting_fix_count < 2 {
            continue;
        }

        let next_state = state.transition_target(current.zone);
        let transition = transition_kind(state, next_state, current.zone);
        events.push(InferredEvent {
            kind: transition,
            lower_tick: current.first_tick,
            upper_tick: current.last_tick,
            confirmation_tick: current.last_tick,
            confirmation_received_tick: fix.received_tick,
            event_gap_ticks: None,
            receive_gap_ticks: None,
            supporting_fix_count: current.supporting_fix_count,
            profile_version: profile.version.clone(),
        });
        state = next_state;
        candidate = None;
    }

    Ok(CallReport {
        call_reference_id,
        mmsi: expected_mmsi.to_string(),
        source_id,
        receiver_id,
        geofence_digest,
        spatial_reference_id,
        profile: profile.clone(),
        profile_version: profile.version.clone(),
        initial_state,
        initial_state_evidence_id: initial_state_evidence_id.map(str::to_string),
        final_state: state,
        accepted_fix_count,
        duplicate_fix_count,
        invalid_fix_count,
        nav_status_conflict_count,
        kinematic_conflict_count,
        observability_gap_count,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(tick: i64, message_id: &str, zone: Zone, speed_milli_knots: u32) -> AisFix {
        AisFix {
            call_reference_id: "SYNTH_CALL_001".to_string(),
            mmsi: "563123456".to_string(),
            message_id: message_id.to_string(),
            source_id: "AIS_FIXTURE_SOURCE_A".to_string(),
            receiver_id: "RX_DURBAN_TEST_A".to_string(),
            geofence_digest: "sha256:fixture-geofence-v1".to_string(),
            spatial_reference_id: "EPSG:4326".to_string(),
            tick,
            received_tick: 100 + tick,
            zone,
            speed_milli_knots,
            position_valid: true,
            nav_status_raw: None,
        }
    }

    fn default_profile() -> DwellProfile {
        DwellProfile::default()
    }

    fn evaluate_from_baselined_outside(fixes: &[AisFix], profile: &DwellProfile) -> CallReport {
        evaluate_call_with_initial_state(
            "563123456",
            ServiceState::Outside,
            Some("SYNTHETIC_OUTSIDE_BASELINE_V0_1"),
            fixes,
            profile,
        )
        .expect("valid call with evidence-backed initial state")
    }

    #[test]
    fn event_scorecard_matches_truth_inside_the_inferred_interval() {
        let fixes = [
            fix(0, "m0", Zone::Approach, 5_000),
            fix(1, "m1", Zone::Berth, 100),
            fix(2, "m2", Zone::Berth, 100),
            fix(3, "m3", Zone::Berth, 100),
            fix(4, "m4", Zone::Berth, 100),
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());
        let truth = [GroundTruthEvent {
            call_reference_id: "SYNTH_CALL_001".to_string(),
            kind: ScoredEventKind::BerthEntry,
            event_tick: 2,
        }];
        let predictions = predictions_from_report(&report);
        let score = score_events(&truth, &predictions, 0).expect("scorecard");

        assert_eq!(score.overall.true_positive, 1);
        assert_eq!(score.overall.false_positive, 0);
        assert_eq!(score.overall.false_negative, 0);
        assert_eq!(score.overall.precision, Some(1.0));
        assert_eq!(score.overall.recall, Some(1.0));
        assert_eq!(score.overall.mean_interval_distance_ticks, Some(0.0));
        assert_eq!(score.overall.total_matched_interval_width_ticks, 3);
    }

    #[test]
    fn event_scorecard_penalizes_duplicate_predictions_one_to_one() {
        let truth = [GroundTruthEvent {
            call_reference_id: "CALL_A".to_string(),
            kind: ScoredEventKind::BerthEntry,
            event_tick: 10,
        }];
        let prediction = ScoredPrediction {
            call_reference_id: "CALL_A".to_string(),
            kind: ScoredEventKind::BerthEntry,
            lower_tick: 9,
            upper_tick: 11,
        };
        let score = score_events(&truth, &[prediction.clone(), prediction], 0).expect("scorecard");

        assert_eq!(score.overall.true_positive, 1);
        assert_eq!(score.overall.false_positive, 1);
        assert_eq!(score.overall.false_negative, 0);
        assert_eq!(score.overall.precision, Some(0.5));
        assert_eq!(score.overall.recall, Some(1.0));
    }

    #[test]
    fn event_scorecard_separates_missed_events_from_false_transitions() {
        let truth = [GroundTruthEvent {
            call_reference_id: "CALL_A".to_string(),
            kind: ScoredEventKind::BerthEntry,
            event_tick: 10,
        }];
        let prediction = ScoredPrediction {
            call_reference_id: "CALL_B".to_string(),
            kind: ScoredEventKind::PortExit,
            lower_tick: 10,
            upper_tick: 10,
        };
        let score = score_events(&truth, &[prediction], 0).expect("scorecard");

        let berth = score
            .by_kind
            .get(&ScoredEventKind::BerthEntry)
            .expect("berth metric");
        let exit = score
            .by_kind
            .get(&ScoredEventKind::PortExit)
            .expect("exit metric");
        assert_eq!(berth.false_negative, 1);
        assert_eq!(berth.recall, Some(0.0));
        assert_eq!(exit.false_positive, 1);
        assert_eq!(exit.precision, Some(0.0));
    }

    #[test]
    fn event_scorecard_enforces_call_and_event_identity_and_tolerance() {
        let truth = [GroundTruthEvent {
            call_reference_id: "CALL_A".to_string(),
            kind: ScoredEventKind::BerthEntry,
            event_tick: 10,
        }];
        let too_late = ScoredPrediction {
            call_reference_id: "CALL_A".to_string(),
            kind: ScoredEventKind::BerthEntry,
            lower_tick: 12,
            upper_tick: 13,
        };
        let score = score_events(&truth, &[too_late], 1).expect("scorecard");
        assert_eq!(score.overall.true_positive, 0);
        assert_eq!(score.overall.false_positive, 1);
        assert_eq!(score.overall.false_negative, 1);
    }

    #[test]
    fn event_scorecard_reports_distance_to_interval_and_tolerance() {
        let truth = [GroundTruthEvent {
            call_reference_id: "CALL_A".to_string(),
            kind: ScoredEventKind::BerthEntry,
            event_tick: 12,
        }];
        let prediction = ScoredPrediction {
            call_reference_id: "CALL_A".to_string(),
            kind: ScoredEventKind::BerthEntry,
            lower_tick: 9,
            upper_tick: 10,
        };
        let score = score_events(&truth, &[prediction], 2).expect("within tolerance");

        assert_eq!(score.overall.true_positive, 1);
        assert_eq!(score.overall.mean_interval_distance_ticks, Some(2.0));
        assert_eq!(score.overall.mean_matched_interval_width_ticks, Some(1.0));
    }

    #[test]
    fn event_scorecard_maximizes_matches_before_minimizing_distance() {
        let truth = [
            GroundTruthEvent {
                call_reference_id: "CALL_A".to_string(),
                kind: ScoredEventKind::BerthEntry,
                event_tick: 10,
            },
            GroundTruthEvent {
                call_reference_id: "CALL_A".to_string(),
                kind: ScoredEventKind::BerthEntry,
                event_tick: 9,
            },
        ];
        let predictions = [
            ScoredPrediction {
                call_reference_id: "CALL_A".to_string(),
                kind: ScoredEventKind::BerthEntry,
                lower_tick: 10,
                upper_tick: 10,
            },
            ScoredPrediction {
                call_reference_id: "CALL_A".to_string(),
                kind: ScoredEventKind::BerthEntry,
                lower_tick: 11,
                upper_tick: 11,
            },
        ];

        // The first prediction can match either truth, while the second can
        // match only the first. Maximum-cardinality matching must find both.
        let score = score_events(&truth, &predictions, 1).expect("scorecard");
        assert_eq!(score.overall.true_positive, 2);
        assert_eq!(score.overall.false_positive, 0);
        assert_eq!(score.overall.false_negative, 0);
        assert_eq!(score.overall.mean_interval_distance_ticks, Some(1.0));
    }

    #[test]
    fn event_scorecard_rejects_invalid_prediction_interval() {
        let prediction = ScoredPrediction {
            call_reference_id: "CALL_A".to_string(),
            kind: ScoredEventKind::BerthEntry,
            lower_tick: 12,
            upper_tick: 11,
        };
        assert!(
            score_events(&[], &[prediction], 0)
                .unwrap_err()
                .contains("lower bound exceeds upper bound")
        );
    }

    #[test]
    fn call_reference_identity_cannot_change_within_trajectory() {
        let first = fix(0, "m0", Zone::Outside, 0);
        let mut second = fix(1, "m1", Zone::Outside, 0);
        second.call_reference_id = "SYNTH_CALL_002".to_string();
        let error = evaluate_call_with_initial_state(
            "563123456",
            ServiceState::Outside,
            Some("SYNTHETIC_OUTSIDE_BASELINE_V0_1"),
            &[first, second],
            &default_profile(),
        )
        .expect_err("call reference change must reject");
        assert!(error.contains("call reference ID changed"));
    }

    #[test]
    fn stable_berth_zone_entry_has_an_interval_and_confirmation_time() {
        let fixes = [
            fix(0, "m0", Zone::Approach, 5_000),
            fix(1, "m1", Zone::Berth, 100),
            fix(2, "m2", Zone::Berth, 100),
            fix(3, "m3", Zone::Berth, 100),
            fix(4, "m4", Zone::Berth, 100),
        ];

        let report = evaluate_from_baselined_outside(&fixes, &default_profile());
        let berth = report
            .events
            .iter()
            .find(|event| event.kind == EventKind::BerthZoneEntry)
            .expect("berth geofence event exists");

        assert_eq!((berth.lower_tick, berth.upper_tick), (1, 4));
        assert_eq!(berth.confirmation_tick, 4);
        assert_eq!(report.final_state, ServiceState::Berth);
    }

    #[test]
    fn brief_transit_does_not_become_a_berth_event() {
        let fixes = [
            fix(0, "m0", Zone::Approach, 5_000),
            fix(1, "m1", Zone::Berth, 100),
            fix(2, "m2", Zone::Approach, 5_000),
            fix(3, "m3", Zone::Outside, 5_000),
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());

        assert!(
            !report
                .events
                .iter()
                .any(|event| event.kind == EventKind::BerthZoneEntry)
        );
    }

    #[test]
    fn a_gap_breaks_candidate_dwell_instead_of_being_bridged() {
        let fixes = [
            fix(0, "m0", Zone::Approach, 5_000),
            fix(1, "m1", Zone::Berth, 100),
            fix(5, "m2", Zone::Berth, 100),
            fix(6, "m3", Zone::Berth, 100),
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());

        assert_eq!(report.observability_gap_count, 1);
        assert!(
            !report
                .events
                .iter()
                .any(|event| event.kind == EventKind::BerthZoneEntry)
        );
        assert_eq!(report.final_state, ServiceState::Indeterminate);
    }

    #[test]
    fn exact_duplicate_replay_is_idempotent() {
        let duplicate = fix(1, "m1", Zone::Berth, 100);
        let fixes = [
            fix(0, "m0", Zone::Approach, 5_000),
            duplicate.clone(),
            duplicate,
            fix(2, "m2", Zone::Berth, 100),
            fix(3, "m3", Zone::Berth, 100),
            fix(4, "m4", Zone::Berth, 100),
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());

        assert_eq!(report.duplicate_fix_count, 1);
        assert_eq!(report.final_state, ServiceState::Berth);
    }

    #[test]
    fn conflicting_message_id_reuse_rejects() {
        let first = fix(1, "reused", Zone::Berth, 100);
        let second = fix(1, "reused", Zone::Approach, 5_000);
        let error = evaluate_call("563123456", &[first, second], &default_profile())
            .expect_err("conflicting replay must reject");

        assert!(error.contains("conflicting content"));
    }

    #[test]
    fn receiver_time_gap_breaks_dwell_even_when_event_ticks_are_contiguous() {
        let fixes = [
            fix(0, "m0", Zone::Outside, 0),
            fix(1, "m1", Zone::Approach, 5_000),
            fix(2, "m2", Zone::Approach, 5_000),
            {
                let mut next = fix(3, "m3", Zone::Approach, 5_000);
                next.received_tick = 120;
                next
            },
            {
                let mut next = fix(4, "m4", Zone::Approach, 5_000);
                next.received_tick = 121;
                next
            },
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());

        assert_eq!(report.observability_gap_count, 1);
        let gap = report
            .events
            .iter()
            .find(|event| event.kind == EventKind::ObservabilityGap)
            .expect("receiver gap recorded");
        assert_eq!(gap.event_gap_ticks, Some(1));
        assert_eq!(gap.receive_gap_ticks, Some(18));
        assert!(
            report
                .events
                .iter()
                .any(|event| event.kind == EventKind::ZoneReacquired)
        );
    }

    #[test]
    fn out_of_order_timestamp_rejects() {
        let fixes = [
            fix(2, "m2", Zone::Berth, 100),
            fix(1, "m1", Zone::Berth, 100),
        ];
        assert!(
            evaluate_call("563123456", &fixes, &default_profile())
                .unwrap_err()
                .contains("out-of-order")
        );
    }

    #[test]
    fn receiver_observation_clock_rollback_rejects() {
        let first = fix(0, "m0", Zone::Outside, 0);
        let mut second = fix(1, "m1", Zone::Outside, 0);
        second.received_tick = 99;

        let error = evaluate_call_with_initial_state(
            "563123456",
            ServiceState::Outside,
            Some("SYNTHETIC_OUTSIDE_BASELINE_V0_1"),
            &[first, second],
            &default_profile(),
        )
        .expect_err("receiver clock rollback must reject");
        assert!(error.contains("out-of-order receiver tick"));
    }

    #[test]
    fn nav_status_is_preserved_as_a_conflict_not_used_as_authority() {
        let mut first = fix(0, "m0", Zone::Outside, 0);
        first.nav_status_raw = Some("moored".to_string());
        let second = fix(1, "m1", Zone::Outside, 0);
        let report = evaluate_from_baselined_outside(&[first, second], &default_profile());

        assert_eq!(report.nav_status_conflict_count, 1);
        assert_ne!(report.final_state, ServiceState::Berth);
    }

    #[test]
    fn departure_is_a_geofence_exit_inference_not_actual_lines_untied() {
        let fixes = [
            fix(0, "m0", Zone::Berth, 100),
            fix(1, "m1", Zone::Berth, 100),
            fix(2, "m2", Zone::Berth, 100),
            fix(3, "m3", Zone::Berth, 100),
            fix(4, "m4", Zone::Approach, 5_000),
            fix(5, "m5", Zone::Approach, 5_000),
            fix(6, "m6", Zone::Approach, 5_000),
            fix(7, "m7", Zone::Outside, 10_000),
            fix(8, "m8", Zone::Outside, 10_000),
            fix(9, "m9", Zone::Outside, 10_000),
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());

        assert!(report.events.iter().any(|event| {
            event.kind == EventKind::BerthExitDetected
                && event.lower_tick == 4
                && event.confirmation_tick == 6
        }));
        assert_eq!(report.final_state, ServiceState::Outside);
        assert!(
            !report
                .events
                .iter()
                .any(|event| { event.kind.label().contains("LINES_UNTIED") })
        );
    }

    #[test]
    fn high_speed_inside_stable_berth_invalidates_the_state_claim() {
        let fixes = [
            fix(0, "m0", Zone::Berth, 100),
            fix(1, "m1", Zone::Berth, 100),
            fix(2, "m2", Zone::Berth, 100),
            fix(3, "m3", Zone::Berth, 100),
            fix(4, "m4", Zone::Berth, 8_000),
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());

        assert_eq!(report.kinematic_conflict_count, 1);
        assert_eq!(report.final_state, ServiceState::Indeterminate);
        assert!(report.events.iter().any(|event| {
            event.kind == EventKind::SpeedZoneConflict && event.confirmation_tick == 4
        }));
    }

    #[test]
    fn geofence_digest_change_within_call_rejects() {
        let first = fix(0, "m0", Zone::Outside, 0);
        let mut second = fix(1, "m1", Zone::Outside, 0);
        second.geofence_digest = "sha256:fixture-geofence-v2".to_string();

        let error = evaluate_call_with_initial_state(
            "563123456",
            ServiceState::Outside,
            Some("SYNTHETIC_OUTSIDE_BASELINE_V0_1"),
            &[first, second],
            &default_profile(),
        )
        .expect_err("mixed geofence versions must reject");
        assert!(error.contains("geofence identity changed"));
    }

    #[test]
    fn receiver_or_spatial_reference_change_within_call_rejects() {
        let first = fix(0, "m0", Zone::Outside, 0);
        let mut changed_receiver = fix(1, "m1", Zone::Outside, 0);
        changed_receiver.receiver_id = "RX_DURBAN_TEST_B".to_string();
        let receiver_error = evaluate_call_with_initial_state(
            "563123456",
            ServiceState::Outside,
            Some("SYNTHETIC_OUTSIDE_BASELINE_V0_1"),
            &[first.clone(), changed_receiver],
            &default_profile(),
        )
        .expect_err("mixed receiver must reject");
        assert!(receiver_error.contains("receiver identity changed"));

        let mut changed_crs = fix(1, "m1", Zone::Outside, 0);
        changed_crs.spatial_reference_id = "EPSG:3857".to_string();
        let spatial_error = evaluate_call_with_initial_state(
            "563123456",
            ServiceState::Outside,
            Some("SYNTHETIC_OUTSIDE_BASELINE_V0_1"),
            &[first, changed_crs],
            &default_profile(),
        )
        .expect_err("mixed spatial reference must reject");
        assert!(spatial_error.contains("spatial reference changed"));
    }

    #[test]
    fn dwell_parameter_sweep_changes_confirmation_not_latent_fixes() {
        let fixes = [
            fix(0, "m0", Zone::Outside, 0),
            fix(1, "m1", Zone::Berth, 100),
            fix(2, "m2", Zone::Berth, 100),
            fix(3, "m3", Zone::Berth, 100),
            fix(4, "m4", Zone::Berth, 100),
        ];
        let mut confirmations = Vec::new();

        for dwell in 1..=4 {
            let mut profile = default_profile();
            profile.berth_dwell_ticks = dwell;
            let report = evaluate_from_baselined_outside(&fixes, &profile);
            let event = report
                .events
                .iter()
                .find(|event| event.kind == EventKind::BerthZoneEntry);
            confirmations.push(event.map(|event| event.confirmation_tick));
        }

        assert_eq!(confirmations, vec![Some(2), Some(3), Some(4), None]);
    }

    #[test]
    fn a_long_gap_while_berthed_invalidates_continuity_then_reacquires() {
        let fixes = [
            fix(0, "m0", Zone::Berth, 100),
            fix(1, "m1", Zone::Berth, 100),
            fix(2, "m2", Zone::Berth, 100),
            fix(3, "m3", Zone::Berth, 100),
            fix(8, "m8", Zone::Berth, 100),
            fix(9, "m9", Zone::Berth, 100),
            fix(10, "m10", Zone::Berth, 100),
            fix(11, "m11", Zone::Berth, 100),
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());

        assert_eq!(report.observability_gap_count, 1);
        assert_eq!(report.final_state, ServiceState::Berth);
        assert!(
            report
                .events
                .iter()
                .any(|event| event.kind == EventKind::ObservabilityGap)
        );
        assert!(
            report
                .events
                .iter()
                .any(|event| event.kind == EventKind::ZoneReacquired)
        );
        assert_eq!(
            report
                .events
                .iter()
                .filter(|event| event.kind == EventKind::BerthZoneEntry)
                .count(),
            1
        );
    }

    #[test]
    fn invalid_position_breaks_continuity_and_sets_indeterminate() {
        let mut invalid = fix(2, "m2", Zone::Berth, 100);
        invalid.position_valid = false;
        let fixes = [
            fix(0, "m0", Zone::Berth, 100),
            fix(1, "m1", Zone::Berth, 100),
            invalid,
            fix(3, "m3", Zone::Berth, 100),
        ];
        let report = evaluate_from_baselined_outside(&fixes, &default_profile());

        assert_eq!(report.invalid_fix_count, 1);
        assert_eq!(report.final_state, ServiceState::Indeterminate);
    }

    #[test]
    fn without_a_qualified_baseline_first_zone_is_reacquisition_not_arrival() {
        let fixes = [
            fix(0, "m0", Zone::Berth, 100),
            fix(1, "m1", Zone::Berth, 100),
            fix(2, "m2", Zone::Berth, 100),
            fix(3, "m3", Zone::Berth, 100),
        ];
        let report = evaluate_call("563123456", &fixes, &default_profile())
            .expect("safe indeterminate initialization");

        assert_eq!(report.initial_state, ServiceState::Indeterminate);
        assert!(report.initial_state_evidence_id.is_none());
        assert_eq!(report.final_state, ServiceState::Berth);
        assert!(
            report
                .events
                .iter()
                .any(|event| event.kind == EventKind::ZoneReacquired)
        );
        assert!(
            !report
                .events
                .iter()
                .any(|event| event.kind == EventKind::BerthZoneEntry)
        );
    }

    #[test]
    fn non_indeterminate_initial_state_requires_an_evidence_reference() {
        let fixes = [fix(0, "m0", Zone::Outside, 0)];
        let error = evaluate_call_with_initial_state(
            "563123456",
            ServiceState::Outside,
            None,
            &fixes,
            &default_profile(),
        )
        .expect_err("unreferenced initial state must reject");
        assert!(error.contains("requires an evidence reference"));
    }

    #[test]
    fn mmsi_and_profile_fail_closed() {
        assert!(
            evaluate_call("bad", &[fix(0, "m0", Zone::Outside, 0)], &default_profile()).is_err()
        );

        let mut bad_profile = default_profile();
        bad_profile.berth_dwell_ticks = 0;
        assert!(
            evaluate_call("563123456", &[fix(0, "m0", Zone::Outside, 0)], &bad_profile).is_err()
        );
    }
}
