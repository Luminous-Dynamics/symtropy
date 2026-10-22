#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Independent arithmetic checks for the synthetic Antarctic station profile.
# Passing proves fixture bookkeeping only, not Antarctic physics or operations.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="${1:-$repo_root/research/antarctic-energy-resilience/cases/antarctic_station_v0.synthetic.json}"

command -v jq >/dev/null 2>&1 || {
  echo "ERROR: jq is required to audit the Antarctic synthetic profile" >&2
  exit 2
}
test -f "$fixture" || {
  echo "ERROR: missing fixture: $fixture" >&2
  exit 2
}

audit_fixture() {
  local candidate="$1"

  jq -e '
    .schema_version == "antarctic-station-profile-v0.1"
    and .evidence_class == "synthetic_software_fixture_only"
    and .physical_data == false
    and (.periods | length) == 4
    and ([.periods[].id] | unique | length) == (.periods | length)
  ' "$candidate" >/dev/null || return 1

  # Independent electric and useful-heat boundary ledgers.
  jq -e --argjson tol 0.001 '
    all(.periods[];
      ((([.electric.sources_kwh[]] | add)
        - ([.electric.sinks_kwh[]] | add)) | abs) <= $tol
      and ((([.heat.sources_kwh_th[]] | add)
        - ([.heat.sinks_kwh_th[]] | add)) | abs) <= $tol
    )
  ' "$candidate" >/dev/null || return 1

  # Recompute storage-state transitions and check bounds/efficiencies.
  jq -e --argjson tol 0.001 '
    all(.periods[];
      ((.battery_storage.opening_kwh_internal
        + .battery_storage.charge_input_kwh_e * .battery_storage.charge_efficiency
        - .battery_storage.discharge_to_bus_kwh_e / .battery_storage.discharge_efficiency
        - .battery_storage.standing_loss_kwh_internal
        - .battery_storage.closing_kwh_internal) | abs) <= $tol
      and ((.thermal_storage.opening_kwh_th_internal
        + .thermal_storage.charge_input_kwh_th * .thermal_storage.charge_efficiency
        - .thermal_storage.discharge_to_service_kwh_th / .thermal_storage.discharge_efficiency
        - .thermal_storage.standing_loss_kwh_th_internal
        - .thermal_storage.closing_kwh_th_internal) | abs) <= $tol
      and (.battery_storage.capacity_kwh_internal > 0)
      and (.battery_storage.opening_kwh_internal >= 0)
      and (.battery_storage.closing_kwh_internal >= 0)
      and (.battery_storage.opening_kwh_internal <= .battery_storage.capacity_kwh_internal)
      and (.battery_storage.closing_kwh_internal <= .battery_storage.capacity_kwh_internal)
      and (.thermal_storage.capacity_kwh_th_internal > 0)
      and (.thermal_storage.opening_kwh_th_internal >= 0)
      and (.thermal_storage.closing_kwh_th_internal >= 0)
      and (.thermal_storage.opening_kwh_th_internal <= .thermal_storage.capacity_kwh_th_internal)
      and (.thermal_storage.closing_kwh_th_internal <= .thermal_storage.capacity_kwh_th_internal)
      and (([.electric.sources_kwh[]]
        + [.electric.sinks_kwh[]]
        + [.heat.sources_kwh_th[]]
        + [.heat.sinks_kwh_th[]]) | all(. >= 0))
      and (.battery_storage.charge_efficiency > 0 and .battery_storage.charge_efficiency <= 1)
      and (.battery_storage.discharge_efficiency > 0 and .battery_storage.discharge_efficiency <= 1)
      and (.thermal_storage.charge_efficiency > 0 and .thermal_storage.charge_efficiency <= 1)
      and (.thermal_storage.discharge_efficiency > 0 and .thermal_storage.discharge_efficiency <= 1)
    )
    and ([.periods[] | select(.id == "P2-polar-night") | .electric.sources_kwh.solar] == [0])
  ' "$candidate" >/dev/null || return 1
}

expect_rejected() {
  local label="$1"
  local filter="$2"
  local mutated
  mutated="$(mktemp)"
  if ! jq "$filter" "$fixture" >"$mutated"; then
    rm -f "$mutated"
    echo "ERROR: could not construct negative control: $label" >&2
    return 1
  fi
  if audit_fixture "$mutated" >/dev/null 2>&1; then
    rm -f "$mutated"
    echo "ERROR: invalid case was accepted: $label" >&2
    return 1
  fi
  rm -f "$mutated"
  echo "PASS: rejected negative control — $label"
}

audit_fixture "$fixture"
echo "PASS: baseline synthetic fixture passes schema, electric/thermal balances, storage transitions, capacity and polar-night checks."

expect_rejected "electric energy imbalance" '.periods[0].electric.sinks_kwh.critical_load += 1'
expect_rejected "useful-heat imbalance" '.periods[0].heat.sinks_kwh_th.critical_useful_heat += 1'
expect_rejected "tampered battery closing state" '.periods[0].battery_storage.closing_kwh_internal += 1'
expect_rejected "battery state above declared capacity" '.periods[0].battery_storage.capacity_kwh_internal = 10'
expect_rejected "thermal storage above declared capacity" '.periods[0].thermal_storage.capacity_kwh_th_internal = 10'
expect_rejected "invalid storage efficiency" '.periods[0].thermal_storage.charge_efficiency = 1.2'
expect_rejected "negative electric source" '.periods[0].electric.sources_kwh.wind = -1'
expect_rejected "solar generation during synthetic polar night" '.periods[1].electric.sources_kwh.solar = 10 | .periods[1].electric.sinks_kwh.electrical_loss += 10'
expect_rejected "missing critical electricity load" 'del(.periods[0].electric.sinks_kwh.critical_load)'
expect_rejected "duplicate period identifier" '.periods[1].id = .periods[0].id'
expect_rejected "incorrect evidence label" '.physical_data = true'

echo "BOUNDARY: synthetic arithmetic and software negative controls only; no measured Antarctic performance or operational station model is established."
sha256sum "$fixture"
