# AERO Non-Flight-Critical Physical Pilot — Preregistration Protocol v0.1

**Tracker:** [#7396](https://github.com/Luminous-Dynamics/symtropy/issues/7396)  
**Composes:** #4165 DFI/SHM; #7455 temporal evidence assurance; #7395 AP242/QIF semantics; Mycelix AeroCommons #6347; Symthaea BASALT #9203/#9210/#9239; uncertainty provenance #7412; evidence-action selector #13112  
**Status:** preregistration template only — no article, method, dataset, sample size, or experiment selected  
**Claim ceiling:** non-flight-critical physical correlation and method-specific evidence only; no aircraft qualification, approved maintenance credit, material allowables, certification, or airworthiness claim

## 1. Purpose and decision question

This protocol turns the AERO physical-pilot issue into a reviewable experiment contract. It is designed to test whether the evidence-qualified engineering workflow can bind a physical observation to the exact article, configuration, method, calibration, time interval, source/receiver evidence view, uncertainty and independent reference examination.

The pilot is **not** intended to demonstrate flight safety or replace an approved inspection programme. Its first objective is narrower:

> On a rights-cleared, non-flight-critical article, can the system preserve raw measurements, distinguish structural indications from sensing/environment changes, and produce an independently replayable, appropriately uncertain engineering result under a preregistered protocol?

A performance improvement is a separate hypothesis from evidence-binding correctness. The pilot must be able to fail on either.

## 2. Eligibility and non-negotiable boundaries

The selected article must be:
- non-flight-critical and not in service;
- physically serialized, with design/configuration and material/process lineage available to the extent required by the question;
- owned or controlled by a party able to authorize testing and publication of the intended evidence;
- accompanied by written rights for all data, images, geometry, test records and derived outputs that will be retained or published;
- representative of a clearly stated material/detail/method scope, but not silently promoted to a broader fleet or aircraft claim.

The pilot must not:
- alter an in-service aircraft, operational safety system or mandatory maintenance programme;
- relax, defer or substitute a required inspection based on an SHM prediction;
- use synthetic data, injected faults or model-generated labels as physical defect ground truth;
- infer airworthiness, structural substantiation, material allowables or part acceptance from software correctness;
- claim NDE probability of detection (POD) outside the specific defect/material/geometry/procedure/operator/equipment domain and statistical method actually evaluated.

If these boundaries cannot be satisfied, the activity remains a simulation or synthetic-fixture study.

## 3. Preregistered article, configuration and population

Complete before confirmatory acquisition.

| Field | Required entry |
|---|---|
| Article identity | Manufacturer/owner ID, serialized article ID, immutable physical marking and identity evidence |
| Definition/configuration | Exact drawing/model revision, geometry, joint/detail, repair state and configuration digest |
| Material/process | Material specification, supplier/source, lot/batch, layup/temper, process route, machine/tooling and known deviations |
| Article condition | Prior loading, fatigue, impacts, repairs, storage/environment and known damage |
| Population represented | Precisely state what units the article can and cannot represent |
| Exclusions | Material, defect, geometry, environment, procedure and sensor conditions outside the claim |
| Claim under test | One falsifiable primary engineering claim and explicit claim ceiling |

The serialized physical article is the experimental object. CAD identity, a specimen label, a filename or a digital-twin ID is not a substitute for verified physical identity.

## 4. Independent unit and dependence structure

Do not define `n` as the number of wave windows, sensor channels, repeated scans or cropped specimens unless that is genuinely the independent unit for the stated inference.

Predefine the hierarchy relevant to the selected design, for example:

`source/supplier → material lot → process batch/run → panel/article → sampling event → specimen/detail → instrument/channel → acquisition → waveform/window`

Record which levels are:
- independent sampling units;
- blocking/stratification factors;
- repeated measurements;
- nested subsamples;
- common-cause/dependence groups.

A specimen cut from one panel is not automatically an independent panel. Multiple channels on one article are not automatically independent physical experiments. Repeated wave windows under the same environmental episode are not independent replications.

The statistical analysis plan must bind the actual independent unit, the meaning of `n`, and the resampling/variance unit. Use whole-lot, whole-panel/article, campaign, installation or time holdouts when those represent the intended generalization claim.

## 5. Governing method selection and reference evidence

The governing NDE/SHM method must be selected by a competent practitioner after the article, material, expected defect families, access, thickness/geometry, surface condition and purpose are known. Do not select a standard merely because its title sounds relevant.

Candidate comparators include:
- **ASTM E2533-21**, active guide for nondestructive examination of polymer-matrix composites used in aerospace applications. Use as a method-selection guide where its scope matches; it is not itself a universal procedure or POD result.
- **ASTM E2580-24**, current practice for ultrasonic testing of flat-panel composites and sandwich-core materials in aerospace applications. Apply only if the selected part and method fit its geometry and procedure.
- **ASTM E2862-23**, active hit/miss POD statistical practice for a specific set of NDE examination parameters. Use only when the selected experiment produces suitable hit/miss data and its assumptions fit.
- **MIL-HDBK-1823A**, NDE system reliability assessment guidance. DLA lists it as active, but the handbook states it is guidance only and is not a standalone contract requirement.
- **SAE ARP6461A**, fixed-wing civil-airframe SHM lifecycle and assurance guidance comparator where the research question is relevant to aircraft SHM. This non-flight-critical pilot does not establish compliance or certification credit.

Before preregistration is frozen, record the exact edition, procedure, application/profile, equipment, sensor/scan layout, calibration requirements, operator qualification, acceptance criteria and deviations. A standard number without a controlled procedure is insufficient.

### Reference/ground-truth plan

Define independently how a structural indication or defect state will be established:
- method and governing procedure;
- defect family, size/severity/orientation/depth, location and uncertainty;
- reference examiner(s), qualification and blinding;
- adjudication rule for disagreement or indeterminate findings;
- destructive/sectioning confirmation, where suitable and authorized;
- limitations of the reference itself.

The reference method is not infallible. Report disagreement, false calls, censored observations and indeterminate truth explicitly. Do not use the SHM model's own reconstruction as its reference truth.

## 6. Experimental arms and blinding

Select arms before confirmatory data collection. Candidate arms, where applicable:
1. conventional qualified NDE / manual engineering plan;
2. signal/reference-change baseline;
3. statistical novelty baseline (PCA/robust PCA or a prespecified equivalent);
4. learned temporal baseline (CNN/TCN or prespecified architecture);
5. physics-informed/ROM baseline where geometry and model inputs permit;
6. sensor-health-only diagnosis;
7. coupled damage + sensor fault + environment + unknown model;
8. qualified human examiner/analyst adjudication;
9. AERO evidence-qualified workflow.

All arms must use the same allowed observations, defect truth, article/configuration scope, operational exposure and mandatory safety requirements. If a method is not applicable, record NOT_APPLICABLE rather than substituting an unrelated weaker baseline.

Blind the confirmatory reference examiner and, where practical, the analyst/model operator to defect labels and hidden intervention schedule. Freeze the detection threshold and alert policy on development/calibration groups before confirmatory scoring. Preserve the randomization seed and intervention assignment under controlled access.

## 7. Environmental and sensor fault design

Separate structural state from sensing and operating state. Relevant nuisance/fault conditions may include:
- temperature, humidity, load, boundary conditions and operating state;
- sensor bonding/attachment degradation, sensor relocation or replacement;
- bias, drift, gain change, saturation, dropout, stuck-at, timing offset, synchronization loss, cross-talk or intermittent connection;
- acquisition software, channel mapping, clock, power and cable/connectivity faults.

Synthetic injection may test the software's fault-handling logic, but injected faults must be labelled synthetic and cannot be counted as physical sensor-fault observations. Where simultaneous damage and sensor fault are studied, damage truth and sensing-fault truth must be established independently.

Raw data are immutable. A corrected/reconstructed signal is a derived artifact with parent digest, method/model version, parameters, uncertainty and reason. It may never overwrite raw measurements or clear a damage indication by itself.

## 8. Three-axis evidence and metrology preregistration

For every confirmatory observation, retain three separate historical coordinates:

`(T_subject_interval, T_evidence, T_trust)`

- **Subject/acquisition:** acquisition-time interval, clock source, synchronization state, time uncertainty and acquisition configuration.
- **Evidence receipt:** exact raw-observation digest, receiver receipt, evidence-view checkpoint and inclusion watermark.
- **Trust/authority:** immutable issuer-authority, certificate/revocation and trust-policy snapshot, including policy digest and verifier profile.

The evidence-view checkpoint must commit to the canonical evidence-view digest and the receipt watermark. Observer-coverage qualification is separate: a valid checkpoint does not prove that all influential source channels were observed. The BASALT owner-native coverage receipt must identify the observer profile, covered channels/roots, exclusions and unresolved inputs. If coverage is incomplete, the result must abstain; if unverified, it cannot support a negative assertion such as “no revocation was present.”

For each instrument and method, bind:
- exact calibration evidence-record identity and artifact reference;
- instrument ID, channel/sensor identity, measurement method and revision;
- half-open validity interval and acquisition-time uncertainty;
- applicable range, environmental constraints and any pre/post checks;
- calibration provider/authority status and evidence snapshot;
- instrument configuration and acquisition software/timebase.

A missing, stale, out-of-range, scope-mismatched or boundary-ambiguous calibration blocks the affected observation. No clock or calibration status is promoted from a self-declared timestamp.

## 9. Sample-size, power and precision analysis

**Do not enter a sample count until the governing method, estimand, independent unit and analysis are selected.**

Before confirmatory testing, document:
- primary endpoint and null/alternative hypotheses;
- target effect or required precision, selected on engineering grounds;
- type-I error, desired power or confidence level and justification;
- prevalence/stratification of each defect family and severity;
- dependence/cluster structure and intraclass correlation assumptions where applicable;
- attrition, censored/invalid scans and indeterminate ground truth;
- POD method and data adequacy if POD is claimed;
- exact independent unit used for the sample-size calculation.

For an exploratory pilot, estimates of variance, prevalence, sensor drift or effect size are hypothesis-generating only. If those estimates are used to redesign the confirmatory study, freeze the revised protocol and keep exploratory observations out of the confirmatory holdout unless the preregistered design explicitly supports their inclusion.

A row count or number of waveforms is not a power analysis.

## 10. Prespecified outcomes and analysis

Choose one primary endpoint per claim; keep secondary endpoints explicitly secondary.

Possible outcomes:
- defect/event sensitivity by prespecified family and severity;
- hit/miss POD with confidence bounds, only when applicable;
- false calls and false alarms per defined operating/inspection exposure;
- detection delay from independently recorded damage introduction or reference time;
- localization/sizing error where independent truth supports it;
- sensor-fault diagnosis and damage-vs-sensor-fault confusion;
- missed-damage rate under simultaneous sensor fault;
- uncertainty calibration, interval coverage/width and subgroup calibration;
- selective risk/coverage and abstention rate;
- raw-to-derived lineage completeness;
- inspection, engineering, compute and maintenance effort.

Report the numerator, denominator, independent unit, uncertainty interval and subgroup breakdown. Publish event-level predictions and abstentions, subject to rights. Avoid accuracy/AUROC-only claims, especially with rare defects. A lower signal reconstruction error never compensates for a missed damage event or false healthy clearance.

### Economic comparison

Measure engineering/lab hours, inspection time, calendar time, compute, retests, rework and evidence-reconciliation burden. A claimed reduction is valid only at matched defect coverage, detection confidence, mandatory inspection obligations and false-accept constraints. No SHM output earns maintenance credit in this pilot.

## 11. Stop, abstention and invalidation rules

The protocol must stop or quarantine affected observations when:
- physical article/specimen identity or configuration is unresolved;
- raw bytes or evidence-view checkpoint digest mismatch;
- observer coverage is incomplete or unqualified for the requested negative claim;
- acquisition-time interval crosses calibration or event-effectivity boundaries;
- calibration instrument/method scope does not match;
- clock rollback, synchronization failure or unexplained sequence gap/fork occurs;
- an issuer/authority, signature, revocation or trust snapshot is unknown or invalid;
- reference truth is missing, corrupted or retrospectively changed without a new append-only report.

A new evidence view or trust snapshot creates a new report. It does not rewrite the historical result. Unknown, incomplete and ambiguous states remain explicit.

## 12. Data custody, rights and reproducibility

Before acquisition, record data owner, permitted use, retention period, publication/redistribution terms, export restrictions and de-identification/controlled-access needs.

Retain, where rights permit:
- raw source bytes and cryptographic hashes;
- file inventory, byte lengths, channel map, units and metadata;
- article/configuration/material/process manifest;
- calibration and reference-method records;
- receipt/checkpoint and observer-coverage commitments;
- preprocessing/conversion source, environment, parameters and output digests;
- frozen train/development/calibration/confirmatory partitions;
- analysis scripts, dependency lockfiles, model weights, thresholds and random seeds;
- preregistered protocol and all deviations/amendments.

No pickle or other executable serialization from an untrusted source may be deserialized on a developer workstation, shared CI runner or production system. Use a disposable network-isolated conversion environment only after source integrity and rights review.

## 13. Independent review and promotion gates

Before confirmatory acquisition:
- qualified NDE/metrology reviewer approves the method and ground-truth plan;
- statistician or competent quantitative reviewer approves the independent unit, dependence model and power/precision analysis;
- data owner/legal reviewer confirms rights;
- systems/security reviewer approves evidence receipt, observer coverage and data-ingestion boundary;
- safety/engineering reviewer confirms non-flight-critical scope and claim ceiling.

After acquisition:
- independent reviewer reproduces the data manifest and primary analysis;
- deviations and exclusions are reported;
- subgroup and adverse outcomes are retained;
- the claim is promoted only to the narrowest scope supported by the physical evidence.

**The pilot does not qualify an aircraft, approve maintenance changes, grant SHM airworthiness credit, establish material allowables, or certify any component.**

## 14. Preregistration status template

Complete and freeze these before sample count and confirmatory collection:

| Item | Required state |
|---|---|
| Article / exact configuration | UNSELECTED |
| Rights and data permissions | UNVERIFIED |
| Governing NDE/SHM procedure | UNSELECTED |
| Reference/ground-truth method | UNSELECTED |
| Independent unit and dependence groups | UNSELECTED |
| Primary endpoint / estimand | UNSELECTED |
| Power/precision analysis | NOT PERFORMED |
| Sample count | NOT SET |
| Blinding/randomization | NOT FROZEN |
| Evidence checkpoint / BASALT coverage profile | NOT QUALIFIED |
| Independent approvals | PENDING |
| Confirmatory data collection | NOT AUTHORIZED |
