#include "../inc/dirlistwidget.h"

DirListWidget::DirListWidget(QWidget *parent)
    : QListWidget(parent)
{
    setAcceptDrops(true);
}

void DirListWidget::dragEnterEvent(QDragEnterEvent *event)
{
    bool allDirs = true;
    if (event->mimeData()->hasUrls()) {
        for (const QUrl &url : event->mimeData()->urls()) {
            QFileInfo fileInfo(url.toLocalFile());
            if (!fileInfo.isDir()) {
                allDirs = false;
                break;
            }
        }

        if (allDirs) {
            event->acceptProposedAction();
        } else {
            event->ignore();
        }
    } else {
        event->ignore();
        setCursor(Qt::ForbiddenCursor);
    }
}

void DirListWidget::dropEvent(QDropEvent *event)
{
    for (const QUrl &url : event->mimeData()->urls()) {
        QString directory = url.toLocalFile();
        if (findItems(directory, Qt::MatchFixedString | Qt::MatchCaseSensitive).isEmpty()) {
            addItem(directory);
        }
    }
    event->acceptProposedAction();
}

void DirListWidget::dragMoveEvent(QDragMoveEvent *event)
{
    event->acceptProposedAction();
}
