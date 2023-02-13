#ifndef WINICONUTILS_H
#define WINICONUTILS_H

#include <QString>

#include <QDir>
#include <QFileInfo>
#include <QRegularExpression>

#include <QList>
#include <QPixmap>
#include <QTemporaryFile>

#include <QRandomGenerator>

#include <list>
#include <iostream>

#include <windows.h>
#include <shlwapi.h>
#include <shlobj.h>

#include <gdiplus.h>
using namespace Gdiplus;

static const WORD FOLDER_ICON_ID = 4;

#pragma pack( push )
#pragma pack( 2 )
typedef struct
{
    BYTE   bWidth;               // Width, in pixels, of the image
    BYTE   bHeight;              // Height, in pixels, of the image
    BYTE   bColorCount;          // Number of colors in image (0 if >=8bpp)
    BYTE   bReserved;            // Reserved
    WORD   wPlanes;              // Color Planes
    WORD   wBitCount;            // Bits per pixel
    DWORD  dwBytesInRes;         // how many bytes in this resource?
    WORD   nID;                  // the ID
} GRPICONDIRENTRY, *LPGRPICONDIRENTRY;
#pragma pack( pop )

#pragma pack( push )
#pragma pack( 2 )
typedef struct
{
    WORD            idReserved;   // Reserved (must be 0)
    WORD            idType;       // Resource type (1 for icons)
    WORD            idCount;      // How many images?
    GRPICONDIRENTRY idEntries[1]; // The entries for each image
} GRPICONDIR, *LPGRPICONDIR;
#pragma pack( pop )


typedef std::list<GRPICONDIRENTRY> IconDirectory;

typedef std::list<GRPICONDIRENTRY>::const_iterator IconDirectoryCIt;

struct BITMAP_AND_BYTES {
    Gdiplus::Bitmap* bmp;
    int32_t* bytes;
};

namespace {
  template<typename T>
  void write(QFile& f, const T t)
  {
    f.write((const char*)&t, sizeof(t));
  }
}


class WinIconUtils
{
private:
    static IconDirectory GetIconDirectory( HMODULE hMod, WORD Id );
    static HICON LoadSpecificIcon( HMODULE hMod, WORD Id );
    static int GetEncoderClsid(const WCHAR* format, CLSID* pClsid);
    static BITMAP_AND_BYTES createAlphaChannelBitmapFromIcon(HICON hIcon);
    static void cleanUpIconsFromDir(const QString &folderPathIn);
    static bool saveQImagesToICO(const QList<QImage>& images, const QString& path);
    static QString generateRandomString(int length);
    static bool previousIconExists(const QString &folderPathIn);
    static QString previousIconIdentifier(const QString &folderPathIn);

public:
    static void dumpDefaultFolderIcons(const QString &folderPathIn);
    static void createICOAndApply(const QList<QImage>& images, const QString& folderPath);
    static void resetFolderIconToDefault(const QString &folderPath);
};

#endif // WINICONUTILS_H
