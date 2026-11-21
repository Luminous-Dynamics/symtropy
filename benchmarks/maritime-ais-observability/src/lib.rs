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
    pub mmsi: String,
    pub message_id: String,
    pub source_id: String,
    pub geofence_digest: String,
    pub tick: i64,
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
    pub supporting_fix_count: usize,
    pub profile_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallReport {
    pub mmsi: String,
    pub source_id: String,
    pub geofence_digest: String,
    pub profile_version: String,
    pub final_state: ServiceState,
    pub accepted_fix_count: usize,
    pub duplicate_fix_count: usize,
    pub invalid_fix_count: usize,
    pub nav_status_conflict_count: usize,
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

fn transition_kind(
    previous: ServiceState,
    next: ServiceState,
    zone: Zone,
) -> EventKind {
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
    profile.validate()?;
    if !valid_mmsi(expected_mmsi) {
        return Err(format!("invalid MMSI {expected_mmsi:?}: expected nine digits"));
    }
    if fixes.is_empty() {
        return Err("trajectory contains no AIS fixes".to_string());
    }

    let first = &fixes[0];
    if first.mmsi != expected_mmsi {
        return Err("first fix MMSI does not match expected trajectory identity".to_string());
    }
    if first.source_id.trim().is_empty() {
        return Err("source ID must be non-empty".to_string());
    }

    let source_id = first.source_id.clone();
    if first.geofence_digest.trim().is_empty() {
        return Err("geofence digest must be non-empty".to_string());
    }
    let geofence_digest = first.geofence_digest.clone();
    let mut seen_messages: BTreeMap<String, AisFix> = BTreeMap::new();
    let mut state = ServiceState::Outside;
    let mut candidate: Option<Candidate> = None;
    let mut last_tick: Option<i64> = None;
    let mut accepted_fix_count = 0;
    let mut duplicate_fix_count = 0;
    let mut invalid_fix_count = 0;
    let mut nav_status_conflict_count = 0;
    let mut kinematic_conflict_count = 0;
    let mut observability_gap_count = 0;
    let mut events = Vec::new();

    for fix in fixes {
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

        if let Some(previous_tick) = last_tick {
            if fix.tick < previous_tick {
                return Err(format!(
                    "out-of-order AIS tick {} follows {}",
                    fix.tick, previous_tick
                ));
            }

            let gap = fix.tick - previous_tick;
            if gap > profile.max_observation_gap_ticks {
                observability_gap_count += 1;
                candidate = None;
                state = ServiceState::Indeterminate;
                events.push(InferredEvent {
                    kind: EventKind::ObservabilityGap,
                    lower_tick: previous_tick,
                    upper_tick: fix.tick,
                    confirmation_tick: fix.tick,
                    supporting_fix_count: 0,
                    profile_version: profile.version.clone(),
                });
            }
        }

        last_tick = Some(fix.tick);
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
                supporting_fix_count: 0,
                profile_version: profile.version.clone(),
            });
            continue;
        }

        if nav_status_conflicts_with_zone(fix.nav_status_raw.as_deref(), fix.zone) {
            nav_status_conflict_count += 1;
        }

        let speed_is_plausible =
            fix.speed_milli_knots <= fix.zone.max_speed_milli_knots(profile);

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
                supporting_fix_count: 1,
                profile_version: profile.version.clone(),
            });
            continue;
        }

        match candidate.as_mut() {
            Some(current)
                if current.zone == fix.zone
                    && fix.tick - current.last_tick <= profile.max_observation_gap_ticks =>
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
        if current.last_tick - current.first_tick < required_dwell
            || current.supporting_fix_count < 2
        {
            continue;
        }

        let next_state = state.transition_target(current.zone);
        let transition = transition_kind(state, next_state, current.zone);
        events.push(InferredEvent {
            kind: transition,
            lower_tick: current.first_tick,
            upper_tick: current.last_tick,
            confirmation_tick: current.last_tick,
            supporting_fix_count: current.supporting_fix_count,
            profile_version: profile.version.clone(),
        });
        state = next_state;
        candidate = None;
    }

    Ok(CallReport {
        mmsi: expected_mmsi.to_string(),
        source_id,
        geofence_digest,
        profile_version: profile.version.clone(),
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

    fn fix(
        tick: i64,
        message_id: &str,
        zone: Zone,
        speed_milli_knots: u32,
    ) -> AisFix {
        AisFix {
            mmsi: "563123456".to_string(),
            message_id: message_id.to_string(),
            source_id: "RX_DURBAN_TEST_A".to_string(),
            geofence_digest: "sha256:fixture-geofence-v1".to_string(),
            tick,
            zone,
            speed_milli_knots,
            position_valid: true,
            nav_status_raw: None,
        }
    }

    fn default_profile() -> DwellProfile {
        DwellProfile::default()
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

        let report = evaluate_call("563123456", &fixes, &default_profile()).expect("valid call");
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
        let report = evaluate_call("563123456", &fixes, &default_profile()).expect("valid call");

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
        let report = evaluate_call("563123456", &fixes, &default_profile()).expect("valid call");

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
        let report = evaluate_call("563123456", &fixes, &default_profile()).expect("valid call");

        assert_eq!(report.duplicate_fix_count, 1);
        assert_eq!(report.final_state, ServiceState::Berth);
    }

    #[test]
    fn conflicting_message_id_reuse_rejects() {
        let first = fix(1, "reused", Zone::Berth, 100);
        let second = fix(1, "reused", Zone::Approach, 5_000);
        let error = evaluate_call(
            "563123456",
            &[first, second],
            &default_profile(),
        )
        .expect_err("conflicting replay must reject");

        assert!(error.contains("conflicting content"));
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
    fn nav_status_is_preserved_as_a_conflict_not_used_as_authority() {
        let mut first = fix(0, "m0", Zone::Outside, 0);
        first.nav_status_raw = Some("moored".to_string());
        let second = fix(1, "m1", Zone::Outside, 0);
        let report = evaluate_call(
            "563123456",
            &[first, second],
            &default_profile(),
        )
        .expect("valid call");

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
        let report = evaluate_call("563123456", &fixes, &default_profile()).expect("valid call");

        assert!(report.events.iter().any(|event| {
            event.kind == EventKind::BerthExitDetected
                && event.lower_tick == 4
                && event.confirmation_tick == 6
        }));
        assert_eq!(report.final_state, ServiceState::Outside);
        assert!(!report.events.iter().any(|event| {
            event.kind.label().contains("LINES_UNTIED")
        }));
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
        let report = evaluate_call("563123456", &fixes, &default_profile()).expect("valid call");

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

        let error = evaluate_call("563123456", &[first, second], &default_profile())
            .expect_err("mixed geofence versions must reject");
        assert!(error.contains("geofence identity changed"));
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
            let report = evaluate_call("563123456", &fixes, &profile).expect("valid call");
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
        let report = evaluate_call("563123456", &fixes, &default_profile()).expect("valid call");

        assert_eq!(report.observability_gap_count, 1);
        assert_eq!(report.final_state, ServiceState::Berth);
        assert!(report.events.iter().any(|event| event.kind == EventKind::ObservabilityGap));
        assert!(report.events.iter().any(|event| event.kind == EventKind::ZoneReacquired));
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
        let report = evaluate_call("563123456", &fixes, &default_profile()).expect("valid call");

        assert_eq!(report.invalid_fix_count, 1);
        assert_eq!(report.final_state, ServiceState::Indeterminate);
    }

    #[test]
    fn mmsi_and_profile_fail_closed() {
        assert!(evaluate_call("bad", &[fix(0, "m0", Zone::Outside, 0)], &default_profile()).is_err());

        let mut bad_profile = default_profile();
        bad_profile.berth_dwell_ticks = 0;
        assert!(evaluate_call("563123456", &[fix(0, "m0", Zone::Outside, 0)], &bad_profile).is_err());
    }
}
