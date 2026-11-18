use std::{env, fs, io::{self, Read}, process::ExitCode};

use lunar_regolith_feeder_supervisor::{FeederSupervisor, Observation, SupervisorConfig};

fn read_input() -> Result<String, String> {
    let mut args = env::args().skip(1);
    if let Some(path) = args.next() {
        if args.next().is_some() {
            return Err("usage: lunar-regolith-feeder-supervisor [input.json]".to_owned());
        }
        fs::read_to_string(&path).map_err(|error| format!("cannot read {path}: {error}"))
    } else {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("cannot read stdin: {error}"))?;
        Ok(input)
    }
}

#[derive(serde::Deserialize)]
struct Input {
    config: SupervisorConfig,
    state: Option<lunar_regolith_feeder_supervisor::SupervisorState>,
    observation: Observation,
}

fn main() -> ExitCode {
    let result = (|| {
        let raw = read_input()?;
        let input: Input =
            serde_json::from_str(&raw).map_err(|error| format!("invalid input JSON: {error}"))?;
        let mut supervisor = FeederSupervisor::new(input.config)?;
        if let Some(state) = input.state {
            supervisor.state = state;
        }
        let decision = supervisor.step(&input.observation)?;
        serde_json::to_string_pretty(&decision)
            .map_err(|error| format!("cannot serialize decision: {error}"))
    })();

    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("feeder supervisor error: {error}");
            ExitCode::FAILURE
        }
    }
}
