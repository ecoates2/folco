#ifndef ICONUTILS_H
#define ICONUTILS_H

#include <qglobal.h>

#if defined(Q_OS_MACOS)

#include "macos/macosiconutils.h"

#elif defined(Q_OS_WIN)

#include "windows/winiconutils.h"

#endif

#include <QString>
#include <QImage>
#include <QDir>
#include <QFileInfo>
#include <QList>

#include <algorithm>

/*
 * Utility class to handle all platform-specific code relating to extracting default icon sets
 * and modifying the icons of directories.
 */

class IconUtils
{
public:
    static void dumpDefaultFolderIcons(const QString &folderPathIn);
    static void createIconAndApply(const QList<QImage> &images, const QList<QString> &folderPathsIn);
    static void resetFolderIconToDefault(const QList<QString> &folderPaths);
    static QList<QImage> getDefaultFolderIcons(const QString &folderPathIn);
};

#endif // ICONUTILS_H
