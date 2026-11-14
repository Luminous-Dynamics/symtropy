// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// MuJoCo 4.0.1 matched operator capture for DUMBBELL-DENSE-REBASIS-001.
// This is an operator-level adapter: it captures M, J, Y and Delassus
// provenance. It does not claim to match Symtropy's midpoint integrator.

#include <mujoco/mujoco.h>

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum { kNv = 6, kMaxError = 2048 };

static int fail(const char* message) {
  fprintf(stderr, "mujoco-adapter: %s\n", message);
  return EXIT_FAILURE;
}

static int load_dense_sparse_rows(
    const int* row_nnz, const int* row_adr, const int* colind,
    const mjtNum* sparse_values, int rows, int columns, mjtNum* dense) {
  if (!row_nnz || !row_adr || !colind || !sparse_values || !dense) {
    return 0;
  }
  for (int row = 0; row < rows; ++row) {
    for (int offset = 0; offset < row_nnz[row]; ++offset) {
      const int address = row_adr[row] + offset;
      const int column = colind[address];
      if (column < 0 || column >= columns) {
        return 0;
      }
      dense[row * columns + column] = sparse_values[address];
    }
  }
  return 1;
}

static int cholesky6(const mjtNum* matrix, mjtNum* lower) {
  memset(lower, 0, kNv * kNv * sizeof(*lower));
  for (int row = 0; row < kNv; ++row) {
    for (int column = 0; column <= row; ++column) {
      mjtNum value = matrix[row * kNv + column];
      for (int inner = 0; inner < column; ++inner) {
        value -= lower[row * kNv + inner] * lower[column * kNv + inner];
      }
      if (row == column) {
        if (!(value > 0.0) || !isfinite(value)) {
          return 0;
        }
        lower[row * kNv + column] = sqrt(value);
      } else {
        lower[row * kNv + column] = value / lower[column * kNv + column];
      }
    }
  }
  return 1;
}

static int solve_spd6(
    const mjtNum* lower, const mjtNum* rhs, mjtNum* solution) {
  mjtNum intermediate[kNv] = {0};
  for (int row = 0; row < kNv; ++row) {
    mjtNum value = rhs[row];
    for (int column = 0; column < row; ++column) {
      value -= lower[row * kNv + column] * intermediate[column];
    }
    intermediate[row] = value / lower[row * kNv + row];
  }
  for (int row = kNv - 1; row >= 0; --row) {
    mjtNum value = intermediate[row];
    for (int column = row + 1; column < kNv; ++column) {
      value -= lower[column * kNv + row] * solution[column];
    }
    solution[row] = value / lower[row * kNv + row];
  }
  for (int i = 0; i < kNv; ++i) {
    if (!isfinite(solution[i])) {
      return 0;
    }
  }
  return 1;
}

static mjtNum dot6(const mjtNum* left, const mjtNum* right) {
  mjtNum result = 0.0;
  for (int i = 0; i < kNv; ++i) {
    result += left[i] * right[i];
  }
  return result;
}

static void print_array(const mjtNum* values, int count) {
  putchar('[');
  for (int i = 0; i < count; ++i) {
    if (i) {
      putchar(',');
    }
    printf("%.17g", (double)values[i]);
  }
  putchar(']');
}

int main(int argc, char** argv) {
  if (argc != 3) {
    fprintf(stderr, "usage: mujoco-dumbbell-capture <model.xml> <result.json>\n");
    return EXIT_FAILURE;
  }

  char error[kMaxError] = {0};
  mjModel* model = mj_loadXML(argv[1], NULL, error, sizeof(error));
  if (!model) {
    fprintf(stderr, "mujoco-adapter: model load failed: %s\n", error);
    return EXIT_FAILURE;
  }

  mjData* data = mj_makeData(model);
  if (!data) {
    mj_deleteModel(model);
    return fail("mj_makeData returned NULL");
  }

  // Use a dual solver so 4.0.1 allocates efc_Y; also request its exact
  // constraint-space inertia diagonal. Capture the current position only.
  model->opt.solver = mjSOL_PGS;
  model->opt.enableflags |= mjENBL_DIAGEXACT;
  model->opt.timestep = 0.02;
  for (int i = 0; i < kNv; ++i) {
    data->qfrc_applied[i] = 0.0;
  }
  data->qfrc_applied[0] = 0.18;
  data->qfrc_applied[1] = -0.07;
  data->qfrc_applied[2] = 0.11;

  mj_forward(model, data);

  if (model->nq != kNv || model->nv != kNv || model->nu != 0 ||
      model->neq != 1 || model->ntendon != 1 ||
      data->nefc != 1 || data->ncon != 0) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("compiled model does not match the declared 6-DOF, one-equality, no-contact fixture");
  }
  if (model->opt.solver != mjSOL_PGS ||
      !(model->opt.enableflags & mjENBL_DIAGEXACT)) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("PGS and exact Delassus diagonal were not enabled");
  }
  if (!data->efc_Y || !data->efc_Y_rownnz || !data->efc_Y_rowadr ||
      !data->efc_Y_colind || !data->efc_J || !data->efc_J_rownnz ||
      !data->efc_J_rowadr || !data->efc_J_colind || !data->efc_diagA ||
      !data->efc_R || !data->efc_force) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("required sparse J/Y or native Delassus diagnostic arrays were not allocated");
  }

  mjtNum mass[kNv * kNv] = {0};
  mj_fullM(model, data, mass);

  mjtNum jacobian[kNv] = {0};
  mjtNum whitened[kNv] = {0};
  if (!load_dense_sparse_rows(
          data->efc_J_rownnz, data->efc_J_rowadr, data->efc_J_colind,
          data->efc_J, 1, kNv, jacobian) ||
      !load_dense_sparse_rows(
          data->efc_Y_rownnz, data->efc_Y_rowadr, data->efc_Y_colind,
          data->efc_Y, 1, kNv, whitened)) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("failed to expand sparse constraint rows");
  }

  mjtNum lower[kNv * kNv] = {0};
  if (!cholesky6(mass, lower)) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("dense mass matrix is not finite positive definite");
  }

  mjtNum correction[kNv] = {0};
  mjtNum direct_delassus = 0.0;
  if (!solve_spd6(lower, jacobian, correction)) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("independent Cholesky solve of M x = J^T failed");
  }
  direct_delassus = dot6(jacobian, correction);
  const mjtNum whitened_delassus = dot6(whitened, whitened);
  const mjtNum denominator =
      fmax(1.0, fmax(fabs(direct_delassus), fabs(whitened_delassus)));
  const mjtNum relative_defect =
      fabs(direct_delassus - whitened_delassus) / denominator;

  if (!isfinite(relative_defect) || relative_defect > 1.0e-10) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("independent direct and whitened Delassus values disagree");
  }
  if (fabs(data->efc_pos[0]) > 1.0e-10) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("fixed-link equality is not at its declared initial length");
  }

  FILE* output = fopen(argv[2], "wb");
  if (!output) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("cannot open result JSON for writing");
  }

  fprintf(output,
      "{\"schema_version\":\"luminous.mujoco_operator_capture.v0.1\","
      "\"benchmark_case_id\":\"DUMBBELL-DENSE-REBASIS-001\","
      "\"engine_version\":\"%s\","
      "\"engine_release_tag\":\"4.0.1\","
      "\"engine_release_commit\":\"4ed093a9f06b170bd1564942fa37e1bb907d93e3\","
      "\"comparison_scope\":\"mass-jacobian-delassus-operator-only\","
      "\"nv\":%d,\"nq\":%d,\"nefc\":%d,\"ncon\":%d,"
      "\"neq\":%d,\"ntendon\":%d,"
      "\"solver\":\"PGS\",\"diagexact_enabled\":true,"
      "\"jacobian_storage\":\"sparse-csr\","
      "\"jacobian_row_nnz\":%d,\"whitened_row_nnz\":%d,"
      "\"mass_matrix\":",
      mj_versionString(), (int)model->nv, (int)model->nq, data->nefc, data->ncon,
      (int)model->neq, (int)model->ntendon,
      data->efc_J_rownnz[0], data->efc_Y_rownnz[0]);

  print_array(mass, kNv * kNv);
  fprintf(output, ",\"constraint_jacobian\":");
  print_array(jacobian, kNv);
  fprintf(output, ",\"whitened_jacobian\":");
  print_array(whitened, kNv);
  fprintf(output, ",\"force_covector\":[0.18,-0.07,0.11,0,0,0]");
  fprintf(output, ",\"direct_delassus\":%.17g", (double)direct_delassus);
  fprintf(output, ",\"whitened_delassus\":%.17g", (double)whitened_delassus);
  fprintf(output, ",\"native_exact_diagA\":%.17g", (double)data->efc_diagA[0]);
  fprintf(output, ",\"native_efc_R\":%.17g", (double)data->efc_R[0]);
  fprintf(output, ",\"native_constraint_force\":%.17g", (double)data->efc_force[0]);
  fprintf(output, ",\"constraint_position_residual\":%.17g", (double)data->efc_pos[0]);
  fprintf(output, ",\"direct_whitened_relative_defect\":%.17g", (double)relative_defect);
  fprintf(output, ",\"model_state_qpos\":");
  print_array(data->qpos, (int)model->nq);
  fprintf(output, ",\"model_state_qvel\":");
  print_array(data->qvel, (int)model->nv);
  fprintf(output, "}\n");

  if (fclose(output) != 0) {
    mj_deleteData(data);
    mj_deleteModel(model);
    return fail("failed to flush result JSON");
  }

  mj_deleteData(data);
  mj_deleteModel(model);
  return EXIT_SUCCESS;
}
