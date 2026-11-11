{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Synthetic inter-pass scan qualification evidence",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "schema_version",
    "benchmark_id",
    "evidence_class",
    "scan_identity",
    "calibration",
    "registration",
    "observation",
    "uncertainty",
    "decision_boundary"
  ],
  "properties": {
    "schema_version": {
      "const": "0.1.0"
    },
    "benchmark_id": {
      "const": "SHIPYARD-SCAN-METROLOGY-001"
    },
    "evidence_class": {
      "const": "synthetic_contract_only"
    },
    "scan_identity": {
      "$ref": "#/$defs/scan_identity"
    },
    "calibration": {
      "$ref": "#/$defs/calibration"
    },
    "registration": {
      "$ref": "#/$defs/registration"
    },
    "observation": {
      "$ref": "#/$defs/observation"
    },
    "uncertainty": {
      "$ref": "#/$defs/uncertainty"
    },
    "decision_boundary": {
      "$ref": "#/$defs/decision_boundary"
    },
    "claim_ceiling": {
      "const": "SYNTHETIC_SCAN_MEASUREMENT_CONTRACT_ONLY"
    }
  },
  "$defs": {
    "sha256": {
      "type": "string",
      "pattern": "^[0-9a-f]{64}$"
    },
    "scan_identity": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "workpiece_id",
        "joint_revision",
        "layer_id",
        "predecessor_pass_id",
        "scan_id",
        "source_scan_sha256",
        "acquisition_config_revision",
        "acquisition_tick"
      ],
      "properties": {
        "workpiece_id": {
          "type": "string",
          "minLength": 1
        },
        "joint_revision": {
          "type": "string",
          "minLength": 1
        },
        "layer_id": {
          "type": "string",
          "minLength": 1
        },
        "predecessor_pass_id": {
          "type": "string",
          "minLength": 1
        },
        "scan_id": {
          "type": "string",
          "minLength": 1
        },
        "source_scan_sha256": {
          "$ref": "#/$defs/sha256"
        },
        "acquisition_config_revision": {
          "type": "string",
          "minLength": 1
        },
        "acquisition_tick": {
          "type": "integer",
          "minimum": 0
        }
      }
    },
    "calibration": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "sensor_id",
        "calibration_receipt_sha256",
        "reference_artifact_id",
        "reference_certificate_id",
        "calibration_config_revision",
        "acquisition_config_revision",
        "valid_from_tick",
        "expires_at_tick",
        "environment_in_calibration_envelope",
        "reference_length_error_units",
        "maximum_reference_length_error_units"
      ],
      "properties": {
        "sensor_id": {
          "type": "string",
          "minLength": 1
        },
        "calibration_receipt_sha256": {
          "$ref": "#/$defs/sha256"
        },
        "reference_artifact_id": {
          "type": "string",
          "minLength": 1
        },
        "reference_certificate_id": {
          "type": "string",
          "minLength": 1
        },
        "calibration_config_revision": {
          "type": "string",
          "minLength": 1
        },
        "acquisition_config_revision": {
          "type": "string",
          "minLength": 1
        },
        "valid_from_tick": {
          "type": "integer",
          "minimum": 0
        },
        "expires_at_tick": {
          "type": "integer",
          "minimum": 0
        },
        "environment_in_calibration_envelope": {
          "type": "boolean"
        },
        "reference_length_error_units": {
          "type": "integer",
          "minimum": 0
        },
        "maximum_reference_length_error_units": {
          "type": "integer",
          "minimum": 0
        }
      }
    },
    "registration": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "source_scan_sha256",
        "registration_receipt_sha256",
        "source_frame",
        "target_frame",
        "icp_converged",
        "rotation_determinant_milli",
        "orthogonality_error_units",
        "maximum_orthogonality_error_units",
        "independent_reference_target_count",
        "required_reference_target_count",
        "reference_target_rms_residual_units",
        "maximum_reference_target_rms_residual_units"
      ],
      "properties": {
        "source_scan_sha256": {
          "$ref": "#/$defs/sha256"
        },
        "registration_receipt_sha256": {
          "$ref": "#/$defs/sha256"
        },
        "source_frame": {
          "type": "string",
          "minLength": 1
        },
        "target_frame": {
          "type": "string",
          "minLength": 1
        },
        "icp_converged": {
          "type": "boolean"
        },
        "rotation_determinant_milli": {
          "type": "integer"
        },
        "orthogonality_error_units": {
          "type": "integer",
          "minimum": 0
        },
        "maximum_orthogonality_error_units": {
          "type": "integer",
          "minimum": 0
        },
        "independent_reference_target_count": {
          "type": "integer",
          "minimum": 0
        },
        "required_reference_target_count": {
          "type": "integer",
          "minimum": 1
        },
        "reference_target_rms_residual_units": {
          "type": "integer",
          "minimum": 0
        },
        "maximum_reference_target_rms_residual_units": {
          "type": "integer",
          "minimum": 0
        }
      }
    },
    "observation": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "coordinate_frame",
        "coordinate_unit",
        "preparation",
        "cleaning_receipt_sha256",
        "occlusion_assessment_receipt_sha256",
        "expected_profile_count",
        "received_profile_count",
        "dropped_profile_count",
        "declared_profile_coverage_permille",
        "minimum_profile_coverage_permille",
        "maximum_dropped_profile_permille",
        "total_point_count",
        "valid_point_count",
        "invalid_point_count",
        "clipped_point_count",
        "minimum_valid_point_count",
        "maximum_invalid_point_permille",
        "occlusion_permille",
        "maximum_occlusion_permille"
      ],
      "properties": {
        "coordinate_frame": {
          "type": "string",
          "minLength": 1
        },
        "coordinate_unit": {
          "const": "um"
        },
        "preparation": {
          "enum": [
            "documented_clean",
            "unknown",
            "slag_or_spatter_present"
          ]
        },
        "cleaning_receipt_sha256": {
          "$ref": "#/$defs/sha256"
        },
        "occlusion_assessment_receipt_sha256": {
          "$ref": "#/$defs/sha256"
        },
        "expected_profile_count": {
          "type": "integer",
          "minimum": 1
        },
        "received_profile_count": {
          "type": "integer",
          "minimum": 0
        },
        "dropped_profile_count": {
          "type": "integer",
          "minimum": 0
        },
        "declared_profile_coverage_permille": {
          "type": "integer",
          "minimum": 0,
          "maximum": 1000
        },
        "minimum_profile_coverage_permille": {
          "type": "integer",
          "minimum": 0,
          "maximum": 1000
        },
        "maximum_dropped_profile_permille": {
          "type": "integer",
          "minimum": 0,
          "maximum": 1000
        },
        "total_point_count": {
          "type": "integer",
          "minimum": 1
        },
        "valid_point_count": {
          "type": "integer",
          "minimum": 0
        },
        "invalid_point_count": {
          "type": "integer",
          "minimum": 0
        },
        "clipped_point_count": {
          "type": "integer",
          "minimum": 0
        },
        "minimum_valid_point_count": {
          "type": "integer",
          "minimum": 0
        },
        "maximum_invalid_point_permille": {
          "type": "integer",
          "minimum": 0,
          "maximum": 1000
        },
        "occlusion_permille": {
          "type": "integer",
          "minimum": 0,
          "maximum": 1000
        },
        "maximum_occlusion_permille": {
          "type": "integer",
          "minimum": 0,
          "maximum": 1000
        }
      }
    },
    "uncertainty_component": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "component_id",
        "standard_uncertainty_units"
      ],
      "properties": {
        "component_id": {
          "type": "string",
          "minLength": 1
        },
        "standard_uncertainty_units": {
          "type": "integer",
          "minimum": 0
        }
      }
    },
    "uncertainty": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "components",
        "correlation_policy",
        "coverage_factor_permille",
        "maximum_expanded_uncertainty_units",
        "decision_tolerance_units"
      ],
      "properties": {
        "components": {
          "type": "array",
          "minItems": 1,
          "maxItems": 16,
          "items": {
            "$ref": "#/$defs/uncertainty_component"
          }
        },
        "correlation_policy": {
          "enum": [
            "independence_declared",
            "correlated_known",
            "correlation_unknown"
          ]
        },
        "coverage_factor_permille": {
          "type": "integer",
          "minimum": 1
        },
        "maximum_expanded_uncertainty_units": {
          "type": "integer",
          "minimum": 1
        },
        "decision_tolerance_units": {
          "type": "integer",
          "minimum": 1
        }
      }
    },
    "decision_boundary": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "decision_tick",
        "maximum_observation_age_ticks",
        "frozen_before_acquisition",
        "future_information_used",
        "policy_id",
        "revision"
      ],
      "properties": {
        "decision_tick": {
          "type": "integer",
          "minimum": 0
        },
        "maximum_observation_age_ticks": {
          "type": "integer",
          "minimum": 0
        },
        "frozen_before_acquisition": {
          "const": true
        },
        "future_information_used": {
          "const": false
        },
        "policy_id": {
          "type": "string",
          "minLength": 1
        },
        "revision": {
          "type": "string",
          "minLength": 1
        }
      }
    }
  }
}