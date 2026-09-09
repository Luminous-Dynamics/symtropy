# Symtropy Three Worlds Lab

LAB-14 is a small, deterministic, headless integration scenario across **Aster**, **Vesper**, and **Helion**.

Its purpose is not to add another civilization subsystem. It exercises the existing civilization stack end to end and checks that authority boundaries remain intact when the systems compose.

## History under test

The scenario intentionally crosses many domains:

- Aster retains an institution-recorded First Speaker while two succession claims coexist.
- Vesper recognizes one claim while Helion disputes it.
- Aster owns a relief freighter while Vesper operates it.
- One provider-owned Aster -> Hub -> Vesper topology is reused by physical transit and signal propagation with different attested timing.
- A treaty proposal reaches Vesper before the relief cargo can physically arrive.
- Cargo cannot appear before its provider-attested arrival floor.
- Aster and Vesper can cite different evidence and disagree about whether the same treaty clause was satisfied.
- A Helion population cohort can materialize one persistent person without changing represented population or inventing pre-materialization identity history.
- Canonical simulation authority for the freighter can move from an Aster shard to a Vesper shard exactly once without changing the asset ledger's Aster ownership relation.
- A Mycelix-like external organization record may be explicitly admitted as evidence for a project-publication request, but admission does not publish the project.
- Helion can independently contribute arrival evidence to the subsequently published project, and Vesper must explicitly review it before completion.
- An Aster/Helion political conflict can enter an accepted ceasefire and negotiated settlement without a global war score or automatic asset/territory mutation.

## Run

From the repository workspace:

```bash
cargo run -p symtropy-three-worlds-lab
```

The binary prints a JSON report containing the integration invariants.

## Qualification

The source and fixtures are an executable specification, but a commit is not considered qualified merely because the code exists. Exact-head formatting, tests, Clippy/check, and the lab binary still need to execute successfully before a PASS claim is made.

The lab deliberately does not replace the owning domains' unit/adversarial tests. It is an integration proof that the same records can participate in one coherent causal history without collapsing distinct authorities into a single omniscient world state.
