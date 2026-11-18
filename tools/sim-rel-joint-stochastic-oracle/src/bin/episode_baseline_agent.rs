//! JSONL child-process baseline policy for scientific-world episode requests.
//!
//! The process receives only EpisodeRequest frames on stdin and writes one
//! EpisodeResponse frame per request on stdout. Diagnostics go to stderr.

use sim_rel_joint_stochastic_oracle::episode::{EpisodeCommand, EpisodeRequest};
use sim_rel_joint_stochastic_oracle::policy::{next_response, BaselineStrategy};
use std::error::Error;
use std::io::{self, BufRead, Write};

fn main() {
    if let Err(error) = run() {
        eprintln!("episode_baseline_agent: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let strategy = parse_strategy()?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut finished = false;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            return Err("blank JSONL request frame".into());
        }
        let request: EpisodeRequest = serde_json::from_str(&line)?;
        let response = next_response(&request, strategy)?;
        let is_finish = matches!(response.command, EpisodeCommand::Finish { .. });
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
        if is_finish {
            finished = true;
            break;
        }
    }

    if !finished {
        return Err("request stream ended before the policy submitted a finish command".into());
    }
    Ok(())
}

fn parse_strategy() -> Result<BaselineStrategy, Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut strategy = "fixed_order".to_owned();
    let mut seed = 23_u64;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--strategy" => {
                strategy = args
                    .next()
                    .ok_or("--strategy requires a value")?;
            }
            "--seed" => {
                seed = args
                    .next()
                    .ok_or("--seed requires a value")?
                    .parse()?;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: episode_baseline_agent [--strategy no_intervention|fixed_order|seeded_random|metadata_only] [--seed N]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    match strategy.as_str() {
        "no_intervention" => Ok(BaselineStrategy::NoIntervention),
        "fixed_order" => Ok(BaselineStrategy::FixedActionOrder),
        "seeded_random" => Ok(BaselineStrategy::SeededRandom { seed }),
        "metadata_only" => Ok(BaselineStrategy::MetadataOnlyFirstCandidate),
        other => Err(format!("unknown strategy: {other}").into()),
    }
}
