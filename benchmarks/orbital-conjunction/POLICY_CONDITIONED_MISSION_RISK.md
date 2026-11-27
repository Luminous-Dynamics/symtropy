# Policy-conditioned mission-risk recomputation

Status: exact finite-support v0.1, built as a follow-on to the one-epoch non-anticipative policy verifier. This is not a continuous-orbit probability guarantee, calibrated observation model, or manoeuvre authorization.

## Why static action feasibility is not policy feasibility

A hard-risk gate can establish that each fixed portfolio is below a scenario-specific mission-union budget. It does not follow that an observation-conditioned policy that selects among those portfolios also remains below the same budget.

The selection action can be correlated with the latent risk mode through the observation history. A policy can therefore combine disjoint contact sets that no one static portfolio contains.

### Exact 75% / 25% counterexample

One scenario has two finite target-measure atoms:

| Atom | Exact mass | Portfolio A | Portfolio C |
|---|---:|---|---|
| World 1 | 75% | Definite mission contact | Definite no-contact |
| World 2 | 25% | Definite no-contact | Definite mission contact |

The original mission-risk budget is 80%.

Static action A has mission union \(3/4=75\%\), and static action C has mission union \(1/4=25\%\). Both pass the 80% gate.

Now suppose the observation at the frozen decision epoch identifies World 1 versus World 2. The non-anticipative policy chooses A after the World 1 observation and C after the World 2 observation. The policy-conditioned contact mass is

\[
\frac{75+25}{100}=1=100\%.
\]

The policy is structurally non-anticipative and every selected action is individually static-feasible, yet the policy itself violates the same 80% mission-risk budget. The static eligible set cannot serve as the adaptive policy's risk certificate.

## Exact finite-support contract

The current implementation adds two pieces of evidence to the shared-exogenous trajectory receipt chain:

- Every shared world receipt carries nonzero target-measure atom_mass_ticks.
- Every portfolio-specific trajectory receipt carries a pathwise mission-union disposition: DefiniteContact, DefiniteNoContact, or Ambiguous.

The receipt campaign rejects an ambiguous disposition. For every scenario it verifies that atom weights sum exactly to the total target support mass in the hard-risk gate. For every eligible static portfolio, it sums atom masses whose trajectory has a definite mission contact and independently checks that the resulting exact rational equals that portfolio's upstream mission_union_mass.

The policy campaign repeats those static checks when it consumes a receipt report; it does not treat a digest alone as a substitute for exact risk arithmetic.

After applying the non-anticipative policy to each complete scenario/world observation trace, it looks up the selected portfolio's pathwise contact disposition for that exact world. It sums the atom mass only where the selected trajectory has a definite mission contact, then reduces the exact rational and compares it with the original scenario-specific budget:

\[
R^{\pi}_s =
\frac{\sum_{u\in\mathcal U_s}
m_{s,u}\,\mathbf 1[
\mathrm{contact}(s,u,\pi(h_{s,u}))
]}
{\sum_{u\in\mathcal U_s}m_{s,u}}.
\]

The policy-level feasibility result is true only when R^pi_s is within the unchanged mission-risk budget in every required epistemic scenario. The report retains the exact per-scenario rational, the budget, the boolean comparison and a top-level policy_mission_risk_feasible result. A policy can produce a valid non-anticipativity report with that result false. No authority is granted and no static budget is re-spent.

## Observation and probability-space boundary

The finite world universe must represent the joint target measure over every latent variable that can affect either:
- the physical trajectory/mission contact; or
- the observation history used by the policy.

If measurement noise, delivery delay, missed detections or source availability are stochastic and can change the selected action, their admitted outcomes must be represented in the finite world/observation support with the correct joint target masses. A physical orbit atom reused across multiple observation outcomes must be expanded into distinct joint world atoms with the appropriate split mass. Do not silently treat an observed measurement as free information about a hidden physical state.

The current v0.1 verifier assumes the supplied world masses and observation traces are complete and correctly generated. Its digests bind content identity but do not prove source authenticity, observation-network completeness or calibration of the measurement likelihood. It does not yet model multiple observation epochs or action-dependent future tracking geometry.

The pathwise contact disposition is a definite finite-support outcome. This exact arithmetic cannot be applied to an ambiguous event, a conditional Pc estimate without a finite truth atom, or an unresolved numerical enclosure. Those cases must abstain or be represented by a separately qualified interval/uncertainty model. No unknown or ambiguous contact is counted as no-contact.

## Validation chain

1. Revalidate the exact hard mission-risk gate and its original scenario-specific budget/total-mass contract.
2. Validate the shared-exogenous world and portfolio trajectory receipt matrix.
3. Verify atom masses sum to the hard-risk gate's total target mass.
4. Recompute every eligible portfolio's static pathwise mission-union mass and compare with the upstream gate.
5. Verify the finite observation inventory for every scenario/world and evaluate the non-anticipative policy mapping.
6. For each world, select the exact portfolio trajectory bound to the observable history.
7. Sum selected pathwise contact mass per scenario and compare with the unchanged original budget.
8. Bind all per-world selected trajectory and contact outputs, per-scenario mass results and policy feasibility into the report digest.

The result is a finite-support arithmetic certificate for the supplied joint world and observation universe. It is not a proof that the universe is physically complete or that a propagator or sensor executed honestly.

## Adversarial controls

- Two static actions each pass an 80% budget, but an observation-conditioned policy combines disjoint 75% and 25% contacts and is marked infeasible at 100%.
- Fixed policy A returns exactly 75% and passes.
- Fixed policy C returns exactly 25% and passes.
- Changing one per-atom contact bit without changing the upstream risk mass is rejected.
- Reclassifying an ambiguous contact as no-contact is rejected.
- Atom mass zero, overflow or a total mass different from the gate is rejected.
- Changing atom mass and refreshing the shared receipt digest still fails if total support mass no longer matches the gate.
- Every eligible portfolio's pathwise static contact mass must reproduce its gate mass exactly.
- An action outside the hard-risk eligible set is rejected before policy-risk calculation.
- Observation-history branches cannot key on hidden scenario/world IDs or future measurement arrivals.
- Scenario A passing while scenario B fails makes the whole policy infeasible; the report preserves both scenario results.

## Next research extensions

- Replace the finite contact bit with conservative event-probability intervals where truth is not pathwise known.
- Extend to multi-epoch policies with action-dependent observation models and stopping times.
- Include measurement noise, delayed delivery and missingness in the joint target measure.
- Add simultaneous/time-uniform inference over adaptive policy branches and all required epistemic scenarios.
- Reproduce the 2026 chance-constrained belief-space MCTS study on its eight CARA cases and 96 tracking configurations before adding multi-encounter/multi-portfolio claims.
- Combine with the #7178 decision-centric multi-fidelity scheduler only when low-fidelity discrepancy enclosures are defensible.

## Related work

- [#7734 — policy-conditioned mission-risk research](https://github.com/Luminous-Dynamics/symtropy/issues/7734)
- [#7731 — finite non-anticipative policy verifier](https://github.com/Luminous-Dynamics/symtropy/pull/7731)
- [#7724 — shared-exogenous trajectory receipts](https://github.com/Luminous-Dynamics/symtropy/pull/7724)
- [#7726 — portfolio-specific manoeuvre execution noise](https://github.com/Luminous-Dynamics/symtropy/pull/7726)
- [#7637 — static hard mission-risk feasibility gate](https://github.com/Luminous-Dynamics/symtropy/pull/7637)
