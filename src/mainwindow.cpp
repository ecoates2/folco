#include "../inc/mainwindow.h"
#include "./ui_mainwindow.h"



MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , ui(new Ui::MainWindow)
{

    customizationManager = new CustomizationManager(this);

    /* Right now, we're using a mix of Qt designer and declaring the UI code by hand. This is messy
     * and difficult to read, so i'm considering moving to just code once everything is finished.
     */

    ui->setupUi(this);

    dirListWidget = new DirListWidget(ui->dirListGroup);
    dirListWidget->setObjectName("dirListWidget");

    ui->dirListGroupLayout->insertWidget(0, dirListWidget);

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

    colorPicker = new QColorDialog(this);

    ui->previewLabel->setStyleSheet("QLabel { background-color : white}");

    ui->previewLabel->setFrameStyle(QFrame::StyledPanel);
    ui->previewLabel->setPixmap(customizationManager->getPreview());

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

void MainWindow::on_selectColorButton_clicked()
{
    QColor currColor = customizationManager->getColor();
    QColor newColor = colorPicker->getColor(currColor, this, "Set color");
    customizationManager->usingCustomColor = true;
    customizationManager->setColor(newColor);

    // Update preview here
    ui->previewLabel->setPixmap(customizationManager->getPreview());
}


void MainWindow::on_resetCheckbox_stateChanged(int state)
{
    if (state == Qt::Checked) {
        customizationManager->customizationEnabled = false;
        ui->customizationGroup->setEnabled(false);
    } else {
        customizationManager->customizationEnabled = true;
        ui->customizationGroup->setEnabled(true);
    }

    // Update preview here
    ui->previewLabel->setPixmap(customizationManager->getPreview());
}




void MainWindow::on_cancelButton_clicked()
{
    QApplication::quit();
}


void MainWindow::on_applyButton_clicked()
{
    QList<QString> folderList;

    for (int i = 0; i < dirListWidget->count(); ++i) {
        folderList.append(dirListWidget->item(i)->text());
    }
    customizationManager->applyCustomization(folderList);
    QApplication::quit();
}

