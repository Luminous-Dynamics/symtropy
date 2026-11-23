#[cfg(test)]
use std::collections::VecDeque;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeRole {
    BusinessEventTime,
    SensorObservationTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeEnclosure {
    pub lower: i64,
    pub upper: i64,
    pub basis_id: String,
    pub role: TimeRole,
    pub qualified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalObservation {
    /// Unique only within its source namespace. Never used as a cross-source join key.
    pub record_key: String,
    /// Revision token supplied by the source's revision owner.
    pub revision_id: String,
    /// SHA-256 digest supplied by the upstream byte verifier; this crate only preserves it.
    pub payload_sha256: String,
    /// Canonical physical identity supplied by the lineage owner.
    pub canonical_box_id: String,
    /// Versioned, normalized physical event kind supplied by the source-semantic owner.
    pub event_kind: String,
    pub facility_id: String,
    pub operation_ref: String,
    pub event_time: TimeEnclosure,
    /// Publication time in the profile's normalized time basis.
    pub published_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrelationProfile {
    pub profile_id: String,
    pub version: String,
    /// SHA-256-shaped digest of the frozen correlation profile; this crate does not hash it.
    pub profile_sha256: String,
    pub source_channel_id: String,
    pub witness_channel_id: String,
    pub time_basis_id: String,
    pub source_time_role: TimeRole,
    pub witness_time_role: TimeRole,
    pub max_interval_gap: i64,
    pub as_of: i64,
    /// These booleans are upstream qualifications from the capture owner.
    /// This crate does not authenticate the channel dependency graph.
    pub source_channel_qualified: bool,
    pub witness_channel_qualified: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    UniquelyAssociated,
    Ambiguous,
    Unmatched,
    TimeOrderUnresolved,
    NotEvaluable,
    Unknown,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UniquelyAssociated => "UNIQUELY_ASSOCIATED",
            Self::Ambiguous => "AMBIGUOUS",
            Self::Unmatched => "UNMATCHED",
            Self::TimeOrderUnresolved => "TIME_ORDER_UNRESOLVED",
            Self::NotEvaluable => "NOT_EVALUABLE",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociationReceipt {
    pub profile_id: String,
    pub profile_version: String,
    pub profile_sha256: String,
    pub as_of: i64,
    pub source_channel_id: String,
    pub source_record_key: String,
    pub source_revision_id: String,
    pub source_payload_sha256: String,
    pub witness_channel_id: String,
    pub witness_record_key: String,
    pub witness_revision_id: String,
    pub witness_payload_sha256: String,
    pub canonical_box_id: String,
    pub event_kind: String,
    pub facility_id: String,
    pub operation_ref: String,
    pub source_event_time: TimeEnclosure,
    pub witness_event_time: TimeEnclosure,
    pub source_time_role: TimeRole,
    pub witness_time_role: TimeRole,
    pub interval_gap: i128,
    pub max_interval_gap: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociationReport {
    pub disposition: Disposition,
    pub source_visible_count: usize,
    pub witness_visible_count: usize,
    pub candidate_edge_count: usize,
    /// Number of time-compatibility checks after exact physical-context blocking.
    pub candidate_pair_evaluations: usize,
    pub receipts: Vec<AssociationReceipt>,
    /// All visible keys remain unresolved unless a unique complete matching exists.
    pub unresolved_source_keys: Vec<String>,
    pub unresolved_witness_keys: Vec<String>,
}

impl AssociationReport {
    fn new(
        disposition: Disposition,
        source_keys: Vec<String>,
        witness_keys: Vec<String>,
        candidate_edge_count: usize,
    ) -> Self {
        Self {
            disposition,
            source_visible_count: source_keys.len(),
            witness_visible_count: witness_keys.len(),
            candidate_edge_count,
            candidate_pair_evaluations: 0,
            receipts: Vec::new(),
            unresolved_source_keys: source_keys,
            unresolved_witness_keys: witness_keys,
        }
    }

    fn with_candidate_pair_evaluations(mut self, evaluations: usize) -> Self {
        self.candidate_pair_evaluations = evaluations;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociationCase {
    pub profile: Option<CorrelationProfile>,
    pub source_events: Vec<PhysicalObservation>,
    pub witness_events: Vec<PhysicalObservation>,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_profile(profile: &CorrelationProfile) -> bool {
    !profile.profile_id.trim().is_empty()
        && !profile.version.trim().is_empty()
        && is_sha256_hex(&profile.profile_sha256)
        && !profile.source_channel_id.trim().is_empty()
        && !profile.witness_channel_id.trim().is_empty()
        && profile.source_channel_id != profile.witness_channel_id
        && !profile.time_basis_id.trim().is_empty()
        && profile.source_time_role == TimeRole::BusinessEventTime
        && profile.witness_time_role == TimeRole::SensorObservationTime
        && profile.max_interval_gap >= 0
        && profile.source_channel_qualified
        && profile.witness_channel_qualified
}

fn visible_observations(
    observations: &[PhysicalObservation],
    profile: &CorrelationProfile,
    expected_time_role: TimeRole,
) -> Result<Vec<PhysicalObservation>, ()> {
    let mut visible: Vec<PhysicalObservation> = observations
        .iter()
        .filter(|observation| observation.published_at <= profile.as_of)
        .cloned()
        .collect();

    visible.sort_by(|left, right| left.record_key.cmp(&right.record_key));

    let mut keys = BTreeSet::new();
    for observation in &visible {
        if observation.record_key.trim().is_empty()
            || observation.revision_id.trim().is_empty()
            || !is_sha256_hex(&observation.payload_sha256)
            || !keys.insert(observation.record_key.clone())
            || observation.canonical_box_id.trim().is_empty()
            || observation.event_kind.trim().is_empty()
            || observation.facility_id.trim().is_empty()
            || observation.operation_ref.trim().is_empty()
            || observation.event_time.lower > observation.event_time.upper
            || !observation.event_time.qualified
            || observation.event_time.role != expected_time_role
            || observation.event_time.basis_id != profile.time_basis_id
            || observation.published_at < observation.event_time.lower
        {
            return Err(());
        }
    }

    Ok(visible)
}

fn interval_gap(left: &TimeEnclosure, right: &TimeEnclosure) -> i128 {
    if left.upper < right.lower {
        i128::from(right.lower) - i128::from(left.upper)
    } else if right.upper < left.lower {
        i128::from(left.lower) - i128::from(right.upper)
    } else {
        0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TemporalCompatibility {
    Proven,
    Unresolved,
    Rejected,
}

fn temporal_compatibility(
    source: &PhysicalObservation,
    witness: &PhysicalObservation,
) -> TemporalCompatibility {
    match (source.event_time.role, witness.event_time.role) {
        (TimeRole::BusinessEventTime, TimeRole::SensorObservationTime) => {
            if witness.event_time.upper <= source.event_time.lower {
                TemporalCompatibility::Proven
            } else if source.event_time.upper < witness.event_time.lower {
                TemporalCompatibility::Rejected
            } else {
                TemporalCompatibility::Unresolved
            }
        }
        _ => TemporalCompatibility::Proven,
    }
}

fn admissible_edge(
    source: &PhysicalObservation,
    witness: &PhysicalObservation,
    profile: &CorrelationProfile,
) -> bool {
    source.canonical_box_id == witness.canonical_box_id
        && source.event_kind == witness.event_kind
        && source.facility_id == witness.facility_id
        && source.operation_ref == witness.operation_ref
        && temporal_compatibility(source, witness) != TemporalCompatibility::Rejected
        && interval_gap(&source.event_time, &witness.event_time)
            <= i128::from(profile.max_interval_gap)
}

type EventContext<'a> = (&'a str, &'a str, &'a str, &'a str);

fn event_context(observation: &PhysicalObservation) -> EventContext<'_> {
    (
        observation.canonical_box_id.as_str(),
        observation.event_kind.as_str(),
        observation.facility_id.as_str(),
        observation.operation_ref.as_str(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CandidateGraph {
    edges: Vec<Vec<usize>>,
    pair_evaluations: usize,
}

#[cfg(test)]
struct IndexedCandidateGraph {
    edges: Vec<Vec<usize>>,
    pair_evaluations: usize,
    index_nodes_visited: usize,
}

fn candidate_graph(
    source: &[PhysicalObservation],
    witness: &[PhysicalObservation],
    profile: &CorrelationProfile,
) -> CandidateGraph {
    // Exact, already-qualified physical context is a necessary condition for
    // every edge. Block witness events by that tuple before evaluating time.
    // This avoids a global source_count × witness_count scan across unrelated
    // containers/event kinds/sites/operations without dropping any admissible edge.
    let mut witness_by_context: BTreeMap<EventContext<'_>, Vec<usize>> = BTreeMap::new();
    for (index, witness_event) in witness.iter().enumerate() {
        witness_by_context
            .entry(event_context(witness_event))
            .or_default()
            .push(index);
    }

    let mut pair_evaluations = 0usize;
    let mut edges = Vec::with_capacity(source.len());
    for source_event in source {
        let candidates = witness_by_context
            .get(&event_context(source_event))
            .into_iter()
            .flatten();

        let mut source_edges = Vec::new();
        for &index in candidates {
            pair_evaluations += 1;
            if admissible_edge(source_event, &witness[index], profile) {
                source_edges.push(index);
            }
        }
        edges.push(source_edges);
    }

    CandidateGraph {
        edges,
        pair_evaluations,
    }
}

#[cfg(test)]
#[derive(Default)]
struct IntervalContextIndex {
    /// Sorted as (lower endpoint, upper endpoint, original witness index).
    by_lower: Vec<(i64, i64, usize)>,
    /// Segment-tree maximum upper endpoint for each lower-sorted range.
    max_upper: Vec<i64>,
}

#[cfg(test)]
impl IntervalContextIndex {
    fn build_max_upper(&mut self) {
        self.by_lower.sort_unstable();
        if self.by_lower.is_empty() {
            self.max_upper.clear();
            return;
        }
        self.max_upper = vec![i64::MIN; self.by_lower.len().saturating_mul(4).max(1)];
        Self::build_node(
            &self.by_lower,
            &mut self.max_upper,
            1,
            0,
            self.by_lower.len(),
        );
    }

    fn build_node(
        entries: &[(i64, i64, usize)],
        tree: &mut [i64],
        node: usize,
        start: usize,
        end: usize,
    ) -> i64 {
        if end - start == 1 {
            tree[node] = entries[start].1;
            return tree[node];
        }
        let middle = start + (end - start) / 2;
        let left_max = Self::build_node(entries, tree, node * 2, start, middle);
        let right_max = Self::build_node(entries, tree, node * 2 + 1, middle, end);
        tree[node] = left_max.max(right_max);
        tree[node]
    }

    fn query(&self, lower_limit: usize, minimum_upper: i128) -> (Vec<usize>, usize) {
        if lower_limit == 0 || self.by_lower.is_empty() {
            return (Vec::new(), 0);
        }
        let mut query = IntervalQuery {
            index: self,
            lower_limit,
            minimum_upper,
            candidates: Vec::new(),
            nodes_visited: 0,
        };
        query.visit(1, 0, self.by_lower.len());
        (query.candidates, query.nodes_visited)
    }
}

#[cfg(test)]
struct IntervalQuery<'a> {
    index: &'a IntervalContextIndex,
    lower_limit: usize,
    minimum_upper: i128,
    candidates: Vec<usize>,
    nodes_visited: usize,
}

#[cfg(test)]
impl IntervalQuery<'_> {
    fn visit(&mut self, node: usize, start: usize, end: usize) {
        self.nodes_visited += 1;
        if start >= self.lower_limit || i128::from(self.index.max_upper[node]) < self.minimum_upper
        {
            return;
        }
        if end - start == 1 {
            self.candidates.push(self.index.by_lower[start].2);
            return;
        }
        let middle = start + (end - start) / 2;
        self.visit(node * 2, start, middle);
        self.visit(node * 2 + 1, middle, end);
    }
}

/// Test-only candidate-window implementation. Exact event context is still
/// required; a lower-endpoint prefix and a segment-tree maximum-upper bound
/// report exactly those witness intervals satisfying:
/// witness.lower <= source.upper AND witness.upper >= source.lower - tolerance.
/// This is equivalent to the frozen v0.1 temporal predicate for the typed
/// BusinessEventTime / SensorObservationTime role pair.
#[cfg(test)]
fn candidate_graph_interval_indexed(
    source: &[PhysicalObservation],
    witness: &[PhysicalObservation],
    profile: &CorrelationProfile,
) -> IndexedCandidateGraph {
    // This pruning bound is deliberately specialized to the validated v0.1
    // BusinessEventTime source / SensorObservationTime witness profile. The
    // role contract rejects a sensor interval that lies wholly after the
    // business interval; changing the supported role pair requires deriving
    // and re-proving the candidate-window bounds.
    debug_assert_eq!(profile.source_time_role, TimeRole::BusinessEventTime);
    debug_assert_eq!(profile.witness_time_role, TimeRole::SensorObservationTime);
    debug_assert!(
        source
            .iter()
            .all(|event| event.event_time.role == TimeRole::BusinessEventTime)
    );
    debug_assert!(
        witness
            .iter()
            .all(|event| event.event_time.role == TimeRole::SensorObservationTime)
    );

    let mut indexes = BTreeMap::<EventContext<'_>, IntervalContextIndex>::new();
    for (index, event) in witness.iter().enumerate() {
        indexes
            .entry(event_context(event))
            .or_default()
            .by_lower
            .push((event.event_time.lower, event.event_time.upper, index));
    }
    for context in indexes.values_mut() {
        context.build_max_upper();
    }

    let mut pair_evaluations = 0usize;
    let mut index_nodes_visited = 0usize;
    let mut edges = Vec::with_capacity(source.len());
    for source_event in source {
        let Some(context) = indexes.get(&event_context(source_event)) else {
            edges.push(Vec::new());
            continue;
        };

        let lower_limit = context
            .by_lower
            .partition_point(|(lower, _, _)| *lower <= source_event.event_time.upper);
        let minimum_upper =
            i128::from(source_event.event_time.lower) - i128::from(profile.max_interval_gap);
        let (mut possible_witnesses, nodes_visited) = context.query(lower_limit, minimum_upper);
        index_nodes_visited += nodes_visited;
        possible_witnesses.sort_unstable();

        let mut source_edges = Vec::new();
        for index in possible_witnesses {
            pair_evaluations += 1;
            if admissible_edge(source_event, &witness[index], profile) {
                source_edges.push(index);
            }
        }
        edges.push(source_edges);
    }

    IndexedCandidateGraph {
        edges,
        pair_evaluations,
        index_nodes_visited,
    }
}

fn augment_matching(
    left: usize,
    graph: &[Vec<usize>],
    seen_right: &mut [bool],
    right_to_left: &mut [Option<usize>],
) -> bool {
    for &right in &graph[left] {
        if seen_right[right] {
            continue;
        }
        seen_right[right] = true;

        let can_assign = match right_to_left[right] {
            None => true,
            Some(previous_left) => {
                augment_matching(previous_left, graph, seen_right, right_to_left)
            }
        };

        if can_assign {
            right_to_left[right] = Some(left);
            return true;
        }
    }

    false
}

#[cfg(test)]
fn hopcroft_karp_matching(graph: &[Vec<usize>], right_count: usize) -> (usize, Vec<Option<usize>>) {
    const UNREACHED: usize = usize::MAX;

    let left_count = graph.len();
    let mut left_to_right = vec![None; left_count];
    let mut right_to_left = vec![None; right_count];
    let mut matched_count = 0usize;

    loop {
        let mut distance = vec![UNREACHED; left_count];
        let mut queue = VecDeque::new();
        for left in 0..left_count {
            if left_to_right[left].is_none() {
                distance[left] = 0;
                queue.push_back(left);
            }
        }

        let mut shortest_augmenting = UNREACHED;
        while let Some(left) = queue.pop_front() {
            if distance[left] >= shortest_augmenting {
                continue;
            }
            for &right in &graph[left] {
                if let Some(next_left) = right_to_left[right] {
                    if distance[next_left] == UNREACHED {
                        distance[next_left] = distance[left] + 1;
                        queue.push_back(next_left);
                    }
                } else {
                    shortest_augmenting = shortest_augmenting.min(distance[left] + 1);
                }
            }
        }
        if shortest_augmenting == UNREACHED {
            break;
        }

        fn augment_layered(
            left: usize,
            graph: &[Vec<usize>],
            distance: &mut [usize],
            shortest_augmenting: usize,
            left_to_right: &mut [Option<usize>],
            right_to_left: &mut [Option<usize>],
        ) -> bool {
            for &right in &graph[left] {
                let can_assign = match right_to_left[right] {
                    Some(previous_left) => {
                        distance[previous_left] == distance[left] + 1
                            && augment_layered(
                                previous_left,
                                graph,
                                distance,
                                shortest_augmenting,
                                left_to_right,
                                right_to_left,
                            )
                    }
                    None => distance[left] + 1 == shortest_augmenting,
                };
                if can_assign {
                    left_to_right[left] = Some(right);
                    right_to_left[right] = Some(left);
                    return true;
                }
            }
            distance[left] = UNREACHED;
            false
        }

        let mut phase_matches = 0usize;
        for left in 0..left_count {
            if left_to_right[left].is_none()
                && augment_layered(
                    left,
                    graph,
                    &mut distance,
                    shortest_augmenting,
                    &mut left_to_right,
                    &mut right_to_left,
                )
            {
                phase_matches += 1;
            }
        }
        if phase_matches == 0 {
            break;
        }
        matched_count += phase_matches;
    }

    (matched_count, right_to_left)
}

#[cfg(test)]
fn classify_graph_hopcroft_karp(graph: &[Vec<usize>], right_count: usize) -> MatchingResult {
    let left_count = graph.len();
    if left_count != right_count {
        return MatchingResult::Unmatched;
    }
    if left_count == 0 {
        return MatchingResult::Unique(Vec::new());
    }

    let (matched_count, right_to_left) = hopcroft_karp_matching(graph, right_count);
    if matched_count != left_count {
        return MatchingResult::Unmatched;
    }

    let mut left_to_right = vec![usize::MAX; left_count];
    for (right, left) in right_to_left.iter().enumerate() {
        let Some(left) = left else {
            return MatchingResult::Unmatched;
        };
        left_to_right[*left] = right;
    }
    if left_to_right.contains(&usize::MAX) {
        return MatchingResult::Unmatched;
    }

    let mut alternating = vec![Vec::new(); left_count];
    for (left, edges) in graph.iter().enumerate() {
        for &right in edges {
            let matched_left = right_to_left[right].expect("perfect matching checked above");
            if matched_left != left {
                alternating[left].push(matched_left);
            }
        }
    }

    if has_directed_cycle(&alternating) {
        MatchingResult::Ambiguous
    } else {
        MatchingResult::Unique(left_to_right)
    }
}

fn has_directed_cycle(graph: &[Vec<usize>]) -> bool {
    fn visit(node: usize, graph: &[Vec<usize>], colors: &mut [u8]) -> bool {
        colors[node] = 1;
        for &next in &graph[node] {
            if colors[next] == 1 {
                return true;
            }
            if colors[next] == 0 && visit(next, graph, colors) {
                return true;
            }
        }
        colors[node] = 2;
        false
    }

    let mut colors = vec![0u8; graph.len()];
    for node in 0..graph.len() {
        if colors[node] == 0 && visit(node, graph, &mut colors) {
            return true;
        }
    }
    false
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MatchingResult {
    Unique(Vec<usize>),
    Ambiguous,
    Unmatched,
}

fn classify_graph(graph: &[Vec<usize>], right_count: usize) -> MatchingResult {
    let left_count = graph.len();
    if left_count != right_count {
        return MatchingResult::Unmatched;
    }

    if left_count == 0 {
        return MatchingResult::Unique(Vec::new());
    }

    let mut right_to_left = vec![None; right_count];
    let mut matched_count = 0usize;

    for left in 0..left_count {
        let mut seen_right = vec![false; right_count];
        if augment_matching(left, graph, &mut seen_right, &mut right_to_left) {
            matched_count += 1;
        }
    }

    if matched_count != left_count {
        return MatchingResult::Unmatched;
    }

    let mut left_to_right = vec![usize::MAX; left_count];
    for (right, left) in right_to_left.iter().enumerate() {
        let Some(left) = left else {
            return MatchingResult::Unmatched;
        };
        left_to_right[*left] = right;
    }

    if left_to_right.contains(&usize::MAX) {
        return MatchingResult::Unmatched;
    }

    // Each non-matching edge left_i -> right_j induces a directed edge
    // left_i -> the left node currently matched to right_j. An alternating
    // directed cycle exists exactly when another perfect matching exists.
    let mut alternating = vec![Vec::new(); left_count];
    for (left, edges) in graph.iter().enumerate() {
        for &right in edges {
            let matched_left = right_to_left[right].expect("perfect matching checked above");
            if matched_left != left {
                alternating[left].push(matched_left);
            }
        }
    }

    if has_directed_cycle(&alternating) {
        MatchingResult::Ambiguous
    } else {
        MatchingResult::Unique(left_to_right)
    }
}

fn unresolved_keys(
    source: &[PhysicalObservation],
    witness: &[PhysicalObservation],
) -> (Vec<String>, Vec<String>) {
    (
        source
            .iter()
            .map(|event| event.record_key.clone())
            .collect(),
        witness
            .iter()
            .map(|event| event.record_key.clone())
            .collect(),
    )
}

pub fn evaluate(case: &AssociationCase) -> AssociationReport {
    let Some(profile) = case.profile.as_ref() else {
        return AssociationReport::new(Disposition::Unknown, Vec::new(), Vec::new(), 0);
    };

    if !valid_profile(profile) {
        return AssociationReport::new(Disposition::NotEvaluable, Vec::new(), Vec::new(), 0);
    }

    let source = match visible_observations(&case.source_events, profile, profile.source_time_role)
    {
        Ok(events) => events,
        Err(()) => {
            return AssociationReport::new(Disposition::NotEvaluable, Vec::new(), Vec::new(), 0);
        }
    };
    let witness =
        match visible_observations(&case.witness_events, profile, profile.witness_time_role) {
            Ok(events) => events,
            Err(()) => {
                return AssociationReport::new(
                    Disposition::NotEvaluable,
                    source
                        .iter()
                        .map(|event| event.record_key.clone())
                        .collect(),
                    Vec::new(),
                    0,
                );
            }
        };

    let (source_keys, witness_keys) = unresolved_keys(&source, &witness);
    let graph = candidate_graph(&source, &witness, profile);
    let candidate_edge_count = graph.edges.iter().map(Vec::len).sum();
    let candidate_pair_evaluations = graph.pair_evaluations;

    match classify_graph(&graph.edges, witness.len()) {
        MatchingResult::Unmatched => AssociationReport::new(
            Disposition::Unmatched,
            source_keys,
            witness_keys,
            candidate_edge_count,
        )
        .with_candidate_pair_evaluations(candidate_pair_evaluations),
        MatchingResult::Ambiguous => AssociationReport::new(
            Disposition::Ambiguous,
            source_keys,
            witness_keys,
            candidate_edge_count,
        )
        .with_candidate_pair_evaluations(candidate_pair_evaluations),
        MatchingResult::Unique(left_to_right) => {
            let has_unresolved_time_order =
                left_to_right
                    .iter()
                    .enumerate()
                    .any(|(source_index, witness_index)| {
                        temporal_compatibility(&source[source_index], &witness[*witness_index])
                            == TemporalCompatibility::Unresolved
                    });
            if has_unresolved_time_order {
                return AssociationReport::new(
                    Disposition::TimeOrderUnresolved,
                    source_keys,
                    witness_keys,
                    candidate_edge_count,
                )
                .with_candidate_pair_evaluations(candidate_pair_evaluations);
            }

            let mut report = AssociationReport::new(
                Disposition::UniquelyAssociated,
                Vec::new(),
                Vec::new(),
                candidate_edge_count,
            )
            .with_candidate_pair_evaluations(candidate_pair_evaluations);
            // Visible denominators describe the whole as-of candidate set, not
            // the unresolved subset (which is correctly empty on a unique pass).
            report.source_visible_count = source.len();
            report.witness_visible_count = witness.len();

            for (source_index, witness_index) in left_to_right.into_iter().enumerate() {
                let source_event = &source[source_index];
                let witness_event = &witness[witness_index];
                report.receipts.push(AssociationReceipt {
                    profile_id: profile.profile_id.clone(),
                    profile_version: profile.version.clone(),
                    profile_sha256: profile.profile_sha256.clone(),
                    as_of: profile.as_of,
                    source_channel_id: profile.source_channel_id.clone(),
                    source_record_key: source_event.record_key.clone(),
                    source_revision_id: source_event.revision_id.clone(),
                    source_payload_sha256: source_event.payload_sha256.clone(),
                    witness_channel_id: profile.witness_channel_id.clone(),
                    witness_record_key: witness_event.record_key.clone(),
                    witness_revision_id: witness_event.revision_id.clone(),
                    witness_payload_sha256: witness_event.payload_sha256.clone(),
                    canonical_box_id: source_event.canonical_box_id.clone(),
                    event_kind: source_event.event_kind.clone(),
                    facility_id: source_event.facility_id.clone(),
                    operation_ref: source_event.operation_ref.clone(),
                    source_event_time: source_event.event_time.clone(),
                    witness_event_time: witness_event.event_time.clone(),
                    source_time_role: source_event.event_time.role,
                    witness_time_role: witness_event.event_time.role,
                    interval_gap: interval_gap(&source_event.event_time, &witness_event.event_time),
                    max_interval_gap: profile.max_interval_gap,
                });
            }

            report
                .receipts
                .sort_by(|left, right| left.source_record_key.cmp(&right.source_record_key));
            report
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> CorrelationProfile {
        CorrelationProfile {
            profile_id: "gate-out-correlation-v0.1".to_owned(),
            version: "0.1".to_owned(),
            profile_sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                .to_owned(),
            source_channel_id: "terminal-epcis".to_owned(),
            witness_channel_id: "independent-gate-sensor".to_owned(),
            time_basis_id: "UTC-REF-A".to_owned(),
            source_time_role: TimeRole::BusinessEventTime,
            witness_time_role: TimeRole::SensorObservationTime,
            max_interval_gap: 5,
            as_of: 100,
            source_channel_qualified: true,
            witness_channel_qualified: true,
        }
    }

    macro_rules! observation {
        (
            $key:expr,
            $box_id:expr,
            $kind:expr,
            $facility:expr,
            $operation:expr,
            $lower:expr,
            $upper:expr,
            $published_at:expr $(,)?
        ) => {
            PhysicalObservation {
                record_key: $key.to_owned(),
                revision_id: "rev-1".to_owned(),
                payload_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
                canonical_box_id: $box_id.to_owned(),
                event_kind: $kind.to_owned(),
                facility_id: $facility.to_owned(),
                operation_ref: $operation.to_owned(),
                event_time: TimeEnclosure {
                    lower: $lower,
                    upper: $upper,
                    basis_id: "UTC-REF-A".to_owned(),
                    role: TimeRole::BusinessEventTime,
                    qualified: true,
                },
                published_at: $published_at,
            }
        };
    }

    fn valid_pair() -> (Vec<PhysicalObservation>, Vec<PhysicalObservation>) {
        (
            vec![observation!(
                "src-A",
                "BOX-1",
                "GATE_OUT",
                "DGT-GATE-1",
                "OP-17",
                10,
                10,
                11,
            )],
            vec![observation!(
                "wit-Z",
                "BOX-1",
                "GATE_OUT",
                "DGT-GATE-1",
                "OP-17",
                10,
                10,
                11,
            )],
        )
    }

    fn case(
        source_events: Vec<PhysicalObservation>,
        witness_events: Vec<PhysicalObservation>,
    ) -> AssociationCase {
        let mut witness_events = witness_events;
        for event in &mut witness_events {
            event.event_time.role = TimeRole::SensorObservationTime;
        }
        AssociationCase {
            profile: Some(profile()),
            source_events,
            witness_events,
        }
    }

    fn known_case(case_id: &str) -> AssociationCase {
        let (source, witness) = valid_pair();
        match case_id {
            "single_exact" => case(source, witness),
            "within_tolerance" => case(
                vec![observation!(
                    "src-A",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    14,
                    15,
                    16,
                )],
                vec![observation!(
                    "wit-Z",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    10,
                    11,
                    12,
                )],
            ),
            "outside_tolerance" => case(
                vec![observation!(
                    "src-A",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    16,
                    16,
                    17,
                )],
                vec![observation!(
                    "wit-Z",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    10,
                    10,
                    11,
                )],
            ),
            "wrong_site" => case(
                source,
                vec![observation!(
                    "wit-Z",
                    "BOX-1",
                    "GATE_OUT",
                    "PIER-1-GATE",
                    "OP-17",
                    10,
                    10,
                    11,
                )],
            ),
            "wrong_kind" => case(
                source,
                vec![observation!(
                    "wit-Z",
                    "BOX-1",
                    "GATE_IN",
                    "DGT-GATE-1",
                    "OP-17",
                    10,
                    10,
                    11,
                )],
            ),
            "wrong_box" => case(
                source,
                vec![observation!(
                    "wit-Z",
                    "BOX-2",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    10,
                    10,
                    11,
                )],
            ),
            "wrong_operation" => case(
                source,
                vec![observation!(
                    "wit-Z",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-18",
                    10,
                    10,
                    11,
                )],
            ),
            "sensor_time_after_business_event" => case(
                source,
                vec![observation!(
                    "wit-Z",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    12,
                    12,
                    13,
                )],
            ),
            "sensor_time_overlaps_business_event" => case(
                vec![observation!(
                    "src-A",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    10,
                    12,
                    13,
                )],
                vec![observation!(
                    "wit-Z",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    11,
                    13,
                    14,
                )],
            ),
            "ambiguous_perfect" => case(
                vec![
                    observation!(
                        "src-A",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                    observation!(
                        "src-B",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                ],
                vec![
                    observation!(
                        "wit-X",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                    observation!(
                        "wit-Y",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                ],
            ),
            "unique_global" => case(
                vec![
                    observation!("src-A", "BOX-1", "GATE_OUT", "DGT-GATE-1", "OP-17", 0, 0, 1),
                    observation!(
                        "src-B",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                ],
                vec![
                    observation!("wit-X", "BOX-1", "GATE_OUT", "DGT-GATE-1", "OP-17", 0, 0, 1),
                    observation!(
                        "wit-Y",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        0,
                        10,
                        11,
                    ),
                ],
            ),
            "no_perfect" => case(
                vec![
                    observation!(
                        "src-A",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                    observation!(
                        "src-B",
                        "BOX-2",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                ],
                vec![
                    observation!(
                        "wit-X",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                    observation!(
                        "wit-Y",
                        "BOX-3",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                ],
            ),
            "extra_source" => case(
                vec![
                    observation!(
                        "src-A",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                    observation!(
                        "src-B",
                        "BOX-2",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                ],
                witness,
            ),
            "duplicate_source_key" => case(
                vec![
                    observation!(
                        "src-A",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                    observation!(
                        "src-A",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                ],
                witness,
            ),
            "duplicate_witness_key" => case(
                source,
                vec![
                    observation!(
                        "wit-Z",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                    observation!(
                        "wit-Z",
                        "BOX-1",
                        "GATE_OUT",
                        "DGT-GATE-1",
                        "OP-17",
                        10,
                        10,
                        11,
                    ),
                ],
            ),
            "unqualified_source" => {
                let mut source = source;
                source[0].event_time.qualified = false;
                case(source, witness)
            }
            "unqualified_witness" => {
                let mut witness = witness;
                witness[0].event_time.qualified = false;
                case(source, witness)
            }
            "time_basis_mismatch" => {
                let mut witness = witness;
                witness[0].event_time.basis_id = "UNTRUSTED-BASIS".to_owned();
                case(source, witness)
            }
            "invalid_interval" => {
                let mut source = source;
                source[0].event_time.lower = 20;
                source[0].event_time.upper = 10;
                case(source, witness)
            }
            "future_publication_asof" => {
                let mut source = source;
                source[0].published_at = 101;
                case(source, witness)
            }
            "no_events" => case(Vec::new(), Vec::new()),
            "published_before_event" => {
                let mut source = source;
                source[0].published_at = 9;
                case(source, witness)
            }
            "source_channel_unqualified" => {
                let mut test = case(source, witness);
                test.profile.as_mut().unwrap().source_channel_qualified = false;
                test
            }
            "witness_channel_unqualified" => {
                let mut test = case(source, witness);
                test.profile.as_mut().unwrap().witness_channel_qualified = false;
                test
            }
            "invalid_profile_digest" => {
                let mut test = case(source, witness);
                test.profile.as_mut().unwrap().profile_sha256 = "not-a-digest".to_owned();
                test
            }
            "unsupported_time_role_profile" => {
                let mut test = case(source, witness);
                test.profile.as_mut().unwrap().source_time_role = TimeRole::SensorObservationTime;
                test
            }
            "source_time_role_mismatch" => {
                let mut test = case(source, witness);
                test.source_events[0].event_time.role = TimeRole::SensorObservationTime;
                test
            }
            "witness_time_role_mismatch" => {
                let mut test = case(source, witness);
                test.witness_events[0].event_time.role = TimeRole::BusinessEventTime;
                test
            }
            "invalid_source_digest" => {
                let mut test = case(source, witness);
                test.source_events[0].payload_sha256 = "not-a-digest".to_owned();
                test
            }
            "invalid_witness_digest" => {
                let mut test = case(source, witness);
                test.witness_events[0].payload_sha256 = "not-a-digest".to_owned();
                test
            }
            "missing_revision" => {
                let mut test = case(source, witness);
                test.source_events[0].revision_id.clear();
                test
            }
            _ => panic!("unknown fixture case: {case_id}"),
        }
    }

    fn exhaustive_oracle(graph: &[Vec<usize>], right_count: usize) -> Disposition {
        if graph.len() != right_count {
            return Disposition::Unmatched;
        }

        fn enumerate(
            left: usize,
            graph: &[Vec<usize>],
            used_right: &mut [bool],
            solutions: &mut usize,
        ) {
            if *solutions > 1 {
                return;
            }
            if left == graph.len() {
                *solutions += 1;
                return;
            }
            for &right in &graph[left] {
                if !used_right[right] {
                    used_right[right] = true;
                    enumerate(left + 1, graph, used_right, solutions);
                    used_right[right] = false;
                    if *solutions > 1 {
                        return;
                    }
                }
            }
        }

        let mut used = vec![false; right_count];
        let mut solutions = 0usize;
        enumerate(0, graph, &mut used, &mut solutions);
        match solutions {
            0 => Disposition::Unmatched,
            1 => Disposition::UniquelyAssociated,
            _ => Disposition::Ambiguous,
        }
    }

    #[test]
    fn frozen_expected_dispositions_match_tsv_oracle() {
        let fixture = include_str!("../fixtures/association-v0.1.tsv");
        let mut count = 0usize;
        for line in fixture
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 2, "malformed fixture row: {line}");
            let actual = evaluate(&known_case(fields[0]));
            assert_eq!(actual.disposition.as_str(), fields[1], "case {}", fields[0]);
            count += 1;
        }
        assert_eq!(count, 31);
    }

    #[test]
    fn hopcroft_karp_matches_dfs_and_exhaustive_oracle_for_all_three_by_three_graphs() {
        let node_count = 3usize;
        let edge_slots = node_count * node_count;
        for mask in 0usize..(1usize << edge_slots) {
            let mut graph = vec![Vec::new(); node_count];
            for (left, edges) in graph.iter_mut().enumerate() {
                for right in 0..node_count {
                    let bit = left * node_count + right;
                    if (mask & (1usize << bit)) != 0 {
                        edges.push(right);
                    }
                }
            }
            let reference = classify_graph(&graph, node_count);
            let challenger = classify_graph_hopcroft_karp(&graph, node_count);
            assert_eq!(challenger, reference, "graph mask {mask}");
            let expected = exhaustive_oracle(&graph, node_count);
            let actual = match challenger {
                MatchingResult::Unique(_) => Disposition::UniquelyAssociated,
                MatchingResult::Ambiguous => Disposition::Ambiguous,
                MatchingResult::Unmatched => Disposition::Unmatched,
            };
            assert_eq!(actual, expected, "oracle graph mask {mask}");
        }
    }

    #[test]
    fn hopcroft_karp_matches_dfs_on_deterministic_six_by_six_graph_panel() {
        let node_count = 6usize;
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        for panel in 0..4096usize {
            let density_cutoff = (panel % 8) as u64;
            let mut graph = vec![Vec::new(); node_count];
            for edges in &mut graph {
                for right in 0..node_count {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    if (state >> 61) <= density_cutoff {
                        edges.push(right);
                    }
                }
            }
            assert_eq!(
                classify_graph_hopcroft_karp(&graph, node_count),
                classify_graph(&graph, node_count),
                "six-by-six panel {panel}"
            );
        }
    }

    #[test]
    fn bipartite_uniqueness_matches_exhaustive_oracle_for_all_three_by_three_graphs() {
        let node_count = 3usize;
        let edge_slots = node_count * node_count;
        for mask in 0usize..(1usize << edge_slots) {
            let mut graph = vec![Vec::new(); node_count];
            for (left, edges) in graph.iter_mut().enumerate() {
                for right in 0..node_count {
                    let bit = left * node_count + right;
                    if (mask & (1usize << bit)) != 0 {
                        edges.push(right);
                    }
                }
            }
            let actual = match classify_graph(&graph, node_count) {
                MatchingResult::Unique(_) => Disposition::UniquelyAssociated,
                MatchingResult::Ambiguous => Disposition::Ambiguous,
                MatchingResult::Unmatched => Disposition::Unmatched,
            };
            assert_eq!(
                actual,
                exhaustive_oracle(&graph, node_count),
                "graph mask {mask}"
            );
        }
    }

    #[test]
    fn a_locally_ambiguous_graph_can_have_one_global_perfect_matching() {
        let report = evaluate(&known_case("unique_global"));
        assert_eq!(report.disposition, Disposition::UniquelyAssociated);
        assert_eq!(report.receipts.len(), 2);
        assert_eq!(report.receipts[0].witness_record_key, "wit-X");
        assert_eq!(report.receipts[1].witness_record_key, "wit-Y");
    }

    #[test]
    fn multiple_perfect_matchings_are_never_broken_by_a_tie_break() {
        let report = evaluate(&known_case("ambiguous_perfect"));
        assert_eq!(report.disposition, Disposition::Ambiguous);
        assert!(report.receipts.is_empty());
        assert_eq!(report.candidate_edge_count, 4);
    }

    #[test]
    fn input_permutation_does_not_change_disposition_or_receipts() {
        let forward = known_case("unique_global");
        let mut reversed = forward.clone();
        reversed.source_events.reverse();
        reversed.witness_events.reverse();
        let left = evaluate(&forward);
        let right = evaluate(&reversed);
        assert_eq!(left.disposition, right.disposition);
        assert_eq!(left.receipts, right.receipts);
    }

    #[test]
    fn widening_time_enclosures_can_reveal_ambiguity_but_not_hide_it() {
        let mut narrow = known_case("unique_global");
        narrow.profile.as_mut().unwrap().max_interval_gap = 0;
        assert_eq!(
            evaluate(&narrow).disposition,
            Disposition::UniquelyAssociated
        );
        for event in &mut narrow.source_events {
            event.event_time.lower = 0;
            event.event_time.upper = 10;
            event.published_at = 11;
        }
        assert_eq!(evaluate(&narrow).disposition, Disposition::Ambiguous);
    }

    #[test]
    fn interval_indexed_candidate_window_matches_full_scan_on_boundary_panel() {
        let source = vec![
            observation!("s0", "BOX-1", "GATE_OUT", "SITE-1", "OP-1", 100, 110, 111),
            observation!("s1", "BOX-1", "GATE_OUT", "SITE-1", "OP-1", 200, 200, 201),
            observation!("s2", "BOX-2", "GATE_IN", "SITE-2", "OP-2", 300, 305, 306),
        ];
        let witness = vec![
            observation!("w0", "BOX-1", "GATE_OUT", "SITE-1", "OP-1", 90, 95, 96),
            observation!("w1", "BOX-1", "GATE_OUT", "SITE-1", "OP-1", 95, 100, 101),
            observation!("w2", "BOX-1", "GATE_OUT", "SITE-1", "OP-1", 108, 112, 113),
            observation!("w3", "BOX-1", "GATE_OUT", "SITE-1", "OP-1", 111, 115, 116),
            observation!("w4", "BOX-1", "GATE_OUT", "SITE-1", "OP-1", 116, 120, 121),
            observation!("w5", "BOX-2", "GATE_IN", "SITE-2", "OP-2", 290, 295, 296),
            observation!("w6", "BOX-2", "GATE_IN", "SITE-2", "OP-2", 299, 301, 302),
            observation!("w7", "BOX-2", "GATE_IN", "SITE-2", "OP-2", 306, 310, 311),
        ];
        let case = case(source, witness);
        let baseline = candidate_graph(
            &case.source_events,
            &case.witness_events,
            case.profile.as_ref().unwrap(),
        );
        let indexed = candidate_graph_interval_indexed(
            &case.source_events,
            &case.witness_events,
            case.profile.as_ref().unwrap(),
        );
        assert_eq!(indexed.edges, baseline.edges);
        assert_eq!(
            indexed.pair_evaluations,
            indexed.edges.iter().map(Vec::len).sum::<usize>()
        );
        assert!(indexed.pair_evaluations <= baseline.pair_evaluations);
        // The source is BusinessEventTime and the witness is SensorObservationTime.
        // A witness wholly after the business interval remains rejected even if its
        // numeric interval gap is within the profile tolerance.
        assert_eq!(
            case.profile.as_ref().unwrap().source_time_role,
            TimeRole::BusinessEventTime
        );
        assert_eq!(
            case.profile.as_ref().unwrap().witness_time_role,
            TimeRole::SensorObservationTime
        );
        assert!(!baseline.edges[0].contains(&3));
        assert!(!indexed.edges[0].contains(&3));
    }

    #[test]
    fn interval_tree_matches_full_scan_for_every_small_temporal_interval_pair() {
        let mut intervals = Vec::new();
        for lower in 0_i64..=5 {
            for upper in lower..=5 {
                intervals.push((lower, upper));
            }
        }

        let mut checked = 0usize;
        for (source_lower, source_upper) in &intervals {
            for (witness_lower, witness_upper) in &intervals {
                for max_gap in 0_i64..=3 {
                    let source = vec![observation!(
                        "source",
                        "BOX-EXHAUSTIVE",
                        "GATE_OUT",
                        "SITE-EXHAUSTIVE",
                        "OP-EXHAUSTIVE",
                        *source_lower,
                        *source_upper,
                        source_upper + 1,
                    )];
                    let witness = vec![observation!(
                        "witness",
                        "BOX-EXHAUSTIVE",
                        "GATE_OUT",
                        "SITE-EXHAUSTIVE",
                        "OP-EXHAUSTIVE",
                        *witness_lower,
                        *witness_upper,
                        witness_upper + 1,
                    )];
                    let mut case = case(source, witness);
                    case.profile.as_mut().unwrap().max_interval_gap = max_gap;

                    let profile = case.profile.as_ref().unwrap();
                    let baseline =
                        candidate_graph(&case.source_events, &case.witness_events, profile);
                    let indexed = candidate_graph_interval_indexed(
                        &case.source_events,
                        &case.witness_events,
                        profile,
                    );
                    assert_eq!(
                        indexed.edges, baseline.edges,
                        "source=[{source_lower},{source_upper}] witness=[{witness_lower},{witness_upper}] max_gap={max_gap}"
                    );
                    assert_eq!(
                        indexed.pair_evaluations,
                        indexed.edges.iter().map(Vec::len).sum::<usize>()
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 900);
    }

    #[test]
    fn interval_indexed_candidate_window_matches_full_scan_on_seeded_mixed_contexts() {
        let mut state = 0x6a09_e667_f3bc_c909u64;
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state
        };
        let mut source = Vec::new();
        let mut witness = Vec::new();
        for index in 0..96 {
            let box_id = format!("BOX-{}", next() % 5);
            let kind = if next() % 2 == 0 {
                "GATE_IN"
            } else {
                "GATE_OUT"
            };
            let facility = format!("SITE-{}", next() % 3);
            let operation = format!("OP-{}", next() % 4);
            let lower = i64::try_from(next() % 800).unwrap();
            let upper = lower + i64::try_from(next() % 24).unwrap();
            source.push(observation!(
                &format!("s-{index}"),
                &box_id,
                kind,
                &facility,
                &operation,
                lower,
                upper,
                upper + 1,
            ));
        }
        for index in 0..96 {
            let box_id = format!("BOX-{}", next() % 5);
            let kind = if next() % 2 == 0 {
                "GATE_IN"
            } else {
                "GATE_OUT"
            };
            let facility = format!("SITE-{}", next() % 3);
            let operation = format!("OP-{}", next() % 4);
            let lower = (next() % 800) as i64;
            let upper = lower + (next() % 24) as i64;
            witness.push(observation!(
                &format!("w-{index}"),
                &box_id,
                kind,
                &facility,
                &operation,
                lower,
                upper,
                upper + 1,
            ));
        }
        let case = case(source, witness);
        let baseline = candidate_graph(
            &case.source_events,
            &case.witness_events,
            case.profile.as_ref().unwrap(),
        );
        let indexed = candidate_graph_interval_indexed(
            &case.source_events,
            &case.witness_events,
            case.profile.as_ref().unwrap(),
        );
        assert_eq!(indexed.edges, baseline.edges);
        assert_eq!(
            indexed.pair_evaluations,
            indexed.edges.iter().map(Vec::len).sum::<usize>()
        );
    }

    #[test]
    fn interval_indexed_candidate_window_reduces_dense_context_evaluations() {
        let mut source = Vec::new();
        let mut witness = Vec::new();
        for index in 0..128 {
            let time = 10_000_i64 + i64::from(index) * 100;
            source.push(observation!(
                &format!("s-{index}"),
                "BOX-ONE",
                "GATE_OUT",
                "SITE-ONE",
                "OP-ONE",
                time,
                time,
                time,
            ));
            witness.push(observation!(
                &format!("w-{index}"),
                "BOX-ONE",
                "GATE_OUT",
                "SITE-ONE",
                "OP-ONE",
                time - 1,
                time - 1,
                time - 1,
            ));
        }
        let case = case(source, witness);
        let baseline = candidate_graph(
            &case.source_events,
            &case.witness_events,
            case.profile.as_ref().unwrap(),
        );
        let indexed = candidate_graph_interval_indexed(
            &case.source_events,
            &case.witness_events,
            case.profile.as_ref().unwrap(),
        );
        assert_eq!(indexed.edges, baseline.edges);
        assert_eq!(indexed.pair_evaluations, 128);
        assert_eq!(baseline.pair_evaluations, 128 * 128);
        assert!(indexed.index_nodes_visited < baseline.pair_evaluations);
    }

    #[test]
    fn exact_context_blocking_avoids_cross_product_time_checks() {
        let mut source = Vec::new();
        let mut witness = Vec::new();
        for index in 0..128 {
            let box_id = format!("BOX-{index}");
            let operation = format!("OP-{index}");
            let source_key = format!("src-{index}");
            let witness_key = format!("wit-{index}");
            source.push(observation!(
                source_key.as_str(),
                box_id.as_str(),
                "GATE_OUT",
                "GATE-1",
                operation.as_str(),
                10,
                10,
                11,
            ));
            witness.push(observation!(
                witness_key.as_str(),
                box_id.as_str(),
                "GATE_OUT",
                "GATE-1",
                operation.as_str(),
                10,
                10,
                11,
            ));
        }

        let report = evaluate(&case(source, witness));
        assert_eq!(report.disposition, Disposition::UniquelyAssociated);
        assert_eq!(report.source_visible_count, 128);
        assert_eq!(report.witness_visible_count, 128);
        assert_eq!(report.candidate_edge_count, 128);
        assert_eq!(report.candidate_pair_evaluations, 128);
        assert_eq!(report.receipts.len(), 128);
        // A global scan would test 128 × 128 = 16,384 pairs. Exact-context
        // blocking tests only the 128 pairs that can possibly form edges.
    }

    #[test]
    fn source_local_ids_are_not_cross_source_join_keys() {
        let mut test = known_case("single_exact");
        test.source_events[0].record_key = "carrier-event-111".to_owned();
        test.witness_events[0].record_key = "reader-record-XYZ".to_owned();
        let report = evaluate(&test);
        assert_eq!(report.disposition, Disposition::UniquelyAssociated);
        assert_eq!(report.receipts[0].source_record_key, "carrier-event-111");
        assert_eq!(report.receipts[0].witness_record_key, "reader-record-XYZ");
    }

    #[test]
    fn no_event_snapshot_is_only_associable_when_both_channels_are_qualified() {
        let report = evaluate(&known_case("no_events"));
        assert_eq!(report.disposition, Disposition::UniquelyAssociated);
        assert!(report.receipts.is_empty());
        assert_eq!(
            evaluate(&known_case("source_channel_unqualified")).disposition,
            Disposition::NotEvaluable
        );
        assert_eq!(
            evaluate(&known_case("witness_channel_unqualified")).disposition,
            Disposition::NotEvaluable
        );
    }

    #[test]
    fn publication_after_the_cutoff_is_not_visible_in_the_historical_snapshot() {
        let report = evaluate(&known_case("future_publication_asof"));
        assert_eq!(report.disposition, Disposition::Unmatched);
        assert_eq!(report.source_visible_count, 0);
        assert_eq!(report.witness_visible_count, 1);
    }

    #[test]
    fn sensor_observation_must_not_follow_the_business_event_time() {
        let report = evaluate(&known_case("sensor_time_after_business_event"));
        assert_eq!(report.disposition, Disposition::Unmatched);
        assert_eq!(report.candidate_edge_count, 0);
    }

    #[test]
    fn overlapping_sensor_and_business_time_enclosures_remain_unresolved() {
        let report = evaluate(&known_case("sensor_time_overlaps_business_event"));
        assert_eq!(report.disposition, Disposition::TimeOrderUnresolved);
        assert_eq!(report.candidate_edge_count, 1);
        assert!(report.receipts.is_empty());
    }

    #[test]
    fn wrong_box_site_kind_or_operation_never_creates_an_edge() {
        for case_id in ["wrong_box", "wrong_site", "wrong_kind", "wrong_operation"] {
            let report = evaluate(&known_case(case_id));
            assert_eq!(report.disposition, Disposition::Unmatched, "{case_id}");
            assert_eq!(report.candidate_edge_count, 0, "{case_id}");
        }
    }

    #[test]
    fn unqualified_or_incomparable_time_evidence_is_not_evaluable() {
        for case_id in [
            "unqualified_source",
            "unqualified_witness",
            "time_basis_mismatch",
            "invalid_interval",
        ] {
            assert_eq!(
                evaluate(&known_case(case_id)).disposition,
                Disposition::NotEvaluable,
                "{case_id}"
            );
        }
    }

    #[test]
    fn duplicate_local_record_keys_are_rejected_within_each_source_namespace() {
        assert_eq!(
            evaluate(&known_case("duplicate_source_key")).disposition,
            Disposition::NotEvaluable
        );
        assert_eq!(
            evaluate(&known_case("duplicate_witness_key")).disposition,
            Disposition::NotEvaluable
        );
    }

    #[test]
    fn receipts_preserve_profile_revision_and_input_artifact_digests() {
        let report = evaluate(&known_case("single_exact"));
        let receipt = &report.receipts[0];
        assert_eq!(
            receipt.profile_sha256,
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        );
        assert_eq!(receipt.source_revision_id, "rev-1");
        assert_eq!(receipt.witness_revision_id, "rev-1");
        assert_eq!(receipt.source_time_role, TimeRole::BusinessEventTime);
        assert_eq!(receipt.witness_time_role, TimeRole::SensorObservationTime);
        assert_eq!(
            receipt.source_payload_sha256,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(
            receipt.witness_payload_sha256,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
    }

    #[test]
    fn malformed_profile_or_input_digests_and_missing_revisions_fail_closed() {
        for case_id in [
            "invalid_profile_digest",
            "invalid_source_digest",
            "invalid_witness_digest",
            "missing_revision",
        ] {
            assert_eq!(
                evaluate(&known_case(case_id)).disposition,
                Disposition::NotEvaluable,
                "{case_id}"
            );
        }
    }

    #[test]
    fn mismatched_temporal_roles_fail_closed() {
        for case_id in [
            "source_time_role_mismatch",
            "witness_time_role_mismatch",
            "unsupported_time_role_profile",
        ] {
            assert_eq!(
                evaluate(&known_case(case_id)).disposition,
                Disposition::NotEvaluable,
                "{case_id}"
            );
        }
    }

    #[test]
    fn publication_cannot_precede_the_earliest_possible_event_time() {
        assert_eq!(
            evaluate(&known_case("published_before_event")).disposition,
            Disposition::NotEvaluable
        );
    }

    #[test]
    fn outside_tolerance_and_cardinality_mismatch_remain_unmatched() {
        let outside = evaluate(&known_case("outside_tolerance"));
        assert_eq!(outside.disposition, Disposition::Unmatched);
        assert_eq!(outside.candidate_edge_count, 0);
        assert_eq!(
            evaluate(&known_case("extra_source")).disposition,
            Disposition::Unmatched
        );
        assert_eq!(
            evaluate(&known_case("no_perfect")).disposition,
            Disposition::Unmatched
        );
    }

    #[test]
    fn the_empty_graph_has_one_empty_perfect_matching() {
        assert_eq!(classify_graph(&[], 0), MatchingResult::Unique(Vec::new()));
    }
}
