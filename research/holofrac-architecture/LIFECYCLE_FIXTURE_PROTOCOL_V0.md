# HoloFrac Component Lifecycle Fixture Protocol v0

**Status:** synthetic append-only lifecycle and mass-balance fixture  
**Implementation:** \`research/holofrac-retrieval/src/lifecycle.rs\`  
**Executable:** \`research/holofrac-retrieval/src/bin/lifecycle_sweep.rs\`  
**Workflow output:** \`component-lifecycle.csv\`  
**Related issue:** [#5985 — adaptive circular building lifecycles and component reuse](https://github.com/Luminous-Dynamics/symtropy/issues/5985)

## 1. Purpose

Test whether a component can be tracked through multiple use cycles without losing identity, material accounting or the distinction between effective time and recorded time.

The fixture is intentionally small: one synthetic 100 kg component is installed as a floor beam, inspected and repaired, removed and recovered, reused as a canopy beam, inspected/repaired again, removed and recovered a second time, then recycled. Recovery losses are explicit and mass is conserved across every event.

This is a ledger/schema proof, not a physical deconstruction model, product passport, service-life model, EPD or real circularity assessment.

## 2. Identity and event contract

The component passport binds:
- stable component ID;
- material and passport identifiers;
- source-geometry identifier;
- connection type;
- initial mass, use class and condition.

The append-only event binds:
- event ID and component ID;
- monotonically increasing revision and exact prior-event ID;
- action and before/after lifecycle state;
- before/after condition and use class;
- effective time and recorded time;
- mass before, mass after, mass loss, recycled mass and disposed mass;
- a synthetic supporting-record ID.

Actions in this slice: install, inspect, repair, remove, recover, reuse, recycle and dispose. A reuse action preserves the same component identity while allowing the functional use class to change. A new component replacement is a separate component identity and should be modeled as a separate ledger in a later portfolio-level extension.

The fixture only models condition-only repair (no added repair material); future profiles must account for added/replaced mass explicitly.

## 3. Transition validation

The ledger rejects:
- component ID changes within a component history;
- duplicate events or non-contiguous source revisions;
- wrong prior-event IDs;
- invalid effective/recorded chronology;
- unsupported state or condition transitions;
- impossible mass changes;
- empty supporting-record identifiers;
- use-class changes not represented as a reuse event.

Terminal states (recycled/disposed) cannot transition back into service. Recovery of a damaged component does not automatically make it reusable; the first slice requires a component to be non-damaged before reuse.

## 4. Mass balance

For every event:

\[
m_{\mathrm{before}} =
m_{\mathrm{after}} +
m_{\mathrm{loss}} +
m_{\mathrm{recycled}} +
m_{\mathrm{disposed}}
\]

At any snapshot cutoff:

\[
m_{\mathrm{initial}} =
m_{\mathrm{active}} +
m_{\mathrm{recycled}} +
m_{\mathrm{disposed}} +
m_{\mathrm{loss}}
\]

The validator applies a declared floating-point tolerance. Recovered/reused quantities are credited only from explicit synthetic events. The fixture cannot infer actual material recovery yield or reuse permission.

## 5. Bitemporal snapshot semantics

A snapshot includes only events whose recorded time is at or before the decision cutoff. The final recycling event is assigned effective time at year 30 and recorded time at year 31.

Therefore:
- at year 30.5, the decision snapshot must still show the component as recovered and not yet recycled;
- at year 31.5, the recycling event is visible and the active component mass is zero.

This prevents late-recorded information from leaking into earlier decision histories. It does not model trusted clock quality or external event-source completeness; real adapters must compose the relevant provenance/time services.

## 6. Initial benchmark output

The executable emits source SHA and a row for each cutoff (0, 6, 10.5, 11.5, 16, 20.5, 30.5 and 31.5 years), including:
- visible lifecycle revision and event count;
- state, condition and functional use class;
- active component mass;
- recycled, disposed and lost mass;
- mass-balance residual;
- stable non-security manifest identifier.

## 7. Next validation increments

1. Portfolio-level ledgers with replacement components and shared assembly/interface dependencies.
2. Multiple material classes and component-specific connection/access profiles.
3. Assembly/disassembly precedence, tool clearance, lifting/handling and damage probability models.
4. Repair material additions, part substitutions and component refurbishment.
5. EPD/material-passport source, version, declared unit, geography and lifecycle module linkage.
6. Multi-cycle second-life matching subject to exact geometry, condition, load/use envelope and owner verification.
7. Independent mass-flow oracle and mutation tests across component/assembly evidence.
8. Matched BCEF / Circl Pavilion or another rights-cleared and fully documented disassemblable-building reproduction.

## 8. Claim ceiling

Passing this fixture establishes append-only identity, allowed synthetic transitions, bitemporal snapshot and mass-accounting correctness for this finite scenario only. It does not establish real building circularity, service life, disassembly feasibility, safety, actual recovery yield, whole-life carbon benefit, code compliance or reuse permission.
