# Reproductive Isolation Evidence v1

## Scope

SEL-09B converts multiple current SEL-09A reproductive-contact/gene-flow studies into a preregistered, complete-barrier reproductive-isolation evidence object.

The governing boundary is:

`reproductive contact evidence != reproductive isolation != species identity != speciation event`

V1 establishes only design-bounded reproductive-isolation evidence for one ordered pair of lineage authorities.

## Preregistered design

`ReproductiveIsolationDesign` is outcome-free. It is declared from current `ValidatedReproductiveContactStudyDesign` capabilities before any SEL-09A outcome-bearing study is consumed.

The design freezes:

- a semantic isolation-design ID;
- one ordered lineage-authority pair;
- the exact SEL-09A study-design cohort;
- one semantic unit ID per study;
- exact start/end generation and generation count per study;
- exact opportunity count per study;
- exact SEL-09A context policy per study;
- per-study independence evidence;
- a cohort-level independence-rule authority;
- a preregistered minimum number of barrier-supporting studies;
- a preregistered minimum number of observed-contact opportunities;
- a preregistered minimum study-generation span;
- context-compatibility policy;
- the exact built-in V1 complete-barrier rule authority.

Study units are canonicalized by unit ID. Duplicate unit IDs fail. Reusing one SEL-09A design digest under two unit IDs fails; one study design cannot masquerade as independent replication.

V1 requires at least two declared studies and at least two barrier-supporting studies for `Supported` to be reachable.

## Independence trust boundary

Per-study `independence_evidence` and `independence_rule_authority` are explicit external trust bindings.

They are part of the canonical design identity. Changing them changes the reproductive-isolation design digest.

Evolution-core does **not** infer or independently prove biological/statistical independence from those opaque authority references. A later stronger lane may materialize executable independence criteria, but V1 does not pretend a hash proves them.

## Context compatibility

`IsolationContextCompatibility::RequireSharedContextPolicy` requires every declared SEL-09A study to carry an exactly equal contact-study context policy.

`AllowDeclaredVariation { authority }` permits declared cross-context variation only under an explicit authority binding.

Context variation is therefore never silently ignored.

## Built-in complete-barrier rule

The V1 decision rule is code-owned and content-digested. Callers do not provide or select a post-outcome rule authority.

`complete_reproductive_isolation_barrier_rule_v1()` binds the exact V1 semantics:

- `NoContact` never supports reproductive isolation;
- `ContactNoPairing` supports a pairing barrier;
- `PairedNoMating` supports a mating barrier;
- `MatingNoConception` supports a conception barrier;
- `ConceptionNoViableOffspring` supports a hybrid-viability barrier;
- `ViableInfertileOffspring` supports a hybrid-fertility barrier;
- `ViableFertileOffspring` contradicts complete reproductive isolation;
- realized hereditary gene flow contradicts complete reproductive isolation;
- viable offspring with unknown fertility is insufficient evidence;
- unavailable reproductive-stage evidence is insufficient evidence;
- unavailable gene-flow evidence is insufficient evidence.

The rule identity also binds the sustained-evidence theorem below.

## Sustained barrier theorem

A study does not become barrier-supporting merely because its declared interval spans several generations.

V1 requires qualifying barrier observations in **at least two distinct preregistered generations within that study**.

Multiple barrier observations concentrated in one generation do not satisfy sustained support even if the study's surrounding start/end interval is long.

Therefore:

`multi-generation study interval != multi-generation observed barrier`

and:

`many same-generation failures != sustained reproductive isolation`

The V1 minimum distinct barrier-generation count is two.

## Complete outcome evidence

`ReproductiveIsolationEvidence::capture(...)` consumes the current `ValidatedReproductiveIsolationDesign` plus one current `ValidatedReproductiveContactStudy` for every preregistered study unit.

Omitted studies, duplicate units, unknown units, SEL-09A design substitution, and duplicate SEL-09A study digests fail closed.

Every persisted `IsolationStudyRecord` stores:

- the semantic isolation-study unit ID;
- the full canonical SEL-09A study snapshot;
- the exact SEL-09A study digest.

The full snapshots are retained so local validation can recompute barrier counts/status instead of trusting a serialized summary.

## Barrier profile

`ReproductiveBarrierProfile` separately retains:

- study count;
- total preregistered opportunities;
- studies with observed contact;
- sustained barrier-supporting studies;
- observed-contact opportunities;
- no-contact observations;
- pairing barriers;
- mating barriers;
- conception barriers;
- hybrid-viability barriers;
- hybrid-fertility barriers;
- viable fertile hybrids;
- viable hybrids with fertility unknown;
- realized gene-flow observations;
- unavailable reproductive observations;
- unavailable gene-flow observations.

These counts are checked with overflow-safe arithmetic and recomputed from the embedded complete SEL-09A study snapshots.

A serialized profile cannot authorize itself. `canonical_digest()` fails if the stored profile does not recompute exactly.

## Status semantics

V1 status is one of:

- `Supported`;
- `NotSupported`;
- `Contradicted`;
- `InsufficientEvidence`.

The decision precedence is fail closed.

### Contradicted

Any observed viable fertile hybrid or realized hereditary gene flow produces `Contradicted`, regardless of how many barrier observations exist elsewhere in the cohort.

Positive barrier counts cannot vote away direct contradiction.

### InsufficientEvidence

`InsufficientEvidence` is emitted when there is no direct contradiction but at least one of these conditions holds:

- reproductive-stage evidence is unavailable;
- realized gene-flow evidence is unavailable;
- viable hybrid fertility is unknown;
- the cohort does not meet the preregistered observed-contact opportunity threshold.

Repeated no-contact observations therefore remain insufficient rather than becoming isolation evidence through geography or missing opportunity.

### Supported

`Supported` requires all of the following:

- no direct contradiction;
- no missing/unknown evidence condition above;
- the preregistered observed-contact denominator is satisfied;
- at least the preregistered number of independent studies are barrier-supporting;
- each counted barrier-supporting study contains qualifying barrier evidence in at least two distinct preregistered generations;
- each counted barrier-supporting study has no disqualifying observation.

### NotSupported

When evidence is otherwise complete and contact denominator is adequate but the independent sustained-barrier study threshold is not reached, the result is `NotSupported`.

This is different from missing evidence.

## No-contact theorem

`NoContact` is not a reproductive barrier in V1 because it may arise from geography, sampling, timing, behavior outside the defined opportunity, or other mechanisms that do not establish reproductive incompatibility.

Therefore:

`no contact != failed reproduction under contact`

and:

`repeated no contact != reproductive isolation`

A study contributes to observed-contact support only when the reproductive process was actually observed at or beyond contact.

## Fertility and gene-flow contradictions

The 09A distinctions remain intact:

`viable hybrid != fertile hybrid != realized hereditary gene flow`

A viable infertile hybrid may support a postzygotic fertility barrier.

A viable fertile hybrid contradicts the V1 complete-barrier claim even if no later ancestry observation is yet available.

Realized hereditary gene flow is stronger counterevidence and independently contradicts the complete-barrier claim.

## Local validation versus current authority

Persisted `ReproductiveIsolationDesign` and `ReproductiveIsolationEvidence` objects are serializable representations.

Their canonical digests establish representation identity only.

Local validation rechecks:

- supported versions;
- canonical study ordering;
- distinct SEL-09A study-design identities;
- generation-span arithmetic;
- context-compatibility invariants;
- exact built-in barrier-rule identity;
- exact persisted design binding;
- exact SEL-09A study snapshot/digest agreement;
- duplicate current-study prevention;
- recomputed barrier profile;
- recomputed final status.

Current authority remains non-Serde:

- `ValidatedReproductiveIsolationDesign<'_>` requires fresh current SEL-09A study-design capabilities and complete preregistration replay;
- `ValidatedReproductiveIsolationEvidence<'_>` requires the current isolation design plus fresh current SEL-09A study capabilities and complete evidence replay.

A restored serialized object cannot regain current authority from its own stored bytes.

## Evidence identity

The isolation-design digest binds the full preregistered cohort and trust boundary, including per-study independence evidence and the content-digested built-in rule.

The isolation-evidence digest additionally binds the complete current SEL-09A study history, recomputed barrier profile, and final typed status.

Two numerically identical profiles under different SEL-09A studies, different independence evidence, different context policy, different thresholds, or a different rule identity are different evidence.

## Adversarial expectations

The V1 corpus must cover at minimum:

- two independent multigeneration barrier-positive studies -> `Supported`;
- viable fertile hybrid -> `Contradicted`;
- realized hereditary gene flow -> `Contradicted`;
- repeated no-contact studies -> `InsufficientEvidence`;
- viable hybrid with unknown fertility -> `InsufficientEvidence`;
- adequate contact concentrated in only one barrier-supporting study -> `NotSupported`;
- barriers concentrated in only one generation of each study -> `NotSupported`;
- one study cannot satisfy the replication requirement;
- one SEL-09A design cannot be duplicated under two unit IDs;
- omitted preregistered study fails;
- serialized barrier-profile tampering fails;
- serialized status tampering fails;
- restored evidence requires fresh current replay;
- wire shape contains no species/speciation authority.

## Explicit nonclaims

Even `ReproductiveIsolationStatus::Supported` does **not** establish:

- that the lineage authorities correspond to universally accepted species;
- a particular species concept;
- irreversible reproductive isolation;
- zero gene flow in every environment or historical period;
- ecological isolation outside the preregistered contexts;
- geographic isolation as a reproductive mechanism;
- complete genomic divergence;
- monophyly;
- taxonomic naming authority;
- a historical time or mechanism of lineage splitting;
- a historical speciation event;
- species identity.

SEL-10 must introduce an explicit species concept, lineage-history evidence, temporal relationship between divergence and barriers, and separate current authority before any species/speciation claim can exist.

## Qualification boundary

Source review, static reasoning, and queued CI do not establish executable qualification.

The frozen CI helper for this tranche must execute against one exact product SHA, assert that SHA, run the 09B main and sustained-barrier corpora plus named prerequisite evolution-core corpora, and preserve a machine-readable receipt.

A queued or unassigned job is neither PASS nor product-code FAIL.
