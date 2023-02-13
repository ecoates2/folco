cmake_minimum_required(VERSION 3.5)

project(Folco VERSION 0.1 LANGUAGES CXX)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(QT NAMES Qt6 Qt5 REQUIRED COMPONENTS Widgets)
find_package(Qt${QT_VERSION_MAJOR} REQUIRED COMPONENTS Widgets)

set(PROJECT_SOURCES
        src/main.cpp
        src/mainwindow.cpp
        inc/mainwindow.h
        src/mainwindow.ui
        inc/iconutils.h
        src/iconutils.cpp
)

if (APPLE)
    set(PLATFORM_SPECIFIC_SOURCES inc/macos/macosiconutils.h src/macos/macosiconutils.mm)
elseif(WIN32)
    set(PLATFORM_SPECIFIC_SOURCES inc/windows/winiconutils.h src/windows/winiconutils.cpp)
endif()

if(${QT_VERSION_MAJOR} GREATER_EQUAL 6)
    if (APPLE)
        set(MACOSX_BUNDLE_ICON_FILE macos_app_icon.icns)
        set(app_icon_resource res/macos_app_icon.icns)

        set_source_files_properties(${app_icon_resource} PROPERTIES
               MACOSX_PACKAGE_LOCATION "Resources")
    elseif(WIN32)
        set(app_icon_resource res/folco.rc)
    endif()

    qt_add_executable(Folco
        MANUAL_FINALIZATION
        ${PROJECT_SOURCES}
        ${PLATFORM_SPECIFIC_SOURCES}
        ${app_icon_resource}
    )

endif()

qt_add_resources(Folco "Folco"
    PREFIX
        "/"
    FILES
        res/icon.svg
)

target_link_libraries(Folco PRIVATE Qt${QT_VERSION_MAJOR}::Widgets)

if (APPLE)
    find_library(COREUI_LIB CoreUI PATHS /System/Library/PrivateFrameworks)
    target_link_libraries(Folco PRIVATE "-framework Foundation" "-framework Cocoa" "-framework ImageIO" ${COREUI_LIB})
elseif(WIN32)
    target_link_libraries(Folco PRIVATE Shlwapi Gdiplus)
endif()

set_target_properties(Folco PROPERTIES
    MACOSX_BUNDLE_GUI_IDENTIFIER my.example.com
    MACOSX_BUNDLE_BUNDLE_VERSION ${PROJECT_VERSION}
    MACOSX_BUNDLE_SHORT_VERSION_STRING ${PROJECT_VERSION_MAJOR}.${PROJECT_VERSION_MINOR}
    MACOSX_BUNDLE TRUE
    WIN32_EXECUTABLE TRUE
)

install(TARGETS Folco
    BUNDLE DESTINATION .
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})

if(QT_VERSION_MAJOR EQUAL 6)
    qt_finalize_executable(Folco)
endif()
