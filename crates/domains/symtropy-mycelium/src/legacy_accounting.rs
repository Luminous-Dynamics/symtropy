// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Diagnostic accounting for the legacy field-only Mycelium V0 step.
//!
//! Values are model units, not physical mass or concentration. This report
//! exposes the current heuristic bookkeeping without reinterpreting or changing
//! its legacy behavior. It is not a conservation ledger or biological model.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegacyMyceliumStepReport {
    pub tick_before: u64,
    pub tick_after: u64,

    pub biomass_before: f32,
    pub biomass_before_decay: f32,
    pub biomass_after_decay: f32,

    pub nutrient_before: f32,
    pub nutrient_after_diffusion_decay: f32,
    pub nutrient_after_step: f32,

    pub toxin_before: f32,
    pub toxin_after_diffusion_decay: f32,

    /// Nutrient field debit used by the legacy local assimilation rule.
    pub assimilation_nutrient_debit: f32,
    /// Matching biomass field credit applied at the assimilation cell.
    pub assimilation_local_biomass_credit: f32,
    /// Additional origin biomass credit: 25% of the same assimilation total.
    /// The legacy step does not subtract this from the source biomass field.
    pub origin_biomass_extra_credit: f32,

    /// Growth biomass added at the selected target cells.
    pub growth_target_biomass_credit: f32,
    /// Growth biomass debited at source cells by the legacy 5% heuristic.
    pub growth_source_biomass_debit: f32,
    /// Nutrient debit used by the legacy growth heuristic (35% of growth).
    pub growth_nutrient_debit: f32,

    /// Observed difference from the biomass field's final decay step.
    /// This is not a typed transfer into a detrital or nutrient pool.
    pub observed_biomass_decay_loss: f32,
}

impl LegacyMyceliumStepReport {
    pub fn origin_credit_fraction_of_assimilation(&self) -> f32 {
        if self.assimilation_local_biomass_credit <= 0.0 {
            0.0
        } else {
            self.origin_biomass_extra_credit / self.assimilation_local_biomass_credit
        }
    }

    pub fn untyped_biomass_loss(&self) -> f32 {
        self.observed_biomass_decay_loss.max(0.0)
    }
}
