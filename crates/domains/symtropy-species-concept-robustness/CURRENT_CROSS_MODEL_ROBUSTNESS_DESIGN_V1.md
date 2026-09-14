# SEL-10E2A Current Cross-Model Robustness Design V1

## Purpose

SEL-10E2A freezes the comparison design for **current-species** cross-model robustness before any model-bound current-species outcome is admitted.

It is intentionally separate from SEL-10E2B. E2A is preregistration and authority composition; E2B is the later outcome-bearing matrix/report.

The governing theorem is:

`agreement across models != semantic independence != fault-domain independence != universal species truth`

and:

`same biological subject != identical model-specific evidence subset`.

## Outcome-free boundary

No E2A type stores or derives a current species conclusion, historical transition conclusion, vote count, scalar taxonomy score, or universal-truth flag.

Family-specific classification **designs** are admissible because they are outcome-free protocol commitments. Model-bound classification **results** belong only in E2B.

Historical-transition robustness is outside V1. A current-status family without an independently qualified historical-transition capability is not given a synthetic historical surface merely to make comparison symmetric.

## Shared biological subject

`SharedEvidenceSubject` freezes:

- the exact ordered lineage pair;
- the exact SEL-10A lineage-history **design** digest;
- an exact external evidence-universe qualification authority.

Every preregistered model row must bind that same ordered pair and SEL-10A design identity.

This proves comparison-design subject parity. It does **not** claim that all families must consume identical operational criteria, nor does it yet bind an observed SEL-10A history result. Outcome/evidence snapshots are an E2B responsibility.

The evidence-universe authority is an explicit external trust edge qualifying that family-specific designs are projections over one frozen biological evidence universe. Its hash/reference is an auditable binding and provenance identity; it is not self-proving scientific truth.

## Family-specific design surfaces

V1 has typed adapters for:

- strict biological species current-status design;
- general-lineage current-status design.

Strict BSC may bind a reproductive-isolation design while general-lineage binds a different preregistered multichannel design. E2A retains those differences rather than forcing all concepts through one global criterion schema.

Every model row binds:

- the exact open family descriptor and digest;
- the exact family-specific outcome-free classification-design identity;
- the exact lineage pair and SEL-10A design identity;
- the exact model fault-domain profile and digest.

The family-specific design variant must agree with the descriptor source kind. Restored bytes cannot relabel a strict-BSC design as general-lineage or vice versa.

Only one authority instance per exact conceptual identity may appear in a design. Qualification variants of one conceptual family cannot inflate conceptual-family coverage.

## Semantic dependency is not independence

E2A consumes current SEL-10E1A3 semantic-relation evidence for every preregistered model pair.

`NestedOrCriterionDependent` and `Overlapping` are never eligible for independent conceptual coverage.

`PotentiallyNonNested` means only that the semantic relationship does not itself rule out independence. It is **not** an independence proof.

Unknown, unavailable, disputed, or otherwise unqualified semantic relation evidence never upgrades pair eligibility.

Strict BSC versus general-lineage can therefore remain scientifically useful as model sensitivity while still failing the strongest independent-coverage theorem if their qualified semantic relation is nested or criterion-dependent.

## Model fault-domain profiles

`ModelFaultDomainProfile` records six named dimensions:

1. qualification organization;
2. qualification process;
3. evidence-source lineage;
4. implementation/toolchain lineage;
5. upstream evidence-authority lineage;
6. semantic-mapping qualification lineage.

These IDs are qualified classifications and provenance. Merely giving two profiles different strings does not prove independence.

The profile qualification authority may legitimately attest more than one model profile. Qualification provenance is not itself conceptual identity or fault-domain identity.

## Preregistered independence policy

V1 does not hard-code that all six dimensions must differ. Instead `FaultDomainIndependencePolicy` freezes the exact dimensions required to differ for this comparison.

The policy must:

- contain at least two distinct dimensions;
- be canonically ordered and duplicate-free;
- require at least one evidence-related dimension (`EvidenceSourceLineage` or `UpstreamEvidenceAuthorityLineage`);
- bind an external qualification authority;
- bind the built-in V1 policy rule.

This prevents a caller from manufacturing an independence claim solely from organizational or implementation diversity while allowing different scientific applications to preregister a justified fault-domain theorem.

Changing the required dimensions, policy qualification, or rule changes the policy digest and therefore the E2A design lineage.

## Pairwise fault-domain qualification

Every preregistered pair has an explicit `PairwiseFaultDomainAssessment` binding:

- canonical conceptual endpoints;
- exact left/right fault-profile digests;
- exact independence-policy digest;
- typed disposition;
- external pair-qualification authority.

`QualifiedSufficientlyDistinct` is valid only if **every dimension required by the frozen policy differs** between the exact two profiles.

Pair qualification is still an external scientific trust edge. The software verifies exact bindings and policy invariants; it does not independently prove that an organization, dataset, toolchain, or evidence lineage is truly independent in the world.

## Complete matrices

For N preregistered conceptual families, E2A requires every unordered pair exactly once in both:

- the semantic-relation matrix;
- the fault-domain-assessment matrix.

A missing pair fails closed. A contradictory or dependent pair cannot be silently omitted after outcomes are known.

Model and pair ordering are canonical and non-semantic.

## Pair eligibility is not robustness

`pair_is_eligible_for_independent_coverage` is deliberately narrow.

It returns true only when:

1. the current semantic relation is `PotentiallyNonNested`; and
2. the current pairwise fault assessment is `QualifiedSufficientlyDistinct` under the exact frozen policy.

That value means only **eligible to contribute to later independent-coverage reasoning**.

It is not a species conclusion, not a robustness result, not a p-value, not a vote, and not universal taxonomic truth.

E2B must still evaluate the complete preregistered model matrix, applicability/missingness, exact result identities, and the independent-coverage rule.

## Persistence versus current authority

Serde E2A objects are persisted representations, not current authority.

`SpeciesModelDesignRecord`, `SemanticRelationRecord`, and `CurrentCrossModelRobustnessDesign` retain auditable exact identities and enforce local invariants, but restored bytes do not regain currentness by themselves.

The non-Serde current-authority layer requires fresh replay:

- `CurrentSpeciesModelDesignRecord::validate_strict_biological` recomputes a strict row from a current strict descriptor, current strict classification design, and current fault profile;
- `CurrentSpeciesModelDesignRecord::validate_general_lineage` recomputes a general-lineage row from current general-lineage descriptor/design authority;
- `CurrentSemanticRelationRecord::validate_current` recomputes a row from current SEL-10E1A3 relation evidence;
- `CurrentCrossModelRobustnessAuthority::validate_current` can be formed only from those current row capabilities plus the current subject/policy/pair trust edges and exact preregistered thresholds.

**SEL-10E2B must consume `CurrentCrossModelRobustnessAuthority`.** A representation-level replay of stored rows is insufficient downstream authority.

The older representation-level `ValidatedCurrentCrossModelRobustnessDesign` helper exists only as local persisted-state replay plumbing in V1. It must not be used as the E2B authority gate.

## Adversarial requirements

The V1 corpus must establish at least:

- real strict-BSC and general-lineage designs can share one subject while retaining different criteria;
- duplicate conceptual identities cannot inflate model count;
- canonical model ordering is non-semantic;
- different SEL-10A history-design subjects fail comparison;
- missing semantic or fault-domain pair coverage fails closed;
- nested/criterion-dependent semantics blocks independent eligibility even when fault domains differ;
- shared required evidence fault domains block `QualifiedSufficientlyDistinct`;
- independence policy cannot omit evidence diversity or collapse to one dimension;
- independence-policy drift invalidates stale pair assessments;
- restored source-kind swaps fail;
- restored semantic-dependency summaries cannot self-authorize;
- restored noncanonical policy/model ordering fails;
- unknown semantic relation evidence never upgrades eligibility;
- persisted model rows require fresh family replay;
- profile/qualification drift cannot regain model-row authority;
- persisted semantic-relation rows require fresh relation evidence;
- relation qualification drift cannot regain relation-row authority;
- top-level current authority requires fresh model/relation row capabilities;
- wire representation contains no model outcome, majority-vote, historical-transition, or universal-taxonomy result.

## Explicit non-claims

SEL-10E2A does not establish:

- any current species conclusion;
- cross-model robustness or contradiction;
- historical speciation/transition robustness;
- statistical independence;
- causal independence of evidence sources;
- a philosophically preferred species concept;
- nomenclatural authority;
- a majority-vote species decision;
- universal taxonomy truth.

Its positive claim is narrower:

> For this exact preregistered set of current species-concept families over this exact comparison-design subject, the family-specific design surfaces, semantic dependencies, fault-domain profiles, independence policy, pair qualifications, and coverage thresholds were frozen with the recorded identities before outcome-bearing cross-model execution.
