// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! First process-separated synthetic benchmark runner for human-systems research.
//!
//! The parent process owns generated truth and scoring. The candidate child
//! receives only a versioned, estimator-facing event stream over stdin.
//! This is a data-flow boundary, not a hostile-code sandbox.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::env;
use std::error::Error;
use std::io::{self, BufRead, BufWriter, Read, Write};
use std::process::{Command, Stdio};

use symtropy_human_systems::intervention::{
    generate_intervention_scenario, InterventionScenarioId, PhysicalExposureTruth,
};
use symtropy_human_systems::observation::{
    generate_scenario, EstimatorView, ObservationEvent, ObservationQuantity, ScenarioId,
    ScorerEventTruth,
};
use symtropy_human_systems::sampling_rate::{
    canonical_rate_profiles, sample_ideal_lowpass, SamplingRegime, FIXTURE_DURATION_SECONDS,
};

const WIRE_MAGIC: &str = "HUMAN-ESTIMATOR-V1";
const CANDIDATE_ARGUMENT: &str = "--candidate-v1";
const BASE_SEED: u64 = 0x5EED_2026_A11C_E5E5;

#[derive(Debug, Clone, Copy)]
enum CandidateReply {
    Prediction(f64),
    Abstention,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let result = if args.len() == 2 && args[1] == CANDIDATE_ARGUMENT {
        candidate_main()
    } else if args.len() == 1 {
        run_benchmark()
    } else {
        Err(invalid_data("unsupported command-line arguments"))
    };

    if let Err(error) = result {
        eprintln!("human-systems-benchmark: {error}");
        std::process::exit(1);
    }
}

fn run_benchmark() -> Result<(), Box<dyn Error>> {
    println!("RUNNER\tprotocol=1\tcandidate=last-observation-baseline\ttruth=parent-only");
    println!("BOUNDARY\tchild_process=true\tenvironment=scrubbed\tfilesystem_sandbox=false\tnetwork_sandbox=false");

    let mut candidate_processes = 0_usize;
    for (index, scenario_id) in ScenarioId::ALL.into_iter().enumerate() {
        let generated = generate_scenario(scenario_id, BASE_SEED ^ index as u64);
        let payload = encode_estimator_view(&generated.estimator_view);
        let replies = run_candidate_process(&payload)?;
        candidate_processes += 1;
        score_scenario(scenario_id, &generated.estimator_view, &generated.scorer_view.event_truth, &generated.scorer_view.truth, generated.scorer_view.alternate_truth.as_deref(), &replies)?;
    }

    run_sampling_reference_lane()?;
    run_intervention_visibility_lane()?;

    println!("COMPLETE\tcandidate_processes={candidate_processes}\thuman_validity=not_assessed\tclinical_validity=not_assessed");
    Ok(())
}

fn encode_estimator_view(view: &EstimatorView) -> String {
    let mut output = format!("{WIRE_MAGIC}\t{}\n", view.scenario_id.as_str());
    for event in &view.observations {
        output.push_str(&encode_event(event));
    }
    output
}

fn encode_event(event: &ObservationEvent) -> String {
    let quantity = match event.quantity {
        ObservationQuantity::PrimarySignal => "PrimarySignal",
        ObservationQuantity::ActivityProxy => "ActivityProxy",
        ObservationQuantity::CompositeSignal => "CompositeSignal",
    };
    let value = event
        .value
        .map(|value| format!("{value:.17}"))
        .unwrap_or_else(|| "-".to_owned());
    let status = format!("{:?}", event.status);

    format!(
        "E\t{}\t{}\t{}\t{}\t{:.17}\t{:.17}\t{}\t{}\t{}\t{}\n",
        event.source_event_id,
        event.delivery_id,
        event.sensor_id,
        quantity,
        event.sensor_timestamp_seconds,
        event.available_at_seconds,
        event.clock_domain,
        value,
        status,
        event.is_duplicate_delivery
    )
}

fn run_candidate_process(payload: &str) -> Result<HashMap<u64, CandidateReply>, Box<dyn Error>> {
    let executable = env::current_exe()?;
    let mut child = Command::new(executable)
        .arg(CANDIDATE_ARGUMENT)
        .env_clear()
        .current_dir(env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        let mut child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| invalid_data("candidate stdin was not created"))?;
        child_stdin.write_all(payload.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(invalid_data(format!(
            "candidate child exited with {}: {}",
            output.status,
            stderr.trim()
        ))
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    parse_candidate_output(&stdout)
        .map_err(|message| invalid_data(format!("invalid candidate response: {message}")).into())
}

fn parse_candidate_output(
    output: &str,
) -> Result<HashMap<u64, CandidateReply>, String> {
    let mut replies = HashMap::new();
    for (line_number, line) in output.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err(format!("line {} must contain three tab-separated fields", line_number + 1));
        }
        let source_id = fields[1]
            .parse::<u64>()
            .map_err(|_| format!("line {} has an invalid source event ID", line_number + 1))?;
        let reply = match fields[0] {
            "P" => {
                let value = parse_finite(fields[2])
                    .map_err(|_| format!("line {} has a non-finite prediction", line_number + 1))?;
                CandidateReply::Prediction(value)
            }
            "A" if fields[2] == "NO_PRIOR_OBSERVATION" => CandidateReply::Abstention,
            "A" => return Err(format!("line {} has an unknown abstention reason", line_number + 1)),
            _ => return Err(format!("line {} has an unknown response record", line_number + 1)),
        };
        if replies.insert(source_id, reply).is_some() {
            return Err(format!("duplicate response for source event {source_id}"));
        }
    }
    Ok(replies)
}

fn score_scenario(
    scenario_id: ScenarioId,
    estimator_view: &EstimatorView,
    event_truth: &[ScorerEventTruth],
    truth: &[symtropy_human_systems::observation::LatentTruthPoint],
    alternate_truth: Option<&[symtropy_human_systems::observation::LatentTruthPoint]>,
    replies: &HashMap<u64, CandidateReply>,
) -> Result<(), Box<dyn Error>> {
    let target_ids: BTreeSet<u64> = estimator_view
        .observations
        .iter()
        .filter(|event| is_target_quantity(event.quantity))
        .map(|event| event.source_event_id)
        .collect();

    let reply_ids: BTreeSet<u64> = replies.keys().copied().collect();
    if target_ids != reply_ids {
        return Err(invalid_data(format!(
            "{} candidate response IDs do not match target event IDs (target={}, replies={})",
            scenario_id.as_str(),
            target_ids.len(),
            reply_ids.len()
        ))
        .into());
    }

    let truth_by_id: HashMap<u64, &ScorerEventTruth> = event_truth
        .iter()
        .map(|row| (row.source_event_id, row))
        .collect();
    let mut visible_sample_indices = BTreeSet::new();
    let mut absolute_errors = Vec::new();
    let mut squared_errors = Vec::new();
    let mut abstentions = 0_usize;
    let mut world_a_squared_errors = Vec::new();
    let mut world_b_squared_errors = Vec::new();

    let alternate_by_index: HashMap<u64, f64> = alternate_truth
        .unwrap_or(&[])
        .iter()
        .map(|point| (point.sample_index, point.latent_primary))
        .collect();

    for source_id in &target_ids {
        let truth_row = truth_by_id
            .get(source_id)
            .ok_or_else(|| invalid_data(format!("scorer map is missing source event {source_id}")))?;
        visible_sample_indices.insert(truth_row.sample_index);

        match replies.get(source_id).expect("reply IDs were checked") {
            CandidateReply::Abstention => abstentions += 1,
            CandidateReply::Prediction(prediction) => {
                let error = prediction - truth_row.latent_primary;
                absolute_errors.push(error.abs());
                squared_errors.push(error * error);

                if alternate_truth.is_some() {
                    world_a_squared_errors.push(error * error);
                    let alternate = alternate_by_index
                        .get(&truth_row.sample_index)
                        .ok_or_else(|| invalid_data("alternate truth lacks a matching sample index"))?;
                    let alternate_error = prediction - alternate;
                    world_b_squared_errors.push(alternate_error * alternate_error);
                }
            }
        }
    }

    let prediction_count = absolute_errors.len();
    let response_coverage = ratio(prediction_count, target_ids.len());
    let capture_coverage = ratio(visible_sample_indices.len(), truth.len());
    let mae = mean(&absolute_errors);
    let rmse = mean(&squared_errors).map(f64::sqrt);

    println!(
        "HOBS\tid={}\ttruth_points={}\tvisible_points={}\tcapture_coverage={}\ttarget_events={}\tpredictions={}\tabstentions={}\tresponse_coverage={}\tmae={}\trmse={}",
        scenario_id.as_str(),
        truth.len(),
        visible_sample_indices.len(),
        format_metric(capture_coverage),
        target_ids.len(),
        prediction_count,
        abstentions,
        format_metric(response_coverage),
        format_metric(mae),
        format_metric(rmse),
    );

    if alternate_truth.is_some() {
        println!(
            "IDENTIFIABILITY\tid={}\tcandidate_invocations=1\toracle_worlds=2\tidentical_candidate_stream=true\tworld_A_rmse={}\tworld_B_rmse={}",
            scenario_id.as_str(),
            format_metric(mean(&world_a_squared_errors).map(f64::sqrt)),
            format_metric(mean(&world_b_squared_errors).map(f64::sqrt)),
        );
    }
    Ok(())
}

fn is_target_quantity(quantity: ObservationQuantity) -> bool {
    matches!(
        quantity,
        ObservationQuantity::PrimarySignal | ObservationQuantity::CompositeSignal
    )
}

fn run_sampling_reference_lane() -> Result<(), Box<dyn Error>> {
    let expected = [1.0, 1.0, 1.0, 1.0, 25.0 / 34.0, 0.0];
    for (profile, expected_retention) in canonical_rate_profiles().iter().zip(expected) {
        if (profile.expected_task_band_retention - expected_retention).abs() > 1.0e-12 {
            return Err(invalid_data("sampling-rate retention differs from the known-truth oracle").into());
        }
        let samples = sample_ideal_lowpass(profile.sampling_rate_hz, FIXTURE_DURATION_SECONDS)?;
        let expected_count = (profile.sampling_rate_hz * FIXTURE_DURATION_SECONDS).round() as usize;
        if samples.len() != expected_count {
            return Err(invalid_data("sampling-rate sample count differs from the expected count").into());
        }
        if samples.windows(2).any(|pair| pair[1].time_seconds <= pair[0].time_seconds) {
            return Err(invalid_data("sampling-rate sample times are not strictly increasing").into());
        }
        println!(
            "RATE\thz={:.5}\tnyquist_hz={:.5}\tretained_task_power={:.9}\tregime={:?}\tsamples={}",
            profile.sampling_rate_hz,
            profile.nyquist_hz,
            profile.expected_task_band_retention,
            profile.regime,
            samples.len()
        );
    }
    println!("RATE_ORACLE\tkind=ideal_brick_wall\tproduction_filter=false");
    Ok(())
}

fn run_intervention_visibility_lane() -> Result<(), Box<dyn Error>> {
    for scenario_id in InterventionScenarioId::ALL {
        let scenario = generate_intervention_scenario(scenario_id, 60.0)?;
        let at_cutoff = scenario.candidate_view_at(60.0)?;
        let later = scenario.candidate_view_at(120.0)?;

        if at_cutoff
            .exposure_observations
            .iter()
            .any(|observation| observation.available_at_seconds > 60.0)
        {
            return Err(invalid_data("future exposure observation leaked into the 60-second view").into());
        }
        if later
            .exposure_observations
            .iter()
            .any(|observation| observation.available_at_seconds > 120.0)
        {
            return Err(invalid_data("future exposure observation leaked into the 120-second view").into());
        }

        let physical_truth = match &scenario.oracle.traces[0].physical_exposure {
            PhysicalExposureTruth::Occurred { .. } => "occurred",
            PhysicalExposureTruth::Blocked(_) => "blocked",
            PhysicalExposureTruth::NotDelivered => "not_delivered",
        };

        if scenario_id == InterventionScenarioId::ExposureOccurredMeasurementDelayed
            && (!at_cutoff.exposure_observations.is_empty()
                || later.exposure_observations.len() != 1)
        {
            return Err(invalid_data("delayed HIE-04 observation did not respect the cutoff").into());
        }

        println!(
            "HIE\tid={}\treceipts_at_60s={}\texposure_observations_at_60s={}\texposure_observations_at_120s={}\toracle_physical_exposure={}",
            scenario_id.as_str(),
            at_cutoff.delivery_receipts.len(),
            at_cutoff.exposure_observations.len(),
            later.exposure_observations.len(),
            physical_truth
        );
    }
    println!("INTERVENTION_ORACLE\tcausal_effects=false\tphysiological_response=false");
    Ok(())
}

fn candidate_main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let header = lines
        .next()
        .ok_or_else(|| invalid_data("missing estimator wire header"))??;
    let header_fields: Vec<&str> = header.split('\t').collect();
    if header_fields.len() != 2 || header_fields[0] != WIRE_MAGIC {
        return Err(invalid_data("unsupported estimator wire version"));
    }
    if !ScenarioId::ALL
        .iter()
        .any(|scenario| scenario.as_str() == header_fields[1])
    {
        return Err(invalid_data("unknown scenario identity"));
    }

    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    let mut seen_source_ids = HashSet::new();
    let mut last_observed: Option<f64> = None;
    let mut last_available_at = 0.0_f64;

    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 11 || fields[0] != "E" {
            return Err(invalid_data("malformed estimator event record"));
        }

        let source_id = parse_u64(fields[1], "source event ID")?;
        let _delivery_id = parse_u64(fields[2], "delivery ID")?;
        let _sensor_id = parse_u16(fields[3], "sensor ID")?;
        let quantity = fields[4];
        if !matches!(quantity, "PrimarySignal" | "ActivityProxy" | "CompositeSignal") {
            return Err(invalid_data("unknown observation quantity"));
        }

        let sensor_timestamp = parse_finite(fields[5])?;
        let available_at = parse_finite(fields[6])?;
        if available_at < last_available_at || available_at < 0.0 {
            return Err(invalid_data("events must be ordered by non-negative availability time"));
        }
        last_available_at = available_at;
        let _clock_domain = parse_u16(fields[7], "clock domain")?;
        let value = if fields[8] == "-" {
            None
        } else {
            Some(parse_finite(fields[8])?)
        };
        let status = fields[9];
        let _is_duplicate_delivery = fields[10]
            .parse::<bool>()
            .map_err(|_| invalid_data("invalid duplicate-delivery flag"))?;

        if !is_target_quantity(match quantity {
            "PrimarySignal" => ObservationQuantity::PrimarySignal,
            "CompositeSignal" => ObservationQuantity::CompositeSignal,
            _ => ObservationQuantity::ActivityProxy,
        }) {
            continue;
        }
        if !seen_source_ids.insert(source_id) {
            continue;
        }

        let estimate = match (status, value) {
            ("Observed", Some(value)) | ("Quantized", Some(value)) => {
                last_observed = Some(value);
                Some(value)
            }
            ("ClippedLow", Some(_)) | ("ClippedHigh", Some(_)) => last_observed,
            (missing, None) if missing.starts_with("Missing(") => last_observed,
            _ => return Err(invalid_data("status and optional value are inconsistent")),
        };

        match estimate {
            Some(value) => writeln!(output, "P\t{source_id}\t{value:.17}")?,
            None => writeln!(output, "A\t{source_id}\tNO_PRIOR_OBSERVATION")?,
        }
    }

    output.flush()
}

fn parse_u64(value: &str, field: &str) -> io::Result<u64> {
    value
        .parse::<u64>()
        .map_err(|_| invalid_data(format!("invalid {field}")))
}

fn parse_u16(value: &str, field: &str) -> io::Result<u16> {
    value
        .parse::<u16>()
        .map_err(|_| invalid_data(format!("invalid {field}")))
}

fn parse_finite(value: &str) -> io::Result<f64> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| invalid_data("invalid floating-point field"))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(invalid_data("non-finite floating-point field"))
    }
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn format_metric(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.6}"))
        .unwrap_or_else(|| "NA".to_owned())
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
