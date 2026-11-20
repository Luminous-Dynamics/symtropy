# AV Foreground/Background Isolation Benchmark v0

**Status:** synthetic trace contract and evaluator fixtures; no production scheduler integration is claimed.  
**Issue:** [#6245](https://github.com/Luminous-Dynamics/symtropy/issues/6245)  
**Primary external comparator:** [StreamArena / StreamMind](https://arxiv.org/abs/2608.05703)

## Research question

Can latency-critical foreground interaction remain responsive while asynchronous memory writing, indexing, compaction, historical retrieval and search consume a bounded shared resource budget—and can background work still make bounded progress after foreground bursts?

This package owns a benchmark trace contract, not a new scheduler, memory engine, turn controller, process supervisor or evidence ledger.

## Existing owners to compose

- Symthaea #5169: foreground floor/stop/yield semantics.
- Symthaea #10842: conversation-level task and turn context.
- Symthaea #3934: visual provenance and persistent uncertain world state.
- Symthaea #10644: event-centric episodic retrieval.
- Symthaea #9075 / #9177: as-known time and proof-aware sufficient evidence.
- Symthaea #12837: online audiovisual task benchmark.
- Symtropy #6183 / #6201: hidden truth vs delivered observations and causal replay.
- Symtropy #6222 / #6225: decision-sufficient observation.
- Symtropy #5053: source/seed/split integrity.

## The architecture hypothesis

StreamArena's StreamMind separates a low-latency frontend for immediate interaction and proactive monitoring from asynchronous memory writing, recall and search. Its benchmark covers 243 videos averaging 88.8 minutes and 3,646 open-ended tasks. The public StreamArena repository now reports StreamMind v2 runs on all 3,646 questions, including real-time perception (RTP), historical retrospection (HR), tool use and proactive interaction.

Our extension is not to recreate StreamMind's workers. It is to test an additional evidence requirement: a background result must not become knowledge at an earlier query cutoff than the time it was actually available, and compaction must preserve links to original audiovisual evidence.

## Time semantics

Do not collapse:
- event/measurement time;
- sensor arrival/availability time;
- background indexing-ready time;
- request/query cutoff;
- response deadline;
- task revision/cancellation time.

For a query at cutoff T, a memory record is eligible only if its evidence and all required derived/indexed representations were actually admitted by T. Event time before T does not imply availability before T.

## Fixture separation

- `fixtures/agent/workload-v1.json`: public workload arrivals, declared resource budgets, task deadlines, public task revisions and currently admitted evidence.
- `fixtures/evaluator-only/scheduler-oracle-v1.json`: evaluator-only completion times, hidden event truth, source dependencies and expected validity.
- `fixtures/expected/known-answer-v1.json`: authored expected outcomes for the finite examples.

Directory layout is not an OS boundary. Any policy benchmark must run in an agent process that cannot read the evaluator-only path. This v0 is not process isolation evidence.

## Initial known-answer episodes

1. **Foreground deadline under compaction:** a non-preemptible shared FIFO compaction job blocks a foreground response. A reserved/preemptible foreground lane must complete before the response deadline under the same total work/resource budget.
2. **As-known cutoff:** an event was measured earlier but becomes indexed after the query cutoff. It must not be returned as memory available at the earlier cutoff.
3. **Cancellation/supersession:** retrieval for request revision 1 completes after revision 2 supersedes it. The stale completion may remain historical evidence but cannot satisfy revision 2.
4. **Background progress:** foreground bursts temporarily consume capacity, but background indexing must make bounded progress when the burst ends. No silent starvation or unbounded queue growth.
5. **Compaction provenance:** compaction may add a summary but must preserve exact source references, digest and revision links for later verification.

## Comparison arms

- B0 shared FIFO / single-loop baseline;
- B1 shared queue with foreground priority;
- B2 independently scheduled foreground and background workers;
- B3 two lanes with reserved foreground capacity, bounded background admission and backpressure;
- B4 actual Symthaea composition using existing controller and memory owners;
- B5 StreamMind reproduction on the official benchmark when feasible and rights-compliant;
- evaluator-only optimal schedule as a diagnostic ceiling, never a competitive result.

All arms receive identical streams, request arrivals, task revisions, total CPU/memory/storage/network budgets and model revisions. More workers must not silently mean more total compute.

## Metrics

Foreground: response-window compliance, missed deadlines, false interventions, correct silence, p50/p95/p99 source-to-response and foreground degradation under background load.

Background: event-to-index-ready lag, oldest backlog age, indexing throughput, starvation, cancellation latency, compaction time, recovery after foreground bursts.

Evidence: as-known/future leakage, stale-revision use, source-anchor retention, digest resolution, summary-to-source lineage, unsupported historical claim rate.

Cost: per-lane CPU/GPU, memory, storage/I/O, bytes/tokens/frames/audio and calibrated energy only when measured.

## Proposed gates (not achieved)

- 0 as-known/future leakage in deterministic trace fixtures.
- 0 source-anchor loss through compaction.
- 0 superseded request completions treated as current.
- Foreground p95 degradation <=5% and p99 <=10% versus the no-background control at the same admitted resource envelope.
- Background backlog bounded under a declared sustainable input rate; overload is explicit.
- Deterministic replay of identical schedule, resource and task-revision inputs.
- At least one held-out workload where foreground deadlines or long-horizon recall improve without increasing total resource budget or weakening evidence gates.

## Claim ceiling

Passing these synthetic traces qualifies only the declared scheduling and time/evidence semantics. It does not establish a production two-lane runtime, human-level interaction, sensor accuracy, privacy compliance or SOTA superiority.
