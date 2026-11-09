# Construction layout robotics — vendor baseline and fair-comparison matrix

**Related:** [Issue #3523](https://github.com/Luminous-Dynamics/symtropy/issues/3523), [PR #3524](https://github.com/Luminous-Dynamics/symtropy/pull/3524)  
**Evidence grade:** vendor-published specifications and self-reported results; not independent comparative performance or safety evidence.

## 1. Market segmentation: do not compare unlike tasks

| Product / class | Vendor-published capability | Intended task envelope | Comparability warning |
|---|---|---|---|
| HP SitePrint (interior slab layout) | ±2 mm layout tolerance and average ±0.8 mm floor-level marking under stated high-accuracy configuration; Value Pack 3.0 describes depth-camera obstacle mapping, route adjustments and shadowing | Interior BIM/CAD floor layout, floor deviation marking | Accuracy depends on a 1-inch total station, SMR prism, High Accuracy Print Mode and 5–30 m setup. Its "up to 30%" faster-point-printing claim compares against its previous software release, not a manual layout baseline; the official product-page footnote reports 8% for interior-wall layouts. |
| Dusty FieldPrinter 2 (interior slab layout) | ±1.6 mm print accuracy with accurate surveyed control; 0.75 m/s solid-line print, 0.50 m/s text/point print, 1.0 m/s navigation; three forward ToF sensors (~0.9 m) and four cliff sensors | Interior slab BIM/VDC layout with dual printheads, text/points/arcs and floor-elevation measurement | Accuracy is conditioned on surveyed control and Leica laser-tracker setup. IP51-equivalent, not rain-safe; cliff sensing has a stated geometry/threshold limitation. |
| CivDot (GNSS point staking) | Up to 8 points/min and up to 30 mm marking accuracy; vendor FAQ states up to 3,000 points/day and up to 8x faster than manual methods | Large-area earthworks, solar, grading and land-survey point marking | A GNSS staking rover, not an indoor millimetre-level slab printer. Productivity is a vendor claim; accuracy depends on RTK/control and site conditions. |
| CivDot+ (laser-assisted point staking) | Up to 3 points/min and 8 mm accuracy | Piling, drilling and foundations requiring higher precision than standard GNSS staking | The accuracy/throughput tradeoff differs from CivDot; not directly comparable with continuous line printing. |
| CivDot Mini (linear marking) | Up to 60 m/min of solid/dashed lines and 25 mm accuracy | Road/highway/airport/parking marking | A road-marking line system; compare on linear metres accepted, not BIM interior points or slab layout. |

Primary vendor sources:
- HP SitePrint: https://www.hp.com/us-en/printers/site-print/layout-robot.html
- HP Value Pack 3.0 datasheet: https://h20195.www2.hp.com/v2/GetDocument.aspx?docname=c09062116
- Dusty FieldPrinter 2 specifications (updated 2026-09-21): https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs
- Civ Robotics layout robots: https://www.civrobotics.com/robots/layout-robots
- CivDot: https://www.civrobotics.com/robots/layout-robots/civdot
- Civ Robotics FAQ: https://www.civrobotics.com/faq

## 2. Benchmark tracks

**Track A — vendor-configuration reproduction.** Reproduce each vendor's supported configuration, calibration, control-network and operating conditions. This answers: “Does the benchmark reproduce the published product specification under its declared conditions?”

**Track B — common independent referee.** Run all eligible systems against frozen layouts and independently surveyed targets using the same external reference metrology, acceptance tolerances, environmental strata and blind scoring. This answers: “How do systems compare on the same task and reference?”

Never mix A and B results. A vendor configuration may require different control hardware or setup; preserve that rather than silently standardizing away product constraints. Report every hardware/software version.

## 3. Common scorecard — separate outcome dimensions

### Geometry and quality
- Signed X/Y/Z error and radial/orthogonal error; bias, repeatability and drift.
- p50/p95/p99 and maximum absolute error with uncertainty and reference-system contribution.
- Point, line, arc, text, edge and obstacle-adjacent feature classes.
- Critical omissions, false marks, duplicated marks, wrong-layer marks and accepted rework.
- Floor-elevation error and spatial resolution reported separately from print-position error.

### Productivity and cost
- Accepted output per robot-hour and crew-hour, by feature type and density.
- Setup/stationing time, control-point survey time, battery/changeover, rework and recovery.
- Total labour and equipment cost per accepted feature or accepted square metre.
- Matched manual baseline and vendor configuration under same design revision, site phase, ODD, acceptance rules and functional unit.
- Do not use vendor “up to” productivity or prior-version uplift as an independent matched baseline.

### Reliability and dynamic-site operation
- Task completion / accepted-job rate; interruptions and safe holds per robot-hour.
- Obstacle/edge detection misses, false stops, recovery and requalification time.
- Performance under changed work-zone geometry, trade overlap, route closures, occlusion, dust, lighting and communication loss.
- Stale/conflicting context command leakage and unqualified restart are hard safety gates, never productivity tradeoffs.

### Evidence quality
- As-built machine/configuration, firmware and software identity.
- Independent reference instrument, calibration interval, uncertainty budget and time synchronization.
- Raw data custody, blind challenge administration, site/configuration holdouts and claim-specific adjudication.
- Independent replication and per-stratum denominators.

## 4. What “better than SOTA” should mean

Do not claim victory by beating a single vendor accuracy number. Establish four independent achievements:

1. **Comparable geometry:** improve on the best eligible vendor baseline on matched feature classes and the same referee network, with p95/p99 and uncertainty—not a rounded tolerance alone.
2. **Better accepted productivity:** increase accepted output per crew-hour after including setup, control-point survey, reprints, recovery, battery swaps and rework.
3. **Dynamic-ODD reliability:** prove that changed site context invalidates prior permission, stale/conflicting context cannot issue commands, and restart requires requalification; benchmark p99/max response against the task-specific stopping envelope.
4. **Evidence portability:** independently reproduce the result at a held-out site and configuration with the same receipt, metrology and adjudication rules.

The current proposed geometry targets in the benchmark (p95 plan-position residual ≤1.0 mm; p99 ≤2.0 mm) are prospective challenge hypotheses, not achieved results. They are not directly comparable with vendor ± tolerances until the measurement definition, task, control network and uncertainty are matched. Preserve these as candidate target thresholds, not pass claims.

## 5. Standards and scope

- **AMRA-201:2026** provides general mobile-robot requirements and test methods across wheeled, legged, tracked and other mobile robots operating on solid travel surfaces; it is a useful measurement-method comparator, not proof of construction-site safety qualification.
  https://amr-alliance.org/amra-201/
- **AMRA-220:2022** covers safety requirements, verification and validation for wheeled mobile robots in indoor industrial/service environments; outdoor applications are outside its stated scope (its principles may still be considered).
  https://www.amr-alliance.org/amra-220/
- **ISO 10218-1/-2** address industrial robots and system integration; applicability depends on whether the system fits the standard's industrial-robot scope. OSHA's overview notes that ISO 10218 does not apply to non-industrial robots, although its safety principles may be used.
  https://www.osha.gov/robotics/standards

Record which standards are applicable, partially applicable, used as guidance or out of scope. Do not infer certification or compliance from a benchmark pass.

## 6. Non-negotiable reporting rule

Report all vendors and manual baselines with identical denominators, exact configuration, declared ODD, exclusions, uncertainties and source citations. Safety and evidence validity are non-compensable. A system with better throughput but an unsafe command, missed critical edge or invalid measurement cannot “win” on an aggregate score.


## 7. Published field evidence and SOTA challenge

Kam et al., *Comparison of dusty robotics and traditional layout methods* (Construction Robotics, 15 September 2025), studied one 8-floor medical office building. The paper reports 68% fewer total labour hours, 18% fewer days, 99% less physically straining work and rework estimates of 6.42% versus 0.25%. It also reports 393 robot print-hours and 204 additional VDC coordination/file-preparation hours for the robot workflow. The traditional-method impacts were estimated rather than directly observed; the study is not a randomized or multi-vendor matched trial.

Source: https://link.springer.com/article/10.1007/s41693-025-00163-z

Our prospective SOTA challenge hypotheses are at least 70% fewer total workflow crew-hours, 20% shorter elapsed layout duration and 90% fewer rework crew-hours than a directly measured, matched manual baseline. These targets deliberately challenge the case-study's 68%/18% reported figures under a stronger method, not a claim that the current system has achieved them. Count design coordination, VDC/model prep, file conversion, control survey, setup, operator/spotter, robot operation, inspection, interruptions, recovery and rework. Do not infer direct manual-baseline productivity from HP's software-version productivity claims or from the 2025 single-case study.

For confirmatory comparison, predeclare the independent experimental unit (matched zone/run), sample-size rationale from pilot variance, acceptance criteria and analysis before unblinding. Repeated points/vertices in one work zone are measurements within a run, not independent experimental units. Preserve failed setups, invalid/indeterminate results, blocked work and all attempted runs.
