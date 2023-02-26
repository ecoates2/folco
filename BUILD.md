# Prerequisites
* A stock installation of Qt for desktop development
  * This is bundled with CMake and Ninja
* Git

Make sure that you have both CMake and Ninja added to your path on Windows and MacOS. 

# Windows
1. Clone the repo to a directory of your choosing and create a build directory.
2. Run the command line shortcut for your version of Qt (Ex. Qt 6.4.2 (MinGW 11.2.0 64-bit) in the start menu).
3. Configure a release build:
```
qt-cmake -S <source-dir> -B <build-dir> -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=g++
```
4. Build and deploy:
```
cmake --build <build-dir>
cd <build-dir>
mkdir bin
copy Folco.exe bin
windeployqt bin
```

# MacOS
1. Clone the repo to a directory of your choosing and create a build directory.
2. Open a terminal window.
3. Configure a release build:
```
cd Qt/<version-number>/macos/bin
./qt-cmake -S <source-dir> -B <build-dir> -G Ninja -DCMAKE_BUILD_TYPE=Release
```
4. Build and deploy:
```
./macdeployqt <build-dir>/Folco.app -dmg
```
