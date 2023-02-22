#ifndef DIRLISTWIDGET_H
#define DIRLISTWIDGET_H

#include <QListWidget>
#include <QDragEnterEvent>
#include <QMimeData>
#include <QFileInfo>

/*
 * Subclass of QListWidget that handles drag and drop. Only allows directories to be dropped in.
 */

class DirListWidget : public QListWidget
{
    Q_OBJECT
public:
    DirListWidget(QWidget *parent = nullptr);

protected:
    void dragEnterEvent(QDragEnterEvent *event) override;
    void dropEvent(QDropEvent *event) override;
    void dragMoveEvent(QDragMoveEvent *event) override;
};

#endif // DIRLISTWIDGET_H
