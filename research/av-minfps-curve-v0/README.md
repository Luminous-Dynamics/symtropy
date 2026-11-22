# AV Minimum Sufficient FPS Curve v0

**Issue:** Symtropy #6860  
**Evidence scorer:** #6856 / draft PR #6859  
**Programme:** #6806  
**Composes:** #6183 observation worlds, #6753/#6555 active sensing, #6834 rank CS, #6840 Pareto CS, #6754 independent custody, Symthaea #12837/#13179 candidate execution and trace export.

## Objective

Evaluate how much video sampling density is needed to produce a **correct, evidence-grounded and timely decision**, rather than merely a plausible answer. The evaluator consumes per-rate candidate outputs already scored by #6856. It does not sample video or run models.

## Frozen input contract

For one episode / policy / candidate configuration, the bundle binds:
- protocol, episode, source-cluster and policy identity;
- candidate model and runtime digests;
- sampler ID and implementation/configuration digest;
- fixed, strictly increasing FPS grid;
- media duration and declared timestamp quantization tolerance;
- seed IDs and minimum success fraction across seeds;
- one attempted run for every seed × FPS condition.

Each rate run includes:
- run status: COMPLETED, TIMEOUT, FAILED or MALFORMED;
- exact frame IDs and capture timestamps with a SHA-256 manifest digest;
- reference to the independent #6856 certificate scorecard for completed runs;
- separate answer-correct, evidence-sufficient, decision-on-time and grounded-success flags emitted from the evidence evaluator;
- resource telemetry, preserving null / NOT_MEASURED values.

The evaluator verifies:
- each completed rate has the expected number of frames for the fixed rate and media duration;
- frame timestamps follow the nominal fixed-rate schedule within the preregistered tolerance;
- all completed sampling schedules are nested, and a shared frame ID has the same timestamp at every rate;
- actual delivered frame count matches the frame manifest;
- every expected seed × rate attempt exists. Failed/partial runs remain failures in the denominator and may retain partial frame manifests.

A lower-rate frame set must be a subset of each higher-rate set for the same seed. Do not call adaptive/event-triggered acquisition a nested fixed-FPS sweep; evaluate those as a separate policy under #6753/#6555 and compare on the Pareto frontier.

## Minimum-FPS semantics

A rate passes only when its **grounded-success fraction** across all preregistered seeds meets the fixed `minimum_success_fraction`. A run's grounded-success flag must come from the independent #6856 certificate scorecard and must also imply:
- answer correctness;
- evidence sufficiency;
- deadline compliance.

The minimum tested rate is the lowest grid rate whose own pass condition and every higher-rate pass condition all succeed.

Statuses:
- `LEFT_CENSORED`: the lowest tested rate already passes; the true threshold may be lower.
- `MIN_FPS_FOUND`: the lowest tested stable rate is inside the tested grid.
- `RIGHT_CENSORED`: even the highest tested rate fails.
- `NO_STABLE_RATE`: successes are non-monotone and there is no passing higher-rate suffix.

No interpolation to a continuous threshold is performed. An isolated low-FPS success followed by a failure is not reported as a stable low minimum.

## Resource reporting

For every rate, report separate means and the number of observed values for delivered frames, video bytes, audio samples/bytes, tokens, model calls, wall time, RAM and VRAM. Null values remain missing and are never converted to zero.

Energy has a stricter contract:
- MEASURED requires a non-negative value, method and evidence digest;
- ESTIMATED and SYNTHETIC remain labelled and do not enter the measured-energy mean;
- NOT_MEASURED requires a null value.

Measured-energy means report their own sample count; they must not be interpreted as complete when some runs were not measured. This is not a hardware energy benchmark until an external meter and evidence receipt support it.

## Run

    python3 -m unittest discover -s tools/av-minfps-curve -p 'test_*.py' -v
    python3 tools/av-minfps-curve/evaluate_minfps_curve.py --input path/to/fps-sweep.json

## External comparison

FPS-Bench (CVPR 2026) is an important reference: >1,000 manually checked high-frame-rate questions, item-level minimum required frame rates, reported average minFPS around 7, and reported VLM QA accuracy 30% vs human 72%. Those are results on its official task/sampler, not universal requirements for every audiovisual task.

Official protocol: https://openaccess.thecvf.com/content/CVPR2026/html/Choudhury_FPS-Bench_A_Benchmark_for_High_Frame-Rate_Video_Understanding_CVPR_2026_paper.html

Do not compare our synthetic fixtures to its leaderboard. First reproduce its item definitions, sampler, split, frame budget and scoring; then run our matched A0–A5 policies. The 0.5/1/2/4/7/15 FPS sweep is a proposed internal grid, not a claim of official FPS-Bench reproduction.

## Explicit v0 limits

- One episode/policy bundle per invocation; cross-episode aggregation is a separate layer.
- Fixed-rate nested schedules only. Active evidence acquisition is scored separately.
- Structured grounded-success flags are consumed from #6856; this evaluator does not recompute claim semantics.
- One declared success fraction across repeated seeds; no confidence intervals for the stochastic success fraction yet.
- No continuous FPS interpolation, no automatic camera control and no actual model-job orchestration.
- No energy saving claim without measured meter receipts.
- No blind result until #6754 supplies independent custody and rights-qualified media.

## Claim boundary

Passing these tests qualifies frame-schedule validation, min-FPS/censoring semantics, failure denominators and resource-provenance accounting. It does not prove a model's performance, universal minimum frame rate, SOTA superiority or physical resource savings.
