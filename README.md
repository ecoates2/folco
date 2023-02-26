![alt text](https://github.com/ecoates2/folco/blob/main/images/logo.png?raw=true)

# Folco
An open-source, cross-platform application to customize folder icons

![alt text](https://github.com/ecoates2/folco/blob/main/images/screenshot_windows.png?raw=true)

# Features
* Set custom colors for either single folders or for an entire group
  * Drag and drop support from Explorer/Finder
* Automatic extraction of base folder icons on MacOS and Windows; modified folders will have an identical look and feel to their default counterparts
  
# Usage
Folco currently supports Windows 10/11 (64 bit) and MacOS 11 Big Sur and up (64 bit).

Right now, the app functions exclusively via the GUI. Open the app and either pick directories from
the "Add" button or by dragging and dropping from Explorer/Finder. To colorize the folder(s), select a color using the
"Color" button. Otherwise, to reset the folder(s) to default, hit the "Reset Folder(s)" checkbox. Finally,
hit "Apply" to apply any changes.

# Planned Features
* Register context menu functionality for Windows
* Command line/scripting capabilities
* Implement more advanced customization options, such as textures, patterns, overlays and gradients
* Add overlay icons (things like checkmarks)
* Possibly add support for some Linux desktop environments
* Clean up the codebase and optimize performance

# Development
Check [BUILD.md](BUILD.md) for build instructions.

# About
The overall goal of this project is to provide a free and open-source solution for customizing folder icons. All of the current existing applications that accomplish this are closed-source.

Folco is written in C++ using the Qt framework. The backend code (i.e. all operations involving dumping assets and setting icons)
is written in C++ on Windows (utilizing the Win32 API), and is written in Objective C on MacOS.
