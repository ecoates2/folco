#include "../inc/iconutils.h"

#include <QDebug>

void IconUtils::dumpDefaultFolderIcons(const QString &folderPathIn) {

#if defined(Q_OS_MACOS)

MacOSIconUtils::dumpDefaultFolderIcons(folderPathIn);

#elif defined(Q_OS_WIN)

WinIconUtils::dumpDefaultFolderIcons(folderPathIn);

#endif

}

void IconUtils::createIconAndApply(const QList<QImage>& images, const QString &folderPathIn) {

#if defined(Q_OS_MACOS)

MacOSIconUtils::createICNSAndApply(images, folderPathIn);

#elif defined(Q_OS_WIN)

WinIconUtils::createICOAndApply(images, folderPathIn);

#endif

}

void IconUtils::resetFolderIconToDefault(const QString &folderPathIn) {

#if defined(Q_OS_MACOS)

MacOSIconUtils::resetFolderIconToDefault(folderPathIn);

#elif defined(Q_OS_WIN)

WinIconUtils::resetFolderIconToDefault(folderPathIn);

#endif

}

QList<QImage> IconUtils::getDefaultFolderIcons(const QString folderPathIn) {

QList<QImage> images;

// For MacOS, "FolderDark" is the default icon set for now, but a setting can be implemented in the future.

#if defined(Q_OS_MACOS)

QDir dir = folderPathIn + "/" + "FolderDark";

#elif defined(Q_OS_WIN)

QDir dir = folderPathIn + "\\" + "default_icons";

#endif

QStringList pngFiles = dir.entryList(QStringList() << "*.png", QDir::Files);

for (const QString &file : pngFiles)
    {
        QFileInfo fileInfo(dir, file);
        QImage image(fileInfo.absoluteFilePath());
        images.append(image);
    }

std::sort(images.begin(), images.end(), [](const QImage& first, const QImage& second) {
        return first.width() > second.width();
    });

return images;
}
