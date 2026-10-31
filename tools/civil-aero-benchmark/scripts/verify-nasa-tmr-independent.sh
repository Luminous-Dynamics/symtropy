#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
package_dir=$(cd -- "$script_dir/.." && pwd)
cd "$package_dir"

echo "[1/4] Verify exact retained bytes"
sha256sum -c data/reference/nasa-tmr/SHA256SUMS

echo "[2/4] Independently parse Ladson force zones with awk"
awk '
function numeric(s) {
  return s ~ /^[-+]?(([0-9]+([.][0-9]*)?)|([.][0-9]+))([eE][-+]?[0-9]+)?$/
}
/^[[:space:]]*$/ || /^#/ || /^variables=/ { next }
/^zone,/ {
  if (match($0, /t="[^"]+"/) == 0) exit 20
  label = substr($0, RSTART + 3, RLENGTH - 4)
  if (label in seen) exit 21
  seen[label] = 1
  zone_count++
  current = label
  next
}
{
  if (current == "") exit 22
  if (NF != 3) exit 23
  if (!numeric($1) || !numeric($2) || !numeric($3)) exit 24
  if (($3 + 0) < 0) exit 25
  rows++
  by_zone[current]++
}
END {
  if (rows != 53 || by_zone["80 grit"] != 17 ||
      by_zone["120 grit"] != 18 || by_zone["180 grit"] != 18 ||
      zone_count != 3) exit 26
  print "force_rows=" rows " zones=3 counts=80:17,120:18,180:18"
}
' data/reference/nasa-tmr/CLCD_Ladson_expdata.dat

echo "[3/4] Independently parse nine Ladson pressure zones with awk"
awk '
function numeric(s) {
  return s ~ /^[-+]?(([0-9]+([.][0-9]*)?)|([.][0-9]+))([eE][-+]?[0-9]+)?$/
}
/^[[:space:]]*$/ || /^#/ || /^variables=/ { next }
/^zone,/ {
  if (match($0, /t="[^"]+"/) == 0) exit 30
  label = substr($0, RSTART + 3, RLENGTH - 4)
  if (label in seen) exit 31
  seen[label] = 1
  zone_count++
  current = label
  next
}
{
  if (current == "") exit 32
  if (NF != 2) exit 33
  if (!numeric($1) || !numeric($2)) exit 34
  x = $1 + 0
  if (x < 0 || x > 1) exit 35
  rows++
  by_zone[current]++
}
END {
  if (rows != 405 || zone_count != 9) exit 36
  for (zone in seen) if (by_zone[zone] != 45) exit 37
  print "ladson_pressure_rows=" rows " zones=9 points_per_zone=45"
}
' data/reference/nasa-tmr/CP_Ladson.dat

echo "[4/4] Independently parse Gregory upper-surface pressure zones with awk"
awk '
function numeric(s) {
  return s ~ /^[-+]?(([0-9]+([.][0-9]*)?)|([.][0-9]+))([eE][-+]?[0-9]+)?$/
}
/^[[:space:]]*$/ || /^#/ || /^variables=/ { next }
/^zone,/ {
  if (match($0, /t="[^"]+"/) == 0) exit 40
  label = substr($0, RSTART + 3, RLENGTH - 4)
  if (label in seen) exit 41
  seen[label] = 1
  zone_count++
  current = label
  next
}
{
  if (current == "") exit 42
  if (NF != 2) exit 43
  if (!numeric($1) || !numeric($2)) exit 44
  x = $1 + 0
  if (x < 0 || x > 1) exit 45
  rows++
  by_zone[current]++
}
END {
  if (rows != 73 || zone_count != 3 ||
      by_zone["alpha=0"] != 25 ||
      by_zone["alpha=10"] != 24 ||
      by_zone["alpha=15"] != 24) exit 46
  print "gregory_pressure_rows=" rows " zones=3 counts=alpha0:25,alpha10:24,alpha15:24"
}
' data/reference/nasa-tmr/CP_Gregory_expdata.dat

echo "independent_raw_parse=PASS; this is source-byte and parser-structure verification, not CFD validation"
