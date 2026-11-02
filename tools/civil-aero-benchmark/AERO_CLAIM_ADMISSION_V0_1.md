# Claim-specific aerospace evidence admission v0.2

Status: software policy prototype. This is not a certification basis, regulatory interpretation, engineering authority delegation or flight-release process.

## 1. Purpose

A verified CFD run bundle answers: "Do these source artifacts, receipts, histories and declared screens belong to one replayable run?"

Claim admission answers a different question: "Is that evidence sufficient for this particular engineering claim, at this exact scope and consequence level?"

A run-bundle pass must not automatically authorize an aerodynamic prediction, structural load, flutter conclusion, mission-performance conclusion or flight decision.

## 2. Claim scope

Every claim carries a SHA-256 digest for a canonical, externally defined claim-scope manifest. The scope manifest should identify, as applicable:

- exact geometry and as-built/reference state;
- mesh, physics configuration and solver/build identity;
- Mach, Reynolds, angle of attack, altitude, temperature and other operating point;
- turbulence/transition/roughness assumptions and boundary conditions;
- time regime, forcing, sampling interval and averaging definition;
- structural configuration, material properties, loads and constraints;
- propulsion/mission payload, range, reserve and environmental assumptions;
- the declared validity envelope and its limits.

The current prototype uses exact hash equality. It deliberately rejects evidence from a different operating point or geometry rather than inferring that it is transferable.

This is conservative and incomplete for real validation campaigns. A later version should introduce typed scopes and explicit set/envelope coverage, including interpolation distance and out-of-domain handling. It must not silently replace exact scope with name matching or a broad label such as "CRM validation".

## 3. Claim classes and required evidence

ClaimClass defines the type of conclusion. DecisionConsequence independently defines the impact of using that conclusion.

| Claim | Base evidence contract |
|---|---|
| Exploratory design ranking | Geometry/input/solver/process/convergence identity plus declared comparison protocol |
| Aerodynamic prediction | Above plus grid convergence, conservation, model uncertainty, experimental validation and validation uncertainty |
| Unsteady aerodynamic prediction | Above plus time-step and temporal convergence, statistical sampling assessment |
| Structural load prediction | Structural verification and validation plus numerical/model uncertainty |
| Aeroelastic stability prediction | Structural evidence, coupled convergence, time-step evidence and flutter validation |
| Mission performance prediction | Aerodynamic/structural evidence, propulsion validation, mission model verification and mission uncertainty |
| Flight release | Broad technical evidence, operational envelope, calibrated measurements, independent review, quality-system evidence and authority decision |

High-consequence claims add independent review, calibration and model-uncertainty evidence. Safety-critical claims additionally require an authority-decision evidence reference.

Even complete evidence does not cause an automatic flight release. Flight-release and safety-critical requests return requires_human_authority_decision. The software cannot act as a regulator, delegated engineering authority or airworthiness approval body.

These policies are an initial internal engineering rubric, not a substitute for the applicable civil aviation regulations, certification specifications, means of compliance or project-specific certification plan.

## 4. Assertion semantics

Every supplied ScopedEvidence has one of three states:

- Verified: artifact SHA-256, verifier identity, verifier SHA-256 and verifier-receipt SHA-256.
- Unknown: a non-empty reason explaining why the evidence is unavailable or not yet established.
- Failed: a non-empty reason explaining a failed verification or qualification.

Missing and unknown required evidence lead to abstention. Failed or out-of-scope required evidence leads to rejection. Scope mismatch is rejected even for a supplemental assertion: the system does not mix evidence from another geometry, operating point or declared envelope into this claim.

The current module validates digest shape and binds verifier references. It does not authenticate a verifier against an external trust store, execute the verifier or independently verify the artifact. A trustworthy verifier registry, signature/attestation policy and verifier revocation mechanism remain separate requirements.

## 5. Evidence-manifest binding

Version 0.2 binds a canonical evidence-manifest digest into the claim-decision receipt.

The digest includes every supplied assertion, ordered by evidence kind:
- evidence kind and exact scope digest;
- exact artifact digest;
- verifier ID and verifier binary/source digest;
- verifier receipt digest;
- or the precise Unknown/Failed reason.

This is separate from the policy digest:
- policy digest: which evidence kinds are required for this claim and consequence;
- evidence-manifest digest: which exact evidence assertions were submitted;
- decision digest: policy, scope, required/satisfied/missing/unknown/failed sets, status and evidence-manifest digest.

Thus two different artifacts cannot produce the same claim decision merely because both were labelled "verified". The digest is an integrity commitment, not proof that the verifier itself is trustworthy.

Supplemental evidence not required by the claim is still bound into the manifest, but does not change the required-evidence outcome unless its scope is mismatched. A future schema may classify optional, adverse, superseding and contradictory evidence more explicitly.

## 6. Decision semantics

| Status | Meaning |
|---|---|
| admitted_within_declared_scope | Every required assertion is structurally marked verified at the exact scope; no authority decision is required by this prototype's consequence policy |
| abstain_insufficient_or_unknown_evidence | At least one required item is missing or unknown |
| rejected_failed_or_mismatched_evidence | Required evidence failed, or supplied evidence is for another scope |
| requires_human_authority_decision | The evidence rubric is complete, but the claim class or consequence forbids automatic approval |

Admission is limited to the declared scope. It is not a guarantee of correctness outside that scope, a universal validation claim or an authorization to manufacture, operate or fly.

## 7. Required next extensions

Before using the module for production engineering decisions:

1. Replace a single undifferentiated scope digest with typed geometry, solver, operating point, model, temporal regime and validity-envelope scopes.
2. Add coverage evaluation for a set of experimental validation points, with explicit interpolation/extrapolation policies and blind holdouts.
3. Bind evidence requirements to model influence, decision consequence and claim-specific uncertainty tolerances.
4. Define supersession, contradictions, adverse evidence, expired calibration and revoked verifier handling.
5. Add a trust store and signed verifier attestations; verify independent verifier build/source provenance and key validity.
6. Make the claim decision machine-readable in the larger engineering evidence graph and link the final design choice to the exact policy, candidate set, uncertainty and decision-loss function.
7. Validate policy completeness with independent aviation V&V and certification specialists before using it in a real certification programme.

## 8. Current evidence ceiling

claim_specific_policy_evaluation_with_exact_scope_and_full_evidence_manifest_digest_binding_not_artifact_verification_not_regulatory_approval_or_flight_authorization

The implementation is a software-policy prototype. Unit tests use synthetic verifier digests and assertions. They do not establish aerospace model accuracy, physical validation, regulatory compliance or airworthiness.
