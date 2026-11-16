# GeoBio Foundry — Symtropy Architecture Fit v0.2

**Date:** 2026-11-16  
**Status:** Repository-grounded design note  
**Target repository:** Luminous-Dynamics/symtropy (public)  
**Implementation boundary:** New research-only directory; no production crate changes in this increment

## 1. Architectural decision

GeoBio Foundry should be a domain layer over Symtropy's existing terrain, basin, physics, and robotics foundations. It should not replace those systems with a second geology simulator or physics engine.

The first code increment is an isolated reference kernel under `research/geobio-foundry/kernel/`. It has its own Cargo workspace so it does not silently join or change the production workspace. After its semantics and tests are independently reviewed, selected interfaces can be proposed for production integration.

## 2. Existing capabilities verified in this repository

| Existing component | Verified current role | GeoBio reuse opportunity | Gap GeoBio must address |
|---|---|---|---|
| `crates/domains/symtropy-terrain` | Volumetric chunks, voxel substrate types (Air, Bedrock, Dolomite, PyriteTailing, Quartzite), density fields, excavation and weathering events, rebuild tasks, fluid coupling | Site geometry, excavation state, substrate domains, changing terrain | No verified end-to-end geological observation provenance, uncertainty-preserving site qualification, or engineering claim-state contract was identified in the inspected entry point |
| `crates/domains/symtropy-basin` | Deterministic basin reference state for water, soil, atmosphere, heat, toxins, radiation, erosion, roots, infrastructure integrity, and trophic/ecological memory | Environmental and bioindustrial state coupling; water and contamination context | No whole-process elemental inventory, feedstock provenance, batch quality, or biological/abiotic lifecycle accounting was identified in the inspected entry point |
| `crates/core/symtropy-physics` | N-dimensional rigid-body physics, collision, continuous collision detection, joints, raycasting, replay | Contact mechanics and physical scenario primitives | Destination-specific geotechnical material models, rock-mass failure calibration, pressure containment qualification |
| `crates/bridges/symtropy-robotics-bridge-core` | Robotics bridge core types and traits over Symtropy physics and consciousness-physics | Common robot/task interfaces | Geological inspection and full task-chain benchmark contracts |
| `docs/tech/ARCHITECTURE.md` | Physics callback and deterministic replay architecture | Simulation coupling and scenario reproducibility | Cross-domain evidence graph and independent measurement qualification |

This is a targeted inspection of the published entry points, manifests and architecture note—not an exhaustive audit of every source file or every runtime path. The gaps are therefore research integration gaps observed at those interfaces, not claims that no related work exists elsewhere in the codebase.

## 3. Proposed responsibility boundaries

### Symtropy terrain
Owns geometric and material-state evolution: occupied/void volumes, excavation and weathering events, and terrain representation.

### GeoBio site/evidence layer
Owns observations and their epistemic metadata: sensor identity, calibration, timestamps, spatial coverage, resolution, provenance, uncertainty, derived-model identity, and whether the state is observed, inferred, modelled, or independently verified.

A terrain voxel must not acquire a confident material property merely because its display material has a categorical enum. Categorical substrate labels and measured geotechnical properties are different fields.

### Symtropy physics
Owns physical simulation primitives. Geomechanical constitutive models, structural failure envelopes, pressure boundaries, and thermal coupling should be introduced only with independent reference solutions and physical calibration datasets.

### Symtropy basin
Owns environmental and ecological state dynamics. GeoBio connects this to a process/resource ledger; it must not assume that a water or nutrient state is physically available as a feedstock without a provenance-qualified inventory.

### Robotics bridge
Owns reusable robot/task exchange. GeoBio task benchmarks add site-specific constraints, contact budgets, retreat conditions, and mission-chain success criteria.

### Symthaea
May propose hypotheses, select measurements, diagnose faults, and plan within a bounded action envelope. It does not supply its own ground truth or authorize high-consequence physical mutation.

### Mycelix
May coordinate resource claims, batches, capacity, contracts and attestations. A reported claim must remain distinct from independently measured physical inventory.

### Sol-Atlas
Maintains geospatial site records and the original observation lineage. Generated interpretations must not overwrite source observations.

## 4. New shared abstraction: GeoBio Resource and Constraint Graph

The initial graph should represent:

- Site, region, surface, subsurface void, layer, fracture and material occurrence.
- Observation, instrument, calibration, coverage, uncertainty and provenance.
- Deposit states: observed occurrence, inferred deposit, characterized resource, accessible reserve, extracted feedstock and qualified product.
- Pressure compartments, engineered boundaries, seals, penetrations, isolation and leak observations.
- Energy generation, storage, consumption, recovery, thermal rejection and exergy quality.
- Material, elemental, water, nutrient, carbon, gas and waste flows.
- Biological process identity, contained reactor, feedstock, batch, product specification, contamination and disposition.
- Robot, tool, contact event, anchor state, task sequence, retreat and recovery.
- Evidence object, independent witness, acceptance rule, validity interval and qualification state.

Every quantity must have an explicit unit. Every claim must have a basis, timestamp, source, uncertainty, applicability domain, and lineage. Unknown and not measured are first-class states.

## 5. The first compiler pipeline

`observations -> evidence-preserving site model -> resource/constraint graph -> candidate design -> model evaluation -> value-of-information plan -> independent test -> qualified capability record`

The compiler may return:
- `insufficient_evidence`
- `out_of_domain`
- `requires_measurement`
- `infeasible`
- `candidate_only`
- `physically_tested`
- `independently_reproduced`
- `qualified_for_declared_envelope`

It must not imply universal safety from one test envelope or promote a model result into a measured result.

## 6. Integration sequence

1. **Research kernel:** resource/elemental accounting and evidence completeness, outside production workspace.
2. **Data adapters:** read-only adapters from terrain and basin records into explicit GeoBio site/process records.
3. **Scenario bridge:** deterministic geometry, resource and process scenarios with frozen inputs and independent reference values.
4. **Model adapters:** geomechanics, pressure, thermal and biological models only when their assumptions and validation data are explicit.
5. **Symthaea planner:** active measurement selection and bounded experiment/maintenance recommendations.
6. **Mycelix attestation:** resource and batch claim exchange without self-authorization.
7. **Sol-Atlas integration:** versioned site observations and preservation constraints.

## 7. Explicit non-goals

- Do not replace Symtropy terrain, physics, robotics or basin models.
- Do not add a production dependency on unvalidated research code.
- Do not treat a simulation score as pressure-vessel or geotechnical certification.
- Do not use a categorical voxel material label as a substitute for measured density, strength, porosity, composition or fracture data.
- Do not equate basin environmental state with verified extracted inventory.
- Do not release engineered organisms into extraterrestrial environments.
- Do not couple biological operations to physical actuation until independent safety limits and authorization are in place.
