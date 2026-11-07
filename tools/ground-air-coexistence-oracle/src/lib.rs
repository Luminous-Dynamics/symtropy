// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded exact source-allocation oracle for simultaneous ground/air sessions.
//!
//! This crate schedules integer source-power quanta against supplied synthetic
//! service coefficients. It does not model or qualify inductive, capacitive,
//! RF or optical physics, field coexistence, aircraft operation, EMC, exposure,
//! converter stability, or hardware protection.

use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::BTreeSet;

const MAX_SLOTS: usize = 8;
const MAX_CHANNELS: usize = 8;
const MAX_SESSIONS: usize = 8;
const MAX_POWER_QUANTA: u64 = 8;
const MAX_ENERGY_WH: u64 = 1_000_000;
const MAX_SEARCH_NODES: u64 = 1_000_000;
const SHA256_HEX_LEN: usize = 64;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(
    value: &'a Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in object(value, path)?.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn uint_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn array_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    field(value, key, path)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an array"))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Domain {
    Ground,
    Air,
}

impl Domain {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ground => "GROUND",
            Self::Air => "AIR",
        }
    }
}

fn parse_domain(value: &str, path: &str) -> Result<Domain, String> {
    match value {
        "GROUND" => Ok(Domain::Ground),
        "AIR" => Ok(Domain::Air),
        _ => Err(fail(path, "domain must be GROUND or AIR")),
    }
}

fn modality_domain(value: &str, path: &str) -> Result<Domain, String> {
    match value {
        "GROUND_DYNAMIC_INDUCTIVE" | "GROUND_STATIC_INDUCTIVE" => Ok(Domain::Ground),
        "AIR_DOCK_INDUCTIVE"
        | "AIR_DOCK_CAPACITIVE"
        | "AIR_HOVER_INDUCTIVE"
        | "AIR_HOVER_CAPACITIVE"
        | "AIR_RF_POWER_BEAMING"
        | "AIR_OPTICAL_POWER_BEAMING" => Ok(Domain::Air),
        _ => Err(fail(path, "unsupported v0 transfer modality")),
    }
}

fn receiver_class_domain(value: &str, path: &str) -> Result<Domain, String> {
    match value {
        "ROAD_CAR" | "ROAD_BUS" | "ROAD_HEAVY_TRUCK" => Ok(Domain::Ground),
        "UAV_DOCKED"
        | "UAV_HOVERING"
        | "UAV_FIXED_WING"
        | "EVTOL_DOCKED"
        | "EVTOL_AIRBORNE" => Ok(Domain::Air),
        _ => Err(fail(path, "unsupported v0 receiver class")),
    }
}

#[derive(Debug, Clone)]
struct Channel {
    channel_id: String,
    domain: Domain,
    maximum_input_quanta_per_slot: u64,
    envelope_sha256: String,
}

#[derive(Debug, Clone)]
struct Session {
    session_id: String,
    domain: Domain,
    channel_index: usize,
    start_slot: usize,
    end_slot: usize,
    requested_useful_energy_wh: u64,
    minimum_useful_energy_wh: u64,
    maximum_input_quanta_per_slot: u64,
    useful_energy_wh_per_input_quantum_slot: u64,
}

#[derive(Debug, Clone)]
struct Scenario {
    episode_id: String,
    site_id: String,
    grid_source_id: String,
    slot_duration_ms: u64,
    slot_count: usize,
    pcc_max_input_quanta_per_slot: u64,
    total_source_energy_budget_wh: u64,
    source_energy_wh_per_quantum_slot: u64,
    maximum_search_nodes: u64,
    channels: Vec<Channel>,
    sessions: Vec<Session>,
}

fn parse_scenario(value: &Value) -> Result<Scenario, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "episode_id",
            "evidence_class",
            "quality_flags",
            "site",
            "time_grid",
            "channels",
            "sessions",
            "policy",
        ],
        "$",
    )?;

    if string_field(value, "schema_version", "$")? != "ground-air-coexistence-scenario-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let episode_id = string_field(value, "episode_id", "$")?.to_string();
    if string_field(value, "evidence_class", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_class",
            "v0 accepts synthetic known-truth scenarios only",
        ));
    }
    let quality_flags = array_field(value, "quality_flags", "$")?;
    if quality_flags.len() != 1 || quality_flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 requires exactly SYNTHETIC_ONLY",
        ));
    }

    let site = field(value, "site", "$")?;
    exact_keys(
        site,
        &[
            "site_id",
            "grid_source_id",
            "pcc_max_input_quanta_per_slot",
            "total_source_energy_budget_wh",
            "source_energy_wh_per_quantum_slot",
        ],
        "$.site",
    )?;
    let site_id = string_field(site, "site_id", "$.site")?.to_string();
    let grid_source_id = string_field(site, "grid_source_id", "$.site")?.to_string();
    let pcc_max_input_quanta_per_slot =
        uint_field(site, "pcc_max_input_quanta_per_slot", "$.site")?;
    let total_source_energy_budget_wh =
        uint_field(site, "total_source_energy_budget_wh", "$.site")?;
    let source_energy_wh_per_quantum_slot =
        uint_field(site, "source_energy_wh_per_quantum_slot", "$.site")?;
    if pcc_max_input_quanta_per_slot == 0
        || pcc_max_input_quanta_per_slot > MAX_POWER_QUANTA
    {
        return Err(fail(
            "$.site.pcc_max_input_quanta_per_slot",
            format!("must be within 1..={MAX_POWER_QUANTA}"),
        ));
    }
    if source_energy_wh_per_quantum_slot == 0 || total_source_energy_budget_wh == 0 {
        return Err(fail(
            "$.site",
            "source energy quantum and total source-energy budget must be positive",
        ));
    }
    if total_source_energy_budget_wh > MAX_ENERGY_WH
        || total_source_energy_budget_wh % source_energy_wh_per_quantum_slot != 0
    {
        return Err(fail(
            "$.site.total_source_energy_budget_wh",
            "must be bounded and exactly divisible by source_energy_wh_per_quantum_slot",
        ));
    }

    let time_grid = field(value, "time_grid", "$")?;
    exact_keys(
        time_grid,
        &["slot_duration_ms", "slot_count"],
        "$.time_grid",
    )?;
    let slot_duration_ms = uint_field(time_grid, "slot_duration_ms", "$.time_grid")?;
    let slot_count = usize::try_from(uint_field(time_grid, "slot_count", "$.time_grid")?)
        .map_err(|_| fail("$.time_grid.slot_count", "exceeds platform index range"))?;
    if slot_duration_ms == 0 || !(2..=MAX_SLOTS).contains(&slot_count) {
        return Err(fail(
            "$.time_grid",
            format!("slot_duration_ms must be positive and slot_count within 2..={MAX_SLOTS}"),
        ));
    }

    let channel_rows = array_field(value, "channels", "$")?;
    if !(2..=MAX_CHANNELS).contains(&channel_rows.len()) {
        return Err(fail(
            "$.channels",
            format!("must contain 2..={MAX_CHANNELS} channels"),
        ));
    }
    let mut channels = Vec::with_capacity(channel_rows.len());
    let mut channel_ids = BTreeSet::new();
    for (index, row) in channel_rows.iter().enumerate() {
        let path = format!("$.channels[{index}]");
        exact_keys(
            row,
            &[
                "channel_id",
                "domain",
                "modality",
                "maximum_input_quanta_per_slot",
                "envelope_sha256",
                "support_state",
                "coexistence_state",
                "safety_state",
            ],
            &path,
        )?;
        let channel_id = string_field(row, "channel_id", &path)?.to_string();
        if !channel_ids.insert(channel_id.clone()) {
            return Err(fail(&path, format!("duplicate channel_id {channel_id}")));
        }
        let domain = parse_domain(string_field(row, "domain", &path)?, &path)?;
        let modality = string_field(row, "modality", &path)?;
        if modality_domain(modality, &format!("{path}.modality"))? != domain {
            return Err(fail(
                &path,
                "channel domain conflicts with its declared transfer modality",
            ));
        }
        let maximum_input_quanta_per_slot =
            uint_field(row, "maximum_input_quanta_per_slot", &path)?;
        if maximum_input_quanta_per_slot == 0
            || maximum_input_quanta_per_slot > MAX_POWER_QUANTA
        {
            return Err(fail(
                &path,
                format!("maximum_input_quanta_per_slot must be within 1..={MAX_POWER_QUANTA}"),
            ));
        }
        let envelope_sha256 = string_field(row, "envelope_sha256", &path)?.to_string();
        if !valid_sha256(&envelope_sha256) {
            return Err(fail(
                &format!("{path}.envelope_sha256"),
                "must be lowercase hexadecimal SHA-256",
            ));
        }
        if string_field(row, "support_state", &path)? != "DECLARED_SYNTHETIC_SUPPORT"
            || string_field(row, "coexistence_state", &path)?
                != "NOT_EVALUATED_SYNTHETIC_ONLY"
            || string_field(row, "safety_state", &path)?
                != "NOT_EVALUATED_SYNTHETIC_ONLY"
        {
            return Err(fail(
                &path,
                "v0 accepts only declared synthetic support; physical safety and coexistence remain NOT_EVALUATED",
            ));
        }
        channels.push(Channel {
            channel_id,
            domain,
            maximum_input_quanta_per_slot,
            envelope_sha256,
        });
    }
    channels.sort_by(|left, right| left.channel_id.cmp(&right.channel_id));

    let session_rows = array_field(value, "sessions", "$")?;
    if !(2..=MAX_SESSIONS).contains(&session_rows.len()) {
        return Err(fail(
            "$.sessions",
            format!("must contain 2..={MAX_SESSIONS} sessions"),
        ));
    }
    let mut sessions = Vec::with_capacity(session_rows.len());
    let mut session_ids = BTreeSet::new();
    for (index, row) in session_rows.iter().enumerate() {
        let path = format!("$.sessions[{index}]");
        exact_keys(
            row,
            &[
                "session_id",
                "domain",
                "receiver_class",
                "channel_id",
                "start_slot",
                "end_slot",
                "requested_useful_energy_wh",
                "minimum_useful_energy_wh",
                "maximum_input_quanta_per_slot",
                "useful_energy_wh_per_input_quantum_slot",
                "authorization_state",
            ],
            &path,
        )?;
        let session_id = string_field(row, "session_id", &path)?.to_string();
        if !session_ids.insert(session_id.clone()) {
            return Err(fail(&path, format!("duplicate session_id {session_id}")));
        }
        let domain = parse_domain(string_field(row, "domain", &path)?, &path)?;
        let receiver_domain = receiver_class_domain(
            string_field(row, "receiver_class", &path)?,
            &format!("{path}.receiver_class"),
        )?;
        if receiver_domain != domain {
            return Err(fail(
                &path,
                "receiver class conflicts with declared session domain",
            ));
        }
        if string_field(row, "authorization_state", &path)? != "SYNTHETIC_FIXTURE_ONLY" {
            return Err(fail(
                &path,
                "v0 cannot authorize physical receiver sessions",
            ));
        }
        let channel_id = string_field(row, "channel_id", &path)?;
        let channel_index = channels
            .iter()
            .position(|channel| channel.channel_id == channel_id)
            .ok_or_else(|| fail(&path, format!("unknown channel_id {channel_id}")))?;
        if channels[channel_index].domain != domain {
            return Err(fail(
                &path,
                "session domain differs from its channel domain",
            ));
        }

        let start_slot = usize::try_from(uint_field(row, "start_slot", &path)?)
            .map_err(|_| fail(&path, "start_slot exceeds platform index range"))?;
        let end_slot = usize::try_from(uint_field(row, "end_slot", &path)?)
            .map_err(|_| fail(&path, "end_slot exceeds platform index range"))?;
        if start_slot >= end_slot || end_slot > slot_count {
            return Err(fail(
                &path,
                "session window must satisfy start_slot < end_slot <= slot_count",
            ));
        }

        let requested_useful_energy_wh =
            uint_field(row, "requested_useful_energy_wh", &path)?;
        let minimum_useful_energy_wh =
            uint_field(row, "minimum_useful_energy_wh", &path)?;
        let maximum_input_quanta_per_slot =
            uint_field(row, "maximum_input_quanta_per_slot", &path)?;
        let useful_energy_wh_per_input_quantum_slot =
            uint_field(row, "useful_energy_wh_per_input_quantum_slot", &path)?;
        if requested_useful_energy_wh == 0
            || requested_useful_energy_wh > MAX_ENERGY_WH
            || minimum_useful_energy_wh == 0
            || minimum_useful_energy_wh > requested_useful_energy_wh
        {
            return Err(fail(
                &path,
                "service request and minimum floor must satisfy 1 <= minimum <= requested <= MAX_ENERGY_WH",
            ));
        }
        if maximum_input_quanta_per_slot == 0
            || maximum_input_quanta_per_slot > MAX_POWER_QUANTA
            || useful_energy_wh_per_input_quantum_slot == 0
            || useful_energy_wh_per_input_quantum_slot > source_energy_wh_per_quantum_slot
        {
            return Err(fail(
                &path,
                "session power cap and useful-energy coefficient are outside the bounded energy contract",
            ));
        }

        sessions.push(Session {
            session_id,
            domain,
            channel_index,
            start_slot,
            end_slot,
            requested_useful_energy_wh,
            minimum_useful_energy_wh,
            maximum_input_quanta_per_slot,
            useful_energy_wh_per_input_quantum_slot,
        });
    }
    sessions.sort_by(|left, right| left.session_id.cmp(&right.session_id));

    let has_ground = sessions.iter().any(|session| session.domain == Domain::Ground);
    let has_air = sessions.iter().any(|session| session.domain == Domain::Air);
    if !has_ground || !has_air {
        return Err(fail(
            "$.sessions",
            "a coexistence episode requires at least one ground and one air session",
        ));
    }
    let has_temporal_overlap = sessions.iter().any(|ground| {
        ground.domain == Domain::Ground
            && sessions.iter().any(|air| {
                air.domain == Domain::Air
                    && ground.start_slot.max(air.start_slot)
                        < ground.end_slot.min(air.end_slot)
            })
    });
    if !has_temporal_overlap {
        return Err(fail(
            "$.sessions",
            "no ground/air session windows overlap; simultaneous delivery is impossible",
        ));
    }

    let policy = field(value, "policy", "$")?;
    exact_keys(
        policy,
        &["maximum_search_nodes", "require_concurrent_delivery"],
        "$.policy",
    )?;
    let maximum_search_nodes = uint_field(policy, "maximum_search_nodes", "$.policy")?;
    if maximum_search_nodes == 0 || maximum_search_nodes > MAX_SEARCH_NODES {
        return Err(fail(
            "$.policy.maximum_search_nodes",
            format!("must be within 1..={MAX_SEARCH_NODES}"),
        ));
    }
    if field(policy, "require_concurrent_delivery", "$.policy")?.as_bool() != Some(true) {
        return Err(fail(
            "$.policy.require_concurrent_delivery",
            "v0 requires actual simultaneous ground-and-air delivery",
        ));
    }

    Ok(Scenario {
        episode_id,
        site_id,
        grid_source_id,
        slot_duration_ms,
        slot_count,
        pcc_max_input_quanta_per_slot,
        total_source_energy_budget_wh,
        source_energy_wh_per_quantum_slot,
        maximum_search_nodes,
        channels,
        sessions,
    })
}

#[derive(Debug, Clone)]
struct Candidate {
    allocations: Vec<Vec<u64>>,
    delivered_useful_energy_wh: Vec<u64>,
    source_quanta_used: u64,
    simultaneous_delivery_slot_count: usize,
}

struct ExactSearch<'a> {
    scenario: &'a Scenario,
    maximum_source_quanta: u64,
    search_nodes: u64,
    best: Option<Candidate>,
}

impl<'a> ExactSearch<'a> {
    fn bump_node(&mut self) -> Result<(), String> {
        self.search_nodes += 1;
        if self.search_nodes > self.scenario.maximum_search_nodes {
            return Err(
                "search-node bound exhausted; no optimality or partial-success claim is emitted"
                    .into(),
            );
        }
        Ok(())
    }

    fn search_slots(
        &mut self,
        slot: usize,
        delivered: &mut Vec<u64>,
        allocations: &mut Vec<Vec<u64>>,
        source_quanta_used: u64,
        simultaneous_delivery_slot_count: usize,
    ) -> Result<(), String> {
        self.bump_node()?;
        if slot == self.scenario.slot_count {
            if simultaneous_delivery_slot_count == 0 {
                return Ok(());
            }
            let candidate = Candidate {
                allocations: allocations.clone(),
                delivered_useful_energy_wh: delivered.clone(),
                source_quanta_used,
                simultaneous_delivery_slot_count,
            };
            let replace = match self.best.as_ref() {
                None => true,
                Some(best) => candidate_is_better(&candidate, best, self.scenario),
            };
            if replace {
                self.best = Some(candidate);
            }
            return Ok(());
        }

        let mut slot_allocations = vec![0_u64; self.scenario.sessions.len()];
        let mut channel_usage = vec![0_u64; self.scenario.channels.len()];
        self.enumerate_sessions(
            slot,
            0,
            &mut slot_allocations,
            &mut channel_usage,
            0,
            delivered,
            allocations,
            source_quanta_used,
            simultaneous_delivery_slot_count,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn enumerate_sessions(
        &mut self,
        slot: usize,
        session_index: usize,
        slot_allocations: &mut Vec<u64>,
        channel_usage: &mut Vec<u64>,
        slot_input_quanta: u64,
        delivered: &mut Vec<u64>,
        allocations: &mut Vec<Vec<u64>>,
        source_quanta_used: u64,
        simultaneous_delivery_slot_count: usize,
    ) -> Result<(), String> {
        self.bump_node()?;
        if session_index == self.scenario.sessions.len() {
            let mut ground_quanta = 0_u64;
            let mut air_quanta = 0_u64;
            for (index, session) in self.scenario.sessions.iter().enumerate() {
                match session.domain {
                    Domain::Ground => ground_quanta += slot_allocations[index],
                    Domain::Air => air_quanta += slot_allocations[index],
                }
            }
            let concurrent_now = ground_quanta > 0 && air_quanta > 0;
            allocations.push(slot_allocations.clone());
            let concurrent_count = simultaneous_delivery_slot_count
                + if concurrent_now { 1 } else { 0 };
            let result = self.search_slots(
                slot + 1,
                delivered,
                allocations,
                source_quanta_used + slot_input_quanta,
                concurrent_count,
            );
            allocations.pop();
            return result;
        }

        let session = &self.scenario.sessions[session_index];
        let channel = &self.scenario.channels[session.channel_index];
        let active = session.start_slot <= slot && slot < session.end_slot;
        let remaining_useful_wh =
            session.requested_useful_energy_wh - delivered[session_index];
        let service_quanta_cap =
            remaining_useful_wh / session.useful_energy_wh_per_input_quantum_slot;
        let site_quanta_remaining =
            self.scenario.pcc_max_input_quanta_per_slot - slot_input_quanta;
        let channel_quanta_remaining =
            channel.maximum_input_quanta_per_slot - channel_usage[session.channel_index];
        let source_budget_remaining = self.maximum_source_quanta
            .saturating_sub(source_quanta_used + slot_input_quanta);
        let maximum_quanta = if active {
            session
                .maximum_input_quanta_per_slot
                .min(channel_quanta_remaining)
                .min(site_quanta_remaining)
                .min(service_quanta_cap)
                .min(source_budget_remaining)
        } else {
            0
        };
        let energy_coefficient = session.useful_energy_wh_per_input_quantum_slot;
        let channel_index = session.channel_index;

        for input_quanta in 0..=maximum_quanta {
            slot_allocations[session_index] = input_quanta;
            channel_usage[channel_index] += input_quanta;
            let useful_wh = input_quanta * energy_coefficient;
            delivered[session_index] += useful_wh;

            let result = self.enumerate_sessions(
                slot,
                session_index + 1,
                slot_allocations,
                channel_usage,
                slot_input_quanta + input_quanta,
                delivered,
                allocations,
                source_quanta_used,
                simultaneous_delivery_slot_count,
            );

            delivered[session_index] -= useful_wh;
            channel_usage[channel_index] -= input_quanta;
            slot_allocations[session_index] = 0;
            result?;
        }
        Ok(())
    }
}

fn ratio_cmp(left: (u64, u64), right: (u64, u64)) -> Ordering {
    let left_product = u128::from(left.0) * u128::from(right.1);
    let right_product = u128::from(right.0) * u128::from(left.1);
    left_product.cmp(&right_product)
}

fn sorted_service_ratios(candidate: &Candidate, scenario: &Scenario) -> Vec<(u64, u64)> {
    let mut ratios = candidate
        .delivered_useful_energy_wh
        .iter()
        .zip(&scenario.sessions)
        .map(|(delivered, session)| (*delivered, session.requested_useful_energy_wh))
        .collect::<Vec<_>>();
    ratios.sort_by(|left, right| ratio_cmp(*left, *right));
    ratios
}

fn minimum_floor_count(candidate: &Candidate, scenario: &Scenario) -> usize {
    candidate
        .delivered_useful_energy_wh
        .iter()
        .zip(&scenario.sessions)
        .filter(|(delivered, session)| **delivered >= session.minimum_useful_energy_wh)
        .count()
}

fn total_useful_energy(candidate: &Candidate) -> u64 {
    candidate.delivered_useful_energy_wh.iter().sum()
}

fn candidate_is_better(left: &Candidate, right: &Candidate, scenario: &Scenario) -> bool {
    let left_floor_count = minimum_floor_count(left, scenario);
    let right_floor_count = minimum_floor_count(right, scenario);
    if left_floor_count != right_floor_count {
        return left_floor_count > right_floor_count;
    }

    let left_ratios = sorted_service_ratios(left, scenario);
    let right_ratios = sorted_service_ratios(right, scenario);
    for (left_ratio, right_ratio) in left_ratios.iter().zip(&right_ratios) {
        match ratio_cmp(*left_ratio, *right_ratio) {
            Ordering::Greater => return true,
            Ordering::Less => return false,
            Ordering::Equal => {}
        }
    }

    let left_total = total_useful_energy(left);
    let right_total = total_useful_energy(right);
    if left_total != right_total {
        return left_total > right_total;
    }
    if left.source_quanta_used != right.source_quanta_used {
        return left.source_quanta_used < right.source_quanta_used;
    }

    // Stable tie-break: session rows are sorted by ID before search.
    left.allocations < right.allocations
}

fn build_report(scenario: &Scenario, search: &ExactSearch<'_>) -> Result<Value, String> {
    let best = search.best.as_ref().ok_or_else(|| {
        "no allocation satisfies simultaneous ground/air delivery and source limits".to_string()
    })?;
    let floor_count = minimum_floor_count(best, scenario);
    let minimum_service_all_met = floor_count == scenario.sessions.len();
    let total_useful_wh = total_useful_energy(best);
    let source_energy_wh =
        best.source_quanta_used * scenario.source_energy_wh_per_quantum_slot;
    let service_to_source_ratio_ppm = if source_energy_wh > 0 {
        ((u128::from(total_useful_wh) * 1_000_000_u128) / u128::from(source_energy_wh)) as u64
    } else {
        0
    };

    let mut maximum_pcc_used = 0_u64;
    let mut per_slot = Vec::with_capacity(scenario.slot_count);
    for (slot_index, slot_allocations) in best.allocations.iter().enumerate() {
        let mut total_input = 0_u64;
        let mut ground_input = 0_u64;
        let mut air_input = 0_u64;
        let mut session_rows = Vec::with_capacity(scenario.sessions.len());
        for (session_index, input_quanta) in slot_allocations.iter().enumerate() {
            total_input += input_quanta;
            match scenario.sessions[session_index].domain {
                Domain::Ground => ground_input += input_quanta,
                Domain::Air => air_input += input_quanta,
            }
            session_rows.push(json!({
                "session_id": scenario.sessions[session_index].session_id,
                "input_power_quanta": input_quanta
            }));
        }
        maximum_pcc_used = maximum_pcc_used.max(total_input);
        per_slot.push(json!({
            "slot_index": slot_index,
            "total_input_power_quanta": total_input,
            "ground_input_power_quanta": ground_input,
            "air_input_power_quanta": air_input,
            "source_energy_wh": total_input * scenario.source_energy_wh_per_quantum_slot,
            "simultaneous_ground_air_delivery": ground_input > 0 && air_input > 0,
            "session_allocations": session_rows
        }));
    }

    let mut per_session = Vec::with_capacity(scenario.sessions.len());
    for (index, session) in scenario.sessions.iter().enumerate() {
        let delivered = best.delivered_useful_energy_wh[index];
        let ratio_ppm = ((u128::from(delivered) * 1_000_000_u128)
            / u128::from(session.requested_useful_energy_wh)) as u64;
        per_session.push(json!({
            "session_id": session.session_id,
            "domain": session.domain.as_str(),
            "channel_id": scenario.channels[session.channel_index].channel_id,
            "envelope_sha256": scenario.channels[session.channel_index].envelope_sha256,
            "requested_useful_energy_wh": session.requested_useful_energy_wh,
            "minimum_useful_energy_wh": session.minimum_useful_energy_wh,
            "delivered_useful_energy_wh": delivered,
            "minimum_service_met": delivered >= session.minimum_useful_energy_wh,
            "service_fulfillment_ratio_ppm": ratio_ppm,
            "input_power_quanta_by_slot": best.allocations.iter().map(|slot| slot[index]).collect::<Vec<_>>()
        }));
    }

    let disposition = if minimum_service_all_met {
        "PASS_SYNTHETIC_OPTIMAL"
    } else {
        "MINIMUM_SERVICE_INFEASIBLE_BEST_EFFORT"
    };
    Ok(json!({
        "schema_version": "ground-air-coexistence-report-v0",
        "episode_id": scenario.episode_id,
        "evidence_class": "SYNTHETIC_KNOWN_TRUTH",
        "disposition": disposition,
        "exact_bounded_optimum": true,
        "site_id": scenario.site_id,
        "grid_source_id": scenario.grid_source_id,
        "slot_duration_ms": scenario.slot_duration_ms,
        "slot_count": scenario.slot_count,
        "pcc_max_input_quanta_per_slot": scenario.pcc_max_input_quanta_per_slot,
        "maximum_pcc_input_quanta_used": maximum_pcc_used,
        "total_source_energy_budget_wh": scenario.total_source_energy_budget_wh,
        "total_source_energy_wh": source_energy_wh,
        "total_source_input_quanta_slots": best.source_quanta_used,
        "total_useful_energy_wh": total_useful_wh,
        "synthetic_useful_to_source_energy_ratio_ppm": service_to_source_ratio_ppm,
        "minimum_service_met_count": floor_count,
        "minimum_service_required_count": scenario.sessions.len(),
        "minimum_service_all_met": minimum_service_all_met,
        "simultaneous_delivery_slot_count": best.simultaneous_delivery_slot_count,
        "search_nodes": search.search_nodes,
        "objective_order": [
            "maximize_sessions_meeting_minimum_service",
            "leximin_service_fulfillment_ratio",
            "maximize_total_useful_energy",
            "minimize_source_energy",
            "stable_lexicographic_schedule"
        ],
        "per_slot": per_slot,
        "per_session": per_session,
        "claim_ceiling": "Synthetic bounded source-allocation arithmetic only. Supplied coefficients and declared synthetic states do not validate physical coexistence, RF/optical/inductive/capacitive safety, EMC, aircraft operation, converter stability or field performance."
    }))
}

/// Evaluate one synthetic site episode using bounded exhaustive integer search.
///
/// A search bound failure returns an error. The function never returns a
/// partial schedule while claiming exact optimality.
pub fn evaluate_scenario(value: &Value) -> Result<Value, String> {
    let scenario = parse_scenario(value)?;
    let maximum_source_quanta =
        scenario.total_source_energy_budget_wh / scenario.source_energy_wh_per_quantum_slot;
    let mut search = ExactSearch {
        scenario: &scenario,
        maximum_source_quanta,
        search_nodes: 0,
        best: None,
    };
    let mut delivered = vec![0_u64; scenario.sessions.len()];
    let mut allocations = Vec::with_capacity(scenario.slot_count);
    search.search_slots(0, &mut delivered, &mut allocations, 0, 0)?;
    build_report(&scenario, &search)
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/ground-air-energy-coexistence-v0/scenario.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic ground-air coexistence scenario JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn known_truth_finds_exact_concurrent_plan_and_meets_both_service_floors() {
        let report = evaluate_scenario(&fixture()).unwrap();
        assert_eq!(report["disposition"], "PASS_SYNTHETIC_OPTIMAL");
        assert_eq!(report["exact_bounded_optimum"], true);
        assert_eq!(report["total_useful_energy_wh"], 28);
        assert_eq!(report["total_source_energy_wh"], 80);
        assert_eq!(report["minimum_service_met_count"], 2);
        assert_eq!(report["minimum_service_all_met"], true);
        assert!(report["simultaneous_delivery_slot_count"].as_u64().unwrap() > 0);
        assert!(report["maximum_pcc_input_quanta_used"].as_u64().unwrap() <= 3);
    }

    #[test]
    fn channel_and_session_row_order_does_not_change_the_report() {
        let original = evaluate_scenario(&fixture()).unwrap();
        let mut reordered = fixture();
        reordered["channels"].as_array_mut().unwrap().reverse();
        reordered["sessions"].as_array_mut().unwrap().reverse();
        let replayed = evaluate_scenario(&reordered).unwrap();
        assert_eq!(original, replayed);
    }

    #[test]
    fn insufficient_source_budget_returns_best_effort_without_false_feasibility() {
        let mut scenario = fixture();
        scenario["site"]["total_source_energy_budget_wh"] = Value::from(20);
        let report = evaluate_scenario(&scenario).unwrap();
        assert_eq!(report["disposition"], "MINIMUM_SERVICE_INFEASIBLE_BEST_EFFORT");
        assert_eq!(report["minimum_service_all_met"], false);
        assert!(report["simultaneous_delivery_slot_count"].as_u64().unwrap() > 0);
        assert!(report["total_source_energy_wh"].as_u64().unwrap() <= 20);
    }

    #[test]
    fn unresolved_safety_or_coexistence_state_is_rejected_not_promoted() {
        let mut scenario = fixture();
        scenario["channels"][0]["safety_state"] = Value::String("UNKNOWN".into());
        assert!(evaluate_scenario(&scenario).unwrap_err().contains("NOT_EVALUATED"));
    }

    #[test]
    fn channel_modality_and_receiver_domain_must_match() {
        let mut scenario = fixture();
        scenario["channels"][0]["domain"] = Value::String("GROUND".into());
        assert!(evaluate_scenario(&scenario).unwrap_err().contains("modality"));

        let mut scenario = fixture();
        scenario["sessions"][0]["domain"] = Value::String("GROUND".into());
        assert!(evaluate_scenario(&scenario).unwrap_err().contains("channel domain"));
    }

    #[test]
    fn non_overlapping_ground_and_air_windows_are_rejected() {
        let mut scenario = fixture();
        scenario["sessions"][0]["start_slot"] = Value::from(0);
        scenario["sessions"][0]["end_slot"] = Value::from(2);
        scenario["sessions"][1]["start_slot"] = Value::from(2);
        scenario["sessions"][1]["end_slot"] = Value::from(4);
        assert!(evaluate_scenario(&scenario).unwrap_err().contains("windows overlap"));
    }

    #[test]
    fn search_bound_exhaustion_never_emits_a_partial_optimum() {
        let mut scenario = fixture();
        scenario["policy"]["maximum_search_nodes"] = Value::from(1);
        assert!(evaluate_scenario(&scenario).unwrap_err().contains("bound exhausted"));
    }

    #[test]
    fn useful_energy_cannot_exceed_the_declared_source_energy_quantum() {
        let mut scenario = fixture();
        scenario["sessions"][0]["useful_energy_wh_per_input_quantum_slot"] = Value::from(11);
        assert!(evaluate_scenario(&scenario).unwrap_err().contains("energy contract"));
    }

    #[test]
    fn unknown_fields_and_non_synthetic_evidence_fail_closed() {
        let mut scenario = fixture();
        scenario["unmodelled_override"] = Value::Bool(true);
        assert!(evaluate_scenario(&scenario).unwrap_err().contains("unknown property"));

        let mut scenario = fixture();
        scenario["evidence_class"] = Value::String("LAB_MEASURED".into());
        assert!(evaluate_scenario(&scenario).unwrap_err().contains("synthetic known-truth"));
    }
}
