# Living World Ethical Impact Fixtures v0.1

This directory freezes the first synthetic known-answer corpus for [LIVING-WORLD-ETHICAL-IMPACT-001, Symtropy #5902](https://github.com/Luminous-Dynamics/symtropy/issues/5902).

## Files

- `schema.json` — closed JSON Schema (Draft 2020-12) for the corpus.
- `fixtures.json` — twelve synthetic fixtures, six conditional premise profiles and twelve anti-derivation controls.

## What the corpus tests

The cases separate physical feasibility, ecological integrity, individual welfare, moral-patient uncertainty, human/community effects, consent/custodianship and intergenerational consequences. Each expected finding is conditional on a named premise, includes evidence references, and explicitly identifies its action-option scope (an empty option list means factset-level). Missing, not-tested, not-recorded, out-of-domain and unqualified evidence remain distinct.

The twelve scenario families are: tumour/parasite countermodel; stable exploitation; higher throughput with lower welfare; predator–prey conflict; invasive-species management; forest restoration; fungal-network evidence; water commons; uncertain possible moral patient; reversible versus irreversible intervention; ecological value without sentience evidence; and identical facts assessed under different premises.

## Integration boundary

This is a **fixture corpus**, not an evaluator. It must be consumed by a read-only adapter that composes the existing [CivDyn premise evaluator](https://github.com/Luminous-Dynamics/symtropy/tree/research/civdyn-normative-audit-2026-11/research/civdyn-normative-audit) only for premise families it actually supports. The existing evaluator's water/energy/food resource rules must not be stretched to represent species welfare, ecological integrity or moral-patient status. Those dimensions remain separate evidence inputs until a justified, reviewed adapter contract exists.

The fixture labels are expected software-test outcomes, not externally validated moral judgments. Profile definitions are illustrative premise bundles, not complete ethical theories or an endorsement of any single profile. A majority, aggregate output or ecosystem function cannot silently override an individual, community, rights or future-generation finding.

## Thermodynamic boundary

Every v0.1 fixture marks thermodynamic observables as `not_qualified`. Resource throughput, kWh inventories, information-theoretic metrics, survival and model dissipation must not be relabelled as entropy production, exergy destruction, welfare or moral value. Qualified physical receipts must come from their owning physical/thermodynamic contracts.

## Validation and claim ceiling

The authoring pass checked JSON syntax, unique fixture IDs, profile/premise references, action-option references, anti-derivation-control references, evidence-reference integrity, and the v0.1 enum/required-field constraints. This is not yet a formal external JSON-Schema validator run or an exact-head CI result. The fixture corpus does not establish empirical ecology, field calibration, sentience, universal moral truth, intervention efficacy or action authorization.

## Executable conformance gate

The standalone dependency-light Rust reader/validator now lives in
`research/living-world-ethical-impact-validator/`. Its typed structs reject
unknown fields and its tests check fixture cardinality, IDs, premise/evidence/
action/control references, explicit `not_recorded` semantics, action authority,
and the thermodynamic claim ceiling.

The dedicated GitHub Actions workflow
`.github/workflows/living-world-ethical-impact.yml` runs formatting, strict
Clippy, the conformance tests and a deterministic report on Ubuntu, Windows and
macOS. The first workflow result must be recorded against the exact PR head
before claiming Rust qualification.

The next research gates are:
1. Add adversarial mutation coverage for each anti-derivation control.
2. Add digest-bound, held-out empirical evidence adapters only after data rights,
   source identity, unit semantics and access constraints are established.
3. Compose existing CIVDYN premise findings only for resource premises in-domain.
4. Keep action authorization external and human-governed; never emit a global
   morality, welfare or life score.
