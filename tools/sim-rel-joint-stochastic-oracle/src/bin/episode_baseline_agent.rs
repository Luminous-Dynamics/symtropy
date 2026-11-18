//! JSONL child-process baseline policy for scientific-world episode requests.
//!
//! The process receives only EpisodeRequest frames on stdin and writes one
//! EpisodeResponse frame per request on stdout. Diagnostics go to stderr.

use sim_rel_joint_stochastic_oracle::episode::{EpisodeCommand, EpisodeRequest};
use sim_rel_joint_stochastic_oracle::policy::{next_response, BaselineStrategy};
use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};

const MAX_JSONL_FRAME_BYTES: usize = 16 * 1024 * 1024;

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
    let mut input = BufReader::new(stdin.lock());
    let mut output = stdout.lock();
    let mut request_frame = Vec::new();
    let mut finished = false;

    while read_bounded_line(&mut input, &mut request_frame, MAX_JSONL_FRAME_BYTES)? {
        if request_frame.iter().all(u8::is_ascii_whitespace) {
            return Err("blank JSONL request frame".into());
        }
        let request: EpisodeRequest = serde_json::from_slice(&request_frame)?;
        let response = next_response(&request, strategy)?;
        let is_finish = matches!(&response.command, EpisodeCommand::Finish { .. });
        let mut response_frame = serde_json::to_vec(&response)?;
        response_frame.push(b'\n');
        if response_frame.len() > MAX_JSONL_FRAME_BYTES {
            return Err("policy response exceeds the JSONL frame byte cap".into());
        }
        output.write_all(&response_frame)?;
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

/// Read exactly one newline-terminated JSONL frame without ever buffering
/// more than the supplied maximum byte count. EOF with a partial frame is rejected.
fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    frame: &mut Vec<u8>,
    maximum: usize,
) -> io::Result<bool> {
    frame.clear();
    loop {
        let mut consumed = 0_usize;
        let mut terminated = false;
        {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                if frame.is_empty() {
                    return Ok(false);
                }
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unterminated JSONL request frame",
                ));
            }
            let count = available
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(available.len(), |index| index + 1);
            if frame.len().saturating_add(count) > maximum {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "JSONL request frame exceeds the byte cap",
                ));
            }
            frame.extend_from_slice(&available[..count]);
            consumed = count;
            terminated = available[count - 1] == b'\n';
        }
        reader.consume(consumed);
        if terminated {
            return Ok(true);
        }
    }
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
                    "Usage: episode_baseline_agent [--strategy no_intervention|fixed_order|seeded_random|one_step_eig|metadata_only] [--seed N]"
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
        "one_step_eig" => Ok(BaselineStrategy::OneStepJointEig),
        "metadata_only" => Ok(BaselineStrategy::MetadataOnlyFirstCandidate),
        other => Err(format!("unknown strategy: {other}").into()),
    }
}
