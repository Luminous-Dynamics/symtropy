# SSP 2.0.1 schema provenance

The vendored XML schemas and license are copied verbatim from the official Modelica Association Project SSP repository.

- Repository: https://github.com/modelica/ssp-standard
- Release/tag: `v2.0.1`
- Root schema: `schema/SystemStructureDescription.xsd`
  - upstream blob SHA-1: `09e745c44953e501b38f4270bb0aa557c96a676e`
- Common schema: `schema/SystemStructureCommon.xsd`
  - upstream blob SHA-1: `7722decc8dec3e7869df043bee45a142fb8015a7`
- License: `LICENSE.txt`
  - upstream blob SHA-1: `1b7dd8982f8c663dfea01993ab1cfde5a2dee315`

The XML schemas are released under the BSD 2-Clause license. The SSP specification documentation is separately released under CC BY-SA 4.0; this repository vendors only the XSD source and the upstream combined license notice, not specification prose.

Do not replace these pinned schemas with mutable `main` branch files. Upgrades must update the tag and blob SHAs together and rerun the frozen package corpus.
