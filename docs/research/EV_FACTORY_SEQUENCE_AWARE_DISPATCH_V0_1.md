# EV factory sequence-aware dispatch v0.1

**Related work:** [EV-FACTORY-SEQ-001 #2412](https://github.com/Luminous-Dynamics/symtropy/issues/2412), [EV-FACTORY-DESIGN-002 #2417](https://github.com/Luminous-Dynamics/symtropy/issues/2417)  
**Status:** controlled synthetic scheduling experiment; no real line is represented

## Paradigm shift: treat variant sequence as a production resource

Capacity-only models imply that another server should be unambiguously beneficial. The mixed-variant simulation showed why that assumption is unsafe: parallel servers, finite buffers and sequence-dependent setup costs interact. A useful factory optimizer should reason about **good throughput + changeovers + WIP + waiting fairness + output order**, not maximize utilization or minimize setup count in isolation.

## Policies now exposed

The simulator configuration carries an explicit `dispatch_policy`:

- `fifo` (default): take the oldest item from the input queue.
- `minimize_changeover`: if a server has a previous product variant, prefer the oldest queued job of that variant **only while the oldest job has waited less than `max_variant_skip_seconds`**. Once the oldest job crosses that threshold, the policy falls back to FIFO.

This makes the behavior reproducible and transparent, not a hidden scheduling heuristic. Ties within a variant preserve queue order. The same policy applies to the raw/source queue and interstation queues; that means its effects can propagate along the whole line. The report records the chosen policy and age threshold.

The second policy uses queue age as a hard bound on variant skipping. This prevents a job from being bypassed indefinitely merely because another variant keeps arriving, but it is not a full due-date-aware or labor-aware scheduling algorithm. The age threshold is an input knob and must be stress-tested over several values and product mixes.

## Run and compare

The standard fixture declares FIFO and 20 fixed seeds:

```sh
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_simulation -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json \
  > fifo-report.json

jq '.dispatch_policy = "minimize_changeover"
    | .max_variant_skip_seconds = 240' \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json \
  > minimize-changeover-fixture.json

cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_simulation -- minimize-changeover-fixture.json \
  > minimize-changeover-report.json
```

Compare the same-seed paired throughput distributions with changeover counts, output transfers/inversions, WIP and blocked/starved/down time. A reduction in setups is not a win if it increases total lead time, queue aging, quality loss, or downstream starvation. Report both the mean paired delta and its confidence interval, plus per-variant waiting-time tails before recommending a dispatch rule.

## Engineering metrics

| Measure | Why it matters |
|---|---|
| Good units/hour, paired by seed | Primary flow outcome; not enough alone |
| Changeover starts per station | Tests the mechanism that motivates variant-aware dispatch |
| Out-of-order station transfers | Simple release-sequence proxy; not a customer-order or tardiness metric |
| Mean/P95/max queue age by variant | Guards against a policy optimizing setups by silently delaying uncommon variants |
| WIP and finite-buffer occupancy | A throughput gain achieved by hiding excess work in queues is not a free gain |
| Rework/scrap and final good yield | Ensures setup reduction doesn't trade away quality |
| Blocked/starved/down time | Reveals constraint migration and how added parallel capacity is used |
| Paired confidence intervals over declared seeds | Separates random variation under this simulator from an apparent single-run effect |

The current implementation reports changeover and sequence counters and applies an aging rule; **per-variant wait-time quantiles and controlled policy-cost/lead-time modeling remain follow-on work**. Do not imply they're already measured by v0.1.

## Required controls

- **No-setup homogeneous-product control:** changeover time is zero and variant count is one. Setup-aware and FIFO dispatch should be identical; an output difference is a bug or unmodeled side effect.
- **Mixed-variant setup stress:** vary batch size, changeover duration, queue capacity and aging threshold.
- **Starvation control:** generate sustained arrivals of one variant while the queue contains an old alternate variant; verify the oldest job is selected after the bound.
- **Tiny-buffer/blocking control:** ensure dispatch policy doesn't exceed finite buffers or double-count blocked output.
- **Intervention comparison:** compare FIFO, variant-aware dispatch and the extra EOL-server scenario under the same seed list. Do not conflate scheduling-policy and capital-capacity effects.

## Claim ceiling

The fixture, rates, buffer sizes, sequence numbers and outcomes are synthetic. The rule is a first scheduling heuristic, not an industrially validated dispatch optimizer. Its job-ID inversion counter is a coarse release-order proxy and does not prove causal throughput loss. More capacity may still be best for the end-of-line constraint; this experiment is meant to reveal system trade-offs, not preselect the result.

`release_ready` remains hard-false. No MES/PLC or manufacturing data was collected. No suppliers, manufacturers, plant operators, integrators, landlords or agencies were contacted; no quotes requested.
