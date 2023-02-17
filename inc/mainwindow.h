#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include "../inc/iconutils.h"

// Generated by CMake
#include "version.h"

#include <QMainWindow>
#include <QStandardPaths>
#include <QDir>

#include <QMessageBox>



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

private slots:
    void about();

private:
    Ui::MainWindow *ui;
};
#endif // MAINWINDOW_H