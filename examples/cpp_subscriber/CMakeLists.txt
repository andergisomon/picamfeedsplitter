cmake_minimum_required(VERSION 3.16)
project(frame_subscriber CXX)

set(CMAKE_CXX_STANDARD 17)

# iceoryx2 C bindings
find_package(iceoryx2-cxx QUIET)
if(NOT iceoryx2-cxx_FOUND)
    message(STATUS "iceoryx2-cxx not found, will use pkg-config")
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(ICEORYX2 REQUIRED iceoryx2-cxx)
endif()

# OpenCV (optional, for image processing)
find_package(OpenCV QUIET)

add_executable(frame_subscriber main.cpp)

if(ICEORYX2_FOUND)
    target_include_directories(frame_subscriber PRIVATE ${ICEORYX2_INCLUDE_DIRS})
    target_link_libraries(frame_subscriber ${ICEORYX2_LIBRARIES})
endif()

if(OpenCV_FOUND)
    target_include_directories(frame_subscriber PRIVATE ${OpenCV_INCLUDE_DIRS})
    target_link_libraries(frame_subscriber ${OpenCV_LIBS})
    target_compile_definitions(frame_subscriber PRIVATE WITH_OPENCV)
endif()
