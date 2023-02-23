#include "../../inc/windows/winiconutils.h"

// TODO: Probably should make this a class member...

QRegularExpression getFileNameRegExp()
{
    static QRegularExpression fileNameRegExp("^folco-");
    return fileNameRegExp;
}


IconDirectory WinIconUtils::GetIconDirectory( HMODULE hMod, WORD Id ) {
    HRSRC hRsrc = FindResource( hMod, MAKEINTRESOURCE( Id ), RT_GROUP_ICON );
    HGLOBAL hGlobal = LoadResource( hMod, hRsrc );
    GRPICONDIR* lpGrpIconDir = (GRPICONDIR*)LockResource( hGlobal );

    IconDirectory dir;
    for ( size_t i = 0; i < lpGrpIconDir->idCount; ++i ) {
        dir.push_back( lpGrpIconDir->idEntries[ i ] );
    }
    return dir;
}

HICON WinIconUtils::LoadSpecificIcon( HMODULE hMod, WORD Id ) {
    HRSRC hRsrc = FindResource( hMod, MAKEINTRESOURCE( Id ), RT_ICON );
    HGLOBAL hGlobal = LoadResource( hMod, hRsrc );
    BYTE* lpData = (BYTE*)LockResource( hGlobal );
    DWORD dwSize = SizeofResource( hMod, hRsrc );

    HICON hIcon = CreateIconFromResourceEx( lpData, dwSize, TRUE, 0x00030000,
                                            0, 0, LR_DEFAULTCOLOR );
    return hIcon;
}

int WinIconUtils::GetEncoderClsid(const WCHAR* format, CLSID* pClsid)
{
   UINT  num = 0;          // number of image encoders
   UINT  size = 0;         // size of the image encoder array in bytes

   ImageCodecInfo* pImageCodecInfo = NULL;

   GetImageEncodersSize(&num, &size);
   if(size == 0)
      return -1;  // Failure

   pImageCodecInfo = (ImageCodecInfo*)(malloc(size));
   if(pImageCodecInfo == NULL)
      return -1;  // Failure

   GetImageEncoders(num, size, pImageCodecInfo);

   for(UINT j = 0; j < num; ++j)
   {
      if( wcscmp(pImageCodecInfo[j].MimeType, format) == 0 )
      {
         *pClsid = pImageCodecInfo[j].Clsid;
         free(pImageCodecInfo);
         return j;  // Success
      }
   }

   free(pImageCodecInfo);
   return -1;  // Failure
}

BITMAP_AND_BYTES WinIconUtils::createAlphaChannelBitmapFromIcon(HICON hIcon) {

    // Get the icon info
    ICONINFO iconInfo = {0};
    GetIconInfo(hIcon, &iconInfo);

    // Get the screen DC
    HDC dc = GetDC(NULL);

    // Get icon size info
    BITMAP bm = {0};
    GetObject( iconInfo.hbmColor, sizeof( BITMAP ), &bm );

    // Set up BITMAPINFO
    BITMAPINFO bmi = {0};
    bmi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bmi.bmiHeader.biWidth = bm.bmWidth;
    bmi.bmiHeader.biHeight = -bm.bmHeight;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    // Extract the color bitmap
    int nBits = bm.bmWidth * bm.bmHeight;
    int32_t* colorBits = new int32_t[nBits];
    GetDIBits(dc, iconInfo.hbmColor, 0, bm.bmHeight, colorBits, &bmi, DIB_RGB_COLORS);

    // Check whether the color bitmap has an alpha channel.
    BOOL hasAlpha = FALSE;
    for (int i = 0; i < nBits; i++) {
        if ((colorBits[i] & 0xff000000) != 0) {
            hasAlpha = TRUE;
            break;
        }
    }

    // If no alpha values available, apply the mask bitmap
    if (!hasAlpha) {
        // Extract the mask bitmap
        int32_t* maskBits = new int32_t[nBits];
        GetDIBits(dc, iconInfo.hbmMask, 0, bm.bmHeight, maskBits, &bmi, DIB_RGB_COLORS);
        // Copy the mask alphas into the color bits
        for (int i = 0; i < nBits; i++) {
            if (maskBits[i] == 0) {
                colorBits[i] |= 0xff000000;
            }
        }
        delete[] maskBits;
    }

    // Release DC and GDI bitmaps
    ReleaseDC(NULL, dc);
    ::DeleteObject(iconInfo.hbmColor);
    ::DeleteObject(iconInfo.hbmMask);

    // Create GDI+ Bitmap
    Gdiplus::Bitmap* bmp = new Gdiplus::Bitmap(bm.bmWidth, bm.bmHeight, bm.bmWidth*4, PixelFormat32bppARGB, (BYTE*)colorBits);
    BITMAP_AND_BYTES ret = {bmp, colorBits};

    return ret;
}

void WinIconUtils::dumpDefaultFolderIcons(const QString &folderPathIn) {

    wchar_t system_directory[MAX_PATH];
    GetSystemDirectoryW(system_directory, MAX_PATH);

    std::wstring dll_path = system_directory;
    dll_path += L"\\shell32.dll";

    Gdiplus::GdiplusStartupInput gdiplusStartupInput;
    ULONG_PTR gdiplusToken;
    GdiplusStartup(&gdiplusToken, &gdiplusStartupInput, NULL);

    CLSID g_pngClsid;

    GetEncoderClsid(L"image/png", &g_pngClsid);


    HMODULE hMod = LoadLibraryExW( dll_path.c_str(),
                                   NULL, LOAD_LIBRARY_AS_IMAGE_RESOURCE);

    IconDirectory dir = GetIconDirectory( hMod, FOLDER_ICON_ID );

    int icons_processed = 0;

    for ( IconDirectoryCIt it = dir.begin(); it != dir.end(); ++it ) {

        std::wstring write_this_ico_to = (folderPathIn + "\\").toStdWString();

        write_this_ico_to.append(std::to_wstring(icons_processed));
        write_this_ico_to.append(L".png");

        HICON hIcon = LoadSpecificIcon( hMod, it->nID );
        BITMAP_AND_BYTES bbs = createAlphaChannelBitmapFromIcon(hIcon);

        IStream* fstrm = NULL;
        SHCreateStreamOnFile(write_this_ico_to.c_str(), STGM_WRITE|STGM_CREATE, &fstrm);
        bbs.bmp->Save(fstrm, &g_pngClsid, NULL);
        fstrm->Release();

        delete bbs.bmp;
        delete[] bbs.bytes;
        DestroyIcon(hIcon);

        icons_processed++;

    }

    Gdiplus::GdiplusShutdown(gdiplusToken);

}

void WinIconUtils::createICOAndApply(const QList<QImage>& images, const QList<QString> &folderPathsIn) {

    QList<QString> deleteFiles;

    for (const QString &folder : folderPathsIn) {

        QDir directory(folder);
        QStringList filters;
        filters << "*.ico";
        directory.setNameFilters(filters);

        directory.setFilter(QDir::Files | QDir::System | QDir::Hidden);
        QFileInfoList fileInfoList = directory.entryInfoList();

        for (const QFileInfo &fileInfo : fileInfoList) {
            QRegularExpressionMatch match = getFileNameRegExp().match(fileInfo.baseName());
            if (match.hasMatch()) {
                deleteFiles.append(fileInfo.absoluteFilePath());
            }
        }

        // From testing, it's necessary for an updated icon to have a different name from the previous one, or else it'll have trouble updating the change.

        QString currIconIdentifier = QString("folco-%1-%2").arg(QDateTime::currentDateTime().toMSecsSinceEpoch()).arg(generateRandomString(6));

        const QString icoPath = folder + "\\" + currIconIdentifier + ".ico";

        // Create the icon file

        saveQImagesToICO(images, icoPath);

        std::string desktopIniPath = (folder + "\\" + "desktop.ini").toStdString();

        std::wstring folderPathW = folder.toStdWString();

        std::wstring icoPathW = icoPath.toStdWString();

        // Create an empty desktop.ini and notify the Windows shell that it exists. Necessary so that the newly applied icon updates immediately.

        SHFOLDERCUSTOMSETTINGS fcs_noicon;

        fcs_noicon.dwSize = sizeof(fcs_noicon);
        fcs_noicon.dwMask = FCSM_ICONFILE;
        fcs_noicon.pszIconFile = NULL;
        fcs_noicon.cchIconFile = 0;
        fcs_noicon.iIconIndex = 0;

        SHGetSetFolderCustomSettings(&fcs_noicon, folderPathW.c_str(), FCS_FORCEWRITE);

        /* SHCNF_FLUSH can be used as a blocking wait for an explorer refresh to complete.
        / From testing, this is necessary for folders that have preview icons to update immediately.
        / One disadvantage of this is that it locks up the UI and takes a long time for a list
        / of folders to update.
        */

        // TODO/Goal: Find a way to instantly update any kind of folder icon, without freezing execution.


        SHChangeNotify(SHCNE_CREATE, SHCNF_PATH | SHCNF_FLUSH, desktopIniPath.c_str(), NULL);

        // Overwrite with pszIconFile pointing to the new icon, notify again

        SHFOLDERCUSTOMSETTINGS fcs;

        fcs.dwSize = sizeof(fcs);
        fcs.dwMask = FCSM_ICONFILE;
        fcs.pszIconFile = icoPathW.data();
        fcs.cchIconFile = 0;
        fcs.iIconIndex = 0;

        SHGetSetFolderCustomSettings(&fcs, folderPathW.c_str(), FCS_FORCEWRITE);

        SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATH, desktopIniPath.c_str(), NULL);
    }

    // Remove all old icons

    for (const QString &file : deleteFiles) {
        QFile delFile(file);
        delFile.remove();
    }
}

void WinIconUtils::resetFolderIconToDefault(const QList<QString> &folderPathsIn) {

    for (const QString &folder : folderPathsIn) {
        std::string desktopIniPath = (folder + "\\" + "desktop.ini").toStdString();

        std::wstring folderPathW = folder.toStdWString();

        // Nullify pszIconFile

        SHFOLDERCUSTOMSETTINGS fcs;

        fcs.dwSize = sizeof(fcs);
        fcs.dwMask = FCSM_ICONFILE;
        fcs.pszIconFile = NULL;
        fcs.cchIconFile = 0;
        fcs.iIconIndex = 0;

        SHGetSetFolderCustomSettings(&fcs, folderPathW.c_str(), FCS_FORCEWRITE);

        // Note: Don't remove desktop.ini altogether, since it could store other unrelated folder settings

        cleanUpIconsFromDir(folder);

        SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATH, desktopIniPath.c_str(), NULL);
    }
}

void WinIconUtils::cleanUpIconsFromDir(const QString &folderPathIn) {

    QDir directory(folderPathIn);
    QStringList filters;
    filters << "*.ico";
    directory.setNameFilters(filters);

    directory.setFilter(QDir::Files | QDir::System | QDir::Hidden);
    QFileInfoList fileInfoList = directory.entryInfoList();

    for (const QFileInfo &fileInfo : fileInfoList) {
        QRegularExpressionMatch match = getFileNameRegExp().match(fileInfo.baseName());
        if (match.hasMatch()) {
            QFile file(fileInfo.filePath());
            file.remove();
        }
    }
}

QString WinIconUtils::generateRandomString(int length) {
    const QString characters("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");
    QString result;
    for (int i = 0; i < length; ++i) {
        result.append(characters.at(QRandomGenerator::global()->bounded(characters.length())));
    }
    return result;
}

bool WinIconUtils::previousIconExists(const QString &folderPathIn) {
    QDir directory(folderPathIn);
    QStringList filters;
    filters << "*.ico";
    directory.setNameFilters(filters);

    directory.setFilter(QDir::Files | QDir::System | QDir::Hidden);
    QFileInfoList fileInfoList = directory.entryInfoList();

    bool iconExists = false;
    for (const QFileInfo &fileInfo : fileInfoList) {
        QRegularExpressionMatch match = getFileNameRegExp().match(fileInfo.baseName());
        if (match.hasMatch()) {
            iconExists = true;
        }
    }

    return iconExists;
}

QString WinIconUtils::previousIconIdentifier(const QString &folderPathIn) {
    QString prevIconID;

    QDir directory(folderPathIn);
    QStringList filters;
    filters << "*.ico";
    directory.setNameFilters(filters);

    directory.setFilter(QDir::Files | QDir::System | QDir::Hidden);
    QFileInfoList fileInfoList = directory.entryInfoList();

    if (fileInfoList.size() > 0) {
        prevIconID = fileInfoList[0].baseName();
    }

    return prevIconID;
}

// Adapted from code provided by cbuchart: https://github.com/cbuchart/stackoverflow/tree/master/54288411-save-a-list-of-qpixmaps-to-ico-file
bool WinIconUtils::saveQImagesToICO(const QList<QImage>& images, const QString& path)
{
    static_assert(sizeof(short) == 2, "short int is not 2 bytes");
    static_assert(sizeof(int) == 4, "int is not 4 bytes");

    QFile f(path);
    if (!f.open(QFile::OpenModeFlag::WriteOnly)) return false;

    // Header
    write<short>(f, 0);
    write<short>(f, 1);
    write<short>(f, images.count());

    QList<int> images_size;
    for (int ii = 0; ii < images.count(); ++ii) {
      QTemporaryFile temp;
      temp.setAutoRemove(true);
      if (!temp.open()) return false;

      const auto& image = images[ii];
      image.save(&temp, "PNG");

      temp.close();

      images_size.push_back(QFileInfo(temp).size());
    }

    // Images directory
    constexpr unsigned int entry_size = sizeof(char) + sizeof(char) + sizeof(char) + sizeof(char) + sizeof(short) + sizeof(short) + sizeof(unsigned int) + sizeof(unsigned int);
    static_assert(entry_size == 16, "wrong entry size");

    unsigned int offset = 3 * sizeof(short) + images.count() * entry_size;
    for (int ii = 0; ii < images.count(); ++ii) {
      const auto& image = images[ii];
      if (image.width() > 256 || image.height() > 256) continue;

      write<char>(f, image.width() == 256 ? 0 : image.width());
      write<char>(f, image.height() == 256 ? 0 : image.height());
      write<char>(f, 0); // palette size
      write<char>(f, 0); // reserved
      write<short>(f, 1); // color planes
      write<short>(f, image.depth()); // bits-per-pixel
      write<unsigned int>(f, images_size[ii]); // size of image in bytes
      write<unsigned int>(f, offset); // offset
      offset += images_size[ii];
    }

    for (int ii = 0; ii < images.count(); ++ii) {
      const auto& image = images[ii];
      if (image.width() > 256 || image.height() > 256) continue;
      image.save(&f, "PNG");
    }

    // Make the ICO a hidden system file. That way, it won't show up in explorer, even with "view hidden files/folders" enabled.

    if (f.exists()) {
        std::wstring pathW = path.toStdWString();
        DWORD attribs = GetFileAttributes(pathW.c_str());
        SetFileAttributes(pathW.c_str(), attribs | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM);
    }

    return true;
}
