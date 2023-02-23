#include "../inc/mainwindow.h"

#include "../inc/iconutils.h"

#include <QApplication>

int main(int argc, char *argv[])
{
    QApplication a(argc, argv);
    a.setApplicationName("Folco");

    a.setApplicationVersion(QString("%1.%2.%3.%4").arg(QString::number(PROJECT_VERSION_MAJOR), QString::number(PROJECT_VERSION_MINOR),
                                                       QString::number(PROJECT_VERSION_PATCH), QString::number(PROJECT_VERSION_TWEAK)));

    MainWindow w;
    w.setWindowTitle("Folco");
    w.setWindowIcon(QIcon(":/res/icon.svg"));

    w.setFixedSize(430,530);

    w.show();
    return a.exec();
}
