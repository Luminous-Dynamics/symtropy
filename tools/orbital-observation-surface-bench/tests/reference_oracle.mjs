import fs from "node:fs";

const SCHEMA = "luminous.orbital-observation-surface.corpus.v0.1";
const CLAIM = "stateful-marked-observation-simulation-only";
const MASK64 = (1n << 64n) - 1n;
const MAX_SAFE_SEED = Number.MAX_SAFE_INTEGER;
const GOLDEN = 0x9E3779B97F4A7C15n;
const MIX_A = 0xBF58476D1CE4E5B9n;
const MIX_B = 0x94D049BB133111EBn;

function fail(message) {
  throw new Error(message);
}

class SplitMix64 {
  constructor(seed) {
    this.state = BigInt(seed);
  }

  nextU64() {
    this.state = (this.state + GOLDEN) & MASK64;
    let value = this.state;
    value = ((value ^ (value >> 30n)) * MIX_A) & MASK64;
    value = ((value ^ (value >> 27n)) * MIX_B) & MASK64;
    return (value ^ (value >> 31n)) & MASK64;
  }

  uniformOpen() {
    const numerator = Number((this.nextU64() >> 12n) + 1n);
    return numerator / (2 ** 52 + 1);
  }
}

function logistic(value) {
  if (value >= 0) return 1 / (1 + Math.exp(-value));
  const exponential = Math.exp(value);
  return exponential / (1 + exponential);
}

function podProbability(event, channel) {
  const linearPredictor = channel.logit_intercept
    + channel.time_slope * (event.time_fraction - 0.5)
    + channel.log_amplitude_slope * Math.log(event.amplitude)
    + channel.log_duration_slope * Math.log(event.duration);
  return logistic(linearPredictor);
}

function inIntervals(time, intervals) {
  return intervals.some(interval => time >= interval.start && time < interval.end);
}

function validateIntervals(intervals) {
  let previousEnd = 0;
  intervals.forEach((item, index) => {
    if (![item.start, item.end].every(Number.isFinite)
      || item.start < 0 || item.end > 1 || item.start >= item.end
      || (index > 0 && item.start < previousEnd)) {
      fail("outage intervals must be sorted, non-overlapping and inside [0,1]");
    }
    previousEnd = item.end;
  });
}

function validateCorpus(corpus) {
  if (corpus.schema !== SCHEMA || corpus.evidence_class !== "synthetic"
    || corpus.claim_ceiling !== CLAIM || !Array.isArray(corpus.scenarios)
    || corpus.scenarios.length === 0) {
    fail("unsupported observation-surface corpus or claim ceiling");
  }

  const scenarioIds = new Set();
  const usedSeeds = new Set();
  for (const scenario of corpus.scenarios) {
    if (typeof scenario.scenario_id !== "string" || !scenario.scenario_id.trim()
      || scenarioIds.has(scenario.scenario_id)
      || typeof scenario.configuration_id !== "string" || !scenario.configuration_id.trim()
      || !Number.isFinite(scenario.exposure_horizon) || scenario.exposure_horizon <= 0
      || !Array.isArray(scenario.events) || scenario.events.length === 0 || scenario.events.length > 100000
      || !Array.isArray(scenario.channels) || scenario.channels.length === 0 || scenario.channels.length > 8) {
      fail("scenario identity, exposure, event count or channel count is invalid");
    }
    scenarioIds.add(scenario.scenario_id);
    validateIntervals(scenario.shared_common_cause_outages);

    const eventIds = new Set();
    let previousTime = -1;
    for (const event of scenario.events) {
      if (typeof event.event_id !== "string" || !event.event_id.trim() || eventIds.has(event.event_id)
        || !Number.isFinite(event.time_fraction) || event.time_fraction < 0 || event.time_fraction > 1
        || event.time_fraction <= previousTime
        || !Number.isFinite(event.amplitude) || event.amplitude <= 0
        || !Number.isFinite(event.duration) || event.duration <= 0) {
        fail("physical events must have unique IDs, strictly increasing times and positive finite marks");
      }
      previousTime = event.time_fraction;
      eventIds.add(event.event_id);
    }

    const channelIds = new Set();
    for (const channel of scenario.channels) {
      const coefficients = [channel.logit_intercept, channel.time_slope,
        channel.log_amplitude_slope, channel.log_duration_slope];
      if (typeof channel.channel_id !== "string" || !channel.channel_id.trim()
        || channelIds.has(channel.channel_id)
        || !coefficients.every(value => Number.isFinite(value) && Math.abs(value) <= 10000)
        || !Number.isFinite(channel.fixed_dead_time) || channel.fixed_dead_time < 0 || channel.fixed_dead_time > 1
        || !Number.isFinite(channel.false_positive_rate) || channel.false_positive_rate < 0
        || channel.false_positive_rate > 10000
        || !Array.isArray(channel.channel_outages)) {
        fail("channel identity, POD coefficients, dead time or false-positive rate is invalid");
      }
      channelIds.add(channel.channel_id);
      validateIntervals(channel.channel_outages);
      for (const seed of [channel.detection_seed, channel.false_positive_seed]) {
        if (!Number.isSafeInteger(seed) || seed < 0 || seed > MAX_SAFE_SEED || usedSeeds.has(seed)) {
          fail("random seeds must be unique across streams and exactly representable in JavaScript");
        }
        usedSeeds.add(seed);
      }
    }
  }
}

function simulateFalsePositives(channel) {
  const random = new SplitMix64(channel.false_positive_seed);
  const times = [];
  if (channel.false_positive_rate === 0) return times;
  let time = 0;
  while (true) {
    time += -Math.log(random.uniformOpen()) / channel.false_positive_rate;
    if (time >= 1) break;
    if (times.length >= 100000) fail("false-positive simulation exceeded the frozen event cap");
    times.push(time);
  }
  return times;
}

function simulateChannel(scenario, channel) {
  const random = new SplitMix64(channel.detection_seed);
  let busyUntil = -Infinity;
  let detectedEventCount = 0;
  const missCounts = {
    shared_common_cause: 0,
    channel_outage: 0,
    dead_time: 0,
    stochastic: 0
  };
  const eventOutcomes = scenario.events.map(event => {
    // Always consume one draw per latent event: outage edits do not shift
    // the later Bernoulli stream.
    const draw = random.uniformOpen();
    const probability = podProbability(event, channel);
    let missReason = null;

    if (inIntervals(event.time_fraction, scenario.shared_common_cause_outages)) {
      missCounts.shared_common_cause += 1;
      missReason = "shared_common_cause";
    } else if (inIntervals(event.time_fraction, channel.channel_outages)) {
      missCounts.channel_outage += 1;
      missReason = "channel_outage";
    } else if (event.time_fraction < busyUntil) {
      missCounts.dead_time += 1;
      missReason = "dead_time";
    } else {
      // Non-paralyzable front-end: every event reaching the detector starts
      // recovery even when the classifier returns a stochastic miss.
      busyUntil = event.time_fraction + channel.fixed_dead_time;
      if (draw < probability) detectedEventCount += 1;
      else {
        missCounts.stochastic += 1;
        missReason = "stochastic_miss";
      }
    }

    return {
      event_id: event.event_id,
      pod_probability: probability,
      detected: missReason === null,
      miss_reason: missReason
    };
  });

  const falsePositiveTimes = simulateFalsePositives(channel).filter(time =>
    !inIntervals(time, scenario.shared_common_cause_outages)
    && !inIntervals(time, channel.channel_outages));

  return {
    channel_id: channel.channel_id,
    latent_event_count: scenario.events.length,
    detected_event_count: detectedEventCount,
    miss_counts: missCounts,
    false_positive_count: falsePositiveTimes.length,
    false_positive_times: falsePositiveTimes,
    event_outcomes: eventOutcomes
  };
}

function evaluate(corpus) {
  validateCorpus(corpus);
  const summary = {
    scenario_count: 0,
    channel_count: 0,
    channel_event_opportunity_count: 0,
    detected_event_count: 0,
    shared_common_cause_misses: 0,
    channel_outage_misses: 0,
    dead_time_misses: 0,
    stochastic_misses: 0,
    false_positive_count: 0,
    physical_reliability_claims: 0,
    qualification_claims: 0,
    authority_grants: 0
  };

  const scenarios = corpus.scenarios.map(scenario => {
    const channels = scenario.channels.map(channel => {
      const result = simulateChannel(scenario, channel);
      summary.channel_count += 1;
      summary.channel_event_opportunity_count += result.latent_event_count;
      summary.detected_event_count += result.detected_event_count;
      summary.shared_common_cause_misses += result.miss_counts.shared_common_cause;
      summary.channel_outage_misses += result.miss_counts.channel_outage;
      summary.dead_time_misses += result.miss_counts.dead_time;
      summary.stochastic_misses += result.miss_counts.stochastic;
      summary.false_positive_count += result.false_positive_count;
      return result;
    });
    summary.scenario_count += 1;
    return {
      scenario_id: scenario.scenario_id,
      configuration_id: scenario.configuration_id,
      channels
    };
  });

  return {
    result: "pass",
    evidence_class: "synthetic",
    claim_ceiling: CLAIM,
    summary,
    scenarios,
    physical_reliability_claim_granted: false,
    qualification_granted: false,
    authority_granted: false
  };
}

try {
  const inputPath = process.argv[2];
  if (!inputPath) fail("usage: node reference_oracle.mjs <corpus.json>");
  const corpus = JSON.parse(fs.readFileSync(inputPath, "utf8"));
  process.stdout.write(JSON.stringify(evaluate(corpus), null, 2) + "\n");
} catch (error) {
  process.stderr.write(String(error?.message ?? error) + "\n");
  process.exitCode = 1;
}
