// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Benchmark-only adapter. This program is intentionally separate from the
// Rust production evaluator and links a pinned external Embree installation.

#include <embree4/rtcore.h>
#include <nlohmann/json.hpp>
#include <openssl/evp.h>

#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <optional>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <tuple>
#include <utility>
#include <vector>

#if defined(__linux__)
#include <sys/utsname.h>
#endif

using json = nlohmann::json;
using Clock = std::chrono::steady_clock;

namespace {

struct Vec3d {
  double x{};
  double y{};
  double z{};
};

struct Identity {
  std::string surface;
  std::string triangle;

  friend bool operator==(const Identity& a, const Identity& b) {
    return a.surface == b.surface && a.triangle == b.triangle;
  }
  friend bool operator<(const Identity& a, const Identity& b) {
    return std::tie(a.surface, a.triangle) < std::tie(b.surface, b.triangle);
  }
};

struct Triangle {
  Identity id;
  std::string kind;
  Vec3d a;
  Vec3d b;
  Vec3d c;
};

struct ExpectedRay {
  std::string status;
  std::optional<double> distance;
  std::vector<Identity> nearest_hits;
};

struct Ray {
  std::string id;
  Vec3d origin;
  Vec3d direction;
  double tnear{};
  double tfar{};
  std::optional<Identity> exclude;
  ExpectedRay expected;
};

struct Fixture {
  std::string fixture_id;
  std::string workload_class;
  double intersection_epsilon{};
  double tie_epsilon{};
  std::vector<Triangle> triangles;
  std::vector<Ray> rays;
  std::string fixture_sha256;
  std::string geometry_sha256;
  std::string ray_set_sha256;
};

struct CapturedHit {
  float distance{};
  unsigned int primitive_id{};
};

struct ExtendedRayQueryContext {
  RTCRayQueryContext embree;
  const std::vector<Triangle>* triangles{};
  std::vector<CapturedHit>* hits{};
  std::vector<std::uint8_t>* seen_primitive{};
  const Identity* exclude{};
  bool invalid_primitive{};
  bool unexpected_packet_size{};
};

static_assert(std::is_standard_layout_v<ExtendedRayQueryContext>);
static_assert(offsetof(ExtendedRayQueryContext, embree) == 0);

struct EmbreeHandles {
  RTCDevice device{};
  RTCScene scene{};
  RTCGeometry geometry{};

  ~EmbreeHandles() {
    if (geometry != nullptr) rtcReleaseGeometry(geometry);
    if (scene != nullptr) rtcReleaseScene(scene);
    if (device != nullptr) rtcReleaseDevice(device);
  }
};

struct Options {
  std::string fixture_path;
  std::string build_quality{"medium"};
  std::string workload_class{"unknown"};
  std::size_t repetitions{1000};
  std::size_t threads{1};
};

Vec3d read_vec3(const json& value, const std::string& field) {
  if (!value.is_array() || value.size() != 3) {
    throw std::runtime_error(field + " must be a three-number array");
  }
  Vec3d out{value[0].get<double>(), value[1].get<double>(), value[2].get<double>()};
  if (!std::isfinite(out.x) || !std::isfinite(out.y) || !std::isfinite(out.z)) {
    throw std::runtime_error(field + " contains a non-finite coordinate");
  }
  return out;
}

double norm(Vec3d value) {
  return std::hypot(std::hypot(value.x, value.y), value.z);
}

float checked_float(double value, const std::string& field) {
  if (!std::isfinite(value) || std::abs(value) > static_cast<double>(std::numeric_limits<float>::max())) {
    throw std::runtime_error(field + " cannot be represented by Embree's float32 API");
  }
  const float converted = static_cast<float>(value);
  if (!std::isfinite(converted)) {
    throw std::runtime_error(field + " overflowed during float32 conversion");
  }
  return converted;
}

std::string sha256(std::string_view bytes) {
  EVP_MD_CTX* raw_context = EVP_MD_CTX_new();
  if (raw_context == nullptr) throw std::runtime_error("EVP_MD_CTX_new failed");
  struct ContextGuard {
    EVP_MD_CTX* value;
    ~ContextGuard() { EVP_MD_CTX_free(value); }
  } guard{raw_context};

  if (EVP_DigestInit_ex(raw_context, EVP_sha256(), nullptr) != 1 ||
      EVP_DigestUpdate(raw_context, bytes.data(), bytes.size()) != 1) {
    throw std::runtime_error("OpenSSL SHA-256 initialization/update failed");
  }
  std::array<unsigned char, EVP_MAX_MD_SIZE> digest{};
  unsigned int digest_length = 0;
  if (EVP_DigestFinal_ex(raw_context, digest.data(), &digest_length) != 1) {
    throw std::runtime_error("OpenSSL SHA-256 finalization failed");
  }
  std::ostringstream out;
  out << std::hex << std::setfill('0');
  for (unsigned int i = 0; i < digest_length; ++i) {
    out << std::setw(2) << static_cast<unsigned int>(digest[i]);
  }
  return out.str();
}

json identity_json(const Identity& id) {
  return json::array({id.surface, id.triangle});
}

Fixture load_fixture(const std::string& path, const std::string& workload_override) {
  std::ifstream input(path, std::ios::binary);
  if (!input) throw std::runtime_error("cannot open fixture: " + path);
  const std::string raw((std::istreambuf_iterator<char>(input)), std::istreambuf_iterator<char>());
  const json root = json::parse(raw);

  if (root.at("fixture_version") != "0.1") throw std::runtime_error("unsupported fixture_version");
  if (root.at("coordinate_system").at("linear_unit") != "metre" ||
      root.at("coordinate_system").at("axis_order") != "xyz" ||
      root.at("coordinate_system").at("handedness") != "right_handed") {
    throw std::runtime_error("only metre / XYZ / right-handed fixtures are supported");
  }
  const auto& contract = root.at("query_contract");
  if (contract.at("direction_normalization") != "unit_length_required" ||
      contract.at("triangle_sidedness") != "two_sided" ||
      contract.at("self_hit_exclusion") != "exact_surface_id_and_triangle_id" ||
      contract.at("expected_hit_policy") != "all_hits_within_nearest_hit_tie_epsilon" ||
      contract.at("coplanar_policy") != "miss") {
    throw std::runtime_error("fixture query contract is unsupported");
  }

  Fixture fixture;
  fixture.fixture_id = root.at("fixture_id").get<std::string>();
  fixture.workload_class = workload_override;
  fixture.intersection_epsilon = contract.at("intersection_epsilon_m").get<double>();
  fixture.tie_epsilon = contract.at("nearest_hit_tie_epsilon_m").get<double>();
  if (!(fixture.intersection_epsilon > 0.0) || !(fixture.tie_epsilon > 0.0)) {
    throw std::runtime_error("fixture metric tolerances must be positive");
  }
  fixture.fixture_sha256 = sha256(raw);

  std::set<Identity> identities;
  for (const auto& item : root.at("triangles")) {
    Triangle triangle;
    triangle.id.surface = item.at("surface_id").get<std::string>();
    triangle.id.triangle = item.at("triangle_id").get<std::string>();
    triangle.kind = item.at("kind").get<std::string>();
    triangle.a = read_vec3(item.at("a"), "triangle.a");
    triangle.b = read_vec3(item.at("b"), "triangle.b");
    triangle.c = read_vec3(item.at("c"), "triangle.c");
    if (triangle.id.surface.empty() || triangle.id.triangle.empty()) {
      throw std::runtime_error("triangle identities must not be empty");
    }
    if (!identities.insert(triangle.id).second) {
      throw std::runtime_error("fixture contains duplicate triangle identities");
    }
    fixture.triangles.push_back(std::move(triangle));
  }
  if (fixture.triangles.empty()) throw std::runtime_error("fixture has no triangles");

  std::vector<const Triangle*> sorted_triangles;
  sorted_triangles.reserve(fixture.triangles.size());
  for (const auto& triangle : fixture.triangles) sorted_triangles.push_back(&triangle);
  std::sort(sorted_triangles.begin(), sorted_triangles.end(),
            [](const Triangle* a, const Triangle* b) { return a->id < b->id; });
  json canonical_geometry = json::array();
  for (const Triangle* triangle : sorted_triangles) {
    canonical_geometry.push_back({
      {"surface_id", triangle->id.surface}, {"triangle_id", triangle->id.triangle},
      {"kind", triangle->kind},
      {"a", {triangle->a.x, triangle->a.y, triangle->a.z}},
      {"b", {triangle->b.x, triangle->b.y, triangle->b.z}},
      {"c", {triangle->c.x, triangle->c.y, triangle->c.z}}
    });
  }
  fixture.geometry_sha256 = sha256(canonical_geometry.dump());

  std::set<std::string> ray_ids;
  json canonical_rays = json::array();
  for (const auto& item : root.at("rays")) {
    Ray ray;
    ray.id = item.at("ray_id").get<std::string>();
    ray.origin = read_vec3(item.at("origin"), "ray.origin");
    ray.direction = read_vec3(item.at("direction"), "ray.direction");
    ray.tnear = item.at("tnear_m").get<double>();
    ray.tfar = item.at("tfar_m").get<double>();
    if (ray.id.empty() || !ray_ids.insert(ray.id).second) {
      throw std::runtime_error("fixture contains an empty or duplicate ray_id");
    }
    if (std::abs(norm(ray.direction) - 1.0) > 1.0e-8 ||
        !std::isfinite(ray.tnear) || !std::isfinite(ray.tfar) ||
        ray.tnear < 0.0 || ray.tfar <= ray.tnear) {
      throw std::runtime_error("ray direction or near/far clipping is invalid");
    }
    if (!item.at("exclude_target").is_null()) {
      ray.exclude = Identity{
        item.at("exclude_target").at(0).get<std::string>(),
        item.at("exclude_target").at(1).get<std::string>()
      };
    }
    const auto& expected = item.at("expected");
    ray.expected.status = expected.at("status").get<std::string>();
    if (!expected.at("nearest_distance_m").is_null()) {
      ray.expected.distance = expected.at("nearest_distance_m").get<double>();
    }
    for (const auto& hit : expected.at("nearest_hits")) {
      ray.expected.nearest_hits.push_back(Identity{hit.at(0).get<std::string>(), hit.at(1).get<std::string>()});
    }
    std::sort(ray.expected.nearest_hits.begin(), ray.expected.nearest_hits.end());

    canonical_rays.push_back({
      {"ray_id", ray.id},
      {"origin", {ray.origin.x, ray.origin.y, ray.origin.z}},
      {"direction", {ray.direction.x, ray.direction.y, ray.direction.z}},
      {"tnear_m", ray.tnear}, {"tfar_m", ray.tfar},
      {"exclude_target", ray.exclude ? identity_json(*ray.exclude) : json(nullptr)}
    });
    fixture.rays.push_back(std::move(ray));
  }
  if (fixture.rays.empty()) throw std::runtime_error("fixture has no rays");
  std::sort(canonical_rays.begin(), canonical_rays.end(),
            [](const json& a, const json& b) { return a.at("ray_id") < b.at("ray_id"); });
  fixture.ray_set_sha256 = sha256(canonical_rays.dump());
  return fixture;
}

RTCBuildQuality build_quality(const std::string& value) {
  if (value == "low") return RTC_BUILD_QUALITY_LOW;
  if (value == "medium") return RTC_BUILD_QUALITY_MEDIUM;
  if (value == "high") return RTC_BUILD_QUALITY_HIGH;
  throw std::runtime_error("build quality must be low, medium or high");
}

void check_device(RTCDevice device, const std::string& operation) {
  const RTCError error = rtcGetDeviceError(device);
  if (error != RTC_ERROR_NONE) {
    throw std::runtime_error(operation + " failed with Embree RTCError=" +
                             std::to_string(static_cast<int>(error)));
  }
}

void gather_all_hits(const RTCFilterFunctionNArguments* args) {
  if (args == nullptr || args->context == nullptr) return;
  auto* context = reinterpret_cast<ExtendedRayQueryContext*>(args->context);
  if (args->N != 1) {
    context->unexpected_packet_size = true;
    for (unsigned int i = 0; i < args->N; ++i) args->valid[i] = 0;
    return;
  }
  if (args->valid[0] != -1) return;

  // Reject every candidate hit so Embree continues traversal and invokes this
  // callback for every intersected primitive, including equal-distance ties.
  args->valid[0] = 0;
  const auto* ray = reinterpret_cast<const RTCRay*>(args->ray);
  const auto* hit = reinterpret_cast<const RTCHit*>(args->hit);
  if (hit->primID >= context->triangles->size()) {
    context->invalid_primitive = true;
    return;
  }
  const Triangle& triangle = (*context->triangles)[hit->primID];
  if (context->exclude != nullptr && triangle.id == *context->exclude) return;
  if ((*context->seen_primitive)[hit->primID] == 0) {
    (*context->seen_primitive)[hit->primID] = 1;
    context->hits->push_back(CapturedHit{ray->tfar, hit->primID});
  }
}

struct QueryResult {
  std::vector<Identity> nearest_ids;
  std::optional<double> nearest_distance;
  double latency_ns{};
};

QueryResult trace_one(RTCDevice device, RTCScene scene,
                      const std::vector<Triangle>& triangles,
                      const Ray& ray, double tie_epsilon) {
  std::vector<CapturedHit> captured;
  captured.reserve(triangles.size());
  std::vector<std::uint8_t> seen(triangles.size(), 0);

  ExtendedRayQueryContext context{};
  rtcInitRayQueryContext(&context.embree);
  context.triangles = &triangles;
  context.hits = &captured;
  context.seen_primitive = &seen;
  context.exclude = ray.exclude ? &*ray.exclude : nullptr;

  RTCRayHit rayhit{};
  rayhit.ray.org_x = checked_float(ray.origin.x, "ray.origin.x");
  rayhit.ray.org_y = checked_float(ray.origin.y, "ray.origin.y");
  rayhit.ray.org_z = checked_float(ray.origin.z, "ray.origin.z");
  rayhit.ray.dir_x = checked_float(ray.direction.x, "ray.direction.x");
  rayhit.ray.dir_y = checked_float(ray.direction.y, "ray.direction.y");
  rayhit.ray.dir_z = checked_float(ray.direction.z, "ray.direction.z");
  rayhit.ray.tnear = checked_float(ray.tnear, "ray.tnear");
  rayhit.ray.tfar = checked_float(ray.tfar, "ray.tfar");
  rayhit.ray.time = 0.0f;
  rayhit.ray.mask = 0xffffffffU;
  rayhit.ray.id = 0;
  rayhit.ray.flags = 0;
  rayhit.hit.geomID = RTC_INVALID_GEOMETRY_ID;
  rayhit.hit.primID = RTC_INVALID_GEOMETRY_ID;
  for (unsigned int& id : rayhit.hit.instID) id = RTC_INVALID_GEOMETRY_ID;

  RTCIntersectArguments args;
  rtcInitIntersectArguments(&args);
  args.context = &context.embree;
  args.filter = gather_all_hits;
  args.flags = RTC_RAY_QUERY_FLAG_INVOKE_ARGUMENT_FILTER | RTC_RAY_QUERY_FLAG_INCOHERENT;

  const auto start = Clock::now();
  rtcIntersect1(scene, &rayhit, &args);
  const auto finish = Clock::now();
  check_device(device, "rtcIntersect1");
  if (context.invalid_primitive) throw std::runtime_error("Embree returned an out-of-range primitive ID");
  if (context.unexpected_packet_size) throw std::runtime_error("Embree filter received a non-single-ray packet");

  QueryResult result;
  result.latency_ns = static_cast<double>(
      std::chrono::duration_cast<std::chrono::nanoseconds>(finish - start).count());
  if (captured.empty()) return result;

  const auto nearest = std::min_element(captured.begin(), captured.end(),
      [](const CapturedHit& a, const CapturedHit& b) { return a.distance < b.distance; });
  result.nearest_distance = static_cast<double>(nearest->distance);
  for (const CapturedHit& hit : captured) {
    if (static_cast<double>(hit.distance) <= *result.nearest_distance + tie_epsilon) {
      result.nearest_ids.push_back(triangles[hit.primitive_id].id);
    }
  }
  std::sort(result.nearest_ids.begin(), result.nearest_ids.end());
  return result;
}

std::string compiler_name() {
#if defined(__clang__)
  return "Clang";
#elif defined(__GNUC__)
  return "GCC";
#elif defined(_MSC_VER)
  return "MSVC";
#else
  return "unknown";
#endif
}

std::string compiler_version() {
#if defined(__clang__)
  return __clang_version__;
#elif defined(__GNUC__)
  return __VERSION__;
#elif defined(_MSC_VER)
  return std::to_string(_MSC_VER);
#else
  return "unknown";
#endif
}

std::string os_name() {
#if defined(__linux__)
  return "linux";
#elif defined(__APPLE__)
  return "macos";
#elif defined(_WIN32)
  return "windows";
#else
  return "unknown";
#endif
}

std::string architecture() {
#if defined(__x86_64__) || defined(_M_X64)
  return "x86_64";
#elif defined(__aarch64__) || defined(_M_ARM64)
  return "aarch64";
#else
  return "unknown";
#endif
}

double percentile(std::vector<double> values, double p) {
  if (values.empty()) return 0.0;
  std::sort(values.begin(), values.end());
  const double rank = std::ceil(p * static_cast<double>(values.size()));
  const std::size_t index = static_cast<std::size_t>(
      std::clamp(rank, 1.0, static_cast<double>(values.size())) - 1.0);
  return values[index];
}

std::vector<Identity> expected_identity_set(const Ray& ray) {
  return ray.expected.nearest_hits;
}

Options parse_options(int argc, char** argv) {
  if (argc < 2) {
    throw std::runtime_error(
      "usage: scan_bim_embree_adapter FIXTURE.json [--quality low|medium|high] "
      "[--threads N] [--repetitions N] [--workload-class CLASS]");
  }
  Options options;
  options.fixture_path = argv[1];
  for (int i = 2; i < argc; ++i) {
    const std::string arg = argv[i];
    auto next = [&]() -> std::string {
      if (i + 1 >= argc) throw std::runtime_error("missing value after " + arg);
      return argv[++i];
    };
    if (arg == "--quality") options.build_quality = next();
    else if (arg == "--threads") options.threads = static_cast<std::size_t>(std::stoull(next()));
    else if (arg == "--repetitions") options.repetitions = static_cast<std::size_t>(std::stoull(next()));
    else if (arg == "--workload-class") options.workload_class = next();
    else throw std::runtime_error("unknown option: " + arg);
  }
  if (options.threads == 0 || options.repetitions == 0) {
    throw std::runtime_error("threads and repetitions must be positive");
  }
  (void)build_quality(options.build_quality);
  return options;
}

json run(const Options& options) {
  Fixture fixture = load_fixture(options.fixture_path, options.workload_class);

  EmbreeHandles handles;
  const std::string device_config = "threads=" + std::to_string(options.threads);
  handles.device = rtcNewDevice(device_config.c_str());
  if (handles.device == nullptr) throw std::runtime_error("rtcNewDevice failed");
  check_device(handles.device, "rtcNewDevice");

  const auto build_start = Clock::now();
  handles.scene = rtcNewScene(handles.device);
  if (handles.scene == nullptr) throw std::runtime_error("rtcNewScene failed");
  rtcSetSceneFlags(handles.scene,
      static_cast<RTCSceneFlags>(RTC_SCENE_FLAG_ROBUST | RTC_SCENE_FLAG_FILTER_FUNCTION_IN_ARGUMENTS));
  rtcSetSceneBuildQuality(handles.scene, build_quality(options.build_quality));

  handles.geometry = rtcNewGeometry(handles.device, RTC_GEOMETRY_TYPE_TRIANGLE);
  if (handles.geometry == nullptr) throw std::runtime_error("rtcNewGeometry failed");
  rtcSetGeometryBuildQuality(handles.geometry, build_quality(options.build_quality));
  rtcSetGeometryEnableFilterFunctionFromArguments(handles.geometry, true);

  struct Float3 { float x; float y; float z; };
  struct UInt3 { unsigned int x; unsigned int y; unsigned int z; };
  auto* vertices = static_cast<Float3*>(rtcSetNewGeometryBuffer(
      handles.geometry, RTC_BUFFER_TYPE_VERTEX, 0, RTC_FORMAT_FLOAT3,
      sizeof(Float3), fixture.triangles.size() * 3));
  auto* indices = static_cast<UInt3*>(rtcSetNewGeometryBuffer(
      handles.geometry, RTC_BUFFER_TYPE_INDEX, 0, RTC_FORMAT_UINT3,
      sizeof(UInt3), fixture.triangles.size()));
  if (vertices == nullptr || indices == nullptr) throw std::runtime_error("Embree geometry buffer allocation failed");

  for (std::size_t i = 0; i < fixture.triangles.size(); ++i) {
    const Triangle& triangle = fixture.triangles[i];
    const std::array<Vec3d, 3> points{triangle.a, triangle.b, triangle.c};
    for (std::size_t j = 0; j < 3; ++j) {
      vertices[i * 3 + j] = Float3{
        checked_float(points[j].x, "triangle.x"),
        checked_float(points[j].y, "triangle.y"),
        checked_float(points[j].z, "triangle.z")
      };
    }
    const unsigned int base = static_cast<unsigned int>(i * 3);
    indices[i] = UInt3{base, base + 1U, base + 2U};
  }

  rtcCommitGeometry(handles.geometry);
  check_device(handles.device, "rtcCommitGeometry");
  (void)rtcAttachGeometry(handles.scene, handles.geometry);
  rtcReleaseGeometry(handles.geometry);
  handles.geometry = nullptr;
  rtcCommitScene(handles.scene);
  check_device(handles.device, "rtcCommitScene");
  const auto build_finish = Clock::now();
  const auto build_time_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(
      build_finish - build_start).count();

  std::vector<double> latencies;
  latencies.reserve(fixture.rays.size() * options.repetitions);
  std::vector<std::string> baseline_results;
  baseline_results.reserve(fixture.rays.size());
  std::vector<std::string> first_results;
  first_results.reserve(fixture.rays.size());
  std::vector<json> mismatches;
  std::size_t matches = 0;
  std::size_t hits = 0;
  std::size_t misses = 0;
  bool repetitions_identical = true;

  const auto query_wall_start = Clock::now();
  for (std::size_t repetition = 0; repetition < options.repetitions; ++repetition) {
    for (std::size_t ray_index = 0; ray_index < fixture.rays.size(); ++ray_index) {
      const Ray& ray = fixture.rays[ray_index];
      QueryResult actual = trace_one(handles.device, handles.scene,
                                     fixture.triangles, ray, fixture.tie_epsilon);
      latencies.push_back(actual.latency_ns);

      json stable = {
        {"distance", actual.nearest_distance ? json(*actual.nearest_distance) : json(nullptr)},
        {"hits", json::array()}
      };
      for (const auto& id : actual.nearest_ids) stable["hits"].push_back(identity_json(id));
      const std::string stable_text = stable.dump();

      if (repetition == 0) first_results.push_back(stable_text);
      else if (first_results[ray_index] != stable_text) repetitions_identical = false;

      if (repetition == 0) {
        if (ray.expected.status == "hit") ++hits;
        else if (ray.expected.status == "miss") ++misses;
        else throw std::runtime_error("unknown expected ray status for " + ray.id);

        bool match = false;
        std::string mismatch_category;
        if (ray.expected.status == "miss") {
          match = !actual.nearest_distance.has_value();
          if (!match) mismatch_category = "false_hit";
        } else if (!actual.nearest_distance.has_value()) {
          mismatch_category = "missed_hit";
        } else {
          if (ray.expected.distance &&
              std::abs(*actual.nearest_distance - *ray.expected.distance) >
                  fixture.intersection_epsilon) {
            mismatch_category = "distance_mismatch";
          } else if (actual.nearest_ids != expected_identity_set(ray)) {
            mismatch_category = ray.expected.nearest_hits.size() > 1
                ? "tie_set_mismatch" : "wrong_nearest";
          } else {
            match = true;
          }
        }
        if (match) ++matches;
        else mismatches.push_back({
          {"ray_id", ray.id},
          {"category", mismatch_category.empty() ? "other" : mismatch_category},
          {"detail", "Embree float32 filter-collected hit set did not match the f64 fixture oracle"}
        });
      }
    }
  }
  const auto query_wall_finish = Clock::now();
  const double query_wall_ns = static_cast<double>(
      std::chrono::duration_cast<std::chrono::nanoseconds>(
        query_wall_finish - query_wall_start).count());
  const std::size_t query_count = fixture.rays.size() * options.repetitions;
  const double qps = query_wall_ns > 0.0
      ? static_cast<double>(query_count) * 1.0e9 / query_wall_ns : 0.0;

  json canonical_outputs = json::array();
  for (const auto& result : first_results) canonical_outputs.push_back(json::parse(result));
  const std::string result_digest = sha256(canonical_outputs.dump());

  std::string adapter_revision;
  if (const char* env = std::getenv("SCAN_BIM_ADAPTER_REVISION")) adapter_revision = env;

  json receipt = {
    {"schema_version", "0.1"},
    {"run_id", fixture.fixture_id + "-embree-" + options.build_quality +
               "-threads-" + std::to_string(options.threads)},
    {"engine", {
      {"engine_id", "intel_embree"},
      {"engine_version", SCAN_BIM_EMBREE_VERSION},
      {"source_revision", SCAN_BIM_EMBREE_REVISION},
      {"adapter_revision", adapter_revision.empty() ? json(nullptr) : json(adapter_revision)},
      {"license", "Apache-2.0"},
      {"precision", "f32"},
      {"build_configuration", {
        {"build_quality", options.build_quality},
        {"device_threads", options.threads},
        {"scene_flags", {"robust", "filter_function_in_arguments"}},
        {"ray_query_flags", {"invoke_argument_filter", "incoherent"},
        {"collect_all_hits", true}
      }}
    }},
    {"input", {
      {"fixture_id", fixture.fixture_id},
      {"fixture_sha256", fixture.fixture_sha256},
      {"geometry_sha256", fixture.geometry_sha256},
      {"ray_set_sha256", fixture.ray_set_sha256},
      {"triangle_count", fixture.triangles.size()},
      {"ray_count", fixture.rays.size()},
      {"workload_class", fixture.workload_class},
      {"coordinate_unit", "metre"}
    }},
    {"environment", {
      {"os", os_name()},
      {"kernel", nullptr},
      {"architecture", architecture()},
      {"cpu_model", nullptr},
      {"cpu_count", std::thread::hardware_concurrency() == 0
          ? json(nullptr) : json(std::thread::hardware_concurrency())},
      {"thread_count", options.threads},
      {"compiler", compiler_name()},
      {"compiler_version", compiler_version()},
      {"compiler_flags", json::array()},
#ifdef NDEBUG
      {"build_profile", "release"}
#else
      {"build_profile", "debug"}
#endif
    }},
    {"build", {
      {"build_time_ns", build_time_ns < 0 ? 0 : build_time_ns},
      {"refit_time_ns", nullptr},
      {"peak_build_memory_bytes", nullptr},
      {"steady_memory_bytes", nullptr}
    }},
    {"query", {
      {"query_count", query_count},
      {"p50_latency_ns", percentile(latencies, 0.50)},
      {"p95_latency_ns", percentile(latencies, 0.95)},
      {"p99_latency_ns", percentile(latencies, 0.99)},
      {"queries_per_second", qps},
      {"primitive_test_count", nullptr},
      {"node_visit_count", nullptr}
    }},
    {"correctness", {
      {"oracle_id", "bruteforce_moller_trumbore_f64_v1"},
      {"rays_checked", fixture.rays.size()},
      {"exact_nearest_identity_matches", matches},
      {"misses", misses},
      {"hits", hits},
      {"mismatches", mismatches}
    }},
    {"determinism", {
      {"repetitions", options.repetitions},
      {"result_digest", result_digest},
      {"all_repetitions_identical", repetitions_identical},
      {"input_permutation_invariant", nullptr}
    }},
    {"qualification", mismatches.empty() && repetitions_identical
        ? "performance_measured" : "rejected"},
    {"claim_scope", "Pinned Intel Embree 4.4.1 float32 run on this fixture, build quality, thread configuration and recorded host/compiler only. No general performance or physical survey claim."}
  };
  return receipt;
}

}  // namespace

int main(int argc, char** argv) {
  try {
    const Options options = parse_options(argc, argv);
    std::cout << run(options).dump(2) << '\n';
    return EXIT_SUCCESS;
  } catch (const std::exception& error) {
    std::cerr << "scan-bim Embree adapter: " << error.what() << '\n';
    return EXIT_FAILURE;
  }
}
