#include "../inc/customizationmanager.h"

CustomizationManager::CustomizationManager(QObject *parent)
{
    QDir dataDir = QDir::toNativeSeparators(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation) + "/" + "default_icons");

    if (!dataDir.exists()) {
        dataDir.mkpath(".");
        IconUtils::dumpDefaultFolderIcons(dataDir.absolutePath());
    }

    defaultIcons = IconUtils::getDefaultFolderIcons(dataDir.absolutePath());

    // To colorize an image by multiplying pixels, we need to start with a grayscale image

    for (const QImage &icon : defaultIcons) {
        QImage grayscale = convertToGrayScale(icon);
        // Adjusting the contrast gives a much better looking result; more variance in shading.
        adjustContrast(grayscale, 35);
        grayscaleAndAdjustedIcons.append(grayscale);
    }

    color = QColor(Qt::blue);
    customizationEnabled = true;
    usingCustomColor = false;

}

void CustomizationManager::applyCustomization(QList<QString> &folders)
{
    if (customizationEnabled) {
        if (usingCustomColor) {
            QList<QImage> customizedIcons;
            for (const QImage &image : grayscaleAndAdjustedIcons) {
                QImage customizedImage = QImage(image);
                colorize(customizedImage, color);
                customizedIcons.append(customizedImage);
            }

            for (const QString &folder : folders) {
                qDebug() << folder;
                IconUtils::createIconAndApply(customizedIcons, QDir::toNativeSeparators(folder));
            }
        } else {
            for (const QString &folder : folders) {
                IconUtils::createIconAndApply(defaultIcons, QDir::toNativeSeparators(folder));
            }
        }

    } else {
        for (const QString &folder : folders) {
            IconUtils::resetFolderIconToDefault(QDir::toNativeSeparators(folder));
        }
    }

}

void CustomizationManager::colorize(QImage& inoutImage, const QColor& tintColor)
        {
            if (tintColor == Qt::white)
                return;

            // Convert to 4-channel 32-bit format if needed
            auto format = inoutImage.format();
            if (format != QImage::Format_ARGB32 && format != QImage::Format_ARGB32_Premultiplied)
            {
                format = QImage::Format_ARGB32_Premultiplied;
                inoutImage = inoutImage.convertToFormat(format);
            }

            const bool isPremultiplied = (format == QImage::Format_ARGB32_Premultiplied);
            const auto tint = tintColor.rgba();

            // Convert scanline by scanline (a bit tricker than using setPixelColor, but much more efficient)
            const int sizeX = inoutImage.width();
            const int sizeY = inoutImage.height();
            for (int y = 0; y < sizeY; ++y)
            {
                // Note: Qt documentation explicitly recommends this cast for 32-bit images
                auto* scanline = (QRgb*)inoutImage.scanLine(y);
                for (int x = 0; x < sizeX; ++x)
                {
                    auto color = scanline[x];
                    if (isPremultiplied)
                        color = qUnpremultiply(color);

                    color = qRgba(
                        (qRed(color) * qRed(tint)) / 255
                        , (qGreen(color) * qGreen(tint)) / 255
                        , (qBlue(color) * qBlue(tint)) / 255
                        , (qAlpha(color) * qAlpha(tint)) / 255
                    );

                    if (isPremultiplied)
                        color = qPremultiply(color);

                    scanline[x] = color;
                }
            }
        }

void CustomizationManager::adjustContrast(QImage &image, int contrast)
{
    // Make sure the contrast value is within the valid range
                contrast = qBound(-100, contrast, 100);

                // Create a lookup table to map the pixel values
                QVector<QRgb> table(256);
                for (int i = 0; i < 256; ++i)
                {
                    int value = qRound(i + (i - 128) * (contrast / 100.0));
                    value = qBound(0, value, 255);
                    table[i] = qRgba(value, value, value, i);
                }

                // Iterate over the image pixels and apply the contrast adjustment
                for (int y = 0; y < image.height(); ++y)
                {
                    QRgb* line = reinterpret_cast<QRgb*>(image.scanLine(y));
                    for (int x = 0; x < image.width(); ++x)
                    {
                        QRgb pixel = line[x];
                        int alpha = qAlpha(pixel);
                        int red = qRed(pixel);
                        int green = qGreen(pixel);
                        int blue = qBlue(pixel);

                        // Adjust the contrast of each color channel
                        red = qRed(table[red]);
                        green = qGreen(table[green]);
                        blue = qBlue(table[blue]);

                        // Set the adjusted pixel value back to the image
                        line[x] = qRgba(red, green, blue, alpha);
                    }
                }
}

QImage CustomizationManager::convertToGrayScale(const QImage &srcImage)
{
    QImage dstImage = QImage(srcImage);
    unsigned int *data = (unsigned int*)dstImage.bits();
                 int pixelCount = dstImage.width() * dstImage.height();

                 // Convert each pixel to grayscale
                 for(int i = 0; i < pixelCount; ++i) {
                    int val = qGray(*data);
                    *data = qRgba(val, val, val, qAlpha(*data));
                    ++data;
                 }

                 return dstImage;
}

void CustomizationManager::setColor(QColor &colorIn)
{
    color = colorIn;
}

QColor CustomizationManager::getColor()
{
    return color;
}

QPixmap CustomizationManager::getPreview()
{
    if (customizationEnabled) {
        if (usingCustomColor) {
            QImage customized = QImage(grayscaleAndAdjustedIcons[0]);
            colorize(customized, color);
            return QPixmap::fromImage(customized);
        } else {
            QImage defaultIcon = defaultIcons[0];
            return QPixmap::fromImage(defaultIcon);
        }

    } else {
        QImage defaultIcon = defaultIcons[0];
        return QPixmap::fromImage(defaultIcon);
    }
}
