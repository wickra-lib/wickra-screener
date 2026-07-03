# Build and run the C and C++ examples against the wickra-screener C ABI.
#
#   cargo build --release -p wickra-screener-c
#   cmake -S examples/c -B examples/c/build
#   cmake --build examples/c/build --config Release
#   ctest --test-dir examples/c/build -C Release --output-on-failure
#
# WICKRA_SCREENER_LIB_DIR defaults to the workspace target/release directory and
# can be overridden for out-of-tree builds.

cmake_minimum_required(VERSION 3.15)
project(wickra_screener_examples C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(WICKRA_SCREENER_LIB_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../target/release"
    CACHE PATH "Directory containing the built wickra-screener C ABI library")

set(WICKRA_SCREENER_INCLUDE "${CMAKE_CURRENT_SOURCE_DIR}/../../bindings/c/include")

find_library(WICKRA_SCREENER_LIB
    NAMES wickra_screener libwickra_screener
    HINTS "${WICKRA_SCREENER_LIB_DIR}"
    REQUIRED)

add_executable(scan scan.c)
target_include_directories(scan PRIVATE "${WICKRA_SCREENER_INCLUDE}")
target_link_libraries(scan PRIVATE "${WICKRA_SCREENER_LIB}")

add_executable(scanpp scan.cpp)
target_include_directories(scanpp PRIVATE "${WICKRA_SCREENER_INCLUDE}")
target_link_libraries(scanpp PRIVATE "${WICKRA_SCREENER_LIB}")

# On Windows the runtime DLL must sit next to the executable (there is no rpath);
# without it the loader pops a modal "DLL not found" dialog that hangs ctest. Copy
# it next to each example after build. (Unix resolves the .so/.dylib via the
# linker's embedded path, so this is Windows-only.)
if(WIN32)
    foreach(example scan scanpp)
        add_custom_command(TARGET ${example} POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${WICKRA_SCREENER_LIB_DIR}/wickra_screener.dll"
                "$<TARGET_FILE_DIR:${example}>"
            VERBATIM)
    endforeach()
endif()

enable_testing()
add_test(NAME scan COMMAND scan)
add_test(NAME scanpp COMMAND scanpp)
# A missing runtime dependency must fail fast, never block the runner for hours.
set_tests_properties(scan scanpp PROPERTIES TIMEOUT 60)
