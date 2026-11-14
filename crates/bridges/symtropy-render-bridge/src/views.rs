// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Purpose-built view policies and dimension-aware projection helpers.
//!
//! Dimensionality and purpose are independent choices. A strategic view can use a
//! planar map or an oblique 2.5D overview; a tactical view can use a 2D plan, a 3D
//! scene, or a 4D slice. These policies never mutate simulation state.
//!
//! The view profiles are recommendations/contracts for host UI and rendering
//! systems. They do not automatically create cameras, aggregate data, or attach
//! overlays; a host must apply the requested policies and retain entity provenance.

use super::projection::Projector;
use bevy::prelude::Resource;
use symtropy_math::Point;

/// User-facing projection family, independent of the purpose of the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewDimension {
    /// A true planar view with no encoded height axis.
    Planar2D,
    /// An oblique orthographic representation that encodes ground depth and height.
    Oblique25D,
    /// A spatial 3D view; camera projection is managed by the host renderer.
    Spatial3D,
    /// A 3D cross-section of a 4D world using an explicitly configured hyperplane.
    HyperSlice4D,
}

/// The primary question a view is designed to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewPurpose {
    /// Where are the important systems, resources, and connections?
    Strategic,
    /// How are routes, facilities, and workflows interacting across a region?
    Operational,
    /// What is happening around selected entities and local constraints right now?
    Tactical,
    /// What do physical or derived fields show, and how trustworthy are they?
    Scientific,
    /// Which contacts, motion vectors, or geometry constraints explain behavior?
    Diagnostic,
}

/// Intended detail-selection policy. The host decides how to implement it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewDetailPolicy {
    /// Replace dense groups with summaries that preserve member identity.
    Aggregated,
    /// Show clusters at distance and individual members near the focus.
    Clustered,
    /// Render each selected/visible entity independently.
    PerEntity,
    /// Preserve individual samples rather than replacing them with aggregates.
    PerSample,
}

/// Intended camera interaction model for a view.
///
/// The host UI must implement these controls; this enum declares intent and does not
/// attach an input system or mutate camera transforms itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewNavigationPolicy {
    /// Pan and zoom in a planar orthographic view.
    PlanarPanZoom,
    /// Pan and orbit around a fixed oblique overview.
    ObliquePanOrbit,
    /// Orbit and pan around a broad spatial overview.
    OverviewOrbit,
    /// Keep the camera focus centered on a selected local entity or group.
    FocusedOrbit,
    /// Free orbit/pan for scientific or diagnostic inspection.
    InspectionOrbit,
    /// Scrub the 4D hyperplane position while allowing 3D pan/orbit in the slice.
    SliceScrubOrbit,
}

/// Optional render layers requested by a view profile.
///
/// These are presentation controls only. They must never alter source measurements,
/// simulation state, physics results, or the membership of the underlying world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewLayerVisibility {
    pub terrain: bool,
    pub infrastructure: bool,
    pub networks: bool,
    pub routes: bool,
    pub entity_models: bool,
    pub contacts: bool,
    pub velocity_vectors: bool,
    pub motion_trails: bool,
    pub scalar_fields: bool,
    pub vector_fields: bool,
    pub uncertainty: bool,
    pub provenance: bool,
    pub legends: bool,
}

/// One-dimensional window through a 4D world, measured in simulation coordinate units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliceWindow {
    /// Signed distance of the slice hyperplane from the origin, in simulation units.
    pub offset: f64,
    /// Positive half-thickness around the hyperplane, in simulation units.
    pub half_thickness: f64,
}

impl SliceWindow {
    pub fn new(offset: f64, half_thickness: f64) -> Result<Self, ViewConfigError> {
        let window = Self {
            offset,
            half_thickness,
        };
        window.validate()?;
        Ok(window)
    }

    pub fn validate(self) -> Result<(), ViewConfigError> {
        if !self.offset.is_finite() {
            return Err(ViewConfigError::NonFiniteParameter);
        }
        if !self.half_thickness.is_finite() {
            return Err(ViewConfigError::NonFiniteParameter);
        }
        if self.half_thickness <= 0.0 {
            return Err(ViewConfigError::NonPositiveSliceThickness);
        }
        Ok(())
    }
}

/// Recommended settings for one purpose and one projection family.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewProfile {
    pub purpose: ViewPurpose,
    pub dimension: ViewDimension,
    pub navigation: ViewNavigationPolicy,
    pub detail: ViewDetailPolicy,
    /// Soft rendering budget for visible entities/samples; never a data deletion limit.
    pub max_visible_items: usize,
    /// Whether the host may aggregate distant objects to preserve legibility.
    pub aggregate_distant_items: bool,
    /// Aggregate output must retain the stable identities of its member records.
    pub preserve_aggregate_membership: bool,
    /// Focused/selected entities should be exempt from ordinary distance-based thinning.
    pub selected_items_bypass_draw_cap: bool,
    pub layers: ViewLayerVisibility,
    /// Required for a 4D view; width is in simulation coordinate units.
    pub slice_window: Option<SliceWindow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewConfigError {
    NonFiniteParameter,
    NonPositiveScale,
    ElevationOutOfRange,
    DegenerateSliceNormal,
    NonPositiveSliceThickness,
    ZeroVisibleItemBudget,
    MissingSliceWindow,
    SliceWindowForNon4DView,
    FieldLayerRequiresLegendAndProvenance,
    AggregateMustPreserveMembership,
}

/// Errors shared by projection constructors.
pub type ProjectionConfigError = ViewConfigError;

impl ViewProfile {
    /// Creates purpose-specific recommended settings for a selected dimensional view.
    ///
    /// The starter 4D window is intentionally conservative but is not a universal
    /// physical scale. Hosts should tune it to the world units and sampling density.
    pub fn recommended(purpose: ViewPurpose, dimension: ViewDimension) -> Self {
        let (detail, max_visible_items, aggregate_distant_items, layers) = match purpose {
            ViewPurpose::Strategic => (
                ViewDetailPolicy::Aggregated,
                2_000,
                true,
                ViewLayerVisibility {
                    terrain: true,
                    infrastructure: true,
                    networks: true,
                    routes: true,
                    entity_models: true,
                    contacts: false,
                    velocity_vectors: false,
                    motion_trails: false,
                    scalar_fields: false,
                    vector_fields: false,
                    uncertainty: false,
                    provenance: false,
                    legends: true,
                },
            ),
            ViewPurpose::Operational => (
                ViewDetailPolicy::Clustered,
                6_000,
                true,
                ViewLayerVisibility {
                    terrain: true,
                    infrastructure: true,
                    networks: true,
                    routes: true,
                    entity_models: true,
                    contacts: false,
                    velocity_vectors: false,
                    motion_trails: true,
                    scalar_fields: false,
                    vector_fields: false,
                    uncertainty: false,
                    provenance: false,
                    legends: true,
                },
            ),
            ViewPurpose::Tactical => (
                ViewDetailPolicy::PerEntity,
                4_000,
                false,
                ViewLayerVisibility {
                    terrain: true,
                    infrastructure: true,
                    networks: false,
                    routes: true,
                    entity_models: true,
                    contacts: true,
                    velocity_vectors: true,
                    motion_trails: true,
                    scalar_fields: false,
                    vector_fields: false,
                    uncertainty: false,
                    provenance: false,
                    legends: true,
                },
            ),
            ViewPurpose::Scientific => (
                ViewDetailPolicy::PerSample,
                10_000,
                false,
                ViewLayerVisibility {
                    terrain: true,
                    infrastructure: false,
                    networks: false,
                    routes: false,
                    entity_models: true,
                    contacts: false,
                    velocity_vectors: false,
                    motion_trails: false,
                    scalar_fields: true,
                    vector_fields: true,
                    uncertainty: true,
                    provenance: true,
                    legends: true,
                },
            ),
            ViewPurpose::Diagnostic => (
                ViewDetailPolicy::PerEntity,
                10_000,
                false,
                ViewLayerVisibility {
                    terrain: false,
                    infrastructure: true,
                    networks: false,
                    routes: false,
                    entity_models: true,
                    contacts: true,
                    velocity_vectors: true,
                    motion_trails: true,
                    scalar_fields: true,
                    vector_fields: true,
                    uncertainty: true,
                    provenance: true,
                    legends: true,
                },
            ),
        };

        Self {
            purpose,
            dimension,
            navigation: Self::navigation_for(purpose, dimension),
            detail,
            max_visible_items,
            aggregate_distant_items,
            preserve_aggregate_membership: true,
            selected_items_bypass_draw_cap: true,
            layers,
            slice_window: if dimension == ViewDimension::HyperSlice4D {
                // Generic starter only; the host should tune this to world scale.
                Some(SliceWindow {
                    offset: 0.0,
                    half_thickness: 0.5,
                })
            } else {
                None
            },
        }
    }

    /// Recommended camera interaction for a given purpose/projection pair.
    pub fn navigation_for(purpose: ViewPurpose, dimension: ViewDimension) -> ViewNavigationPolicy {
        match dimension {
            ViewDimension::Planar2D => ViewNavigationPolicy::PlanarPanZoom,
            ViewDimension::Oblique25D => ViewNavigationPolicy::ObliquePanOrbit,
            ViewDimension::Spatial3D => match purpose {
                ViewPurpose::Strategic | ViewPurpose::Operational => {
                    ViewNavigationPolicy::OverviewOrbit
                }
                ViewPurpose::Tactical => ViewNavigationPolicy::FocusedOrbit,
                ViewPurpose::Scientific | ViewPurpose::Diagnostic => {
                    ViewNavigationPolicy::InspectionOrbit
                }
            },
            ViewDimension::HyperSlice4D => ViewNavigationPolicy::SliceScrubOrbit,
        }
    }

    /// Changes the dimensional projection family while retaining view purpose.
    pub fn with_dimension(mut self, dimension: ViewDimension) -> Self {
        self.dimension = dimension;
        self.navigation = Self::navigation_for(self.purpose, dimension);
        self.slice_window = if dimension == ViewDimension::HyperSlice4D {
            Some(SliceWindow {
                offset: 0.0,
                half_thickness: 0.5,
            })
        } else {
            None
        };
        self
    }

    pub fn validate(self) -> Result<(), ViewConfigError> {
        if self.max_visible_items == 0 {
            return Err(ViewConfigError::ZeroVisibleItemBudget);
        }

        if self.aggregate_distant_items && !self.preserve_aggregate_membership {
            return Err(ViewConfigError::AggregateMustPreserveMembership);
        }

        let has_field_layer = self.layers.scalar_fields || self.layers.vector_fields;
        if has_field_layer && (!self.layers.legends || !self.layers.provenance) {
            return Err(ViewConfigError::FieldLayerRequiresLegendAndProvenance);
        }

        match (self.dimension, self.slice_window) {
            (ViewDimension::HyperSlice4D, Some(window)) => window.validate()?,
            (ViewDimension::HyperSlice4D, None) => {
                return Err(ViewConfigError::MissingSliceWindow);
            }
            (_, Some(_)) => return Err(ViewConfigError::SliceWindowForNon4DView),
            (_, None) => {}
        }
        Ok(())
    }
}

/// Oblique 2.5D orthographic projection of a 3D point.
///
/// The ground-plane coordinates are rotated by azimuth; ground depth and world height
/// are then combined into screen-up and depth coordinates using the elevation angle.
/// Elevation is measured from the ground plane: 0 radians is a near-horizon view and
/// PI/2 is a top-down plan view. This is a coordinate projection, not a Bevy camera;
/// align the host camera with the resulting basis to avoid applying perspective twice.
#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct ProjectorOblique25D {
    pub scale_factor: f32,
    pub azimuth_radians: f64,
    pub elevation_radians: f64,
    pub center: [f64; 3],
}

impl ProjectorOblique25D {
    pub fn new(
        scale_factor: f32,
        azimuth_radians: f64,
        elevation_radians: f64,
    ) -> Result<Self, ProjectionConfigError> {
        let projector = Self {
            scale_factor,
            azimuth_radians,
            elevation_radians,
            center: [0.0; 3],
        };
        projector.validate()?;
        Ok(projector)
    }

    pub fn set_center(&mut self, center: [f64; 3]) -> Result<(), ProjectionConfigError> {
        if !center.iter().all(|component| component.is_finite()) {
            return Err(ViewConfigError::NonFiniteParameter);
        }
        self.center = center;
        Ok(())
    }

    pub fn validate(self) -> Result<(), ProjectionConfigError> {
        if !self.scale_factor.is_finite()
            || !self.azimuth_radians.is_finite()
            || !self.elevation_radians.is_finite()
            || !self.center.iter().all(|component| component.is_finite())
        {
            return Err(ViewConfigError::NonFiniteParameter);
        }
        if self.scale_factor <= 0.0 {
            return Err(ViewConfigError::NonPositiveScale);
        }
        if !(0.0..=std::f64::consts::FRAC_PI_2).contains(&self.elevation_radians) {
            return Err(ViewConfigError::ElevationOutOfRange);
        }
        Ok(())
    }

    fn project_coordinates(self, coordinates: [f64; 3]) -> Option<[f32; 3]> {
        if self.validate().is_err() || !coordinates.iter().all(|component| component.is_finite()) {
            return None;
        }
        let [x, y, z] = [
            coordinates[0] - self.center[0],
            coordinates[1] - self.center[1],
            coordinates[2] - self.center[2],
        ];
        let (sin_azimuth, cos_azimuth) = self.azimuth_radians.sin_cos();
        let (sin_elevation, cos_elevation) = self.elevation_radians.sin_cos();

        let horizontal = x * cos_azimuth - y * sin_azimuth;
        let ground_depth = x * sin_azimuth + y * cos_azimuth;
        let screen_up = ground_depth * sin_elevation + z * cos_elevation;
        let depth = ground_depth * cos_elevation - z * sin_elevation;
        let scale = f64::from(self.scale_factor);
        let projected = [horizontal * scale, screen_up * scale, depth * scale];

        if projected
            .iter()
            .all(|value| value.is_finite() && value.abs() <= f64::from(f32::MAX))
        {
            Some([
                projected[0] as f32,
                projected[1] as f32,
                projected[2] as f32,
            ])
        } else {
            None
        }
    }
}

impl Projector<3> for ProjectorOblique25D {
    fn project(&self, point: &Point<3>) -> [f32; 3] {
        self.project_coordinates([point.coord(0), point.coord(1), point.coord(2)])
            .unwrap_or([f32::NAN; 3])
    }

    fn is_visible(&self, point: &Point<3>) -> bool {
        self.project_coordinates([point.coord(0), point.coord(1), point.coord(2)])
            .is_some()
    }

    fn scale(&self) -> f32 {
        self.scale_factor
    }
}

/// An arbitrary oriented hyperplane slice through a 4D world.
///
/// The normal is normalized at construction. The projector orthogonally projects
/// visible points onto the slice, then expresses the projected point in a deterministic
/// orthonormal 3-vector basis spanning the hyperplane. It performs point/slice
/// projection, not exact mesh/collider intersection; continuous geometry cross-sections
/// require the corresponding intersection adapter.
#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct OrientedProjector4D {
    normal: [f64; 4],
    offset: f64,
    half_thickness: f64,
    scale_factor: f32,
    basis: [[f64; 4]; 3],
}

impl OrientedProjector4D {
    pub fn new(
        normal: [f64; 4],
        offset: f64,
        half_thickness: f64,
        scale_factor: f32,
    ) -> Result<Self, ProjectionConfigError> {
        if !normal.iter().all(|component| component.is_finite())
            || !offset.is_finite()
            || !half_thickness.is_finite()
            || !scale_factor.is_finite()
        {
            return Err(ViewConfigError::NonFiniteParameter);
        }
        if half_thickness <= 0.0 {
            return Err(ViewConfigError::NonPositiveSliceThickness);
        }
        if scale_factor <= 0.0 {
            return Err(ViewConfigError::NonPositiveScale);
        }

        let norm = normal.iter().map(|value| value * value).sum::<f64>().sqrt();
        if !norm.is_finite() || norm <= f64::EPSILON {
            return Err(ViewConfigError::DegenerateSliceNormal);
        }
        let normalized_normal = normal.map(|component| component / norm);
        let basis =
            Self::build_basis(normalized_normal).ok_or(ViewConfigError::DegenerateSliceNormal)?;

        Ok(Self {
            normal: normalized_normal,
            offset,
            half_thickness,
            scale_factor,
            basis,
        })
    }

    fn build_basis(normal: [f64; 4]) -> Option<[[f64; 4]; 3]> {
        let mut basis: Vec<[f64; 4]> = Vec::with_capacity(3);

        for mut candidate in [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ] {
            let normal_projection = dot4(candidate, normal);
            for (coordinate, normal_axis) in candidate.iter_mut().zip(normal) {
                *coordinate -= normal_projection * normal_axis;
            }

            for existing in &basis {
                let component = dot4(candidate, *existing);
                for (coordinate, basis_component) in candidate.iter_mut().zip(*existing) {
                    *coordinate -= component * basis_component;
                }
            }

            let length = dot4(candidate, candidate).sqrt();
            if length > 1e-10 {
                for coordinate in &mut candidate {
                    *coordinate /= length;
                }
                basis.push(candidate);
                if basis.len() == 3 {
                    break;
                }
            }
        }

        if basis.len() != 3 {
            return None;
        }
        Some([basis[0], basis[1], basis[2]])
    }

    pub fn normal(self) -> [f64; 4] {
        self.normal
    }

    pub fn basis(self) -> [[f64; 4]; 3] {
        self.basis
    }

    pub fn offset(self) -> f64 {
        self.offset
    }

    pub fn half_thickness(self) -> f64 {
        self.half_thickness
    }

    pub fn signed_distance(&self, point: &Point<4>) -> Option<f64> {
        let coordinates = [
            point.coord(0),
            point.coord(1),
            point.coord(2),
            point.coord(3),
        ];
        if !coordinates.iter().all(|component| component.is_finite()) {
            return None;
        }
        Some(dot4(coordinates, self.normal) - self.offset)
    }

    pub fn alpha(&self, point: &Point<4>) -> f32 {
        let Some(distance) = self.signed_distance(point) else {
            return 0.0;
        };
        let distance = distance.abs();
        if distance >= self.half_thickness {
            return 0.0;
        }
        (1.0 - distance / self.half_thickness) as f32
    }

    fn project_coordinates(&self, coordinates: [f64; 4]) -> Option<[f32; 3]> {
        if !coordinates.iter().all(|component| component.is_finite()) {
            return None;
        }
        let distance = dot4(coordinates, self.normal) - self.offset;
        let projected_onto_plane =
            std::array::from_fn(|index| coordinates[index] - distance * self.normal[index]);
        let projected = self.basis.map(|basis_vector| {
            dot4(projected_onto_plane, basis_vector) * f64::from(self.scale_factor)
        });

        if projected
            .iter()
            .all(|value| value.is_finite() && value.abs() <= f64::from(f32::MAX))
        {
            Some([
                projected[0] as f32,
                projected[1] as f32,
                projected[2] as f32,
            ])
        } else {
            None
        }
    }
}

impl Projector<4> for OrientedProjector4D {
    fn project(&self, point: &Point<4>) -> [f32; 3] {
        self.project_coordinates([
            point.coord(0),
            point.coord(1),
            point.coord(2),
            point.coord(3),
        ])
        .unwrap_or([f32::NAN; 3])
    }

    fn is_visible(&self, point: &Point<4>) -> bool {
        self.signed_distance(point)
            .is_some_and(|distance| distance.abs() < self.half_thickness)
            && self
                .project_coordinates([
                    point.coord(0),
                    point.coord(1),
                    point.coord(2),
                    point.coord(3),
                ])
                .is_some()
    }

    fn scale(&self) -> f32 {
        self.scale_factor
    }
}

fn dot4(left: [f64; 4], right: [f64; 4]) -> f64 {
    left.iter().zip(right).map(|(a, b)| *a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "expected {left} near {right} within {tolerance}"
        );
    }

    #[test]
    fn purpose_profiles_have_distinct_intents() {
        let strategic = ViewProfile::recommended(ViewPurpose::Strategic, ViewDimension::Planar2D);
        let operational =
            ViewProfile::recommended(ViewPurpose::Operational, ViewDimension::Oblique25D);
        let tactical = ViewProfile::recommended(ViewPurpose::Tactical, ViewDimension::Spatial3D);
        let scientific =
            ViewProfile::recommended(ViewPurpose::Scientific, ViewDimension::HyperSlice4D);

        assert_eq!(strategic.navigation, ViewNavigationPolicy::PlanarPanZoom);
        assert_eq!(strategic.detail, ViewDetailPolicy::Aggregated);
        assert!(strategic.aggregate_distant_items);
        assert!(!strategic.layers.contacts);

        assert_eq!(
            operational.navigation,
            ViewNavigationPolicy::ObliquePanOrbit
        );
        assert_eq!(operational.detail, ViewDetailPolicy::Clustered);
        assert!(operational.layers.routes);
        assert!(operational.layers.networks);

        assert_eq!(tactical.navigation, ViewNavigationPolicy::FocusedOrbit);
        assert_eq!(tactical.detail, ViewDetailPolicy::PerEntity);
        assert!(!tactical.aggregate_distant_items);
        assert!(tactical.layers.contacts);
        assert!(tactical.layers.velocity_vectors);

        assert_eq!(scientific.navigation, ViewNavigationPolicy::SliceScrubOrbit);
        assert_eq!(scientific.detail, ViewDetailPolicy::PerSample);
        assert!(scientific.layers.scalar_fields);
        assert!(scientific.layers.uncertainty);
        assert!(scientific.layers.provenance);
        assert!(scientific.slice_window.is_some());
        assert!(strategic.validate().is_ok());
        assert!(operational.validate().is_ok());
        assert!(tactical.validate().is_ok());
        assert!(scientific.validate().is_ok());
    }

    #[test]
    fn purpose_and_dimension_are_independent_choices() {
        let tactical_2d = ViewProfile::recommended(ViewPurpose::Tactical, ViewDimension::Planar2D);
        let tactical_4d = tactical_2d.with_dimension(ViewDimension::HyperSlice4D);
        assert_eq!(tactical_4d.purpose, ViewPurpose::Tactical);
        assert_eq!(tactical_4d.dimension, ViewDimension::HyperSlice4D);
        assert_eq!(
            tactical_4d.navigation,
            ViewNavigationPolicy::SliceScrubOrbit
        );
        assert!(tactical_4d.validate().is_ok());
    }

    #[test]
    fn field_layers_cannot_be_configured_without_legend_and_provenance() {
        let mut profile =
            ViewProfile::recommended(ViewPurpose::Scientific, ViewDimension::Spatial3D);
        profile.layers.provenance = false;
        assert_eq!(
            profile.validate(),
            Err(ViewConfigError::FieldLayerRequiresLegendAndProvenance)
        );
    }

    #[test]
    fn four_d_views_require_a_valid_slice_window() {
        let mut profile =
            ViewProfile::recommended(ViewPurpose::Tactical, ViewDimension::HyperSlice4D);
        profile.slice_window = None;
        assert_eq!(profile.validate(), Err(ViewConfigError::MissingSliceWindow));

        profile.slice_window = Some(SliceWindow {
            offset: 0.0,
            half_thickness: 0.0,
        });
        assert_eq!(
            profile.validate(),
            Err(ViewConfigError::NonPositiveSliceThickness)
        );
    }

    #[test]
    fn two_point_five_d_projection_keeps_height_distinct_from_ground_depth() {
        let projector = ProjectorOblique25D::new(1.0, 0.0, std::f64::consts::FRAC_PI_4).unwrap();
        let ground = Point::new([0.0, 0.0, 0.0]);
        let elevated = Point::new([0.0, 0.0, 2.0]);
        let ground_projected = projector.project(&ground);
        let elevated_projected = projector.project(&elevated);

        near(
            ground_projected[0] as f64,
            elevated_projected[0] as f64,
            1e-6,
        );
        assert!(elevated_projected[1] > ground_projected[1]);
        assert!(elevated_projected[2] < ground_projected[2]);
    }

    #[test]
    fn two_point_five_d_projection_centers_and_rejects_invalid_configuration() {
        let mut projector =
            ProjectorOblique25D::new(2.0, 0.0, std::f64::consts::FRAC_PI_2).unwrap();
        projector.set_center([3.0, 4.0, 0.0]).unwrap();
        let projected = projector.project(&Point::new([4.0, 6.0, 0.0]));
        near(projected[0] as f64, 2.0, 1e-6);
        near(projected[1] as f64, 2.0, 1e-6);

        assert_eq!(
            ProjectorOblique25D::new(0.0, 0.0, 0.5),
            Err(ViewConfigError::NonPositiveScale)
        );
        assert_eq!(
            ProjectorOblique25D::new(1.0, 0.0, 2.0),
            Err(ViewConfigError::ElevationOutOfRange)
        );
    }

    #[test]
    fn oriented_four_d_slice_handles_arbitrary_normal_and_offset() {
        let projector = OrientedProjector4D::new([1.0, 0.0, 0.0, 0.0], 2.0, 0.5, 1.0).unwrap();
        let on_plane = Point::new([2.0, 3.0, 4.0, 5.0]);
        let near_plane = Point::new([2.25, 3.0, 4.0, 5.0]);
        let outside = Point::new([3.0, 3.0, 4.0, 5.0]);

        assert!(projector.is_visible(&on_plane));
        near(projector.alpha(&on_plane) as f64, 1.0, 1e-6);
        assert!(projector.is_visible(&near_plane));
        near(projector.alpha(&near_plane) as f64, 0.5, 1e-6);
        assert!(!projector.is_visible(&outside));
        let projected = projector.project(&on_plane);
        near(projected[0] as f64, 3.0, 1e-6);
        near(projected[1] as f64, 4.0, 1e-6);
        near(projected[2] as f64, 5.0, 1e-6);
    }

    #[test]
    fn oriented_four_d_w_slice_matches_xyz_projection_on_the_plane() {
        let projector = OrientedProjector4D::new([0.0, 0.0, 0.0, 1.0], 2.0, 0.25, 2.0).unwrap();
        let point = Point::new([1.0, 2.0, 3.0, 2.0]);
        let projected = projector.project(&point);
        near(projected[0] as f64, 2.0, 1e-6);
        near(projected[1] as f64, 4.0, 1e-6);
        near(projected[2] as f64, 6.0, 1e-6);
        assert!(projector.is_visible(&point));
    }

    #[test]
    fn oriented_four_d_basis_is_orthonormal_for_an_oblique_hyperplane() {
        let projector = OrientedProjector4D::new([1.0, 2.0, 3.0, 4.0], 2.5, 0.25, 1.0).unwrap();
        let normal = projector.normal();
        let basis = projector.basis();
        near(dot4(normal, normal), 1.0, 1e-10);
        for (index, vector) in basis.iter().enumerate() {
            near(dot4(*vector, normal), 0.0, 1e-10);
            near(dot4(*vector, *vector), 1.0, 1e-10);
            for other in basis.iter().skip(index + 1) {
                near(dot4(*vector, *other), 0.0, 1e-10);
            }
        }
    }

    #[test]
    fn oriented_four_d_slice_rejects_boundary_and_nonfinite_points() {
        let projector = OrientedProjector4D::new([0.0, 0.0, 0.0, 1.0], 0.0, 0.5, 1.0).unwrap();
        let boundary = Point::new([1.0, 2.0, 3.0, 0.5]);
        let nonfinite = Point::new([1.0, 2.0, 3.0, f64::NAN]);

        assert!(!projector.is_visible(&boundary));
        near(projector.alpha(&boundary) as f64, 0.0, 1e-6);
        assert!(!projector.is_visible(&nonfinite));
        assert!(
            !projector
                .project(&nonfinite)
                .iter()
                .all(|value| value.is_finite())
        );
    }

    #[test]
    fn aggregate_policy_cannot_drop_source_membership() {
        let mut profile = ViewProfile::recommended(ViewPurpose::Strategic, ViewDimension::Planar2D);
        profile.preserve_aggregate_membership = false;
        assert_eq!(
            profile.validate(),
            Err(ViewConfigError::AggregateMustPreserveMembership)
        );
    }

    #[test]
    fn oriented_four_d_slice_rejects_degenerate_or_nonfinite_parameters() {
        assert_eq!(
            OrientedProjector4D::new([0.0; 4], 0.0, 1.0, 1.0),
            Err(ViewConfigError::DegenerateSliceNormal)
        );
        assert_eq!(
            OrientedProjector4D::new([0.0, 0.0, 0.0, 1.0], 0.0, 0.0, 1.0),
            Err(ViewConfigError::NonPositiveSliceThickness)
        );
        assert_eq!(
            OrientedProjector4D::new([0.0, 0.0, 0.0, f64::NAN], 0.0, 1.0, 1.0),
            Err(ViewConfigError::NonFiniteParameter)
        );
    }
}
