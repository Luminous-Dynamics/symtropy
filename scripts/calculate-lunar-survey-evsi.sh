#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# One-step expected value of sample information (EVSI) for lunar site surveys.
# This is a decision-analysis reference, not a geophysical inversion solver.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

usage() {
  echo "Usage: bash scripts/calculate-lunar-survey-evsi.sh SCENARIO.json" >&2
  exit 2
}

[[ "$#" -eq 1 ]] || usage
scenario="$1"
[[ -f "$scenario" && ! -L "$scenario" ]] || {
  echo "EVSI scenario is missing or is a symlink: $scenario" >&2
  exit 1
}
command -v jq >/dev/null 2>&1 || {
  echo "jq is required" >&2
  exit 1
}

fail() {
  echo "lunar survey EVSI: FAIL: $*" >&2
  exit 1
}

jq -e '
  .schema_version == "0.1.0"
  and (.scenario_id | type == "string" and length > 0)
  and (.source_class == "synthetic_fixture"
       or .source_class == "published_evidence_model"
       or .source_class == "physical_analogue_model")
  and (.prior_hypotheses | type == "array" and length >= 2)
  and (.decisions | type == "array" and length >= 2)
  and (.survey_options | type == "array" and length >= 1)
' "$scenario" >/dev/null || fail "scenario structure is invalid"

jq -e '
  . as $s
  | [$s.prior_hypotheses[].hypothesis_id] as $hypothesis_ids
  | [$s.decisions[].decision_id] as $decision_ids
  | [$s.survey_options[].survey_id] as $survey_ids
  | ([$s.prior_hypotheses[].prior_probability] | add) as $prior_sum
  | ($hypothesis_ids | length) == ($hypothesis_ids | unique | length)
  and ($decision_ids | length) == ($decision_ids | unique | length)
  and ($survey_ids | length) == ($survey_ids | unique | length)
  and ((if ($prior_sum - 1) < 0 then 1 - $prior_sum else $prior_sum - 1 end) <= 1e-9)
  and all($s.decisions[];
    ((.utility_by_hypothesis | keys | sort) == ($hypothesis_ids | sort))
    and all(.utility_by_hypothesis[]; type == "number"))
  and all($s.survey_options[];
    . as $survey
    | ([$survey.outcomes[].outcome_id] | length) >= 2
      and ([$survey.outcomes[].outcome_id] | length)
          == ([$survey.outcomes[].outcome_id] | unique | length)
      and all($survey.outcomes[];
        ((.likelihood_by_hypothesis | keys | sort) == ($hypothesis_ids | sort))
        and all(.likelihood_by_hypothesis[];
          type == "number" and . >= 0 and . <= 1))
      and all($hypothesis_ids[];
        . as $hypothesis_id
        | ([$survey.outcomes[].likelihood_by_hypothesis[$hypothesis_id]] | add) as $sum
        | (if ($sum - 1) < 0 then 1 - $sum else $sum - 1 end) <= 1e-9)
      and ($survey.cost_utility | type == "number" and . >= 0)
      and ($survey.resources.instrument_mass_kg | type == "number" and . >= 0)
      and ($survey.resources.energy_wh | type == "number" and . >= 0)
      and ($survey.resources.duration_h | type == "number" and . >= 0))
' "$scenario" >/dev/null || fail "probabilities, decision utility keys, survey likelihoods, costs or IDs are invalid"

jq -e '
  . as $s
  | if $s.source_class == "synthetic_fixture" then
      (all($s.prior_hypotheses[]; .evidence_status == "synthetic_assumption"))
      and (all($s.decisions[]; .utility_status == "synthetic_assumption"))
      and (all($s.survey_options[]; .likelihood_status == "synthetic_assumption"))
    else
      (all($s.prior_hypotheses[];
        if .evidence_status == "synthetic_assumption" then true else (.source_refs | length) > 0 end))
      and (all($s.decisions[];
        if .utility_status == "synthetic_assumption" then true else (.utility_refs | length) > 0 end))
      and (all($s.survey_options[];
        if .likelihood_status == "synthetic_assumption" then true else (.likelihood_refs | length) > 0 end))
    end
' "$scenario" >/dev/null || fail "evidence status/source reference policy failed"

jq '
  def expected_utility($weights; $utilities):
    reduce $weights[] as $weight
      (0; . + ($weight.probability * $utilities[$weight.hypothesis_id]));

  def entropy_bits($weights):
    reduce $weights[] as $weight
      (0;
        if $weight.probability <= 0 then .
        else . - ($weight.probability * (($weight.probability | log) / (2 | log)))
        end);

  . as $scenario
  | $scenario.prior_hypotheses as $hypotheses
  | $scenario.decisions as $decisions
  | [$hypotheses[] |
      {hypothesis_id, probability: .prior_probability}] as $prior_weights
  | ([
      $decisions[] |
      {decision_id,
       expected_utility: expected_utility($prior_weights; .utility_by_hypothesis)}
    ] | max_by(.expected_utility)) as $baseline
  | entropy_bits($prior_weights) as $prior_entropy
  | [
      $scenario.survey_options[] as $survey
      | [
          $survey.outcomes[] as $outcome
          | (reduce $hypotheses[] as $hypothesis
              (0; . + ($hypothesis.prior_probability
                       * $outcome.likelihood_by_hypothesis[$hypothesis.hypothesis_id])))
            as $observation_probability
          | if $observation_probability <= 0 then
              {outcome_id: $outcome.outcome_id,
               probability: 0,
               posterior: null,
               best_decision_id: null,
               conditional_expected_utility: null,
               posterior_entropy_bits: null}
            else
              ([
                $hypotheses[] |
                {hypothesis_id,
                 probability: (.prior_probability
                   * $outcome.likelihood_by_hypothesis[.hypothesis_id]
                   / $observation_probability)}
              ]) as $posterior
              | ([
                  $decisions[] |
                  {decision_id,
                   expected_utility: expected_utility($posterior; .utility_by_hypothesis)}
                ] | max_by(.expected_utility)) as $posterior_decision
              | {outcome_id: $outcome.outcome_id,
                 probability: $observation_probability,
                 posterior: $posterior,
                 best_decision_id: $posterior_decision.decision_id,
                 conditional_expected_utility: $posterior_decision.expected_utility,
                 posterior_entropy_bits: entropy_bits($posterior)}
            end
        ] as $outcome_results
      | (reduce $outcome_results[] as $result
          (0; . + ($result.probability * ($result.conditional_expected_utility // 0))))
        as $expected_utility_after_survey
      | (reduce $outcome_results[] as $result
          (0; . + ($result.probability * ($result.posterior_entropy_bits // 0))))
        as $expected_posterior_entropy
      | {
          survey_id: $survey.survey_id,
          modality: $survey.modality,
          likelihood_status: $survey.likelihood_status,
          baseline_decision_id: $baseline.decision_id,
          baseline_expected_utility: $baseline.expected_utility,
          expected_utility_after_survey: $expected_utility_after_survey,
          gross_evsi: ($expected_utility_after_survey - $baseline.expected_utility),
          cost_utility: $survey.cost_utility,
          net_evsi: ($expected_utility_after_survey - $baseline.expected_utility - $survey.cost_utility),
          prior_entropy_bits: $prior_entropy,
          expected_posterior_entropy_bits: $expected_posterior_entropy,
          expected_entropy_reduction_bits: ($prior_entropy - $expected_posterior_entropy),
          resources: $survey.resources,
          net_evsi_per_instrument_kg:
            (if $survey.resources.instrument_mass_kg > 0
             then ($expected_utility_after_survey - $baseline.expected_utility - $survey.cost_utility)
                  / $survey.resources.instrument_mass_kg else null end),
          net_evsi_per_energy_wh:
            (if $survey.resources.energy_wh > 0
             then ($expected_utility_after_survey - $baseline.expected_utility - $survey.cost_utility)
                  / $survey.resources.energy_wh else null end),
          net_evsi_per_duration_h:
            (if $survey.resources.duration_h > 0
             then ($expected_utility_after_survey - $baseline.expected_utility - $survey.cost_utility)
                  / $survey.resources.duration_h else null end),
          outcomes: $outcome_results
        }
    ] as $results
  | {
      evidence_class: $scenario.source_class,
      scenario_id: $scenario.scenario_id,
      utility_unit: $scenario.utility_unit,
      baseline_decision_id: $baseline.decision_id,
      baseline_expected_utility: $baseline.expected_utility,
      prior_entropy_bits: $prior_entropy,
      survey_results: ($results | sort_by(.net_evsi) | reverse),
      highest_net_evsi_survey:
        ([$results[] | select(.net_evsi > 0)] | sort_by(.net_evsi) | reverse | first // null),
      decision_rule:
        "rank survey options by net EVSI; do not update posterior for invalid, stale, uncalibrated, or unavailable measurements",
      claim_ceiling:
        "conditional decision arithmetic from supplied priors, likelihoods, utilities and costs; not geological detection, stability, entry, or construction qualification"
    }
' "$scenario"
