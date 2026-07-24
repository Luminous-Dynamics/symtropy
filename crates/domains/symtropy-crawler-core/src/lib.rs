// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! CLV-7 Wayhouse physical budgets, subsystem graph, cargo, and route qualification.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_game_state::StableId;

/// Canonical CLV-7 platform identity.
pub const CLV7_PROFILE_ID: &str = "continuance-crawler.clv-7-wayhouse";

/// Reference platform values transcribed from the v3.1 physical envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlerProfile {
    /// Overall road-stowed length.
    pub length_mm: u32,
    /// Overall road-stowed width.
    pub width_stowed_mm: u32,
    /// Overall road-stowed height.
    pub height_stowed_mm: u32,
    /// Width with radiator bloom deployed.
    pub radiator_deployed_width_mm: u32,
    /// Nominal ground contact area.
    pub ground_contact_area_cm2: u32,
    /// Reference gross mass.
    pub reference_gross_mass_kg: u32,
    /// Maximum permitted field mass.
    pub maximum_field_mass_kg: u32,
    /// Installed mass before mission cargo and occupant effects.
    pub installed_fixed_mass_kg: u32,
    /// Reference mission cargo.
    pub reference_mission_cargo_kg: u32,
    /// Reference occupants and personal effects.
    pub reference_occupant_effects_kg: u32,
    /// Continuous source power.
    pub continuous_power_supply_w: u32,
    /// Reference normal demand.
    pub normal_power_demand_w: u32,
    /// Usable battery energy.
    pub usable_battery_energy_j: u64,
    /// Total tracked water inventory.
    pub reference_water_kg: u32,
    /// Sustained heat rejection in the reference environment.
    pub sustained_heat_rejection_w: u32,
    /// Nominal resident count.
    pub nominal_residents: u16,
    /// Evacuation seated positions.
    pub evacuation_positions: u16,
}

impl CrawlerProfile {
    /// Returns the canonical CLV-7 Wayhouse reference platform.
    pub const fn clv7_wayhouse() -> Self {
        Self {
            length_mm: 34_800,
            width_stowed_mm: 7_800,
            height_stowed_mm: 6_400,
            radiator_deployed_width_mm: 15_800,
            ground_contact_area_cm2: 368_000,
            reference_gross_mass_kg: 320_000,
            maximum_field_mass_kg: 346_000,
            installed_fixed_mass_kg: 294_000,
            reference_mission_cargo_kg: 18_000,
            reference_occupant_effects_kg: 8_000,
            continuous_power_supply_w: 2_800_000,
            normal_power_demand_w: 2_520_000,
            usable_battery_energy_j: 20_160_000_000,
            reference_water_kg: 25_000,
            sustained_heat_rejection_w: 1_100_000,
            nominal_residents: 24,
            evacuation_positions: 48,
        }
    }

    /// Checks the published reference mass and normal power closures.
    pub fn validate(&self) -> Result<(), CrawlerError> {
        let closed_mass = self
            .installed_fixed_mass_kg
            .saturating_add(self.reference_mission_cargo_kg)
            .saturating_add(self.reference_occupant_effects_kg);
        if closed_mass != self.reference_gross_mass_kg {
            return Err(CrawlerError::ReferenceMassDoesNotClose {
                expected: self.reference_gross_mass_kg,
                actual: closed_mass,
            });
        }
        if self.normal_power_demand_w > self.continuous_power_supply_w {
            return Err(CrawlerError::ReferencePowerDoesNotClose {
                supply_w: self.continuous_power_supply_w,
                demand_w: self.normal_power_demand_w,
            });
        }
        Ok(())
    }
}

/// Major Crawler subsystem represented in the dependency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SubsystemKind {
    /// Front-left and front-right traction pods.
    MobilityFront,
    /// Rear-left and rear-right traction pods.
    MobilityRear,
    /// Generator set A.
    GeneratorA,
    /// Generator set B.
    GeneratorB,
    /// Battery modules and power electronics.
    Battery,
    /// Primary thermal loop.
    ThermalLoopA,
    /// Redundant thermal loop.
    ThermalLoopB,
    /// Water storage and treatment.
    Water,
    /// Atmosphere, habitation, and sanitation.
    Habitat,
    /// Clinic and protected medical loads.
    Clinic,
    /// Workshop and fabrication loads.
    Workshop,
    /// Sensors, communications, and compute.
    Communications,
    /// Articulation between sections A and B.
    ArticulationAB,
    /// Articulation between sections B and C.
    ArticulationBC,
}

/// Degradable subsystem state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubsystemState {
    /// Health from 0 to 10,000.
    pub health: u16,
    /// Present electrical or mechanical load.
    pub load_w: u32,
    /// Temperature in tenths of a degree Celsius.
    pub temperature_deci_c: i16,
    /// Whether the subsystem is intentionally isolated.
    pub isolated: bool,
    /// Whether it can currently perform its primary function.
    pub available: bool,
}

/// Physical mission cargo with custody and restraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoItem {
    /// Stable cargo identity.
    pub id: StableId,
    /// Mass.
    pub mass_kg: u32,
    /// Bounding volume for loading-path checks.
    pub volume_l: u32,
    /// Longitudinal centre position from the front datum.
    pub longitudinal_mm: i32,
    /// Lateral centre position from vehicle centreline.
    pub lateral_mm: i32,
    /// Vertical centre position above reference ground.
    pub vertical_mm: i32,
    /// Rated restraint capacity.
    pub restraint_rating_n: u32,
    /// Whether all required restraints are connected and inspected.
    pub secured: bool,
    /// Responsible person, household, or institution.
    pub custodian_id: StableId,
    /// Required access order such as immediate, en-route, or destination.
    pub access_priority: String,
}

/// Aggregate centre of mass in millimetres from the platform datum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CentreOfMass {
    /// Longitudinal coordinate.
    pub longitudinal_mm: i32,
    /// Lateral coordinate.
    pub lateral_mm: i32,
    /// Vertical coordinate.
    pub vertical_mm: i32,
}

/// Operational state with explicit debt and failure meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrawlerOperatingState {
    /// Full reference redundancy and margin.
    Normal,
    /// Reduced redundancy, poor route, overload below field maximum, or limited cooling.
    Stressed,
    /// Time-bounded operation spending life-safety reserve.
    Emergency,
    /// Primary mission unavailable while hazards remain contained.
    FailedSafe,
    /// Fire, instability, toxic release, flood ingress, or uncontrolled motion.
    FailedDangerous,
    /// Field repair cannot restore a safe load path or required service.
    FieldIrrecoverable,
}

/// Route segment used for physical qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteSegment {
    /// Stable route identity.
    pub id: StableId,
    /// Clear width.
    pub clear_width_mm: u32,
    /// Clear height.
    pub clear_height_mm: u32,
    /// Declared bearing capacity.
    pub bearing_capacity_pa: u32,
    /// Cross slope in milliradians.
    pub cross_slope_millirad: i32,
    /// Longitudinal grade in milliradians.
    pub grade_millirad: i32,
    /// Confidence in the route record from 0 to 10,000.
    pub confidence: u16,
}

/// Result of evaluating a Crawler against a route segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteClass {
    /// Reference margins are available.
    Green,
    /// Reduced margin or bounded operating restriction.
    Amber,
    /// Crossing needs load reduction, engineering work, or an explicit emergency procedure.
    Red,
    /// Physical clearance or bearing limits prohibit traversal.
    Black,
}

/// Explainable route qualification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteQualification {
    /// Resulting route class.
    pub class: RouteClass,
    /// Calculated nominal ground pressure.
    pub ground_pressure_pa: u32,
    /// Player- and operator-readable reasons.
    pub reasons: Vec<String>,
}

/// Persistent CLV-7 state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlerState {
    /// Platform identity.
    pub id: StableId,
    /// Reference profile.
    pub profile: CrawlerProfile,
    /// Actual occupant count.
    pub occupants: u16,
    /// Actual occupants and personal-effects mass.
    pub occupant_effects_kg: u32,
    /// Water inventory.
    pub water_kg: u32,
    /// Fuel inventory in litres.
    pub fuel_l: u32,
    /// Usable battery energy currently stored.
    pub battery_energy_j: u64,
    /// Mission cargo by identity.
    pub cargo: BTreeMap<StableId, CargoItem>,
    /// Major subsystem states.
    pub subsystems: BTreeMap<SubsystemKind, SubsystemState>,
    /// Current operating classification.
    pub operating_state: CrawlerOperatingState,
}

impl CrawlerState {
    /// Creates the reference departure configuration before cargo-specific loading.
    pub fn reference(seed: u64) -> Self {
        let profile = CrawlerProfile::clv7_wayhouse();
        Self {
            id: StableId::derive("crawler", seed, 0),
            profile: profile.clone(),
            occupants: profile.nominal_residents,
            occupant_effects_kg: profile.reference_occupant_effects_kg,
            water_kg: profile.reference_water_kg,
            fuel_l: 18_000,
            battery_energy_j: profile.usable_battery_energy_j,
            cargo: BTreeMap::new(),
            subsystems: reference_subsystems(),
            operating_state: CrawlerOperatingState::Normal,
        }
    }

    /// Returns actual gross mass from fixed installation, occupants/effects, and cargo.
    pub fn gross_mass_kg(&self) -> u32 {
        self.profile
            .installed_fixed_mass_kg
            .saturating_add(self.occupant_effects_kg)
            .saturating_add(self.cargo.values().map(|cargo| cargo.mass_kg).sum::<u32>())
    }

    /// Loads one physical cargo item if field mass and geometry remain representable.
    pub fn load_cargo(&mut self, cargo: CargoItem) -> Result<(), CrawlerError> {
        let proposed_mass = self.gross_mass_kg().saturating_add(cargo.mass_kg);
        if proposed_mass > self.profile.maximum_field_mass_kg {
            return Err(CrawlerError::FieldMassExceeded {
                proposed_kg: proposed_mass,
                maximum_kg: self.profile.maximum_field_mass_kg,
            });
        }
        if cargo.longitudinal_mm < 0
            || cargo.longitudinal_mm > i32::try_from(self.profile.length_mm).unwrap_or(i32::MAX)
            || cargo.lateral_mm.unsigned_abs() > self.profile.width_stowed_mm / 2
            || cargo.vertical_mm < 0
            || cargo.vertical_mm > i32::try_from(self.profile.height_stowed_mm).unwrap_or(i32::MAX)
        {
            return Err(CrawlerError::CargoOutsideEnvelope(cargo.id));
        }
        self.cargo.insert(cargo.id.clone(), cargo);
        self.refresh_operating_state();
        Ok(())
    }

    /// Marks cargo restraints inspected and connected.
    pub fn secure_cargo(&mut self, cargo_id: &StableId) -> Result<(), CrawlerError> {
        let cargo = self
            .cargo
            .get_mut(cargo_id)
            .ok_or_else(|| CrawlerError::UnknownCargo(cargo_id.clone()))?;
        cargo.secured = true;
        Ok(())
    }

    /// Returns cargo that blocks safe departure.
    pub fn unsecured_cargo(&self) -> Vec<&CargoItem> {
        self.cargo.values().filter(|cargo| !cargo.secured).collect()
    }

    /// Computes aggregate centre of mass from fixed platform, occupants, and cargo.
    pub fn centre_of_mass(&self) -> CentreOfMass {
        let base_mass = i64::from(self.profile.installed_fixed_mass_kg);
        let occupant_mass = i64::from(self.occupant_effects_kg);
        let mut total_mass = base_mass + occupant_mass;
        let mut longitudinal_moment = base_mass * 17_400 + occupant_mass * 15_800;
        let mut lateral_moment = 0i64;
        let mut vertical_moment = base_mass * 2_500 + occupant_mass * 2_700;
        for cargo in self.cargo.values() {
            let mass = i64::from(cargo.mass_kg);
            total_mass += mass;
            longitudinal_moment += mass * i64::from(cargo.longitudinal_mm);
            lateral_moment += mass * i64::from(cargo.lateral_mm);
            vertical_moment += mass * i64::from(cargo.vertical_mm);
        }
        CentreOfMass {
            longitudinal_mm: i32::try_from(longitudinal_moment / total_mass).unwrap_or(i32::MAX),
            lateral_mm: i32::try_from(lateral_moment / total_mass).unwrap_or(i32::MAX),
            vertical_mm: i32::try_from(vertical_moment / total_mass).unwrap_or(i32::MAX),
        }
    }

    /// Applies health damage and updates mission state.
    pub fn damage_subsystem(&mut self, subsystem: SubsystemKind, damage: u16) {
        if let Some(state) = self.subsystems.get_mut(&subsystem) {
            state.health = state.health.saturating_sub(damage);
            state.available = state.health >= 1_500 && !state.isolated;
        }
        self.refresh_operating_state();
    }

    /// Qualifies a route using clearance, bearing, slope, load, and record confidence.
    pub fn qualify_route(&self, route: &RouteSegment) -> RouteQualification {
        let pressure =
            nominal_ground_pressure_pa(self.gross_mass_kg(), self.profile.ground_contact_area_cm2);
        let mut class = RouteClass::Green;
        let mut reasons = Vec::new();
        if route.clear_width_mm < self.profile.width_stowed_mm
            || route.clear_height_mm < self.profile.height_stowed_mm
        {
            class = RouteClass::Black;
            reasons
                .push("physical clearance is smaller than the road-stowed vehicle envelope".into());
        }
        if route.bearing_capacity_pa < pressure {
            class = RouteClass::Black;
            reasons.push(format!(
                "nominal ground pressure {pressure} Pa exceeds bearing capacity {} Pa",
                route.bearing_capacity_pa
            ));
        } else if route.bearing_capacity_pa < pressure.saturating_mul(13) / 10 {
            class = class.max(RouteClass::Red);
            reasons.push("bearing margin is below the normal route policy".into());
        }
        let cross_slope = route.cross_slope_millirad.unsigned_abs();
        if cross_slope > 140 {
            class = RouteClass::Black;
            reasons.push("cross slope exceeds the emergency platform envelope".into());
        } else if cross_slope > 80 {
            class = class.max(RouteClass::Red);
            reasons
                .push("cross slope requires engineering controls and reduced dynamic load".into());
        } else if cross_slope > 55 {
            class = class.max(RouteClass::Amber);
            reasons.push("cross slope reduces lateral-load margin".into());
        }
        if route.grade_millirad.unsigned_abs() > 180 {
            class = RouteClass::Black;
            reasons.push("grade exceeds the field route envelope".into());
        } else if route.grade_millirad.unsigned_abs() > 100 {
            class = class.max(RouteClass::Red);
            reasons.push("grade requires low-speed stressed operation".into());
        }
        if route.confidence < 5_000 {
            class = class.max(RouteClass::Red);
            reasons.push("route record confidence is below 0.5 and needs local inspection".into());
        } else if route.confidence < 7_500 {
            class = class.max(RouteClass::Amber);
            reasons.push("route condition is uncertain".into());
        }
        let centre = self.centre_of_mass();
        if centre.lateral_mm.unsigned_abs() > 350 || centre.vertical_mm > 2_850 {
            class = class.max(RouteClass::Red);
            reasons.push("cargo centre of mass reduces rollover margin".into());
        }
        if !self.unsecured_cargo().is_empty() {
            class = RouteClass::Black;
            reasons.push("unsecured cargo prohibits movement".into());
        }
        if reasons.is_empty() {
            reasons.push(
                "reference clearances, bearing, slope, and load margins are available".into(),
            );
        }
        RouteQualification {
            class,
            ground_pressure_pa: pressure,
            reasons,
        }
    }

    fn refresh_operating_state(&mut self) {
        let available_generators = [SubsystemKind::GeneratorA, SubsystemKind::GeneratorB]
            .into_iter()
            .filter(|kind| {
                self.subsystems
                    .get(kind)
                    .is_some_and(|state| state.available)
            })
            .count();
        let available_mobility = [SubsystemKind::MobilityFront, SubsystemKind::MobilityRear]
            .into_iter()
            .filter(|kind| {
                self.subsystems
                    .get(kind)
                    .is_some_and(|state| state.available)
            })
            .count();
        let dangerous = self
            .subsystems
            .values()
            .any(|state| state.temperature_deci_c > 950 || (state.health < 700 && !state.isolated));
        let irrecoverable = [SubsystemKind::ArticulationAB, SubsystemKind::ArticulationBC]
            .into_iter()
            .any(|kind| {
                self.subsystems
                    .get(&kind)
                    .is_some_and(|state| state.health < 300)
            });
        self.operating_state = if dangerous {
            CrawlerOperatingState::FailedDangerous
        } else if irrecoverable {
            CrawlerOperatingState::FieldIrrecoverable
        } else if available_mobility == 0 || available_generators == 0 {
            CrawlerOperatingState::FailedSafe
        } else if self.gross_mass_kg() > self.profile.reference_gross_mass_kg
            || available_mobility == 1
            || available_generators == 1
        {
            CrawlerOperatingState::Stressed
        } else {
            CrawlerOperatingState::Normal
        };
    }
}

impl Ord for RouteClass {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        rank(*self).cmp(&rank(*other))
    }
}

impl PartialOrd for RouteClass {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn rank(class: RouteClass) -> u8 {
    match class {
        RouteClass::Green => 0,
        RouteClass::Amber => 1,
        RouteClass::Red => 2,
        RouteClass::Black => 3,
    }
}

fn nominal_ground_pressure_pa(mass_kg: u32, contact_area_cm2: u32) -> u32 {
    // 9.80665 N/kg × 10_000 cm²/m², rounded to five decimal places.
    let pressure = u64::from(mass_kg).saturating_mul(98_067) / u64::from(contact_area_cm2);
    u32::try_from(pressure).unwrap_or(u32::MAX)
}

fn reference_subsystems() -> BTreeMap<SubsystemKind, SubsystemState> {
    let mut states = BTreeMap::new();
    for kind in [
        SubsystemKind::MobilityFront,
        SubsystemKind::MobilityRear,
        SubsystemKind::GeneratorA,
        SubsystemKind::GeneratorB,
        SubsystemKind::Battery,
        SubsystemKind::ThermalLoopA,
        SubsystemKind::ThermalLoopB,
        SubsystemKind::Water,
        SubsystemKind::Habitat,
        SubsystemKind::Clinic,
        SubsystemKind::Workshop,
        SubsystemKind::Communications,
        SubsystemKind::ArticulationAB,
        SubsystemKind::ArticulationBC,
    ] {
        states.insert(
            kind,
            SubsystemState {
                health: 10_000,
                load_w: 0,
                temperature_deci_c: 293,
                isolated: false,
                available: true,
            },
        );
    }
    states
}

/// Crawler profile, cargo, or route failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlerError {
    /// Published mass components do not sum to reference gross mass.
    ReferenceMassDoesNotClose { expected: u32, actual: u32 },
    /// Published normal demand exceeds continuous source power.
    ReferencePowerDoesNotClose { supply_w: u32, demand_w: u32 },
    /// Proposed field load exceeds the platform maximum.
    FieldMassExceeded { proposed_kg: u32, maximum_kg: u32 },
    /// Cargo centre is outside the stowed physical envelope.
    CargoOutsideEnvelope(StableId),
    /// Requested cargo identity is not loaded.
    UnknownCargo(StableId),
}

impl fmt::Display for CrawlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReferenceMassDoesNotClose { expected, actual } => write!(
                formatter,
                "reference mass does not close: expected {expected} kg, got {actual} kg"
            ),
            Self::ReferencePowerDoesNotClose { supply_w, demand_w } => write!(
                formatter,
                "reference demand {demand_w} W exceeds supply {supply_w} W"
            ),
            Self::FieldMassExceeded {
                proposed_kg,
                maximum_kg,
            } => write!(
                formatter,
                "proposed mass {proposed_kg} kg exceeds field maximum {maximum_kg} kg"
            ),
            Self::CargoOutsideEnvelope(id) => {
                write!(formatter, "cargo {id} lies outside the Crawler envelope")
            }
            Self::UnknownCargo(id) => write!(formatter, "unknown Crawler cargo {id}"),
        }
    }
}

impl Error for CrawlerError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test identifier is valid")
    }

    fn cargo(name: &str, mass_kg: u32, lateral_mm: i32, vertical_mm: i32) -> CargoItem {
        CargoItem {
            id: id(name),
            mass_kg,
            volume_l: 5_000,
            longitudinal_mm: 20_000,
            lateral_mm,
            vertical_mm,
            restraint_rating_n: 200_000,
            secured: false,
            custodian_id: id("custodian:firstlight"),
            access_priority: "en-route".into(),
        }
    }

    #[test]
    fn reference_profile_closes() {
        CrawlerProfile::clv7_wayhouse()
            .validate()
            .expect("reference budgets close");
    }

    #[test]
    fn field_overload_is_rejected() {
        let mut crawler = CrawlerState::reference(1);
        assert!(matches!(
            crawler.load_cargo(cargo("cargo:overload", 50_000, 0, 2_000)),
            Err(CrawlerError::FieldMassExceeded { .. })
        ));
    }

    #[test]
    fn unsecured_cargo_blocks_movement() {
        let mut crawler = CrawlerState::reference(2);
        let item = cargo("cargo:cribbing", 5_100, 0, 1_100);
        let item_id = item.id.clone();
        crawler.load_cargo(item).expect("load cargo");
        let route = RouteSegment {
            id: id("route:yard-egress"),
            clear_width_mm: 12_000,
            clear_height_mm: 8_500,
            bearing_capacity_pa: 180_000,
            cross_slope_millirad: 10,
            grade_millirad: 20,
            confidence: 9_400,
        };
        assert_eq!(crawler.qualify_route(&route).class, RouteClass::Black);
        crawler.secure_cargo(&item_id).expect("secure cargo");
        assert_ne!(crawler.qualify_route(&route).class, RouteClass::Black);
    }

    #[test]
    fn lateral_high_cargo_reduces_route_margin() {
        let mut crawler = CrawlerState::reference(3);
        let item = cargo("cargo:high-lateral", 18_000, 3_500, 5_900);
        let item_id = item.id.clone();
        crawler.load_cargo(item).expect("load cargo");
        crawler.secure_cargo(&item_id).expect("secure cargo");
        let route = RouteSegment {
            id: id("route:east-berm"),
            clear_width_mm: 9_400,
            clear_height_mm: 9_000,
            bearing_capacity_pa: 135_000,
            cross_slope_millirad: 60,
            grade_millirad: 50,
            confidence: 8_000,
        };
        assert!(matches!(
            crawler.qualify_route(&route).class,
            RouteClass::Red | RouteClass::Black
        ));
    }

    #[test]
    fn loss_of_one_generator_is_stressed_not_magic_failure() {
        let mut crawler = CrawlerState::reference(4);
        crawler.damage_subsystem(SubsystemKind::GeneratorA, 9_000);
        assert_eq!(crawler.operating_state, CrawlerOperatingState::Stressed);
    }
}
