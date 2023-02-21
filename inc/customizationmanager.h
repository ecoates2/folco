#ifndef CUSTOMIZATIONMANAGER_H
#define CUSTOMIZATIONMANAGER_H

#include <QObject>
#include <QColor>
#include <QStandardPaths>
#include <QPixmap>

#include "iconutils.h"

class CustomizationManager: public QObject
{
Q_OBJECT
public:
    CustomizationManager(QObject *parent = nullptr);
    void applyCustomization(QList<QString> &folders);
    void setColor(QColor &colorIn);
    QColor getColor();
    QPixmap getPreview();
    bool customizationEnabled;
    bool usingCustomColor;
private:
    QList<QImage> defaultIcons;
    QList<QImage> grayscaleAndAdjustedIcons;
    QColor color;

    QImage convertToGrayScale(const QImage &srcImage);
    void adjustContrast(QImage &image, int contrast);
    void colorize(QImage& inoutImage, const QColor& tintColor);
};

#endif // CUSTOMIZATIONMANAGER_H
