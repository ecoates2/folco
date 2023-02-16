#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include "../inc/iconutils.h"

#include <QMainWindow>
#include <QStandardPaths>
#include <QDir>

QT_BEGIN_NAMESPACE
namespace Ui { class MainWindow; }
QT_END_NAMESPACE

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    MainWindow(QWidget *parent = nullptr);
    ~MainWindow();

    QList<QImage> defaultIcons;


private:
    Ui::MainWindow *ui;
};
#endif // MAINWINDOW_H
