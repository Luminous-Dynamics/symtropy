#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"
spec="tools/lunar-granular-flow-benchmark/controller-tournament.json"

fail() {
  printf 'lunar feeder controller tournament: FAIL: %s\n' "$*" >&2
  exit 1
}

command -v jq >/dev/null 2>&1 || fail "jq is required"
jq -e '
  .schema_version == "0.1.0"
  and .benchmark_id == "lunar-feeder-controller-tournament-v0"
  and .evidence_class == "benchmark_protocol_only"
  and .status == "planned_not_run"
  and ([.controllers[].id] | length == (unique | length))
  and ([.controllers[].id] | sort) == [
    "constrained_model_predictive_control",
    "feeder_pid_fixed_vibration",
    "fixed_vibration",
    "gain_scheduled_response_surface",
    "passive_no_vibration",
    "rule_based_supervisor",
    "safe_bayesian_experiment_selection"
  ]
  and ([.source_profiles[].id] | length) == 3
  and ([.source_profiles[].apparatus_boundary] | unique | length) == 3
  and ([.holdout_axes[]] | length == (unique | length))
  and .constraints.split_by_complete_batches_for_generalization == true
  and .constraints.blind_partition_requires_external_custodian == true
  and .constraints.no_controller_may_exceed_commissioned_frequency_gamma_or_actuator_stroke == true
  and .improvement_targets.hard_bound_violations_allowed == 0
  and .improvement_targets.status == "proposed_hypotheses_not_achieved"
  and (.claim_ceiling | contains("No head-to-head controller results"))
' "$spec" >/dev/null || fail "controller benchmark protocol or claim ceiling invalid"

printf 'lunar feeder controller tournament: PASS (protocol fixture only; no controller results)\n'
