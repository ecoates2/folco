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

    /* Right now, we're using a mix of Qt designer and declaring the UI code by hand. This is messy
     * and difficult to read, so i'm considering moving to just code once everything is finished.
     */

    ui->setupUi(this);

    dirListWidget = new DirListWidget(ui->dirListGroup);
    dirListWidget->setObjectName("dirListWidget");

    ui->verticalLayout_2->insertWidget(0, dirListWidget);

    connect(ui->actionExit, &QAction::triggered, this, &QWidget::close);
    connect(ui->actionAbout, &QAction::triggered, this, &MainWindow::about);
    connect(ui->actionAbout_Qt, &QAction::triggered, qApp, &QApplication::aboutQt);

    dirListSelectionModel = dirListWidget->selectionModel();
    connect(dirListSelectionModel, &QItemSelectionModel::selectionChanged, this, [=]() {
        ui->removeDirButton->setEnabled(!dirListSelectionModel->selectedIndexes().isEmpty());
    });

    dirListItemModel = dirListWidget->model();
    connect(dirListItemModel, &QAbstractItemModel::rowsInserted, this, [=]() {
        ui->clearDirsButton->setEnabled(dirListWidget->count() > 0);
    });
    connect(dirListItemModel, &QAbstractItemModel::rowsRemoved, this, [=]() {
        ui->clearDirsButton->setEnabled(dirListWidget->count() > 0);
    });






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

void MainWindow::on_addDirButton_clicked()
{
    QString directory = QFileDialog::getExistingDirectory(this, tr("Select Directory"), QDir::homePath());
    if (!directory.isEmpty() && dirListWidget->findItems(directory, Qt::MatchFixedString | Qt::MatchCaseSensitive).isEmpty()) {
        dirListWidget->addItem(directory);
    }
}


void MainWindow::on_removeDirButton_clicked()
{
    qDeleteAll(dirListWidget->selectedItems());
}


void MainWindow::on_clearDirsButton_clicked()
{
    QList<QListWidgetItem*> itemsToDelete;
    for (int i = 0; i < dirListWidget->count(); ++i) {
        itemsToDelete.append(dirListWidget->item(i));
    }
    qDeleteAll(itemsToDelete);
}

