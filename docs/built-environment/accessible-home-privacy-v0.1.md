# ACCESS-HOME-PRIVACY-001 — resident-controlled evidence and privacy contract v0.1

**Status:** research / architecture proposal. This is not legal advice, a POPIA compliance declaration, a security audit or permission to process real resident information.

## 1. Design principle: minimize knowledge, not merely access

Accessible-home planning may expose disability and health needs, preferences, household/support arrangements, fine-grained floor plans, entrances/exits, alert placement, daily routines and location. South Africa's POPIA definition of personal information includes disability, physical or mental health, location/address and personal opinions or preferences. A detailed floor plan or unusual combination of access needs can also increase linkability even after names are removed.

Treat the resident's evidence as resident-controlled material rather than ordinary product telemetry. The application should keep as little information as possible, locally by default, and reveal only the minimum evidence needed for the next decision. A public benchmark receipt may say "a synthetic fixture was internally consistent"; it must never contain a real plan, raw measurements, identifying details or an unkeyed digest of sensitive real evidence.

Research sources:
- [Protection of Personal Information Act 4 of 2013 (official Justice PDF)](https://www.justice.gov.za/legislation/acts/2013-004.pdf)
- [South African Government POPIA overview](https://www.gov.za/documents/protection-personal-information-act)
- [Information Regulator privacy principles](https://inforegulator.bizportal.gov.za/Pps/Principles.aspx)
- [WHO Housing and health guidelines](https://www.who.int/publications/i/item/9789241550376)
- [ISO 21542:2021 status and scope](https://committee.iso.org/standard/71860.html)

These sources inform this design; they do not establish compliance. An Information Officer/privacy counsel and affected residents must review any real deployment against the actual purposes, actors, data flows and applicable law.

## 2. Data classes and default handling

### P0 — Public synthetic artifacts
Examples: invented fixtures, schemas, rule definitions, synthetic benchmark receipts.
- Allowed in public repositories and CI.
- Explicit `synthetic` claim boundary.
- No real floor plans, copied resident notes, real measurement datasets, unique resident configurations or real-world evidence references.
- SHA-256 may identify the exact synthetic fixture bytes for reproducibility.

### P1 — Resident-local evidence
Examples: resident's exact goals and preferences, detailed home measurements, annotated routes, photos, device locations, assistive/support arrangements.
- Local-first; no account or network dependency required for core planning.
- Not uploaded by default; no analytics or remote crash telemetry containing plan data.
- Encrypt stored data where the platform supports it, protect key material separately, lock application access, and offer resident-chosen encrypted export/backup with a documented recovery story.
- Raw photos, video, floor plans and free-text notes are especially sensitive. Collect only if needed for a resident-selected purpose.

### P2 — Project-confidential evidence
Examples: measurement reports, specialist reviews, installation drawings, vendor/service tickets.
- Separate from resident identity/contact mapping wherever practical.
- Explicit named audience, scope and expiry; no public issue attachments.
- Fine-grained access to only the evidence required for a role/task, not a single all-or-nothing project folder.
- A new party, purpose or use requires a compatibility/authority review and the appropriate resident notice/consent before sharing.

### P3 — Shareable derivative
Examples: resident-approved redacted constraints, a measurement summary, or an aggregate result prepared for a review.
- Must be deliberately produced and inspected for direct identifiers, indirect/linkable details and hidden metadata.
- Resident or authorized decision-maker approves the exact recipient, purpose and fields disclosed.
- A derivative does not become anonymous merely because the name was removed. Small cohorts, rare need combinations, exact locations and detailed layouts can enable re-identification.
- Public release is a separate decision. Once downloaded or copied, revocation cannot guarantee that every copy can be recalled; disclose this limitation before consent.

## 3. Minimal evidence envelope

Every non-public evidence item should have a **minimal control envelope** outside the evidence payload:
- random, non-semantic evidence ID (must not encode initials, address, diagnosis or room);
- purpose ID and version;
- data class and default access tier;
- resident/controller role and permitted reviewer role(s);
- source type and declared scope, with the payload stored separately;
- collection date or date precision chosen for the purpose;
- evidence state and uncertainty;
- retention/expiry rule and deletion mechanism;
- consent/authority record reference where required;
- schema/rule version and audit/change history.

Do not place the resident's name, street address, account ID, disability diagnosis, raw floor plan or exact measurements in a public ID, file path, log message, tag, commit title, branch name, PR, fixture or receipt.

Use random IDs with a project-local mapping held separately. The mapping should be kept in resident-controlled or appropriately restricted storage. Pseudonymous data remain personal information when a party can reconnect it to a person.

## 4. Hashes, signatures and evidence truth

- A digest proves byte identity relative to a known value; it **does not** prove that the underlying claim is true, current, complete, professionally reviewed or safe.
- Hashing is not anonymization. Do not publish raw hashes of real resident evidence, low-entropy preferences, exact room dimensions, photos, identifiable plans or short notes. An attacker may guess candidate contents and compare their hashes.
- The existing SHA-256 receipts are for committed *synthetic fixtures*. Do not reuse that public-input digest policy for real resident payloads.
- If a real project's integrity receipt is justified, keep it in the project's restricted boundary. Consider keyed HMAC or another reviewed commitment design only after a threat assessment; document key custody, rotation, access and recovery. A keyed digest is not proof of truth, consent or anonymity.
- Do not upload evidence payloads to a third-party hashing or analysis service just to produce a receipt.
- Retain independent reviewer/source metadata, not just a hash. Record how a claim was established and what scope the reviewer actually checked.

## 5. Threat model

Assume one or more of the following could occur:
- a developer accidentally commits a real annotated floor plan or resident narrative;
- a public CI log, generated receipt or test failure prints private notes or input bytes;
- a compromised/shared household device exposes a draft to another household member;
- a landlord, employer, insurer, vendor or coercive person requests more information than is needed;
- an authorized service provider downloads or reuses a plan for an unrelated purpose;
- a rare combination of preferences, household context and location re-identifies a pseudonymous resident;
- a breach reveals entrances, exits, alarm placement, support schedules or other security-relevant layout information;
- a key is lost or stolen, leaving the resident unable to recover their own records or allowing another party to read them;
- consent is withdrawn, but project copies or public artifacts persist.

Mitigations are purpose limitation, no default upload, local control, per-role access, separate identity mapping, deliberate export review, data minimization, encryption, backups/recovery, retention/deletion, redacted logs, and an incident-response process. No single mitigation eliminates all risks.

## 6. Consent, control and lifecycle

Before each collection or disclosure, provide accessible notice explaining:
- exactly what will be collected/disclosed;
- why it is needed and which decision it supports;
- who can see it, including processors/vendors;
- how long it is retained and where backups may persist;
- whether it might leave the device or jurisdiction;
- how to access, correct, export, withdraw consent, restrict further use and request deletion;
- what cannot be recalled after a deliberate public release;
- what happens if optional data are not supplied.

Consent must be appropriately granular for storage, professional review, research reuse, public release, images/recordings and third-party sharing. Do not bundle public publication into ordinary use of the planner. Do not treat withdrawal of research participation as permission to remove an essential service; handle previously collected data according to the disclosed protocol, ethics requirements and applicable law.

Use a lifecycle:
1. Collect the minimum information for a defined resident-selected goal.
2. Store locally or within an explicitly authorized restricted project store.
3. Share only selected fields with a named role for a recorded purpose.
4. Track copies/recipients and expiry to the extent technically possible.
5. Correct inaccurate claims and preserve a trace of material corrections without preserving unnecessary personal payloads.
6. At expiry/withdrawal, delete payloads and derived copies when required, propagate deletion requests to known recipients/backups where feasible, and document any legal/ethics retention exceptions.
7. Handle suspected compromise under a pre-agreed incident process led by the appropriate responsible party.

## 7. Implementation boundary for Symtropy

- Public repo and CI: P0 synthetic data only.
- Resident planner: no required network, account, analytics or third-party upload for core planning.
- Evidence references in public receipts: synthetic identifiers only.
- Reports: omit raw note fields by default; use role-scoped summaries.
- Exports: preview exactly what will be shared and support field-level redaction.
- Logs: do not log request bodies, raw fixtures from resident workspaces, paths, measurements or contact details; use coarse error categories and random request correlation IDs.
- Testing: include negative controls proving that the export/report layer cannot accidentally include sample identity, address, raw notes, or source payload; test deletion and export boundaries.
- Real-project evidence store, key management, authentication and cloud sync must not be shipped based on this document alone. They require specific implementation, threat modeling, security review, and legal/privacy review.

## 8. Proposed acceptance targets

- **Zero** requirement to upload resident evidence to a public repo or external service to use core planning.
- **Zero** real or identifying resident evidence in public fixtures, CI logs, issue comments, PR artifacts or synthetic receipts.
- **100%** of non-public evidence objects have a recorded purpose, control/access tier, source/scope, retention rule and deletion/withdrawal path, or are refused/kept local without export.
- **100%** of exports show a preview and can remove selected fields before sharing.
- **100%** of tested expired/withdrawn payloads follow the specified deletion policy across known storage and derived-copy locations, subject to clearly documented lawful/ethics exceptions.
- **Zero** publication of a digest of raw real resident/home evidence in a public receipt.
- **Zero** secondary use/publication without separate purpose/authority and appropriate resident notice/consent/review.
- **Zero** claims that hashing, encryption or pseudonymization alone proves anonymity, safety or evidence truth.
- **100%** of privacy incidents and export leaks in test fixtures trigger a documented incident-response path with no sensitive value in logs.

Targets concern implementation quality; real-world audits are needed to test them. Do not claim legal compliance based on passing these tests.

## 9. Public-repository hygiene

Before each commit/PR/fixture release:
- check staged and generated files for real addresses, identifiable names/contact details, geolocation, image metadata, detailed residential plans, original evidence payloads and unexpected attachments;
- make fixtures from deliberately invented examples, not redacted copies of field records;
- ensure test error messages quote field names/IDs, not raw input values;
- ensure receipts and snapshots omit payloads and location-derived identifiers;
- inspect screenshots and build artifacts for unintended details;
- review the exact export as the resident would see it.

A scanner can catch patterns but cannot establish that a floor plan or rare need combination is non-identifying. Human review remains necessary.

## 10. Research and governance gates

Before real deployment, update the relevant standards/law review, identify the responsible party and Information Officer as applicable, complete a context-specific privacy/security impact assessment, involve resident representatives, define incident responsibilities and retention schedules, and test access/correction/deletion rights. POPIA applicability depends on the actual processing and context; obtain qualified advice rather than treating this document as a compliance checklist.

The purpose is to keep residents in control and minimize the consequences of misuse—not to create an attractive privacy badge.
