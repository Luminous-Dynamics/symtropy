# DNA Molecular Memory Channel — v0.1

**Programme:** MOLECULAR-MEMORY-CHANNEL-001  
**Tracking:** [Symtropy #7567](https://github.com/Luminous-Dynamics/symtropy/issues/7567)  
**Compiler:** [Symtropy #7482](https://github.com/Luminous-Dynamics/symtropy/issues/7482)  
**Physical systems:** [Symtropy #7447](https://github.com/Luminous-Dynamics/symtropy/issues/7447)

## Evidence basis

Öztürk et al., “DNA Data Storage: A Critical Review of Bio-Algorithmic Integration, Performance-Cost Tradeoffs, and Future Directions for Scalable Architectures,” published 3 June 2026, reviews synthesis, PCR/CRISPR access, sequence coding, biochemical noise and the integration of biological constraints with algorithmic optimization:
https://link.springer.com/article/10.1007/s11831-026-10651-2

The review identifies synthesis cost, sequencing errors, write/read efficiency and retrieval latency as scaling barriers. It describes substitution, insertion, deletion, oligo dropout, sequence constraints and amplification bias. These are platform-/protocol-dependent effects. The review's example error figures must not be treated as universal DNA-channel constants.

## Why DNA requires a distinct adapter

The physical channel is not simply a byte array with a bit-error rate. It is a sequence of biochemical and computational stages:

digital data → constrained nucleotide encoding → synthesis → storage/encapsulation → selective retrieval → amplification → sequencing → basecalling → alignment/clustering → synchronization-aware decoding → exact payload verification.

The adapter preserves raw reads and quality observations separately from aligned sequences and expected labels. Ground truth is held in a separately referenced corpus so the decoder/optimizer cannot use it at inference time.

## Error and uncertainty vocabulary

Keep distinct:
- **Substitution:** one nucleotide called as another.
- **Insertion/deletion:** sequence length changes and synchronization shifts.
- **Dropout:** an encoded oligo is not recovered in a read batch.
- **Abundance bias:** unequal copy/read abundance across oligos, including PCR amplification bias.
- **Duplicate reads:** repeated observations that are not independent molecules.
- **Contamination/cross-talk:** reads assigned to the wrong library or index.
- **Sequence constraint violation:** GC, homopolymer or motif rules not met.

A decoder must not count PCR duplicates as independent evidence without a validated model. A missing oligo is not the same mechanism as an indel within a recovered sequence.

## Baseline tournament

- Constrained sequence encoders that enforce declared GC/homopolymer/motif rules.
- Marker and synchronization codes for insertion/deletion recovery.
- VT/marker/indel-aware codes where their assumptions fit the measured channel.
- Reed–Solomon/BCH for substitutions after sequence alignment.
- Fountain/Luby Transform or other erasure coding for oligo dropout.
- LDPC/soft-decision alternatives where per-base quality scores have been calibrated.
- Primer/index and random-access methods evaluated for selection accuracy, cross-talk and retrieval overhead.

Compare the end-to-end chain, not isolated code rate. Measure net recoverable bits per nucleotide after sequence constraints, index, parity, dropout redundancy and manifest overhead.

## Required measurements

- Synthesis platform, chemistry, batch, oligo length distribution and accepted-oligo yield.
- Sequencing platform, chemistry, basecaller/version, raw reads and quality-score calibration.
- Substitution, insertion, deletion, dropout and abundance distributions by position and sequence context.
- PCR cycle conditions and amplification bias; duplicate-read clustering.
- Random-access primer selectivity, cross-talk, retrieval latency and coverage.
- Storage medium/encapsulation, environment, duration and recovery after ageing.
- Synthesis cost, sequencing cost, energy and accepted exact-payload throughput as separate terms.
- Exact payload hash match, unrecoverable fraction and held-out library performance.

## Data split and evidence gates

Partition by synthesis batch, oligo library, storage condition and sequencing run. Do not randomly split reads from the same oligo between training and test; this leaks molecule- and batch-specific signals.

Synthetic fixtures test parsing and decoder interfaces only. Published reproductions keep their original platform and conditions. Physical data requires raw bundles, specimen/library IDs, process lineage, instrument logs, calibration, sample sizes and uncertainty.

## Evidence boundary

The accompanying v0.1 schema and fixture are data contracts, not a DNA storage implementation. The synthetic reads are illustrative sequence edits, not measured error rates. No commercial DNA archive, net density, cost advantage, lifetime, read/write speed or physical reliability is claimed. The molecular medium remains research-only and must not be ranked as production-qualified by the compiler.


## 10. Updated 2026 coding and soft-decision baselines

The DNA coding landscape now has two direct 2026 benchmark anchors beyond the general review.

**Experimental codec tournament:** Gimpel et al., *Nature Communications* 17, 3963 (14 March 2026), compare six codecs in standardized in-silico and in-vitro scenarios. They test realistic read nonidealities with DT4DDS, read clustering and an in-vitro verification workflow. The study reports 43 EB/g for material-deposition synthesis and 13 EB/g for electrochemical synthesis; its realistic-workflow in-silico analysis reports feasibility up to 117 EB/g. The paper also reports isolated test scenarios tolerating up to 14% errors and up to 65% sequence loss. These are scenario-specific benchmark results, not general channel guarantees or commercial archive metrics.

The six codec references are DNA-Aeon, DNA Fountain, DNA-RS, Goldman, HEDGES and Yin–Yang. Source: https://doi.org/10.1038/s41467-026-70548-3. Publication-associated benchmark code and data are at https://github.com/fml-ethz/dt4dds-benchmark, archived code DOI 10.5281/zenodo.17391326, notebooks DOI 10.5281/zenodo.17391328 and ENA accession PRJEB90546.

**Soft-decision enhancement:** Ding et al., *Bioinformatics* 42(8), btag563 (3 August 2026), introduce Polus/SeqFormer. The system uses multi-read sequence context and Phred quality values to produce platform-specific per-base confidence scores, then tries progressively larger erasure sets after hard-decision/CRC failure. The paper reports, in its in-silico evaluation, a 38.9% reduction in DNA Fountain sequencing coverage for exact recovery, approximately 80% more effective physical density in that setup, and resolution of some YYC indel failures. It also describes targeted resequencing with lower read overhead.

Source: https://doi.org/10.1093/bioinformatics/btag563. Code: https://github.com/dinglulu/Polus (observed commit 656b58008e196dbad42c9b09f8ec1bf350ee8614). SeqFormer repository: https://github.com/dinglulu/SeqFormer; its commit must be pinned before a reproduction is accepted.

The Polus figures are in-silico results, not in-vitro demonstrations. The 2026 codec tournament's in-vitro density demonstrations are separate. Do not merge their evidence classes.

## 11. Consequence for our programme

Symtropy's biochemical channel should emit raw reads, quality values, molecule/read multiplicity, synthesis and sequencing lineage, and held-out partition identity. Symthaea should then:
1. reproduce the six-codec DT4DDS tournament;
2. reproduce the Polus/SeqFormer soft-decision pipeline, including the hard-decision ablation;
3. test read clustering and per-base probability calibration on held-out libraries and platforms;
4. identify sequencing-derived errors versus persistent synthesis errors before deciding whether resequencing can help;
5. compare uniform deeper sequencing with confidence-targeted adaptive resequencing;
6. accept a file only after byte-exact restoration and independent manifest verification.

A useful new target is to outperform Polus by at least 10% in reads or sequencing cost on an independently held-out library/platform cohort, with identical exact-file recovery, code constraints and no synthesis-cost regression. This is a challenge target only; no such advantage is currently established.

The software baselines can run in pinned, isolated containers. The trusted scoring/receipt layer should remain Rust-first and independent of model-provided labels. Polus/SeqFormer and legacy codec dependencies must not become production authority or leak into the physical-memory compiler's evidence gate.
