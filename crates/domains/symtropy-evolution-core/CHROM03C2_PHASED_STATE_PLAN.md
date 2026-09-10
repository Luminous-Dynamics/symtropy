# CHROM-03C2 phased-state successor boundary

Status: design note only; no C2 runtime authority is implemented by CHROM-03C1.

C2 should add an optional phased hereditary sidecar rather than changing the meaning of `HereditaryState`.

The sidecar should bind simultaneously to:

- exact `HereditarySchemaDigest`;
- exact `ChromosomeMapDigest`;
- one explicit chromosome-copy count compatible with the declared C2 model;
- for every chromosome copy, one allele at every mapped locus in chromosome order.

C2 must preserve these distinctions:

- phased state != unphased `HereditaryState`;
- homolog/copy index != persistent chromosome lineage identity;
- chromosome phase != parental origin unless provenance separately establishes it;
- chromosome map != meiosis/crossover authority;
- exact phased individual state != aggregate population haplotype-frequency authority.

The first reference model may be explicitly diploid if it fails closed for other ploidies. Arbitrary polyploid meiosis remains a later model.

A C2 phased state should be able to project losslessly to the existing unphased hereditary state by forgetting phase. The reverse projection is one-to-many and must never be invented automatically.

C3 may then consume C2 state plus an explicit recombination operator to create gametes and interval inheritance provenance for PHYLO-04.
