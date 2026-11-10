cmake_minimum_required(VERSION 3.20)
project(scan_bim_embree_adapter VERSION 0.1.0 LANGUAGES CXX)

include(CTest)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(embree 4.4.1 EXACT CONFIG REQUIRED)
find_package(nlohmann_json 3.11.3 EXACT CONFIG REQUIRED)
find_package(OpenSSL 3.0 REQUIRED COMPONENTS Crypto)

if(NOT EMBREE_VERSION STREQUAL "4.4.1")
  message(FATAL_ERROR "Expected Intel Embree 4.4.1, found '${EMBREE_VERSION}'")
endif()

add_executable(scan_bim_embree_adapter main.cpp)
target_include_directories(scan_bim_embree_adapter PRIVATE ${EMBREE_INCLUDE_DIRS})
target_link_libraries(scan_bim_embree_adapter
  PRIVATE
    ${EMBREE_LIBRARIES}
    nlohmann_json::nlohmann_json
    OpenSSL::Crypto
)
target_compile_options(scan_bim_embree_adapter PRIVATE
  "$<$<CXX_COMPILER_ID:GNU,Clang>:-Wall;-Wextra;-Wpedantic;-Wconversion;-Wshadow>"
)
target_compile_definitions(scan_bim_embree_adapter PRIVATE
  SCAN_BIM_EMBREE_VERSION="4.4.1"
  SCAN_BIM_EMBREE_REVISION="f590db83ef6559387df7f6d8725c34fb7acf851d"
)

if(BUILD_TESTING)
  get_filename_component(SCAN_BIM_REPOSITORY_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../.." ABSOLUTE)
  set(SCAN_BIM_FIXTURE_DIR
    "${SCAN_BIM_REPOSITORY_ROOT}/research/scan-bim-rayquery-comparator-001/fixtures")

  add_test(
    NAME embree_known_answers
    COMMAND scan_bim_embree_adapter
      "${SCAN_BIM_FIXTURE_DIR}/ray-query-known-answers-v0-1.json"
      --quality high --threads 1 --repetitions 10
      --workload-class analytic_known_answer
  )
  add_test(
    NAME embree_scale_metamorphism
    COMMAND scan_bim_embree_adapter
      "${SCAN_BIM_FIXTURE_DIR}/ray-query-known-answers-scale-1e3-v0-1.json"
      --quality high --threads 1 --repetitions 10
      --workload-class analytic_known_answer
  )
  add_test(
    NAME embree_overlap_stress
    COMMAND scan_bim_embree_adapter
      "${SCAN_BIM_FIXTURE_DIR}/ray-query-stress-overlap-v0-1.json"
      --quality high --threads 1 --repetitions 10
      --workload-class dense_overlap
  )
endif()
