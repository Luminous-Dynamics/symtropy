# AV Observation Replay Contract v0

**Status:** research adapter / source-authored, execution pending.  
**Parent:** [Symtropy #6183](https://github.com/Luminous-Dynamics/symtropy/issues/6183)  
**Scenario owner:** [#1595](https://github.com/Luminous-Dynamics/symtropy/issues/1595)  
**Acoustic propagation owner:** [#1594](https://github.com/Luminous-Dynamics/symtropy/issues/1594)  
**Typed world/sensor contract:** [#1587](https://github.com/Luminous-Dynamics/symtropy/issues/1587)  
**Split-integrity guard:** [#5053](https://github.com/Luminous-Dynamics/symtropy/issues/5053)  
**Truth/observation separation precedent:** [#5601](https://github.com/Luminous-Dynamics/symtropy/issues/5601)

## What this increment does

This isolated Rust package defines a strict, versioned agent-observation bundle for audiovisual benchmark inputs. It provides:

- typed video-frame and PCM-chunk metadata with exact source intervals;
- immutable relative artifact references with SHA-256 and byte-length commitments;
- format/shape validation for raw Gray8/RGB8/RGBA8 frames and S16LE/F32LE PCM;
- clock-domain identity including reset generation, without guessing cross-clock transforms;
- a separate availability tick and arrival order for every transport event;
- explicit Delivered, Reordered, Duplicate, StaleReplay and Dropped transport dispositions;
- a causal replay cursor that cannot release an event before its declared availability tick;
- strict deserialization that rejects unknown fields, including accidental world/truth labels;
- deterministic fixtures and tests for integrity negatives.

The bundle contains no canonical entity IDs, world positions, hidden event classes, source coordinates, reference transcripts, simulator state or evaluator labels. The replay cursor emits only the observations and transport metadata present in the agent bundle.

## What it deliberately does not do

This is **not**:

- an RGB camera or video renderer;
- a PCM waveform renderer, microphone-array propagation model or acoustic solver;
- a clock synchronization or clock-transform service;
- a persistent entity/event graph, episodic memory or fusion engine;
- an ASR/VLM/vision model or response controller;
- an OS sandbox or proof of process-level hidden-truth isolation;
- physical sensor qualification.

The existing Bevy scene/capture path can produce candidate visual artifacts; #1594 owns the declared acoustic propagation and emission model. This crate defines the artifact and causal delivery boundary between those owners and the AV evaluator. It does not qualify either producer.

## Time semantics

A capture interval is a bounded interval on its named clock domain. A clock identity includes domain ID, generation and time-scale ID. A clock reset creates a different generation. Two numerically equal timestamps from different clock identities are **incomparable** here; no transform is inferred.

Source measurement/capture time is distinct from transport availability time. Replay availability ticks determine when the agent may first receive an observation. Arrival order provides a deterministic tie-break and preserves reordering. Packet arrival time is never substituted for capture time.

Intervals use half-open source sample ranges for audio. A loss is represented by a Dropped transport event without an observation artifact; a subsequent chunk must retain its actual source sample range rather than closing the gap.

## Evidence-plane and execution boundary

The intended future layout remains the one defined by #6183:

- agent observations: rendered media artifacts plus permitted capture/transport metadata;
- evaluator truth: canonical world/event/source labels, kept outside the agent process;
- receipts: exact inputs, artifact digests, runner revision, replay output and split disposition.

This library enforces a **schema boundary**, not a security boundary. A real hidden-truth evaluation still needs separate filesystem roots and a process/container boundary that does not mount evaluator truth into the agent runtime. Never claim that strict unknown-field rejection alone provides isolation.

## Run locally

Run:
cargo test --manifest-path research/av-observation-replay-v0/Cargo.toml

The package declares its own Cargo workspace so it does not silently join the larger Symtropy workspace. The current repository execution environment did not have Rust/Cargo available when this source was authored; test execution and exact-head CI are therefore pending.

## Fixture

The fixture JSON references two tiny raw artifacts under fixtures/media. They exist only to exercise format, length and digest validation. They are synthetic byte fixtures, not camera footage, a codec conformance corpus, calibrated audio or a physical measurement.

## Promotion gates

Before this can be treated as a usable AV fixture adapter:

1. exact-head Rust format, compile, unit tests and strict Clippy pass;
2. artifact loader resolves paths only beneath an explicitly supplied fixture root and verifies bytes against each digest;
3. Symtropy's existing scenario runner supplies the availability schedule without exposing evaluator truth;
4. #1594 acoustic outputs and Bevy visual outputs are bound to their actual producer/profile identities;
5. hidden-truth separation is demonstrated at the process boundary;
6. the resulting agent observations are consumed causally by a Symthaea adapter with no future-data leakage.

A pass here would establish a typed observation/replay contract only. It would not establish rendered audiovisual simulation fidelity, physical sensor accuracy, AV reasoning accuracy or SOTA performance.
