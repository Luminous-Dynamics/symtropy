use std::collections::{BTreeMap, BTreeSet};

pub const COHORTS_CSV: &str = include_str!("../data/cohorts.csv");
const COHORTS_HEADER: &str =
    "scenario_id,call_id,queue_entry_tick,as_of_tick,outcome,outcome_tick,observed_tick";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    Berth,
    Diverted,
    Cancelled,
    Censored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CensorReason {
    Administrative,
    ReportingLag,
}

impl Outcome {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "BERTH" => Ok(Self::Berth),
            "DIVERTED" => Ok(Self::Diverted),
            "CANCELLED" => Ok(Self::Cancelled),
            "CENSORED" => Ok(Self::Censored),
            other => Err(format!("unknown outcome {other:?}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    scenario_id: String,
    call_id: String,
    entry_tick: i64,
    as_of_tick: i64,
    outcome: Outcome,
    outcome_tick: i64,
    observed_tick: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResolvedCall {
    entry_tick: i64,
    end_tick: i64,
    duration_ticks: u64,
    outcome: Outcome,
    censor_reason: Option<CensorReason>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RiskStep {
    pub duration_ticks: u64,
    pub at_risk: usize,
    pub berth_events: usize,
    pub diverted_events: usize,
    pub cancelled_events: usize,
    pub censored: usize,
    pub event_free_survival: f64,
    pub berth_cumulative_incidence: f64,
    pub diversion_cumulative_incidence: f64,
    pub cancellation_cumulative_incidence: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CohortReport {
    pub scenario_id: String,
    pub as_of_tick: i64,
    pub call_count: usize,
    pub berth_count: usize,
    pub diversion_count: usize,
    pub cancellation_count: usize,
    pub censored_count: usize,
    pub administrative_censored_count: usize,
    pub reporting_lag_censored_count: usize,
    pub total_vessel_ticks: u128,
    pub integrated_queue_vessel_ticks: u128,
    pub mean_observed_queue_ticks: f64,
    pub p50_berth_incidence_tick: Option<u64>,
    pub p90_berth_incidence_tick: Option<u64>,
    pub p95_berth_incidence_tick: Option<u64>,
    pub risk_steps: Vec<RiskStep>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkReport {
    pub cohorts: Vec<CohortReport>,
}

fn parse_i64(value: &str, name: &str, line: usize) -> Result<i64, String> {
    value
        .parse::<i64>()
        .map_err(|_| format!("line {line}: {name} must be an integer, got {value:?}"))
}

fn split_row<'a>(line: &'a str, line_number: usize) -> Result<Vec<&'a str>, String> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() != 7 {
        return Err(format!(
            "line {line_number}: expected 7 comma-separated fields, found {}",
            fields.len()
        ));
    }
    if fields.iter().any(|field| field.is_empty()) {
        return Err(format!("line {line_number}: blank fields are not permitted"));
    }
    Ok(fields)
}

fn parse_observations(csv: &str) -> Result<Vec<Observation>, String> {
    let mut lines = csv.lines();
    let header = lines
        .next()
        .ok_or_else(|| "cohort CSV is empty".to_string())?
        .trim();
    if header != COHORTS_HEADER {
        return Err(format!("unexpected cohort CSV header: {header:?}"));
    }

    let mut observations = Vec::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        if line.trim().is_empty() {
            return Err(format!("line {line_number}: blank records are not permitted"));
        }
        let f = split_row(line, line_number)?;
        observations.push(Observation {
            scenario_id: f[0].to_string(),
            call_id: f[1].to_string(),
            entry_tick: parse_i64(f[2], "queue_entry_tick", line_number)?,
            as_of_tick: parse_i64(f[3], "as_of_tick", line_number)?,
            outcome: Outcome::parse(f[4])?,
            outcome_tick: parse_i64(f[5], "outcome_tick", line_number)?,
            observed_tick: parse_i64(f[6], "observed_tick", line_number)?,
        });
    }

    if observations.is_empty() {
        return Err("cohort CSV contains no calls".to_string());
    }
    Ok(observations)
}

fn resolve_call(observation: &Observation) -> Result<ResolvedCall, String> {
    if observation.entry_tick > observation.as_of_tick {
        return Err(format!(
            "call {:?} entered after its cohort cutoff",
            observation.call_id
        ));
    }

    // The source may publish an actual outcome only after the historical
    // snapshot cutoff. That later outcome cannot rewrite the earlier cohort.
    let (effective_outcome, end_tick, censor_reason) =
        if observation.observed_tick > observation.as_of_tick {
            (
                Outcome::Censored,
                observation.as_of_tick,
                Some(CensorReason::ReportingLag),
            )
        } else {
            match observation.outcome {
                Outcome::Censored => {
                    if observation.outcome_tick != observation.as_of_tick
                        || observation.observed_tick != observation.as_of_tick
                    {
                        return Err(format!(
                            "censored call {:?} must be observed at its declared cutoff",
                            observation.call_id
                        ));
                    }
                    (
                        Outcome::Censored,
                        observation.as_of_tick,
                        Some(CensorReason::Administrative),
                    )
                }
                outcome => {
                    if observation.outcome_tick < observation.entry_tick {
                        return Err(format!(
                            "call {:?} outcome precedes queue entry",
                            observation.call_id
                        ));
                    }
                    if observation.outcome_tick > observation.as_of_tick {
                        return Err(format!(
                            "call {:?} has an outcome after its observation cutoff",
                            observation.call_id
                        ));
                    }
                    if observation.observed_tick < observation.outcome_tick {
                        return Err(format!(
                            "call {:?} outcome was observed before it occurred",
                            observation.call_id
                        ));
                    }
                    (outcome, observation.outcome_tick, None)
                }
            }
        };

    let duration = end_tick
        .checked_sub(observation.entry_tick)
        .ok_or_else(|| format!("call {:?} duration overflow", observation.call_id))?;
    let duration_ticks = u64::try_from(duration)
        .map_err(|_| format!("call {:?} has a negative queue duration", observation.call_id))?;

    Ok(ResolvedCall {
        entry_tick: observation.entry_tick,
        end_tick,
        duration_ticks,
        outcome: effective_outcome,
        censor_reason,
    })
}

#[derive(Default)]
struct EventBucket {
    berth: usize,
    diverted: usize,
    cancelled: usize,
    censored: usize,
}

fn incidence_quantile(steps: &[RiskStep], threshold: f64) -> Option<u64> {
    steps
        .iter()
        .find(|step| step.berth_cumulative_incidence + 1e-12 >= threshold)
        .map(|step| step.duration_ticks)
}

fn integrate_queue_curve(calls: &[ResolvedCall]) -> Result<u128, String> {
    let mut boundaries = BTreeSet::new();
    for call in calls {
        boundaries.insert(call.entry_tick);
        boundaries.insert(call.end_tick);
    }

    let points: Vec<i64> = boundaries.into_iter().collect();
    let mut area = 0_u128;
    for interval in points.windows(2) {
        let start = interval[0];
        let end = interval[1];
        let duration = u128::try_from(
            end.checked_sub(start)
                .ok_or_else(|| "queue interval duration overflow".to_string())?,
        )
        .map_err(|_| "queue interval has negative duration".to_string())?;

        let queue_count = calls
            .iter()
            .filter(|call| call.entry_tick <= start && call.end_tick > start)
            .count() as u128;
        area = area
            .checked_add(
                queue_count
                    .checked_mul(duration)
                    .ok_or_else(|| "queue curve area overflow".to_string())?,
            )
            .ok_or_else(|| "queue curve area overflow".to_string())?;
    }
    Ok(area)
}

fn evaluate_scenario(
    scenario_id: &str,
    as_of_tick: i64,
    observations: &[Observation],
) -> Result<CohortReport, String> {
    let mut seen_call_ids = BTreeSet::new();
    let mut resolved = Vec::with_capacity(observations.len());
    let mut total_vessel_ticks = 0_u128;
    let mut berth_count = 0;
    let mut diversion_count = 0;
    let mut cancellation_count = 0;
    let mut censored_count = 0;
    let mut administrative_censored_count = 0;
    let mut reporting_lag_censored_count = 0;
    let mut buckets: BTreeMap<u64, EventBucket> = BTreeMap::new();

    for observation in observations {
        if !seen_call_ids.insert(observation.call_id.clone()) {
            return Err(format!(
                "scenario {scenario_id:?} has duplicate call ID {:?}",
                observation.call_id
            ));
        }
        if observation.as_of_tick != as_of_tick {
            return Err(format!(
                "scenario {scenario_id:?} has inconsistent observation cutoffs"
            ));
        }

        let call = resolve_call(observation)?;
        total_vessel_ticks = total_vessel_ticks
            .checked_add(u128::from(call.duration_ticks))
            .ok_or_else(|| "total vessel-tick exposure overflow".to_string())?;

        let bucket = buckets.entry(call.duration_ticks).or_default();
        match call.outcome {
            Outcome::Berth => {
                berth_count += 1;
                bucket.berth += 1;
            }
            Outcome::Diverted => {
                diversion_count += 1;
                bucket.diverted += 1;
            }
            Outcome::Cancelled => {
                cancellation_count += 1;
                bucket.cancelled += 1;
            }
            Outcome::Censored => {
                censored_count += 1;
                bucket.censored += 1;
                match call.censor_reason {
                    Some(CensorReason::Administrative) => administrative_censored_count += 1,
                    Some(CensorReason::ReportingLag) => reporting_lag_censored_count += 1,
                    None => {
                        return Err(format!(
                            "scenario {scenario_id:?} has censored call without a censor reason"
                        ));
                    }
                }
            }
        }
        resolved.push(call);
    }

    let mut at_risk = resolved.len();
    let mut survival = 1.0_f64;
    let mut cif_berth = 0.0_f64;
    let mut cif_diverted = 0.0_f64;
    let mut cif_cancelled = 0.0_f64;
    let mut risk_steps = Vec::with_capacity(buckets.len());

    for (duration_ticks, bucket) in buckets {
        let competing_events = bucket.berth + bucket.diverted + bucket.cancelled;
        if competing_events + bucket.censored > at_risk {
            return Err(format!(
                "scenario {scenario_id:?} risk set underflow at duration {duration_ticks}"
            ));
        }

        let survival_before = survival;
        if at_risk > 0 {
            let risk = at_risk as f64;
            let event_count = competing_events as f64;
            survival *= (risk - event_count) / risk;
            cif_berth += survival_before * bucket.berth as f64 / risk;
            cif_diverted += survival_before * bucket.diverted as f64 / risk;
            cif_cancelled += survival_before * bucket.cancelled as f64 / risk;
        }

        risk_steps.push(RiskStep {
            duration_ticks,
            at_risk,
            berth_events: bucket.berth,
            diverted_events: bucket.diverted,
            cancelled_events: bucket.cancelled,
            censored: bucket.censored,
            event_free_survival: survival,
            berth_cumulative_incidence: cif_berth,
            diversion_cumulative_incidence: cif_diverted,
            cancellation_cumulative_incidence: cif_cancelled,
        });

        at_risk -= competing_events + bucket.censored;
    }

    if at_risk != 0 {
        return Err(format!(
            "scenario {scenario_id:?} has calls remaining outside its final risk bucket"
        ));
    }

    let call_count = resolved.len();
    let integrated_queue_vessel_ticks = integrate_queue_curve(&resolved)?;
    if integrated_queue_vessel_ticks != total_vessel_ticks {
        return Err(format!(
            "scenario {scenario_id:?} exposure mismatch: per-call sum {total_vessel_ticks}, integrated queue curve {integrated_queue_vessel_ticks}"
        ));
    }

    let mean_observed_queue_ticks = if call_count == 0 {
        0.0
    } else {
        total_vessel_ticks as f64 / call_count as f64
    };

    let mass = survival + cif_berth + cif_diverted + cif_cancelled;
    if (mass - 1.0).abs() > 1e-9 {
        return Err(format!(
            "scenario {scenario_id:?} competing-risk probability mass does not reconcile: {mass}"
        ));
    }

    Ok(CohortReport {
        scenario_id: scenario_id.to_string(),
        as_of_tick,
        call_count,
        berth_count,
        diversion_count,
        cancellation_count,
        censored_count,
        administrative_censored_count,
        reporting_lag_censored_count,
        total_vessel_ticks,
        integrated_queue_vessel_ticks,
        mean_observed_queue_ticks,
        p50_berth_incidence_tick: incidence_quantile(&risk_steps, 0.50),
        p90_berth_incidence_tick: incidence_quantile(&risk_steps, 0.90),
        p95_berth_incidence_tick: incidence_quantile(&risk_steps, 0.95),
        risk_steps,
    })
}

pub fn evaluate_cohorts(csv: &str) -> Result<BenchmarkReport, String> {
    let observations = parse_observations(csv)?;
    let mut grouped: BTreeMap<String, Vec<Observation>> = BTreeMap::new();

    for observation in observations {
        grouped
            .entry(observation.scenario_id.clone())
            .or_default()
            .push(observation);
    }

    let mut cohorts = Vec::with_capacity(grouped.len());
    for (scenario_id, calls) in grouped {
        let as_of_tick = calls[0].as_of_tick;
        cohorts.push(evaluate_scenario(&scenario_id, as_of_tick, &calls)?);
    }

    Ok(BenchmarkReport { cohorts })
}

pub fn evaluate_reference_cohorts() -> Result<BenchmarkReport, String> {
    evaluate_cohorts(COHORTS_CSV)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-10,
            "actual {actual:.12}, expected {expected:.12}"
        );
    }

    fn cohort<'a>(report: &'a BenchmarkReport, id: &str) -> &'a CohortReport {
        report
            .cohorts
            .iter()
            .find(|cohort| cohort.scenario_id == id)
            .expect("scenario exists")
    }

    #[test]
    fn five_frozen_cohorts_match_the_independent_golden_summary() {
        let report = evaluate_reference_cohorts().expect("cohort corpus");
        assert_eq!(report.cohorts.len(), 5);

        let s01 = cohort(&report, "S01");
        assert_eq!(
            (
                s01.call_count,
                s01.berth_count,
                s01.diversion_count,
                s01.cancellation_count,
                s01.censored_count
            ),
            (6, 3, 1, 1, 1)
        );
        assert_eq!(s01.total_vessel_ticks, 22);
        assert_eq!(s01.integrated_queue_vessel_ticks, 22);
        assert_eq!(s01.administrative_censored_count, 1);
        assert_eq!(s01.reporting_lag_censored_count, 0);
        assert_eq!(s01.p50_berth_incidence_tick, Some(4));
        assert_eq!(s01.p90_berth_incidence_tick, None);
        assert_eq!(s01.p95_berth_incidence_tick, None);
        let s01_last = s01.risk_steps.last().unwrap();
        close(
            s01_last.event_free_survival
                + s01_last.berth_cumulative_incidence
                + s01_last.diversion_cumulative_incidence
                + s01_last.cancellation_cumulative_incidence,
            1.0,
        );
        close(s01.risk_steps.last().unwrap().event_free_survival, 1.0 / 6.0);
        close(s01.risk_steps.last().unwrap().berth_cumulative_incidence, 0.5);
        close(s01.risk_steps.last().unwrap().diversion_cumulative_incidence, 1.0 / 6.0);
        close(s01.risk_steps.last().unwrap().cancellation_cumulative_incidence, 1.0 / 6.0);

        let s02 = cohort(&report, "S02");
        assert_eq!(
            (
                s02.call_count,
                s02.berth_count,
                s02.diversion_count,
                s02.cancellation_count,
                s02.censored_count
            ),
            (5, 2, 1, 1, 1)
        );
        assert_eq!(s02.total_vessel_ticks, 15);
        assert_eq!(s02.integrated_queue_vessel_ticks, 15);
        assert_eq!(s02.administrative_censored_count, 1);
        assert_eq!(s02.reporting_lag_censored_count, 0);
        assert_eq!(s02.p50_berth_incidence_tick, None);
        close(s02.risk_steps.last().unwrap().event_free_survival, 0.2);
        close(s02.risk_steps.last().unwrap().berth_cumulative_incidence, 0.4);
        close(s02.risk_steps.last().unwrap().diversion_cumulative_incidence, 0.2);
        close(s02.risk_steps.last().unwrap().cancellation_cumulative_incidence, 0.2);

        let s03 = cohort(&report, "S03");
        assert_eq!(
            (
                s03.call_count,
                s03.berth_count,
                s03.diversion_count,
                s03.cancellation_count,
                s03.censored_count
            ),
            (4, 0, 0, 0, 4)
        );
        assert_eq!(s03.total_vessel_ticks, 20);
        assert_eq!(s03.integrated_queue_vessel_ticks, 20);
        assert_eq!(s03.administrative_censored_count, 4);
        assert_eq!(s03.reporting_lag_censored_count, 0);
        assert_eq!(s03.p50_berth_incidence_tick, None);
        close(s03.risk_steps.last().unwrap().event_free_survival, 1.0);
        close(s03.risk_steps.last().unwrap().berth_cumulative_incidence, 0.0);

        let s04 = cohort(&report, "S04");
        assert_eq!(
            (
                s04.call_count,
                s04.berth_count,
                s04.diversion_count,
                s04.cancellation_count,
                s04.censored_count
            ),
            (5, 2, 1, 1, 1)
        );
        assert_eq!(s04.total_vessel_ticks, 19);
        assert_eq!(s04.integrated_queue_vessel_ticks, 19);
        assert_eq!(s04.administrative_censored_count, 1);
        assert_eq!(s04.reporting_lag_censored_count, 0);
        assert_eq!(s04.p50_berth_incidence_tick, None);
        close(s04.risk_steps.last().unwrap().event_free_survival, 0.2);
        close(s04.risk_steps.last().unwrap().berth_cumulative_incidence, 0.4);
        close(s04.risk_steps.last().unwrap().diversion_cumulative_incidence, 0.2);
        close(s04.risk_steps.last().unwrap().cancellation_cumulative_incidence, 0.2);

        let s05 = cohort(&report, "S05");
        assert_eq!(
            (
                s05.call_count,
                s05.berth_count,
                s05.diversion_count,
                s05.cancellation_count,
                s05.censored_count
            ),
            (5, 1, 1, 1, 2)
        );
        assert_eq!(s05.total_vessel_ticks, 23);
        assert_eq!(s05.integrated_queue_vessel_ticks, 23);
        assert_eq!(s05.administrative_censored_count, 1);
        assert_eq!(s05.reporting_lag_censored_count, 1);
        assert_eq!(s05.p50_berth_incidence_tick, None);
        close(s05.risk_steps.last().unwrap().event_free_survival, 0.4);
        close(s05.risk_steps.last().unwrap().berth_cumulative_incidence, 0.2);
        close(s05.risk_steps.last().unwrap().diversion_cumulative_incidence, 0.2);
        close(s05.risk_steps.last().unwrap().cancellation_cumulative_incidence, 0.2);
    }

    #[test]
    fn exact_risk_sets_and_tied_competing_events_are_pinned() {
        let report = evaluate_reference_cohorts().expect("cohort corpus");
        let s01 = cohort(&report, "S01");
        let expected: [(u64, usize, usize, usize, usize, usize, f64, f64, f64, f64); 4] = [
            (2, 6, 1, 0, 0, 0, 5.0 / 6.0, 1.0 / 6.0, 0.0, 0.0),
            (3, 5, 0, 1, 0, 0, 4.0 / 6.0, 1.0 / 6.0, 1.0 / 6.0, 0.0),
            (4, 4, 2, 0, 1, 0, 1.0 / 6.0, 0.5, 1.0 / 6.0, 1.0 / 6.0),
            (5, 1, 0, 0, 0, 1, 1.0 / 6.0, 0.5, 1.0 / 6.0, 1.0 / 6.0),
        ];

        assert_eq!(s01.risk_steps.len(), expected.len());
        for (step, expected) in s01.risk_steps.iter().zip(expected) {
            assert_eq!(
                (
                    step.duration_ticks,
                    step.at_risk,
                    step.berth_events,
                    step.diverted_events,
                    step.cancelled_events,
                    step.censored
                ),
                (expected.0, expected.1, expected.2, expected.3, expected.4, expected.5)
            );
            close(step.event_free_survival, expected.6);
            close(step.berth_cumulative_incidence, expected.7);
            close(step.diversion_cumulative_incidence, expected.8);
            close(step.cancellation_cumulative_incidence, expected.9);
        }

        let s04 = cohort(&report, "S04");
        assert_eq!(s04.risk_steps[0].at_risk, 5);
        assert_eq!(s04.risk_steps[0].berth_events, 1);
        assert_eq!(s04.risk_steps[0].diverted_events, 1);
        assert_eq!(s04.risk_steps[0].cancelled_events, 1);
        close(s04.risk_steps[0].event_free_survival, 0.4);
    }

    #[test]
    fn late_observed_berth_is_censored_at_prior_cutoff() {
        let report = evaluate_reference_cohorts().expect("cohort corpus");
        let s05 = cohort(&report, "S05");
        assert_eq!(s05.berth_count, 1);
        assert_eq!(s05.censored_count, 2);
        assert_eq!(s05.total_vessel_ticks, 23);
        assert_eq!(s05.risk_steps.last().unwrap().duration_ticks, 6);
    }

    #[test]
    fn all_censored_cohort_has_no_fabricated_berth_quantile() {
        let report = evaluate_reference_cohorts().expect("cohort corpus");
        let s03 = cohort(&report, "S03");
        assert_eq!(s03.p50_berth_incidence_tick, None);
        assert_eq!(s03.p90_berth_incidence_tick, None);
        assert_eq!(s03.p95_berth_incidence_tick, None);
    }

    #[test]
    fn competing_risk_probability_mass_reconciles_at_every_step() {
        let report = evaluate_reference_cohorts().expect("cohort corpus");
        for cohort in report.cohorts {
            for step in cohort.risk_steps {
                close(
                    step.event_free_survival
                        + step.berth_cumulative_incidence
                        + step.diversion_cumulative_incidence
                        + step.cancellation_cumulative_incidence,
                    1.0,
                );
            }
        }
    }

    #[test]
    fn row_permutation_does_not_change_cohort_results() {
        let mut rows: Vec<&str> = COHORTS_CSV.lines().skip(1).collect();
        rows.reverse();
        let permuted = format!("{COHORTS_HEADER}\n{}\n", rows.join("\n"));
        assert_eq!(
            evaluate_reference_cohorts().expect("original"),
            evaluate_cohorts(&permuted).expect("permuted")
        );
    }

    #[test]
    fn duplicate_calls_and_inconsistent_cutoffs_reject() {
        let duplicate = format!(
            "{COHORTS_HEADER}\nS,1,0,5,CENSORED,5,5\nS,1,0,5,CENSORED,5,5\n"
        );
        assert!(evaluate_cohorts(&duplicate).unwrap_err().contains("duplicate call ID"));

        let mixed_cutoff = format!(
            "{COHORTS_HEADER}\nS,1,0,5,CENSORED,5,5\nS,2,0,6,CENSORED,6,6\n"
        );
        assert!(
            evaluate_cohorts(&mixed_cutoff)
                .unwrap_err()
                .contains("inconsistent observation cutoffs")
        );
    }

    #[test]
    fn malformed_rows_and_time_inversions_fail_closed() {
        let blank = format!("{COHORTS_HEADER}\n\nS,1,0,5,CENSORED,5,5\n");
        assert!(evaluate_cohorts(&blank).unwrap_err().contains("blank records"));

        let inverted = format!(
            "{COHORTS_HEADER}\nS,1,3,5,BERTH,2,2\n"
        );
        assert!(evaluate_cohorts(&inverted).unwrap_err().contains("outcome precedes queue entry"));
    }
}
