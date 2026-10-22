# Stone Recovery Benchmark Contract v0.1

**Status:** synthetic reference contract  
**Primary owner:** Symtropy  
**Cross-stack consumers:** Symthaea, Mycelix, Sol-Atlas  
**Evidence class:** synthetic fixtures only

## 1. Goal

Freeze an independently verifiable geometry-and-accounting kernel for quarry-to-product recovery before implementing a more complex optimizer. The first question is deliberately narrow:

> Given a rectangular synthetic rock-mass envelope, axis-aligned declared discontinuities, candidate product blocks, and declared non-product material streams, does the candidate avoid explicit geometric conflicts and reconcile material volume without double counting?

This is a foundation, not the final quarry model. It does not model real geological uncertainty, stress, cutting forces, machine kinematics, water or energy behavior, market quality, or safe extraction procedures.

## 2. Research basis and design implication

- [Environmentally sustainable mining in quarries to reduce waste production and loss of resources using the developed optimization algorithm](https://www.nature.com/articles/s41598-023-49633-w) treats geological discontinuities, cutting surfaces, water, energy, consumables and waste as coupled optimization variables. Its case-study results are not universal defaults.
- [Discontinuity modelling and rock block geometry identification to optimize production in dimension stone quarries](https://www.sciencedirect.com/science/article/pii/S0013795217304234) reports the importance of block geometry, joint networks, orientation and cut spacing to recovery.
- [NIST Digital Twins for Advanced Manufacturing](https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing) identifies verification, validation and uncertainty quantification as central needs for reliable manufacturing digital twins.
- NIST's 2026 [Digital Twins Workshops Summary Report](https://www.nist.gov/publications/digital-twins-workshops-summary-report) identifies interoperability, VVUQ, cybersecurity and workforce readiness as persistent challenges.

These sources motivate our modeling boundaries; they do not validate this implementation or provide expected values for our synthetic fixtures.

## 3. Paradigm shift

Do not optimize tonnes extracted or count nominal blocks in isolation. Build toward a staged, evidence-bearing chain:

```
geological observations + uncertainty
  -> candidate cut geometry
  -> block geometry + quality evidence
  -> slabs / blanks / nesting
  -> finished conforming parts
  -> recoverable offcuts + residue treatment
```

The long-term objective is useful conforming output under hard geometry, quality, utility, ecological, safety, custody and legal constraints. v0.1 implements only a small subset of geometric and accounting invariants.

## 4. v0.1 semantics

- All dimensions are integer millimetres; volumes are integer cubic millimetres.
- The source envelope is a synthetic axis-aligned cuboid.
- A discontinuity is an axis-aligned plane with an explicit crossing policy: forbidden or permitted.
- Candidate blocks are axis-aligned boxes with explicit origins and dimensions.
- Strict interior overlap is a conflict. Sharing a face or touching a fracture plane at an exact block boundary is not crossing/overlap.
- A block may not extend outside the source envelope.
- A load-bearing intended use requires qualified quality evidence; synthetic, unknown, estimated and observed states do not satisfy that gate.
- Product block volumes plus recoverable offcuts, kerf loss and fines/unclassified volume must equal source volume within the declared parts-per-million tolerance. The frozen corpus currently uses zero tolerance.
- Exact JSON fields are required; unknown fields are rejected by the typed deserializer.
- Every scenario has a frozen expected disposition and exact expected reason codes. The validator must reproduce negative controls; intentional rejection is not a suite failure when it matches the expected result.
- Identical corpus bytes and source revision produce a deterministic receipt including SHA-256 hashes.

## 5. Frozen scenarios

1. **Clean face:** two disjoint blocks and explicit kerf close exactly.
2. **Forbidden joint:** a candidate spans a plane declared non-crossable.
3. **Overlapping products:** two boxes overlap and their accounted volume exceeds the source.
4. **Unknown load-bearing quality:** geometry and accounting close, but required application evidence is unresolved.

Additional adversarial cases should be added only with explicit known-answer outputs: invalid/missing units, bounds overflow, duplicate IDs, uncertain/missing geological coverage, permitted vs forbidden discontinuities, alternative orientations, thin recoverable offcuts, and partial/contradictory measurements.

## 6. Benchmark ladder

### Stage A — geometry and accounting contract
- deterministic replay;
- exact known-answer negative controls;
- schema/unit/bounds and mass-balance rejection;
- receipt binds corpus digest, source commit and validator version.

### Stage B — geometry-aware candidate generation
Add cutting-grid orientation, joint-network relationships, minimum commercial dimensions, kerf, tool approach limits, finishing allowances and a baseline-versus-optimizer comparison. Compare on identical frozen synthetic scenes.

### Stage C — finished-part nesting and recovery
Given an explicit demand mix, pack candidate slabs/blanks into required part geometries. Report conforming volume by product class, cutting surface, kerf, offcuts, fines/slurry, water, energy, tooling and cycle time independently where each is modeled.

### Stage D — measured data and model credibility
Admit measured geological/block datasets only after source, license, measurement method, coordinate frame, sampling coverage, uncertainty and independent reference truth are established. Split holdout by source/site/quarry, not merely random blocks from the same scene. Quantify prediction errors and calibration; don't convert missing geology into intact rock.

### Stage E — bounded physical pilot
Only with authorized operations, qualified equipment and procedures, suitable inspection/metrology, environmental management, and independently reviewed safety controls. The simulator must not command quarry machinery.

## 7. Benchmark metrics — do not collapse into one score

- geometric feasibility and fracture-conflict rate;
- block dimensional and quality-class yield;
- conforming finished volume by product class;
- volume reconciliation error and double-counted/omitted material;
- cutting surface or cut length, kerf and finishing losses;
- reusable offcut and residue disposition;
- energy, water, tool consumption and throughput where supported by measurement/model evidence;
- p50/p95 runtime, candidate evaluations and timeout rate;
- calibration/uncertainty coverage on representative holdouts;
- fully burdened cost per conforming unit only with declared and auditable cost inputs.

Hard constraints are filters, not tradeable penalties. A larger output number cannot offset a prohibited fracture crossing or unresolved required quality evidence.

## 8. Required non-claims

A synthetic PASS proves only that the validator reproduces the declared synthetic contract. It does not establish geological truth, reserves, recoverability, optimality, quarry safety, regulatory compliance, permitted extraction, environmental impact, material strength, economic savings, or suitability of any stone for a structure.

## 9. Stack ownership

- **Symtropy:** geometry, candidate generation, deterministic simulation and benchmark replay.
- **Symthaea:** bounded hypotheses, uncertainty analysis and candidate explanations; no invented geology or qualification.
- **Mycelix:** lot, custody and transformation events using existing provenance owners; these events are not stone-grade certificates.
- **Sol-Atlas:** read-only site, transport, utilities and source metadata; mapped presence is not reserve, permit or facility-capability evidence.
- **Independent verifier:** schema, geometry, accounting, negative-control, and receipt verification.

Compose existing units, engineering-material, manufacturing, lifecycle and provenance owners. Do not add another generic scheduler, material ontology, custody ledger, or permission system.
