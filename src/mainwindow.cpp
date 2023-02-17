#include "../inc/mainwindow.h"
#include "./ui_mainwindow.h"

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , ui(new Ui::MainWindow)
{

    QDir dataDir = QDir::toNativeSeparators(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation) + "/" + "default_icons");

    if (!dataDir.exists()) {
        dataDir.mkpath(".");
        IconUtils::dumpDefaultFolderIcons(dataDir.absolutePath());
    }

    defaultIcons = IconUtils::getDefaultFolderIcons(dataDir.absolutePath());

    for (const QImage &image : defaultIcons) {
        qDebug() << image.width();
    }



    ui->setupUi(this);

    connect(ui->actionExit, &QAction::triggered, this, &QWidget::close);
    connect(ui->actionAbout, &QAction::triggered, this, &MainWindow::about);
    connect(ui->actionAbout_Qt, &QAction::triggered, qApp, &QApplication::aboutQt);
}

MainWindow::~MainWindow()
{
    delete ui;
}

void MainWindow::about()
{
    QString contents = (QString("<h2>Folco %1</h2>"
                       "<p>Copyright &copy; 2023 Ethan Coates"
                       "<p>Folco is an open source, cross-platform folder customization utility.").arg(QApplication::instance()->applicationVersion()));
    QMessageBox::about(this, "About Folco",
                contents);
}
