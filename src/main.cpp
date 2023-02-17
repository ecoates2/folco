#include "../inc/mainwindow.h"

#include "../inc/iconutils.h"

#include <QApplication>

#include <QDebug>

int main(int argc, char *argv[])
{
    QApplication a(argc, argv);
    a.setApplicationName("Folco");

    a.setApplicationVersion(QString("%1.%2.%3.%4").arg(QString::number(PROJECT_VERSION_MAJOR), QString::number(PROJECT_VERSION_MINOR),
                                                       QString::number(PROJECT_VERSION_PATCH), QString::number(PROJECT_VERSION_TWEAK)));

    MainWindow w;
    w.setWindowTitle("Folco");
    w.setWindowIcon(QIcon(":/res/icon.svg"));











    // TEST CODE

    //QString directory = "/Users/johndoe/Downloads/Macos icon sizes/MyIcon.iconset";

    QString directory = "C:\\Users\\welma\\Desktop\\windows_icon_test_source";

      QDir dir2(directory);
      QFileInfoList fileList = dir2.entryInfoList(QDir::Files);
      QList<QImage> images;

      for (int i = 0; i < fileList.size(); ++i) {
        QFileInfo fileInfo = fileList.at(i);
        QImage image;
        if (image.load(fileInfo.absoluteFilePath())) {
          images.append(image);
        }
      }

    //QList<QImage> defaultSet = IconUtils::getDefaultFolderIcons(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation));

    IconUtils::createIconAndApply(images, QDir::toNativeSeparators("C:\\Users\\welma\\Desktop\\colortest5"));

    //IconUtils::resetFolderIconToDefault(QDir::toNativeSeparators("C:\\Users\\welma\\Desktop\\colortest5"));


    //IconUtils::dumpDefaultFolderIcons(data_directory);


    w.show();
    return a.exec();
}
