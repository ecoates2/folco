#ifndef CUSTOMIZATIONMANAGER_H
#define CUSTOMIZATIONMANAGER_H

#include <QObject>
#include <QColor>
#include <QStandardPaths>
#include <QPixmap>

#include "iconutils.h"

#if defined(Q_OS_MACOS)

#define CONTRAST_ADJUSTMENT 25
#define BRIGHTNESS_ADJUSTMENT 45

#elif defined(Q_OS_WIN)

#define CONTRAST_ADJUSTMENT 35

#endif

/* Class that handles image modifications and passing off image data to
 * the platform-specific code
 */

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
    void adjustBrightness(QImage& image, int brightness);
    void adjustContrast(QImage &image, int contrast);
    void colorize(QImage& inoutImage, const QColor& tintColor);
};

#endif // CUSTOMIZATIONMANAGER_H
