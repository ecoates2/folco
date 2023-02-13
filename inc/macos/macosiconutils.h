#ifndef MACOSICONUTILS_H
#define MACOSICONUTILS_H

#include <QString>
#include <QImage>

class MacOSIconUtils
{
public:
    static void dumpDefaultFolderIcons(const QString &folderPathIn);
    static void createICNSAndApply(const QList<QImage>& images, const QString& folderPath);
    static void resetFolderIconToDefault(const QString &folderPath);
};

#endif // MACOSICONUTILS_H
