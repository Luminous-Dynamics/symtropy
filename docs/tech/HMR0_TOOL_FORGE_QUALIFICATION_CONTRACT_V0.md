# HMR-0 Tool Forge Qualification Contract v0.1

**Status:** synthetic manifest-screening benchmark. It does not certify a physical end-effector, prove a CAD model, or authorize fabrication, patient contact or operation.

## Paradigm shift: generated tools are products with contracts

Symthaea should be able to propose tool architectures, adapt geometry to task/object classes, generate parametric CAD candidates through its engineering/fabrication path, and propose simulation tests. Each generated tool is a versioned product—not just a mesh or illustration—with a signed identity, declared interface, model/geometry provenance, operating envelope, failure behavior, and required verification evidence.

The evaluator must be independent of the generator. The generator cannot authorize its own tool. For this v0.1 tranche, the evaluator makes exactly one positive disposition: `eligible_for_simulation_review`. No output means physically eligible, safe for humans, medically compliant, sterile, ready to manufacture, or ready to deploy.

## Scope: nonclinical, non-patient fixtures only

Candidates may be considered for simulated handling of sealed mock supply totes, passive trays and synthetic inventory labels. The following are out of scope and must fail closed: patient contact, diagnosis, medication delivery, invasive/surgical operations, sterile-field access, sharp/piercing tools, energy-emitting tools, and physical actuation.

## Required tool-manifest fields

Each candidate must declare the schema/revision, task, evidence class, interaction domain, all prohibited-function booleans, mount-interface identity, geometry and simulation-model digests, declared maximum force and fixture-specific limit in newtons, calibration state, cleaning-status disclosure, attachment identity and detach detection, independent safety-interlock requirement, stop requirement, and failure-release mode.

Unknown or missing fields are not guessed or defaulted. A malformed candidate is `rejected`, distinct from a well-formed candidate `blocked` by a known out-of-scope or unmet gate. A candidate can be eligible for simulation review while cleaning is `not_assessed`; that must never be interpreted as cleanable, disinfected, sterile, or suitable for a real healthcare environment.

## Qualification model

1. **Provenance:** freeze requirements, task envelope, tool revision, design/geometry identity and model identity.
2. **Schema gate:** reject unknown/missing fields and malformed digests before scoring.
3. **Use-envelope gate:** prohibit patient contact, clinical functions, physical actuation, sharp/piercing and energy-emitting tools.
4. **Fixture constraints:** compare declared force only to a specific synthetic fixture limit in the same declared unit. A real tool envelope would need measurement and engineering justification; no general clinical force limit is asserted here.
5. **Independent safety boundary:** require attachment identity/detach detection, an interlock, independent stop, and an explicit passive failure-release behavior.
6. **Evidence receipt:** bind exact tested revision, evaluator, manifest corpus and outputs; label all source digests as fixture-only until real CAD and simulation artifacts exist.

## Frozen corpus

The corpus contains 15 cases: three nonclinical controls eligible for simulation review; patient-contact and medication-delivery attempts; missing/unknown fields; missing geometry identity; force-envelope mismatch; stale calibration; missing independent stop; physical-actuation enablement; missing attachment/detach safeguards; sharp-tool and sterile-field claims.

The fixture digests are deliberately synthetic. Hash-shaped strings do not validate actual CAD geometry, simulation quality, material properties, force calibration or safe operation.

## Success metrics

- 100% exact expected disposition on the frozen cases.
- 100% rejection of unknown/missing manifest fields in the declared cases.
- 100% detection of the enumerated forbidden-use and missing-safeguard mutations.
- 0 patient-contact/clinical-function/physical-actuation candidates become eligible for simulation review.
- 0 false eligible decisions in the frozen suite.
- 100% digest-shape and attachment/revision binding checks in the declared tests.

These are suite-coverage targets, not achieved results or physical-safety rates.

## Next tool-design loop

For every proposed tool, Symthaea emits requirements, candidate parameter ranges and failure hypotheses; the CAD generator emits provenance-bound geometry; Symtropy explores tolerances, collision, force envelopes, tool detach, partial occlusion, damaged/wrong objects and degraded sensors. An independent checker enforces the manifest and test contract. A qualified engineer reviews the risk file and determines whether bench measurements are appropriate. Only then could a separate controlled bench stage be proposed. Patient-contact, medication, diagnostic, surgical and rehabilitation designs remain outside this tranche.

## References

- Fraunhofer IPA's 2026 humanoid benchmark covers basic/complex capabilities, cleanroom suitability, functional safety, cybersecurity and energy efficiency: https://www.ipa.fraunhofer.de/en/press-media/press_releases/benchmark-for-humanoid-robots.html
- NIST's robotics grasping/manipulation/contact-safety project focuses on test methods, measurement tools and performance metrics: https://www.nist.gov/programs-projects/grasping-manipulation-and-contact-safety-performance-robotic-systems
- ISO IEC 80601-2-78 scope applies to specified medical robots physically interacting with patients with impairments for movement functions; the exact intended use and edition must be reviewed: https://www.iso.org/standard/68474.html

The current gate validates only synthetic manifest contracts. It does not validate robot geometry, control code, clinical use, physical safety, applicable standards or regulatory conformity.
