use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeEnclosure {
    pub lower: i64,
    pub upper: i64,
    pub basis_id: String,
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
    NotEvaluable,
    Unknown,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UniquelyAssociated => "UNIQUELY_ASSOCIATED",
            Self::Ambiguous => "AMBIGUOUS",
            Self::Unmatched => "UNMATCHED",
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
    pub interval_gap: i128,
    pub max_interval_gap: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssociationReport {
    pub disposition: Disposition,
    pub source_visible_count: usize,
    pub witness_visible_count: usize,
    pub candidate_edge_count: usize,
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
            receipts: Vec::new(),
            unresolved_source_keys: source_keys,
            unresolved_witness_keys: witness_keys,
        }
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
        && profile.max_interval_gap >= 0
        && profile.source_channel_qualified
        && profile.witness_channel_qualified
}

fn visible_observations(
    observations: &[PhysicalObservation],
    profile: &CorrelationProfile,
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

fn admissible_edge(
    source: &PhysicalObservation,
    witness: &PhysicalObservation,
    profile: &CorrelationProfile,
) -> bool {
    source.canonical_box_id == witness.canonical_box_id
        && source.event_kind == witness.event_kind
        && source.facility_id == witness.facility_id
        && source.operation_ref == witness.operation_ref
        && interval_gap(&source.event_time, &witness.event_time)
            <= i128::from(profile.max_interval_gap)
}

fn candidate_graph(
    source: &[PhysicalObservation],
    witness: &[PhysicalObservation],
    profile: &CorrelationProfile,
) -> Vec<Vec<usize>> {
    source
        .iter()
        .map(|source_event| {
            witness
                .iter()
                .enumerate()
                .filter_map(|(index, witness_event)| {
                    admissible_edge(source_event, witness_event, profile).then_some(index)
                })
                .collect()
        })
        .collect()
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

    let source = match visible_observations(&case.source_events, profile) {
        Ok(events) => events,
        Err(()) => {
            return AssociationReport::new(Disposition::NotEvaluable, Vec::new(), Vec::new(), 0);
        }
    };
    let witness = match visible_observations(&case.witness_events, profile) {
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
    let candidate_edge_count = graph.iter().map(Vec::len).sum();

    match classify_graph(&graph, witness.len()) {
        MatchingResult::Unmatched => AssociationReport::new(
            Disposition::Unmatched,
            source_keys,
            witness_keys,
            candidate_edge_count,
        ),
        MatchingResult::Ambiguous => AssociationReport::new(
            Disposition::Ambiguous,
            source_keys,
            witness_keys,
            candidate_edge_count,
        ),
        MatchingResult::Unique(left_to_right) => {
            let mut report = AssociationReport::new(
                Disposition::UniquelyAssociated,
                Vec::new(),
                Vec::new(),
                candidate_edge_count,
            );

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
                    10,
                    11,
                    12,
                )],
                vec![observation!(
                    "wit-Z",
                    "BOX-1",
                    "GATE_OUT",
                    "DGT-GATE-1",
                    "OP-17",
                    14,
                    15,
                    16,
                )],
            ),
            "outside_tolerance" => case(
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
                    16,
                    16,
                    17,
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
        assert_eq!(count, 26);
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
    fn publication_cannot_precede_the_earliest_possible_event_time() {
        assert_eq!(
            evaluate(&known_case("published_before_event")).disposition,
            Disposition::NotEvaluable
        );
    }

    #[test]
    fn outside_tolerance_and_cardinality_mismatch_remain_unmatched() {
        assert_eq!(
            evaluate(&known_case("outside_tolerance")).disposition,
            Disposition::Unmatched
        );
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
