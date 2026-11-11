use std::env;
use std::fs;
use std::process::ExitCode;

use symtropy_observation_frame_reference::{
    MAX_INPUT_BYTES, ObservationFrameInputV1, compile_observation_frame,
};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "usage: observation-frame-reference <input.json>",
        )
    })?;
    if args.next().is_some() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "expected exactly one input path",
        )
        .into());
    }

    let metadata = fs::metadata(&path)?;
    if metadata.len() > MAX_INPUT_BYTES as u64 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "input exceeds the 64 MiB reference-oracle bound",
        )
        .into());
    }
    let bytes = fs::read(&path)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "input exceeds the 64 MiB reference-oracle bound",
        )
        .into());
    }
    let input: ObservationFrameInputV1 = serde_json::from_slice(&bytes)?;
    let receipt = compile_observation_frame(input)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("observation-frame-reference: {error}");
            ExitCode::FAILURE
        }
    }
}
