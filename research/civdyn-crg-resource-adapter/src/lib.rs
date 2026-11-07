// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Open-loop adapter from the existing collective-risk action trace to the
//! existing unit-separated socio-ecological resource ledger.
//!
//! The bridge intentionally does not feed ledger state back into policy
//! selection. One contribution unit maps to one kWh only inside the frozen
//! synthetic scenario manifest; it is not a physical conversion law.

use core::fmt;

use civdyn_collective_risk::{
    ACTIONS, ENDOWMENT, EpisodeOutcome, GameConfig, GameError, ROUNDS, SEATS, Strategy, TARGET,
    run_episode,
};
use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, StepReport, TransferPolicy, TransferRequest,
    World, WorldError,
};

pub const DONOR_REGIONS: usize = SEATS;
pub const SHARED_REGION: usize = DONOR_REGIONS;
pub const REGION_COUNT: usize = DONOR_REGIONS + 1;
pub const TICKS: usize = ROUNDS;
pub const CONTRIBUTION_UNIT_KWH: f64 = 1.0;
pub const DONOR_OPENING_KWH: f64 = 50.0;
pub const DONOR_DEMAND_KWH_PER_TICK: f64 = 1.0;
pub const SHARED_POPULATION: f64 = 12.0;
pub const SHARED_DEMAND_KWH_PER_PERSON_TICK: f64 = 1.0;
pub const RISK_GRID: [f64; 3] = [0.1, 0.5, 0.9];

const CSV_HEADER: &str = "record_type,scenario_id,risk,roster_id,agent_id,tick,policy,action_units,requested_kwh,delivered_kwh,donor_demand_kwh,donor_served_kwh,donor_unmet_kwh,donor_closing_kwh,shared_demand_kwh,shared_served_kwh,shared_unmet_kwh,shared_closing_kwh,game_total_contribution_units,game_target_reached,game_expected_group_welfare,game_payoff_gini,water_residual_m3,energy_residual_kwh,food_residual_kg";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyRoster {
    pub id: &'static str,
    pub strategies: [Strategy; SEATS],
}

pub fn standard_rosters() -> [PolicyRoster; 5] {
    [
        PolicyRoster {
            id: "all_zero",
            strategies: [Strategy::AlwaysZero; SEATS],
        },
        PolicyRoster {
            id: "all_fixed_fair_share",
            strategies: [Strategy::FixedFairShare; SEATS],
        },
        PolicyRoster {
            id: "all_fixed_maximum",
            strategies: [Strategy::FixedMaximum; SEATS],
        },
        PolicyRoster {
            id: "all_risk_pivot",
            strategies: [Strategy::RiskPivot; SEATS],
        },
        PolicyRoster {
            id: "heterogeneous_six_policy",
            strategies: [
                Strategy::AlwaysZero,
                Strategy::FixedFairShare,
                Strategy::FixedMaximum,
                Strategy::RiskPivot,
                Strategy::LastRoundDefection,
                Strategy::CopyPreviousGroupMean,
            ],
        },
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct BridgeEpisode {
    pub scenario_id: String,
    pub risk: f64,
    pub roster_id: &'static str,
    pub strategies: [Strategy; SEATS],
    pub game: EpisodeOutcome,
    pub action_history: Vec<[u8; SEATS]>,
    pub ticks: Vec<StepReport>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeError {
    Game(GameError),
    Ledger(WorldError),
    Invariant(&'static str),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Game(error) => write!(f, "collective-risk reference failed: {error}"),
            Self::Ledger(error) => write!(f, "resource-ledger replay failed: {error}"),
            Self::Invariant(message) => write!(f, "bridge invariant failed: {message}"),
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<GameError> for BridgeError {
    fn from(value: GameError) -> Self {
        Self::Game(value)
    }
}

impl From<WorldError> for BridgeError {
    fn from(value: WorldError) -> Self {
        Self::Ledger(value)
    }
}

/// Run the unchanged collective-risk policies, then replay their exact action
/// trace through the resource ledger. No policy receives resource state.
pub fn run_case(risk: f64, roster: PolicyRoster) -> Result<BridgeEpisode, BridgeError> {
    let config = GameConfig {
        catastrophe_probability: risk,
    };
    let game = run_episode(config, &roster.strategies)?;
    let action_history = game.contribution_history.clone();

    let mut world = World::new(scenario_regions())?;
    let mut ticks = Vec::with_capacity(TICKS);

    for actions in &action_history {
        let transfers = actions
            .iter()
            .enumerate()
            .map(|(seat, action)| TransferRequest {
                from_region: seat,
                to_region: SHARED_REGION,
                amount: ResourceVector::new(0.0, f64::from(*action) * CONTRIBUTION_UNIT_KWH, 0.0),
            })
            .collect();

        let input = StepInput {
            production: vec![Production::default(); REGION_COUNT],
            transfers,
        };
        let report = world.step(&input, TransferPolicy::Unrestricted)?;
        ticks.push(report);
    }

    let episode = BridgeEpisode {
        scenario_id: format!("risk-{risk:.1}-{}", roster.id),
        risk,
        roster_id: roster.id,
        strategies: roster.strategies,
        game,
        action_history,
        ticks,
    };
    validate_episode(&episode)?;
    Ok(episode)
}

/// Run all five frozen policy rosters at low, pivot and high risk.
pub fn run_standard_matrix() -> Result<Vec<BridgeEpisode>, BridgeError> {
    let mut episodes = Vec::with_capacity(standard_rosters().len() * RISK_GRID.len());
    for risk in RISK_GRID {
        for roster in standard_rosters() {
            episodes.push(run_case(risk, roster)?);
        }
    }
    Ok(episodes)
}

fn scenario_regions() -> Vec<Region> {
    let mut regions = (0..DONOR_REGIONS)
        .map(|seat| Region {
            id: format!("agent_{seat}"),
            population: 1.0,
            opening_stock: ResourceVector::new(0.0, DONOR_OPENING_KWH, 0.0),
            essential_demand_per_capita: ResourceVector::new(0.0, DONOR_DEMAND_KWH_PER_TICK, 0.0),
        })
        .collect::<Vec<_>>();

    regions.push(Region {
        id: "shared_service".to_owned(),
        population: SHARED_POPULATION,
        opening_stock: ResourceVector::default(),
        essential_demand_per_capita: ResourceVector::new(
            0.0,
            SHARED_DEMAND_KWH_PER_PERSON_TICK,
            0.0,
        ),
    });
    regions
}

fn close_enough(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9
}

fn validate_episode(episode: &BridgeEpisode) -> Result<(), BridgeError> {
    if episode.action_history.len() != TICKS || episode.ticks.len() != TICKS {
        return Err(BridgeError::Invariant(
            "the CRG trace and ledger replay must both contain exactly ten ticks",
        ));
    }

    let mut contributions_by_seat = [0_u32; SEATS];
    let mut total_contribution = 0_u32;

    for (tick_index, actions) in episode.action_history.iter().enumerate() {
        let report = &episode.ticks[tick_index];
        if report.tick != (tick_index + 1) as u64 {
            return Err(BridgeError::Invariant(
                "ledger tick numbers are not contiguous",
            ));
        }
        if report.regions.len() != REGION_COUNT || report.transfers.len() != DONOR_REGIONS {
            return Err(BridgeError::Invariant(
                "each ledger tick must contain seven regions and six donor receipts",
            ));
        }

        let mut tick_transfer_total = 0.0;
        for (seat, &action) in actions.iter().enumerate() {
            if !ACTIONS.contains(&action) {
                return Err(BridgeError::Invariant(
                    "CRG trace contains a noncanonical action",
                ));
            }
            contributions_by_seat[seat] += u32::from(action);
            if contributions_by_seat[seat] > ENDOWMENT {
                return Err(BridgeError::Invariant(
                    "CRG trace exceeds a seat's cumulative endowment",
                ));
            }
            total_contribution += u32::from(action);

            let receipt = report.transfers[seat];
            if receipt.request_index != seat
                || receipt.from_region != seat
                || receipt.to_region != SHARED_REGION
                || !close_enough(
                    receipt.requested.energy_kwh,
                    f64::from(action) * CONTRIBUTION_UNIT_KWH,
                )
                || !close_enough(receipt.requested.water_m3, 0.0)
                || !close_enough(receipt.requested.food_kg, 0.0)
            {
                return Err(BridgeError::Invariant(
                    "transfer receipt does not match the source agent's action",
                ));
            }
            if receipt.delivered.energy_kwh < 0.0
                || receipt.delivered.energy_kwh > receipt.requested.energy_kwh + 1e-9
            {
                return Err(BridgeError::Invariant(
                    "delivered energy is outside the requested transfer interval",
                ));
            }
            tick_transfer_total += receipt.delivered.energy_kwh;

            let donor = &report.regions[seat];
            if donor.region_id != format!("agent_{seat}")
                || !close_enough(donor.essential_demand.energy_kwh, DONOR_DEMAND_KWH_PER_TICK)
                || !close_enough(donor.served.energy_kwh, DONOR_DEMAND_KWH_PER_TICK)
                || !close_enough(donor.unmet.energy_kwh, 0.0)
                || donor.closing_stock.energy_kwh < -1e-9
                || !donor.ledger.is_conserved()
            {
                return Err(BridgeError::Invariant(
                    "donor identity, essential service or conservation check failed",
                ));
            }
        }

        if !close_enough(
            report.regions[SHARED_REGION].essential_demand.energy_kwh,
            SHARED_POPULATION * SHARED_DEMAND_KWH_PER_PERSON_TICK,
        ) {
            return Err(BridgeError::Invariant(
                "shared-service demand differs from the frozen 12 kWh per tick",
            ));
        }
        if !report.regions[SHARED_REGION].ledger.is_conserved() {
            return Err(BridgeError::Invariant(
                "shared-service resource ledger is not conserved",
            ));
        }
        let shared_imports = report.regions[SHARED_REGION].ledger.imports.energy_kwh;
        if !close_enough(shared_imports, tick_transfer_total) {
            return Err(BridgeError::Invariant(
                "shared-service imports do not equal delivered donor transfers",
            ));
        }
    }

    if contributions_by_seat != episode.game.contributions_by_seat
        || total_contribution != episode.game.total_contribution
    {
        return Err(BridgeError::Invariant(
            "independent action-trace totals disagree with the CRG episode outcome",
        ));
    }

    Ok(())
}

/// Render the full action/resource/equity trace as deterministic CSV.
pub fn render_csv(episodes: &[BridgeEpisode]) -> String {
    let mut output = String::new();
    output.push_str(CSV_HEADER);
    output.push('\n');

    for episode in episodes {
        let mut summary = blank_row();
        set_case_metadata(&mut summary, episode, "episode_summary", None);
        summary[18] = episode.game.total_contribution.to_string();
        summary[19] = episode.game.target_reached.to_string();
        summary[20] = format_float(episode.game.expected_group_welfare);
        summary[21] = format_float(episode.game.payoff_gini);
        push_row(&mut output, summary);

        for (tick_index, report) in episode.ticks.iter().enumerate() {
            for (seat, strategy) in episode.strategies.iter().enumerate() {
                let mut row = blank_row();
                set_case_metadata(&mut row, episode, "agent_tick", Some(tick_index + 1));
                row[4] = format!("agent_{seat}");
                row[6] = strategy.name().to_owned();
                let action = episode.action_history[tick_index][seat];
                let receipt = report.transfers[seat];
                let donor = &report.regions[seat];
                row[7] = action.to_string();
                row[8] = format_float(receipt.requested.energy_kwh);
                row[9] = format_float(receipt.delivered.energy_kwh);
                row[10] = format_float(donor.essential_demand.energy_kwh);
                row[11] = format_float(donor.served.energy_kwh);
                row[12] = format_float(donor.unmet.energy_kwh);
                row[13] = format_float(donor.closing_stock.energy_kwh);
                row[18] = episode.game.total_contribution.to_string();
                row[19] = episode.game.target_reached.to_string();
                row[20] = format_float(episode.game.expected_group_welfare);
                row[21] = format_float(episode.game.payoff_gini);
                set_residuals(&mut row, donor.ledger.conservation_residual());
                push_row(&mut output, row);
            }

            let mut shared = blank_row();
            set_case_metadata(&mut shared, episode, "shared_tick", Some(tick_index + 1));
            let service = &report.regions[SHARED_REGION];
            shared[14] = format_float(service.essential_demand.energy_kwh);
            shared[15] = format_float(service.served.energy_kwh);
            shared[16] = format_float(service.unmet.energy_kwh);
            shared[17] = format_float(service.closing_stock.energy_kwh);
            shared[18] = episode.game.total_contribution.to_string();
            shared[19] = episode.game.target_reached.to_string();
            shared[20] = format_float(episode.game.expected_group_welfare);
            shared[21] = format_float(episode.game.payoff_gini);
            set_residuals(&mut shared, service.ledger.conservation_residual());
            push_row(&mut output, shared);
        }
    }
    output
}

fn blank_row() -> [String; 25] {
    core::array::from_fn(|_| String::new())
}

fn set_case_metadata(
    row: &mut [String; 25],
    episode: &BridgeEpisode,
    record_type: &str,
    tick: Option<usize>,
) {
    row[0] = record_type.to_owned();
    row[1] = episode.scenario_id.clone();
    row[2] = format_float(episode.risk);
    row[3] = episode.roster_id.to_owned();
    if let Some(tick) = tick {
        row[5] = tick.to_string();
    }
}

fn set_residuals(row: &mut [String; 25], residual: ResourceVector) {
    row[22] = format_float(residual.water_m3);
    row[23] = format_float(residual.energy_kwh);
    row[24] = format_float(residual.food_kg);
}

fn format_float(value: f64) -> String {
    format!("{value:.9}")
}

fn push_row(output: &mut String, row: [String; 25]) {
    let encoded = row
        .into_iter()
        .map(csv_escape)
        .collect::<Vec<_>>()
        .join(",");
    output.push_str(&encoded);
    output.push('\n');
}

fn csv_escape(value: String) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roster(id: &str) -> PolicyRoster {
        standard_rosters()
            .into_iter()
            .find(|roster| roster.id == id)
            .expect("named reference roster must exist")
    }

    fn shared_total_unmet(episode: &BridgeEpisode) -> f64 {
        episode
            .ticks
            .iter()
            .map(|report| report.regions[SHARED_REGION].unmet.energy_kwh)
            .sum()
    }

    #[test]
    fn zero_transfer_negative_control_keeps_donors_served_and_shared_service_unserved() {
        let episode = run_case(0.1, roster("all_zero")).unwrap();
        assert_eq!(episode.game.total_contribution, 0);
        assert!(close_enough(shared_total_unmet(&episode), 120.0));
        for report in &episode.ticks {
            for donor in &report.regions[..DONOR_REGIONS] {
                assert!(close_enough(donor.served.energy_kwh, 1.0));
                assert!(close_enough(donor.unmet.energy_kwh, 0.0));
            }
            let shared = &report.regions[SHARED_REGION];
            assert!(close_enough(shared.served.energy_kwh, 0.0));
            assert!(close_enough(shared.unmet.energy_kwh, 12.0));
        }
    }

    #[test]
    fn fixed_fair_share_positive_control_meets_each_shared_service_tick_exactly() {
        let episode = run_case(0.5, roster("all_fixed_fair_share")).unwrap();
        assert_eq!(episode.game.total_contribution, TARGET);
        for report in &episode.ticks {
            let shared = &report.regions[SHARED_REGION];
            assert!(close_enough(shared.served.energy_kwh, 12.0));
            assert!(close_enough(shared.unmet.energy_kwh, 0.0));
            assert!(close_enough(shared.closing_stock.energy_kwh, 0.0));
        }
    }

    #[test]
    fn maximum_contribution_preserves_game_endowment_and_exposes_shared_surplus() {
        let episode = run_case(0.9, roster("all_fixed_maximum")).unwrap();
        assert_eq!(episode.game.total_contribution, 240);
        assert_eq!(episode.game.overshoot, 120);
        assert_eq!(episode.game.contributions_by_seat, [40; SEATS]);
        assert!(close_enough(shared_total_unmet(&episode), 0.0));
        assert!(close_enough(
            episode.ticks.last().unwrap().regions[SHARED_REGION]
                .closing_stock
                .energy_kwh,
            120.0,
        ));
        for report in &episode.ticks {
            for donor in &report.regions[..DONOR_REGIONS] {
                assert!(close_enough(donor.served.energy_kwh, 1.0));
                assert!(donor.closing_stock.energy_kwh >= 0.0);
            }
        }
    }

    #[test]
    fn risk_pivot_is_preserved_and_pivot_tie_is_not_forced_to_one_mode() {
        let low = run_case(0.1, roster("all_risk_pivot")).unwrap();
        let tie_zero = run_case(0.5, roster("all_zero")).unwrap();
        let tie_reach = run_case(0.5, roster("all_risk_pivot")).unwrap();
        let high = run_case(0.9, roster("all_risk_pivot")).unwrap();

        assert_eq!(low.game.total_contribution, 0);
        assert_eq!(tie_zero.game.total_contribution, 0);
        assert_eq!(tie_reach.game.total_contribution, TARGET);
        assert_eq!(high.game.total_contribution, TARGET);
        assert!(close_enough(
            tie_zero.game.expected_group_welfare,
            tie_reach.game.expected_group_welfare,
        ));
        assert!(!close_enough(
            shared_total_unmet(&tie_zero),
            shared_total_unmet(&tie_reach),
        ));
    }

    #[test]
    fn every_policy_roster_preserves_each_donors_essential_service() {
        for risk in RISK_GRID {
            for roster in standard_rosters() {
                let episode = run_case(risk, roster).unwrap();
                for report in &episode.ticks {
                    for donor in &report.regions[..DONOR_REGIONS] {
                        assert!(close_enough(donor.served.energy_kwh, 1.0));
                        assert!(close_enough(donor.unmet.energy_kwh, 0.0));
                        assert!(donor.closing_stock.energy_kwh >= 0.0);
                    }
                }
            }
        }
    }

    #[test]
    fn independent_trace_totals_and_all_region_ledgers_close() {
        for episode in run_standard_matrix().unwrap() {
            let mut total = 0_u32;
            let mut by_seat = [0_u32; SEATS];
            for (tick_index, actions) in episode.action_history.iter().enumerate() {
                for (seat, action) in actions.iter().copied().enumerate() {
                    total += u32::from(action);
                    by_seat[seat] += u32::from(action);
                }
                for region in &episode.ticks[tick_index].regions {
                    assert!(region.ledger.is_conserved());
                }
            }
            assert_eq!(total, episode.game.total_contribution);
            assert_eq!(by_seat, episode.game.contributions_by_seat);
        }
    }

    #[test]
    fn heterogeneous_roster_is_equivariant_under_seat_permutation() {
        let original_roster = roster("heterogeneous_six_policy");
        let original = run_case(0.9, original_roster).unwrap();
        let rotated_strategies =
            core::array::from_fn(|seat| original_roster.strategies[(seat + 2) % SEATS]);
        let rotated = run_case(
            0.9,
            PolicyRoster {
                id: "rotated_heterogeneous_six_policy",
                strategies: rotated_strategies,
            },
        )
        .unwrap();

        for (tick, rotated_actions) in rotated.action_history.iter().enumerate() {
            for (seat, &rotated_action) in rotated_actions.iter().enumerate() {
                assert_eq!(
                    rotated_action,
                    original.action_history[tick][(seat + 2) % SEATS],
                );
                assert!(close_enough(
                    rotated.ticks[tick].regions[seat].served.energy_kwh,
                    original.ticks[tick].regions[(seat + 2) % SEATS]
                        .served
                        .energy_kwh,
                ));
            }
        }
        assert_eq!(
            rotated.game.total_contribution,
            original.game.total_contribution
        );
        assert!(close_enough(
            rotated.game.expected_group_welfare,
            original.game.expected_group_welfare,
        ));
    }

    #[test]
    fn csv_replay_is_byte_deterministic_and_contains_full_trace() {
        let first = run_standard_matrix().unwrap();
        let second = run_standard_matrix().unwrap();
        let first_csv = render_csv(&first);
        assert_eq!(first_csv, render_csv(&second));
        assert!(first_csv.starts_with(CSV_HEADER));
        assert!(first_csv.contains("episode_summary"));
        assert!(first_csv.contains("agent_tick"));
        assert!(first_csv.contains("shared_tick"));
        assert!(!first_csv.contains("NaN"));
        assert!(!first_csv.contains("inf"));
        assert_eq!(
            first_csv.lines().count(),
            1 + 15 * (1 + TICKS * (SEATS + 1))
        );
    }

    #[test]
    fn invalid_risk_fails_closed_before_resource_replay() {
        assert!(matches!(
            run_case(f64::NAN, roster("all_zero")),
            Err(BridgeError::Game(GameError::InvalidRisk(_))),
        ));
    }

    #[test]
    fn frozen_roster_matrix_contains_five_distinct_policy_conditions() {
        let rosters = standard_rosters();
        assert_eq!(rosters.len(), 5);
        let ids = rosters.map(|roster| roster.id);
        assert_eq!(
            ids,
            [
                "all_zero",
                "all_fixed_fair_share",
                "all_fixed_maximum",
                "all_risk_pivot",
                "heterogeneous_six_policy",
            ],
        );
    }

    fn tiny_region(id: &str, energy_kwh: f64) -> Region {
        Region {
            id: id.to_owned(),
            population: 0.0,
            opening_stock: ResourceVector::new(0.0, energy_kwh, 0.0),
            essential_demand_per_capita: ResourceVector::default(),
        }
    }

    #[test]
    fn over_transfer_is_reported_as_a_partial_receipt_not_silent_delivery() {
        let mut world =
            World::new(vec![tiny_region("donor", 1.0), tiny_region("shared", 0.0)]).unwrap();
        let request = TransferRequest {
            from_region: 0,
            to_region: 1,
            amount: ResourceVector::new(0.0, 1.0, 0.0),
        };
        let input = StepInput {
            production: vec![Production::default(); 2],
            transfers: vec![request, request],
        };
        let report = world.step(&input, TransferPolicy::Unrestricted).unwrap();

        assert!(close_enough(report.transfers[0].delivered.energy_kwh, 1.0,));
        assert!(close_enough(report.transfers[1].requested.energy_kwh, 1.0,));
        assert!(close_enough(report.transfers[1].delivered.energy_kwh, 0.0,));
        assert!(
            report
                .regions
                .iter()
                .all(|region| region.ledger.is_conserved())
        );
    }

    #[test]
    fn ledger_rejects_out_of_range_and_self_transfer_endpoints() {
        let mut world =
            World::new(vec![tiny_region("donor", 1.0), tiny_region("shared", 0.0)]).unwrap();
        let invalid_destination = StepInput {
            production: vec![Production::default(); 2],
            transfers: vec![TransferRequest {
                from_region: 0,
                to_region: 2,
                amount: ResourceVector::new(0.0, 1.0, 0.0),
            }],
        };
        assert!(matches!(
            world.step(&invalid_destination, TransferPolicy::Unrestricted),
            Err(WorldError::RegionIndexOutOfRange { index: 2, .. }),
        ));

        let self_transfer = StepInput {
            production: vec![Production::default(); 2],
            transfers: vec![TransferRequest {
                from_region: 0,
                to_region: 0,
                amount: ResourceVector::new(0.0, 1.0, 0.0),
            }],
        };
        assert!(matches!(
            world.step(&self_transfer, TransferPolicy::Unrestricted),
            Err(WorldError::SelfTransfer { .. }),
        ));
    }

    #[test]
    fn adapter_rejects_wrong_destination_and_wrong_resource_unit_mutations() {
        let mut wrong_destination = run_case(0.9, roster("all_fixed_fair_share")).unwrap();
        wrong_destination.ticks[0].transfers[0].to_region = 0;
        assert!(matches!(
            validate_episode(&wrong_destination),
            Err(BridgeError::Invariant(_)),
        ));

        let mut wrong_unit = run_case(0.9, roster("all_fixed_fair_share")).unwrap();
        wrong_unit.ticks[0].transfers[0].requested.water_m3 = 1.0;
        assert!(matches!(
            validate_episode(&wrong_unit),
            Err(BridgeError::Invariant(_)),
        ));
    }

    #[test]
    fn roster_label_changes_do_not_change_game_or_resource_outcomes() {
        let original = run_case(0.5, roster("all_zero")).unwrap();
        let relabeled = run_case(
            0.5,
            PolicyRoster {
                id: "narrative_label_perturbed",
                strategies: [Strategy::AlwaysZero; SEATS],
            },
        )
        .unwrap();

        assert_eq!(original.game, relabeled.game);
        assert_eq!(original.action_history, relabeled.action_history);
        assert_eq!(original.ticks, relabeled.ticks);
    }
}
