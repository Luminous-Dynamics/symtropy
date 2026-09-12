# FEP-06 follow-up — canonical relapse threshold binding

FEP-06 currently uses a local `0.8` relapse-alert threshold. The psychology domain already exposes `BURNOUT_THRESHOLD = 0.8` as the canonical burnout boundary and enforces related threshold ordering at compile time.

Before FEP-06 is considered merge-ready, the relapse transition should bind directly to `crate::systems::psychology::BURNOUT_THRESHOLD` rather than retaining an independently editable literal. The values are equal today; this follow-up exists to prevent semantic drift later.

No behavioral change is claimed by this document, and no runtime qualification is implied.
