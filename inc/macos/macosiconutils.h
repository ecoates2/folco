#ifndef MACOSICONUTILS_H
#define MACOSICONUTILS_H

#include <QString>
#include <QImage>

class MacOSIconUtils
{
public:
    static void dumpDefaultFolderIcons(const QString &folderPathIn);
    static void createICNSAndApply(const QList<QImage>& images, const QList<QString>& folderPaths);
    static void resetFolderIconToDefault(const QList<QString> &folderPath);
};

#endif // MACOSICONUTILS_H
