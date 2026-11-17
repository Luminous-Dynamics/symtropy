// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::Write;
use std::process::{Command, Stdio};

const RUNNER: &str = env!("CARGO_BIN_EXE_human-systems-benchmark");

#[test]
fn full_runner_executes_candidate_separately_and_scores_observationally_equivalent_worlds() {
    let output = Command::new(RUNNER)
        .env_clear()
        .output()
        .expect("benchmark runner should start");

    assert!(
        output.status.success(),
        "runner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("runner output should be UTF-8");
    assert!(stdout.contains("RUNNER\tprotocol=1"));
    assert!(stdout.contains("BOUNDARY\tchild_process=true\tenvironment=scrubbed"));
    assert!(stdout.contains("IDENTIFIABILITY\tid=HOBS-11\tcandidate_invocations=1\toracle_worlds=2\tidentical_candidate_stream=true"));
    assert!(stdout.contains("RATE_ORACLE\tkind=ideal_brick_wall"));
    assert!(stdout.contains("HIE\tid=HIE-04"));
    assert!(stdout.contains("COMPLETE\tcandidate_processes=12"));

    let scenario_rows = stdout.lines().filter(|line| line.starts_with("HOBS\t")).count();
    assert_eq!(scenario_rows, 12, "every canonical HOBS scenario must be scored");
}

#[test]
fn candidate_process_rejects_an_unknown_wire_version() {
    let mut child = Command::new(RUNNER)
        .arg("--candidate-v1")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("candidate child should start");

    child
        .stdin
        .take()
        .expect("candidate stdin should exist")
        .write_all(b"UNKNOWN-WIRE\tHOBS-00\n")
        .expect("malformed payload should be delivered");

    let output = child
        .wait_with_output()
        .expect("candidate child should exit after malformed input");
    assert!(!output.status.success(), "unknown wire versions must fail closed");
}
