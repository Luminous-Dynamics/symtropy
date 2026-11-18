#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Regression tests for the synthetic active-survey EVSI reference.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

scenario="tools/lunar-subsurface-survey-evsi/examples/synthetic-site-decision.json"
schema="tools/lunar-subsurface-survey-evsi/scenario.schema.json"
report="$(mktemp)"
invalid_prior="$(mktemp)"
invalid_likelihood="$(mktemp)"
trap 'rm -f "$report" "$invalid_prior" "$invalid_likelihood"' EXIT

fail() {
  echo "lunar survey EVSI fixture: FAIL: $*" >&2
  exit 1
}

jq -e '.["$schema"] == "https://json-schema.org/draft/2020-12/schema"
  and .title == "Lunar Subsurface Survey EVSI Scenario"' "$schema" >/dev/null \
  || fail "scenario schema JSON is malformed or unexpected"

bash scripts/calculate-lunar-survey-evsi.sh "$scenario" > "$report" \
  || fail "EVSI calculator rejected the synthetic reference scenario"

jq -e '
  def close($a; $b):
    if ($a - $b) < 0 then ($b - $a) < 0.00001 else ($a - $b) < 0.00001 end;
  .evidence_class == "synthetic_fixture"
  and .baseline_decision_id == "engineered_rock_tunnel"
  and close(.baseline_expected_utility; 40)
  and .survey_results[0].survey_id == "passive_seismic_fusion"
  and close(.survey_results[0].gross_evsi; 15.3125)
  and close(.survey_results[0].net_evsi; 7.3125)
  and close(.survey_results[0].expected_entropy_reduction_bits; 0.5621753)
  and .survey_results[1].survey_id == "orbital_radar_sounding"
  and close(.survey_results[1].gross_evsi; 7.625)
  and close(.survey_results[1].net_evsi; 2.625)
  and .survey_results[2].survey_id == "uninformative_sensor_control"
  and close(.survey_results[2].gross_evsi; 0)
  and close(.survey_results[2].net_evsi; -0.5)
  and close(.survey_results[2].expected_entropy_reduction_bits; 0)
  and .highest_net_evsi_survey.survey_id == "passive_seismic_fusion"
' "$report" >/dev/null || fail "EVSI known answers or survey ranking changed"

jq -e '
  .survey_results[]
  | select(.survey_id == "uninformative_sensor_control")
  | all(.outcomes[];
      .best_decision_id == "engineered_rock_tunnel"
      and (.posterior[0].probability == 0.25)
      and (.posterior[1].probability == 0.5)
      and (.posterior[2].probability == 0.25))
' "$report" >/dev/null || fail "uninformative negative control changed the posterior or decision"

jq '.prior_hypotheses[0].prior_probability = 0.4' "$scenario" > "$invalid_prior"
if bash scripts/calculate-lunar-survey-evsi.sh "$invalid_prior" >/dev/null 2>&1; then
  fail "invalid prior distribution was accepted"
fi

jq '.survey_options[0].outcomes[0].likelihood_by_hypothesis.lava_tube_corridor = 0.9' \
  "$scenario" > "$invalid_likelihood"
if bash scripts/calculate-lunar-survey-evsi.sh "$invalid_likelihood" >/dev/null 2>&1; then
  fail "invalid outcome-likelihood normalization was accepted"
fi

echo "lunar survey EVSI fixture: PASS (synthetic decision arithmetic only)"
