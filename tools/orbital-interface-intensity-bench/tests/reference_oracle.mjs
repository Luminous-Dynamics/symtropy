#!/usr/bin/env node
import fs from "node:fs";

const TOLERANCE = 1e-10;

function poissonCdfAtMost(events, mean) {
  let mass = Math.exp(-mean);
  let sum = mass;
  for (let index = 1; index <= events; index += 1) {
    mass *= mean / index;
    sum += mass;
  }
  return Math.min(1, sum);
}

// Independent probability-domain recurrence; Rust uses log-sum-exp.
function poissonMeanUpperBound(events, confidence) {
  const alpha = 1 - confidence;
  if (events === 0) return -Math.log(alpha);
  let lower = 0;
  let upper = Math.max(
    events + 12 * Math.sqrt(events + 1) + 10,
    -Math.log(alpha) + 1,
  );
  while (poissonCdfAtMost(events, upper) > alpha) upper *= 2;
  for (let iteration = 0; iteration < 120; iteration += 1) {
    const midpoint = lower + (upper - lower) / 2;
    if (poissonCdfAtMost(events, midpoint) > alpha) lower = midpoint;
    else upper = midpoint;
  }
  return upper;
}

function approximatelyEqual(actual, expected, tolerance = TOLERANCE) {
  return Math.abs(actual - expected) <= tolerance * Math.max(1, Math.abs(actual));
}

function fail(message) {
  process.stderr.write(String(message) + "\n");
  process.exitCode = 1;
}

const path = process.argv[2];
if (!path) {
  fail("usage: node reference_oracle.mjs <intensity-cases.json>");
} else {
  const corpus = JSON.parse(fs.readFileSync(path, "utf8"));
  if (corpus.schema !== "luminous.orbital-interface-intensity.corpus.v0.1") {
    fail("unsupported corpus schema");
  }
  if (corpus.evidence_class !== "synthetic") fail("oracle accepts synthetic fixtures only");
  if (corpus.cases.length !== 34) fail("expected 34 frozen cases, received " + corpus.cases.length);

  let arithmeticCases = 0;
  for (const scenario of corpus.cases) {
    const expected = scenario.expected;
    if (expected.upper_bound_per_million_hours === null) continue;

    const byRecordId = new Map();
    for (const article of scenario.articles) {
      const prior = byRecordId.get(article.record_id);
      if (prior && JSON.stringify(prior) !== JSON.stringify(article)) {
        fail(scenario.case_id + ": conflicting record ID in numerical case");
        continue;
      }
      if (!prior) byRecordId.set(article.record_id, article);
    }

    const records = [...byRecordId.values()];
    const exposureSeconds = records.reduce((sum, article) => sum + article.exposure_seconds, 0);
    const events = records.reduce((sum, article) => sum + article.failure_events, 0);
    const exposureHours = exposureSeconds / 3600;
    const meanUpper = poissonMeanUpperBound(events, scenario.confidence);
    const upperRate = meanUpper / exposureHours * 1_000_000;
    const observedRate = events / exposureHours * 1_000_000;
    const meetsTarget = upperRate <= scenario.target_failure_intensity_per_million_hours;
    const requiredHours =
      -Math.log(1 - scenario.confidence) *
      1_000_000 /
      scenario.target_failure_intensity_per_million_hours;

    if (records.length !== expected.unique_articles) fail(scenario.case_id + ": article count mismatch");
    if (exposureSeconds !== expected.total_exposure_seconds) fail(scenario.case_id + ": exposure mismatch");
    if (events !== expected.failure_events) fail(scenario.case_id + ": event-count mismatch");
    if (!approximatelyEqual(upperRate, expected.upper_bound_per_million_hours)) {
      fail(scenario.case_id + ": upper-bound mismatch");
    }
    if (meetsTarget !== expected.meets_target) fail(scenario.case_id + ": target decision mismatch");
    if (!approximatelyEqual(observedRate, expected.observed_rate_per_million_hours)) {
      fail(scenario.case_id + ": observed-rate mismatch");
    }
    if (!approximatelyEqual(requiredHours, expected.minimum_zero_failure_exposure_hours)) {
      fail(scenario.case_id + ": zero-failure exposure mismatch");
    }
    if ((observedRate <= scenario.target_failure_intensity_per_million_hours && !meetsTarget) !==
        expected.naive_false_assurance) {
      fail(scenario.case_id + ": naive false-assurance mismatch");
    }

    const permuted = [...records].reverse();
    const permutedExposure = permuted.reduce((sum, article) => sum + article.exposure_seconds, 0);
    const permutedEvents = permuted.reduce((sum, article) => sum + article.failure_events, 0);
    const permutedUpper = poissonMeanUpperBound(permutedEvents, scenario.confidence) /
      (permutedExposure / 3600) * 1_000_000;
    if (!approximatelyEqual(permutedUpper, upperRate)) {
      fail(scenario.case_id + ": article-order permutation changed result");
    }
    arithmeticCases += 1;
  }

  if (!approximatelyEqual(poissonMeanUpperBound(0, 0.9), 2.302585092994046, 1e-12)) {
    fail("zero-event known answer mismatch");
  }
  if (!approximatelyEqual(poissonMeanUpperBound(1, 0.9), 3.889720169867429, 1e-10)) {
    fail("one-event known answer mismatch");
  }
  if (!approximatelyEqual(
    poissonMeanUpperBound(1, 0.9) * 1_000_000 / 100,
    38_897.20169867429,
    1e-10,
  )) {
    fail("one-allowed-failure exposure-sizing known answer mismatch");
  }
  if (arithmeticCases < 10) fail("expected at least 10 numerical cases, received " + arithmeticCases);

  if (process.exitCode !== 1) {
    process.stdout.write(
      "PASS: " + corpus.cases.length + " frozen cases; " +
      arithmeticCases + " independent numerical replays\n",
    );
  }
}
