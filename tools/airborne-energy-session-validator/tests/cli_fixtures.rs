// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/airborne-energy-session-v0")
        .join(name)
}

fn run_fixture(name: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_airborne-energy-session-validator"))
        .arg(fixture(name))
        .output()
        .expect("CLI should start")
}

#[test]
fn cli_accepts_positive_reacquisition_trace() {
    let output = run_fixture("session.synthetic-reacquire.example.json");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS_SYNTHETIC_SESSION"));
    assert!(stdout.contains("power_intervals=2"));
}

#[test]
fn cli_accepts_not_run_without_promoting_it_to_a_pass() {
    let output = run_fixture("session.NOT_RUN.template.json");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("final_state=NOT_RUN"));
    assert!(stdout.contains("events=0"));
}

#[test]
fn cli_rejects_revoked_authorization_negative_control() {
    let output = run_fixture("session.adversarial-revoked-authorization.example.json");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("authorization_state=VALID_SYNTHETIC"));
}
