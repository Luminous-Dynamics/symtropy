//! Reference arithmetic for the candidate City Power Business Conventional tariff.
//!
//! This is intentionally not a billing authority. It computes only the explicitly
//! represented progressive energy blocks and published fixed monthly charge amounts.
//! The identity/label of the secondary fixed component is disputed across sources.
//! Unresolved surcharges, VAT, and account-specific fees stay excluded and visible.

use std::{
    env,
    error::Error,
    fs,
    io,
    path::Path,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Season {
    HighSeason,
    LowSeason,
}

impl Season {
    fn rate(self, block: &EnergyBlock) -> u64 {
        match self {
            Self::HighSeason => block.high_season_microzar_per_kwh,
            Self::LowSeason => block.low_season_microzar_per_kwh,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TariffSchedule {
    schema_version: String,
    tariff_id: String,
    tariff_class: String,
    valid_from: String,
    valid_to: String,
    currency: String,
    monetary_storage_unit: String,
    source_status: String,
    account_verified: bool,
    sources: Vec<String>,
    source_notes: Vec<String>,
    energy_charge_basis: String,
    blocks: Vec<EnergyBlock>,
    fixed_charges: Vec<FixedCharge>,
    deliberately_unapplied_charges: Vec<UnappliedCharge>,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EnergyBlock {
    block_capacity_kwh: Option<u64>,
    high_season_microzar_per_kwh: u64,
    low_season_microzar_per_kwh: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixedCharge {
    capacity_band: String,
    service_microzar_per_month: u64,
    secondary_fixed_component_microzar_per_month: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UnappliedCharge {
    charge_id: String,
    reason: String,
    amount_microzar_per_kwh: Option<u64>,
    percent_basis_points: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswerFile {
    schema_version: String,
    schedule_id: String,
    answer_basis: String,
    cases: Vec<KnownCase>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownCase {
    case_id: String,
    season: Season,
    monthly_energy_kwh: u64,
    capacity_band: String,
    expected_block_use: Vec<ExpectedBlockUse>,
    expected_energy_charge_microzar: u64,
    expected_service_charge_microzar: u64,
    expected_secondary_fixed_component_microzar: u64,
    expected_subtotal_excluding_unapplied_and_tax_microzar: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExpectedBlockUse {
    block_index: usize,
    used_kwh: u64,
    rate_microzar_per_kwh: u64,
    charge_microzar: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct CalculatedCase {
    case_id: String,
    season: Season,
    monthly_energy_kwh: u64,
    capacity_band: String,
    computed_block_use: Vec<ExpectedBlockUse>,
    energy_charge_microzar: u64,
    service_charge_microzar: u64,
    secondary_fixed_component_microzar: u64,
    subtotal_excluding_unapplied_and_tax_microzar: u64,
    unapplied_charge_ids: Vec<String>,
    account_verified: bool,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize)]
struct TariffReferenceReport {
    schema_version: &'static str,
    tariff_id: String,
    tariff_schedule_sha256: String,
    known_answer_fixture_sha256: String,
    tariff_class: String,
    valid_from: String,
    valid_to: String,
    source_status: String,
    account_verified: bool,
    source_urls: Vec<String>,
    source_notes: Vec<String>,
    currency: String,
    monetary_storage_unit: String,
    answer_basis: String,
    result_count: usize,
    results: Vec<CalculatedCase>,
    claim_ceiling: String,
}

fn invalid_data(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}

fn validate_schedule(schedule: &TariffSchedule) -> Result<(), Box<dyn Error>> {
    if schedule.schema_version != "energy-sa-tariff-schedule-v0.1"
        || schedule.tariff_id.trim().is_empty()
        || schedule.tariff_class.trim().is_empty()
        || schedule.valid_from >= schedule.valid_to
        || schedule.currency != "ZAR"
        || schedule.monetary_storage_unit != "microZAR"
        || schedule.energy_charge_basis != "progressive monthly usage blocks"
        || schedule.source_status != "candidate_schedule_not_account_verified"
        || schedule.account_verified
        || schedule.sources.is_empty()
        || schedule.source_notes.is_empty()
        || schedule.claim_ceiling.trim().is_empty()
    {
        return Err(invalid_data("tariff schedule metadata is missing or outside the supported candidate-only contract"));
    }
    if schedule.blocks.is_empty()
        || schedule.blocks.last().is_none_or(|block| block.block_capacity_kwh.is_some())
        || schedule.blocks.iter().take(schedule.blocks.len().saturating_sub(1)).any(|block| {
            block.block_capacity_kwh.is_none_or(|capacity| capacity == 0)
        })
    {
        return Err(invalid_data("tariff must have positive finite block capacities followed by one unbounded final block"));
    }
    for block in &schedule.blocks {
        if block.high_season_microzar_per_kwh == 0 || block.low_season_microzar_per_kwh == 0 {
            return Err(invalid_data("missing/zero tariff rates are not valid; do not infer missing rates as zero"));
        }
    }
    if schedule.fixed_charges.is_empty() || schedule.deliberately_unapplied_charges.is_empty() {
        return Err(invalid_data("fixed tariff charges and unresolved excluded charges must be explicit"));
    }
    let mut fixed_bands = schedule.fixed_charges.iter().map(|item| item.capacity_band.as_str()).collect::<Vec<_>>();
    fixed_bands.sort_unstable();
    fixed_bands.dedup();
    if fixed_bands.len() != schedule.fixed_charges.len() {
        return Err(invalid_data("duplicate capacity-band fixed-charge entries"));
    }
    if schedule.deliberately_unapplied_charges.iter().any(|item| item.charge_id.trim().is_empty() || item.reason.trim().is_empty()) {
        return Err(invalid_data("each excluded/unresolved charge must have an ID and reason"));
    }
    Ok(())
}

fn calculate_case(
    schedule: &TariffSchedule,
    case: &KnownCase,
) -> Result<CalculatedCase, Box<dyn Error>> {
    let fixed = schedule
        .fixed_charges
        .iter()
        .filter(|charge| charge.capacity_band == case.capacity_band)
        .collect::<Vec<_>>();
    if fixed.len() != 1 {
        return Err(invalid_data(format!(
            "capacity band {} has no unique fixed-charge mapping; refusing to infer it",
            case.capacity_band
        )));
    }

    let mut remaining = case.monthly_energy_kwh;
    let mut energy_charge = 0_u64;
    let mut computed_blocks = Vec::with_capacity(schedule.blocks.len());

    for (index, block) in schedule.blocks.iter().enumerate() {
        let used = if remaining == 0 {
            0
        } else {
            block.block_capacity_kwh.map_or(remaining, |capacity| remaining.min(capacity))
        };
        let rate = case.season.rate(block);
        let charge = used.checked_mul(rate).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "tariff block multiplication overflow")
        })?;
        energy_charge = energy_charge.checked_add(charge).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "tariff energy charge sum overflow")
        })?;
        remaining = remaining.checked_sub(used).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "tariff block usage underflow")
        })?;
        computed_blocks.push(ExpectedBlockUse {
            block_index: index,
            used_kwh: used,
            rate_microzar_per_kwh: rate,
            charge_microzar: charge,
        });
    }

    if remaining != 0 {
        return Err(invalid_data("tariff blocks do not cover the requested monthly usage"));
    }

    let service = fixed[0].service_microzar_per_month;
    let secondary_fixed = fixed[0].secondary_fixed_component_microzar_per_month;
    let subtotal = energy_charge
        .checked_add(service)
        .and_then(|value| value.checked_add(secondary_fixed))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "tariff subtotal overflow"))?;

    Ok(CalculatedCase {
        case_id: case.case_id.clone(),
        season: case.season,
        monthly_energy_kwh: case.monthly_energy_kwh,
        capacity_band: case.capacity_band.clone(),
        computed_block_use: computed_blocks,
        energy_charge_microzar: energy_charge,
        service_charge_microzar: service,
        secondary_fixed_component_microzar: secondary_fixed,
        subtotal_excluding_unapplied_and_tax_microzar: subtotal,
        unapplied_charge_ids: schedule.deliberately_unapplied_charges
            .iter()
            .map(|item| item.charge_id.clone())
            .collect(),
        account_verified: schedule.account_verified,
        claim_ceiling: schedule.claim_ceiling.clone(),
    })
}

fn load_and_calculate(
    schedule_bytes: &[u8],
    cases_bytes: &[u8],
) -> Result<TariffReferenceReport, Box<dyn Error>> {
    let schedule: TariffSchedule = serde_json::from_slice(schedule_bytes)?;
    let known: KnownAnswerFile = serde_json::from_slice(cases_bytes)?;
    validate_schedule(&schedule)?;

    if known.schema_version != "energy-sa-tariff-known-answers-v0.1"
        || known.schedule_id != schedule.tariff_id
        || known.cases.is_empty()
        || known.answer_basis.trim().is_empty()
    {
        return Err(invalid_data("known-answer fixture does not bind to this schedule or supported schema"));
    }

    let mut results = Vec::with_capacity(known.cases.len());
    for case in &known.cases {
        let actual = calculate_case(&schedule, case)?;
        if actual.computed_block_use != case.expected_block_use
            || actual.energy_charge_microzar != case.expected_energy_charge_microzar
            || actual.service_charge_microzar != case.expected_service_charge_microzar
            || actual.secondary_fixed_component_microzar != case.expected_secondary_fixed_component_microzar
            || actual.subtotal_excluding_unapplied_and_tax_microzar
                != case.expected_subtotal_excluding_unapplied_and_tax_microzar
        {
            return Err(invalid_data(format!(
                "tariff known answer mismatch for {}; refusing to emit qualified report",
                case.case_id
            )));
        }
        results.push(actual);
    }

    Ok(TariffReferenceReport {
        schema_version: "energy-sa-tariff-reference-report-v0.1",
        tariff_id: schedule.tariff_id.clone(),
        tariff_schedule_sha256: sha256_hex(schedule_bytes),
        known_answer_fixture_sha256: sha256_hex(cases_bytes),
        tariff_class: schedule.tariff_class.clone(),
        valid_from: schedule.valid_from.clone(),
        valid_to: schedule.valid_to.clone(),
        source_status: schedule.source_status.clone(),
        account_verified: schedule.account_verified,
        source_urls: schedule.sources.clone(),
        source_notes: schedule.source_notes.clone(),
        currency: schedule.currency.clone(),
        monetary_storage_unit: schedule.monetary_storage_unit.clone(),
        answer_basis: known.answer_basis,
        result_count: results.len(),
        results,
        claim_ceiling: "Exact arithmetic for the declared candidate tariff schedule only; not an account-verified bill or savings estimate.".to_owned(),
    })
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_sa_tariff_reference <tariff-schedule.json> <known-answers.json> <output-report.json>",
        )
        .into());
    }

    let schedule_bytes = fs::read(&args[1])?;
    let known_bytes = fs::read(&args[2])?;
    let report = load_and_calculate(&schedule_bytes, &known_bytes)?;
    let output_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(&args[3], &output_bytes)?;
    println!("PASS: computed {} exact tariff-reference cases (subtotal excludes unresolved charges and tax)", report.result_count);
    println!("tariff_schedule_sha256={}", report.tariff_schedule_sha256);
    println!("known_answer_fixture_sha256={}", report.known_answer_fixture_sha256);
    println!("report_sha256={}", sha256_hex(&output_bytes));
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy tariff reference failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEDULE: &str = include_str!("../../fixtures/energy-sa-tariff-schedule-2026-27.reference.json");
    const ANSWERS: &str = include_str!("../../fixtures/energy-sa-tariff-known-answers-v0.1.json");

    #[test]
    fn published_candidate_schedule_matches_exact_hand_calculated_cases() {
        let schedule: TariffSchedule = serde_json::from_str(SCHEDULE).expect("schedule fixture parses");
        let known: KnownAnswerFile = serde_json::from_str(ANSWERS).expect("known answers parse");
        validate_schedule(&schedule).expect("candidate schedule passes structural gate");
        assert_eq!(known.cases.len(), 2);

        for case in &known.cases {
            let actual = calculate_case(&schedule, case).expect("known tariff case calculates");
            assert_eq!(actual.computed_block_use, case.expected_block_use, "{}", case.case_id);
            assert_eq!(actual.energy_charge_microzar, case.expected_energy_charge_microzar, "{}", case.case_id);
            assert_eq!(actual.service_charge_microzar, case.expected_service_charge_microzar, "{}", case.case_id);
            assert_eq!(actual.secondary_fixed_component_microzar, case.expected_secondary_fixed_component_microzar, "{}", case.case_id);
            assert_eq!(
                actual.subtotal_excluding_unapplied_and_tax_microzar,
                case.expected_subtotal_excluding_unapplied_and_tax_microzar,
                "{}",
                case.case_id
            );
            assert!(!actual.account_verified, "source-only tariff must never be upgraded to account-verified");
            assert_eq!(actual.unapplied_charge_ids.len(), schedule.deliberately_unapplied_charges.len());
        }
    }

    #[test]
    fn ambiguous_surcharges_and_tax_are_never_silently_added_or_zero_scored() {
        let schedule: TariffSchedule = serde_json::from_str(SCHEDULE).unwrap();
        let known: KnownAnswerFile = serde_json::from_str(ANSWERS).unwrap();
        let actual = calculate_case(&schedule, &known.cases[0]).unwrap();

        assert!(schedule.deliberately_unapplied_charges.iter().any(|charge| charge.charge_id == "network_surcharge_6c_per_kwh"));
        assert!(schedule.deliberately_unapplied_charges.iter().any(|charge| charge.charge_id == "dsm_levy_6c_per_kwh"));
        assert!(schedule.deliberately_unapplied_charges.iter().any(|charge| charge.charge_id == "vat"));
        assert!(schedule.deliberately_unapplied_charges.iter().any(|charge| charge.charge_id == "secondary_fixed_charge_band_and_label_conflict"));
        assert_eq!(
            actual.subtotal_excluding_unapplied_and_tax_microzar,
            actual.energy_charge_microzar
                + actual.service_charge_microzar
                + actual.secondary_fixed_component_microzar
        );
        assert!(!actual.account_verified);
    }

    #[test]
    fn altered_tariff_known_answer_is_rejected_by_the_cli_calculation_path() {
        let schedule = SCHEDULE.as_bytes();
        let tampered = ANSWERS.replace(
            "\"expected_energy_charge_microzar\": 11094610000",
            "\"expected_energy_charge_microzar\": 11094610001",
        );
        assert_ne!(tampered, ANSWERS);
        assert!(load_and_calculate(schedule, tampered.as_bytes()).is_err());
    }

    #[test]
    fn missing_capacity_band_fails_closed_instead_of_inventing_fixed_charges() {
        let schedule: TariffSchedule = serde_json::from_str(SCHEDULE).unwrap();
        let known: KnownAnswerFile = serde_json::from_str(ANSWERS).unwrap();
        let mut case = known.cases[0].clone();
        case.capacity_band = "unknown-customer-class".to_owned();
        assert!(calculate_case(&schedule, &case).is_err());
    }

    #[test]
    fn zero_rate_or_unbounded_nonfinal_block_is_rejected() {
        let mut schedule: TariffSchedule = serde_json::from_str(SCHEDULE).unwrap();
        schedule.blocks[0].low_season_microzar_per_kwh = 0;
        assert!(validate_schedule(&schedule).is_err());

        let mut schedule: TariffSchedule = serde_json::from_str(SCHEDULE).unwrap();
        schedule.blocks[0].block_capacity_kwh = None;
        assert!(validate_schedule(&schedule).is_err());
    }
}
