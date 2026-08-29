// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symthaea_bevy_brain::{
    ArtistDepthConfig, ArtistDepthError, DepthPlaneEncoding, analyze_depth_plane,
};

#[test]
fn row_padding_is_not_interpreted_as_depth_samples() {
    // width=2, stride=4: the last two values of each row are padding. If padding
    // leaked into evidence, the enormous sentinel values would dominate depth.
    let samples = [
        2.0f32, 2.0, 9999.0, 9999.0,
        2.0, 2.0, 9999.0, 9999.0,
    ];
    let evidence = analyze_depth_plane(
        2,
        2,
        4,
        &samples,
        DepthPlaneEncoding::LinearMeters,
        ArtistDepthConfig::default(),
    )
    .unwrap();
    assert_eq!(evidence.distribution.minimum_meters, Some(2.0));
    assert_eq!(evidence.distribution.maximum_meters, Some(2.0));
    assert_eq!(evidence.distribution.clipped_far_fraction, 0.0);
}

#[test]
fn invalid_samples_become_missing_not_zero_distance() {
    let samples = [1.0f32, f32::NAN, -1.0, f32::INFINITY];
    let evidence = analyze_depth_plane(
        4,
        1,
        4,
        &samples,
        DepthPlaneEncoding::LinearMeters,
        ArtistDepthConfig::default(),
    )
    .unwrap();
    assert_eq!(evidence.distribution.valid_fraction, 0.25);
    assert_eq!(evidence.distribution.minimum_meters, Some(1.0));
    assert_eq!(evidence.distribution.maximum_meters, Some(1.0));
}

#[test]
fn invalid_metric_layer_order_is_rejected() {
    let config = ArtistDepthConfig {
        far_clip_meters: 100.0,
        discontinuity_threshold_meters: 0.25,
        near_split_meters: 50.0,
        far_split_meters: 10.0,
    };
    let error = analyze_depth_plane(
        1,
        1,
        1,
        &[1.0],
        DepthPlaneEncoding::LinearMeters,
        config,
    )
    .unwrap_err();
    assert_eq!(error, ArtistDepthError::InvalidConfig);
}

#[test]
fn malformed_linear01_encoding_is_rejected() {
    let error = analyze_depth_plane(
        1,
        1,
        1,
        &[0.5],
        DepthPlaneEncoding::Linear01 {
            near_meters: 10.0,
            far_meters: 1.0,
            reversed: false,
        },
        ArtistDepthConfig::default(),
    )
    .unwrap_err();
    assert_eq!(error, ArtistDepthError::InvalidEncoding);
}
