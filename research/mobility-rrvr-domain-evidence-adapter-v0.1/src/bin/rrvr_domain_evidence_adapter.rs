// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::process::ExitCode;

use mobility_rrvr_domain_evidence_adapter::{adapt_known_campaign, envelope_json};

fn main() -> ExitCode {
    match adapt_known_campaign().and_then(|envelope| envelope_json(&envelope)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("RRVR_DOMAIN_EVIDENCE_ADAPTER_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}
