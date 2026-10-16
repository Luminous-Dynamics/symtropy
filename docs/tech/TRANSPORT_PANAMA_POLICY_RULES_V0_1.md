# Panama Canal Reservation Policy Rules v0.1

**Reviewed:** 2026-10-16  
**Status:** public-rule transcription and conformance-test design; original PDF bytes are not yet acquired/hash-frozen and full rights/use review remains open.  
**Tracking issue:** [TRANSPORT-POLICY-001 #1681](https://github.com/Luminous-Dynamics/symtropy/issues/1681)  
**Source-admission gate:** [TRANSPORT-DATA-PANAMA-001 #1677](https://github.com/Luminous-Dynamics/symtropy/issues/1677)  
**Historical replay:** [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671)  
**Provisional row set:** [TRANSPORT_PANAMA_EVENT_TIMELINE_PROVISIONAL_V0_1.csv](TRANSPORT_PANAMA_EVENT_TIMELINE_PROVISIONAL_V0_1.csv)  
**Synthetic golden fixtures (not actual awards):** [TRANSPORT_PANAMA_POLICY_GOLDEN_CASES_V0_1.csv](TRANSPORT_PANAMA_POLICY_GOLDEN_CASES_V0_1.csv)

## 1. Paradigm shift

The Canal is not simply a pipe with a daily capacity integer. It has a changing **allocation mechanism**: booking competitions, market-segment minima and fallback rules, customer ranking, auction routes, long-term packages, substitutions, cancellation rules, and special cases across booking periods.

A valid simulation therefore needs two models that must not be conflated:

1. **Physical feasibility:** whether a proposed transit satisfies the applicable vessel, draft, lock, time-window and operating constraints.
2. **Reservation-policy eligibility/allocation:** whether a request is eligible for a slot under the rules known and effective for that competition/booking date.

A proposed eligible allocation is not proof the actual Canal awarded that slot. Customer rankings, competing requests, bids and the complete live allocation state are not established by public rule text alone.

## 2. Public rule matrix (summary, not complete extraction)

| Rule source | Published | Effective / applicable date | Rule change | Conformance requirement |
|---|---|---|---|---|
| [A-30-2026](https://pancanal.com/wp-content/uploads/2026/08/ADV-30-2026-Modifications-to-the-Transit-Reservation-Booking-System.pdf) | 25 Aug 2026 | Effective immediately | Reallocation priority for cancelled/newly available slots; last-minute reservation eligibility; distinct NetZero swap versus substitution requirements | Preserve segment order, customer-ranking predicate, and separate swap/substitution eligibility. |
| [A-31-2026](https://pancanal.com/wp-content/uploads/2026/04/ADV-31-2026-Transit-Reservation-Booking-System-Rule-modifications.pdf) | 28 Aug 2026 | Effective 30 Aug for booking dates from 13 Sep | Neopanamax multiple slots per customer/date and consecutive booking dates; 63 offered slots per week; segment minima, vessel capacity ranges and fallback order | Scope only to Neopanamax where stated; reproduce segment-specific limits, weekly vs daily units and fallbacks. |
| [A-35-2026](https://pancanal.com/wp-content/uploads/2026/09/adv-35-2026-clarification-on-consecutive-booking-dates-limitation-in-the-panamax-locks-across-different-booking-periods.pdf) | 16 Sep 2026 | Effective immediately | Consecutive-date restriction does not apply across different booking periods; applies to Panamax locks only | Same-period and cross-period cases must yield different eligibility where specified; do not apply this exception to Neopanamax or erase the same-period restriction. |
| [A-36-2026](https://pancanal.com/wp-content/uploads/2026/04/ADV-36-2026-Increase-in-the-Number-of-Neopanamax-Booking-Slots-Draft-Adjustment-and-Transit-Reservation-System.pdf) | 28 Sep 2026 | Slot policy for booking dates from 15 Oct; draft change effective immediately | Neopanamax slots rise from 9 to 10; 33 total daily slots stated; maximum draft changes to 14.94 m | Slot count and maximum draft are separate constraints and separate effective dates. |
| [A-37-2026](https://pancanal.com/wp-content/uploads/2026/10/ADV-37-2026-Long-Term-Slot-Allocation-Program-2027-and-Modification-to-Transit-Reservation-System-tariffs.pdf) | 5 Oct 2026 | LoTSA booking horizon 3 Jan–3 Apr 2027 | Neopanamax LoTSA continues; new Panamax sealed-bid LoTSA pilot; roughly 3 slots/day average for each program; cancellation-surcharge threshold changes from 7 to 4 days effective 15 Oct | Competition date, award window, booking date and transit date are distinct. Do not infer actual awards or bids. |
| [A-37-2026 key dates](https://pancanal.com/wp-content/uploads/2026/10/ADV-37-2026-Long-Term-Slot-Allocation-Program-2027-and-Modification-to-Transit-Reservation-System-tariffs.pdf) | 5 Oct 2026 | Neopanamax competition 17 Nov 2026; Panamax sealed-bid competition 24 Nov 2026; first priority windows 29 Nov and 16 Dec respectively | Long-term packages precede their 2027 booking periods | Conformance cases must not mistake the November competition for the actual transit/booking horizon. |

This is a high-level public rule summary. Exact eligibility, allocation tables, footnotes, exception chains, package details and tariff cross-references must be extracted from the original PDFs and manually reviewed before the policy set is called complete.

## 3. Minimum bitemporal rule representation

Each executable rule needs these distinct fields (reuse a canonical source/evidence schema if one exists):

- stable rule ID and rule version;
- source artifact ID/digest, exact page/table or clause locator;
- published/known-at time and local time convention;
- effective-from/effective-until interval;
- scope: lock system, booking period, booking date vs transit date, vessel/market segment, and any capacity unit/range;
- predicate and result with explicit units;
- min/quota, fallback order, ranking/tie-break and allowed exceptions where actually stated;
- predecessor/supersession/amendment links, retaining earlier rule versions instead of overwriting;
- rights/allowed-use state and unknown/unparsed fields.

**Fail-closed invariant:** an unparsed footnote, missing applicability domain or unresolved source conflict cannot silently become “unrestricted” eligibility.

## 4. Exact golden cases for an independent rule evaluator

All fixtures below are synthetic **rule-conformance cases**, not historical award reconstruction.

1. **Knowledge-time leakage:** A-37 is not knowable to a simulated planner before its 5 Oct publication. Its Nov competition details can inform later forecasts, but not a decision allegedly made in September.
2. **A-31 activation:** an eligibility request for a Neopanamax booking date before the new rule's applicability must use the previously effective rule; a booking date on/after 13 Sep must use the stated A-31 rules, assuming all other prerequisites.
3. **A-31 scope:** the multiple-slot/consecutive-date exception must not spill over to Panamax.
4. **A-35 cross-period exception:** two consecutive Panamax booking dates awarded in different periods can pass the specific consecutive-date check if all other requirements hold; the same-period negative control remains subject to the same-period restriction.
5. **Segment minima/fallback:** allocate fixtures with and without eligible Full Container/LPG/LNG/Car Carrier/RoRo/Bulker/Other requests; check only the documented fallback transitions and preserve the minimum-slot denominator/time basis.
6. **Cancelled slot priority:** a cancelled slot enters the A-30 special-competition rule path; do not allocate it through an invented first-come-first-served or shortest-vessel rule.
7. **NetZero swap vs substitution:** equal eligibility and PCUMS constraints for a swap differ from the equal-or-greater PCUMS requirement for substitution.
8. **Slot vs physical constraint:** A-36's booking-slot increase does not relax draft, vessel class, maintenance, or actual transit constraints.
9. **Cancellation fee cutoff:** the A-37 four-day threshold applies only from 15 Oct 2026; before that effective date, the prior threshold and tariff must be represented according to the actual prior rule text. Test exact boundary values at five, four and three days.
10. **LoTSA timeline:** the Nov 17/24 competitions are not the Jan 3–Apr 3 2027 booking window; Panamax sealed bids and Neopanamax competition remain separate program rules.
11. **Supersession / correction:** a rule effective after a later amendment must not override the amendment within the latter's scope; retain both in evidence history.
12. **Input-order determinism:** reorder customer/request fixtures and rule-load order; canonical decisions and rejection explanations must be identical if the underlying content is unchanged.

## 5. Benchmarks

- 100% agreement with manually reviewed golden eligibility/outcome cases within the explicitly modeled public-rule domain.
- Zero rule application before its publication/known-at time or outside its effective scope.
- Zero segment quota/unit violations; no substitution of daily, weekly and package-period amounts.
- 100% of rules either parsed and qualified or surfaced as unsupported/unknown; no silent omission of table footnotes.
- 100% deterministic results under request/rule ordering permutations.
- Every rejection identifies the violated rule version, source clause and applicability window.
- Fair-access reporting must publish denominators by market segment (requests, eligible requests, awards if available, and unserved requests). Do not publish a universal fairness score.
- No empirical “fairer access,” service, revenue or customer-outcome improvement claim without observed request/award/outcome evidence or a predeclared experimental design.

## 6. 2027 prospective policy evaluation

The publicly announced LoTSA competition dates are 17 Nov 2026 (Neopanamax) and 24 Nov 2026 (Panamax sealed bids), with 2027 booking dates from 3 Jan to 3 Apr. This creates a future opportunity to freeze:

- the public rule set and its known-at cutoff;
- synthetic conformance fixtures and independent checker;
- public, observable output targets (if any) and permitted outcome sources;
- the policy analysis baseline and fairness/service metrics.

Do **not** invent customer bids/ranks or assume exact live awards are public. If only public rule conformity can be tested, the result must remain a rule-conformance evaluation—not empirical allocation validation.

## 7. Rights, integration and nonclaims

- Official rule text is a source lead; original bytes, hashes and use terms remain unqualified until the artifact admission gate is completed.
- Do not copy private booking-system/customer data or claim the public PDF terms grant rights over private operational records.
- Do not make the planner reservation-authoritative. An accepted proposal does not book or dispatch a vessel.
- Keep policy eligibility, physical feasibility, realized award outcomes and observed transit outcomes as distinct evidence types.
- Compose the existing Symthaea measurement-validity/evidence graph, Mycelix provenance boundary, and Sol Atlas projection. No fifth canonical transport ontology, source registry or event ledger.
