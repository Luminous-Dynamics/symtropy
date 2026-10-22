#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Independent arithmetic checks for the synthetic Antarctic station profile.
# These checks validate fixture bookkeeping only, not Antarctic physics or operations.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$repo_root/research/antarctic-energy-resilience/cases/antarctic_station_v0.synthetic.json"

command -v jq >/dev/null 2>&1 || {
  echo "ERROR: jq is required to audit the Antarctic synthetic profile" >&2
  exit 2
}
test -f "$fixture" || {
  echo "ERROR: missing fixture: $fixture" >&2
  exit 2
}

jq -e '
  .schema_version == "antarctic-station-profile-v0.1"
  and .evidence_class == "synthetic_software_fixture_only"
  and .physical_data == false
  and (.periods | length) == 4
  and (([.periods[].id] | unique | length) == (.periods | length))
' "$fixture" >/dev/null

# Every interval has balanced electric and useful-heat ledgers.
jq -e --argjson tol 0.001 '
  all(.periods[];
    ((([.electric.sources_kwh[]] | add) - ([.electric.sinks_kwh[]] | add)) | fabs) <= $tol
  )
' "$fixture" >/dev/null

jq -e --argjson tol 0.001 '
  all(.periods[];
    ((([.heat.sources_kwh_th[]] | add) - ([.heat.sinks_kwh_th[]] | add)) | fabs) <= $tol
  )
' "$fixture" >/dev/null

# Independently recompute internal storage transitions.
jq -e --argjson tol 0.001 '
  all(.periods[];
    (((.battery_storage.opening_kwh_internal
       + .battery_storage.charge_input_kwh_e * .battery_storage.charge_efficiency
       - .battery_storage.discharge_to_bus_kwh_e / .battery_storage.discharge_efficiency
       - .battery_storage.standing_loss_kwh_internal)
       - .battery_storage.closing_kwh_internal) | fabs) <= $tol
  )
' "$fixture" >/dev/null

jq -e --argjson tol 0.001 '
  all(.periods[];
    (((.thermal_storage.opening_kwh_th_internal
       + .thermal_storage.charge_input_kwh_th * .thermal_storage.charge_efficiency
       - .thermal_storage.discharge_to_service_kwh_th / .thermal_storage.discharge_efficiency
       - .thermal_storage.standing_loss_kwh_th_internal)
       - .thermal_storage.closing_kwh_th_internal) | fabs) <= $tol
  )
' "$fixture" >/dev/null

# Reject negative flows/states, invalid efficiencies, negative closing energy,
# and a polar-night row that includes solar generation.
jq -e '
  all(.periods[];
    (([.electric.sources_kwh[]]
      + [.electric.sinks_kwh[]]
      + [.heat.sources_kwh_th[]]
      + [.heat.sinks_kwh_th[]]) | all(. >= 0))
    and .battery_storage.opening_kwh_internal >= 0
    and .battery_storage.closing_kwh_internal >= 0
    and .thermal_storage.opening_kwh_th_internal >= 0
    and .thermal_storage.closing_kwh_th_internal >= 0
    and (.battery_storage.charge_efficiency > 0 and .battery_storage.charge_efficiency <= 1)
    and (.battery_storage.discharge_efficiency > 0 and .battery_storage.discharge_efficiency <= 1)
    and (.thermal_storage.charge_efficiency > 0 and .thermal_storage.charge_efficiency <= 1)
    and (.thermal_storage.discharge_efficiency > 0 and .thermal_storage.discharge_efficiency <= 1)
  )
  and ([.periods[] | select(.id == "P2-polar-night") | .electric.sources_kwh.solar] == [0])
' "$fixture" >/dev/null

echo "PASS: Antarctic synthetic fixture schema labels, electric/thermal balances, storage transitions, and listed negative controls."
echo "BOUNDARY: software arithmetic only; values are not measurements or an operational station model."
sha256sum "$fixture"
