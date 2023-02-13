#include "../../inc/macos/macosiconutils.h"
#import <Foundation/Foundation.h>
#import <Cocoa/Cocoa.h>
#import <ImageIO/ImageIO.h>

#import <QDebug>


// Constants

NSURL* const ASSETS_LOCATION = [NSURL fileURLWithPath:@"/System/Library/PrivateFrameworks/IconFoundation.framework/Versions/A/Resources/Assets.car"];

NSString* const FOLDER_NAME = @"Folder";
NSString* const FOLDER_DARK_NAME = @"FolderDark";
NSString* const SMART_FOLDER_NAME = @"SmartFolder";
NSString* const SMART_FOLDER_DARK_NAME = @"SmartFolderDark";

// CoreUI stuff; This is a private framework containing the classes needed to parse Asset Catalog files.

@interface CUINamedData : NSObject

@end

@interface CUINamedImage : NSObject

@property (copy, nonatomic) NSString *name;
@property (readonly, nonatomic) int exifOrientation;
@property (readonly, nonatomic) BOOL isStructured;
@property (readonly, nonatomic) NSInteger templateRenderingMode;
@property (readonly, nonatomic) BOOL isTemplate;
@property (readonly, nonatomic) BOOL isVectorBased;
@property (readonly, nonatomic) BOOL hasSliceInformation;
@property (readonly, nonatomic) NSInteger resizingMode;
@property (readonly, nonatomic) int blendMode;
@property (readonly, nonatomic) CGFloat opacity;
@property (readonly, nonatomic) NSInteger imageType;
@property (readonly, nonatomic) CGFloat scale;
@property (readonly, nonatomic) NSSize size;
@property (readonly, nonatomic) CGImageRef image;

@end

#define kCoreThemeStateNone -1

NSString *themeStateNameForThemeState(long long state) {
    switch (state) {
        case 0:
            return @"Normal";
            break;
        case 1:
            return @"Rollover";
            break;
        case 2:
            return @"Pressed";
            break;
        case 3:
            return @"Inactive";
            break;
        case 4:
            return @"Disabled";
            break;
        case 5:
            return @"DeeplyPressed";
            break;
    }

    return nil;
}

struct _renditionkeytoken {
    unsigned short identifier;
    unsigned short value;
};

@interface CUIRenditionKey: NSObject

@property (readonly) struct _renditionkeytoken *keyList;

@property (readonly) long long themeScale;
@property (readonly) long long themeState;
@property (readonly) long long themeDirection;
@property (readonly) long long themeSize;
@property (readonly) long long themeElement;
@property (readonly) long long themePart;
@property (readonly) long long themeIdentifier;

+ (CUIRenditionKey *)renditionKeyWithKeyList:(const struct _renditionkeytoken *)arg1;

@end

@interface CUIThemeRendition: NSObject

@property (nonatomic, readonly) CGFloat scale;
@property (nonatomic, readonly) NSString *name;
@property (nonatomic, readonly) NSData *data;
@property (nonatomic, readonly) CGImageRef unslicedImage;

@end

@interface CUICommonAssetStorage: NSObject
{
    struct _carheader {
        unsigned int _field1;
        unsigned int _field2;
        unsigned int _field3;
        unsigned int _field4;
        unsigned int _field5;
        char _field6[128];
        char _field7[256];
        unsigned char _field8[16];
        unsigned int _field9;
        unsigned int _field10;
        unsigned int _field11;
        unsigned int _field12;
    } *_header;
}

@property (readonly) NSArray <CUIRenditionKey *> *allAssetKeys;

@end

@interface CUIStructuredThemeStore : NSObject

- (instancetype)initWithURL:(NSURL *)URL;
- (instancetype)initWithPath:(NSString *)path;
- (NSData *)lookupAssetForKey:(struct _renditionkeytoken *)key;
- (CUIThemeRendition *)renditionWithKey:(const struct _renditionkeytoken *)key;
- (const struct _renditionkeytoken *)renditionKeyForName:(NSString *)arg1;

@property (readonly) CUICommonAssetStorage *themeStore;

@end

@interface CUICatalog : NSObject

+ (instancetype)systemUICatalog;
+ (instancetype)defaultUICatalog;

- (instancetype)initWithURL:(NSURL *)url error:(NSError **)outError;

@property (nonatomic, readonly) NSArray *allImageNames;
- (CUINamedImage *)imageWithName:(NSString *)name scaleFactor:(CGFloat)scaleFactor;


- (CUIStructuredThemeStore *)_themeStore;

@end

void MacOSIconUtils::dumpDefaultFolderIcons(const QString &folderPathIn) {

    NSString* folderPath = folderPathIn.toNSString();


    if (folderPath == nil) {
        return;
    }

    NSURL *folderPathUrl = [NSURL fileURLWithPath:folderPath];

    NSError *catalogError;
    CUICatalog *catalog = [[CUICatalog alloc] initWithURL:ASSETS_LOCATION error:&catalogError];
    if (!catalog) {
        NSLog(@"Error creating CUICatalog: %@", catalogError);
        return;
    }

    // Determine the theme identifier for each type of folder asset

    long long folderKeyId = [[CUIRenditionKey renditionKeyWithKeyList:[[catalog _themeStore] renditionKeyForName:FOLDER_NAME]] themeIdentifier];
    long long folderDarkKeyId = [[CUIRenditionKey renditionKeyWithKeyList:[[catalog _themeStore] renditionKeyForName:FOLDER_DARK_NAME]] themeIdentifier];
    long long smartFolderKeyId = [[CUIRenditionKey renditionKeyWithKeyList:[[catalog _themeStore] renditionKeyForName:SMART_FOLDER_NAME]] themeIdentifier];
    long long smartFolderDarkKeyId = [[CUIRenditionKey renditionKeyWithKeyList:[[catalog _themeStore] renditionKeyForName:SMART_FOLDER_DARK_NAME]] themeIdentifier];

    NSURL *folderOutputDirectory = [folderPathUrl URLByAppendingPathComponent:FOLDER_NAME];
    NSURL *folderDarkOutputDirectory = [folderPathUrl URLByAppendingPathComponent:FOLDER_DARK_NAME];
    NSURL *smartFolderOutputDirectory = [folderPathUrl URLByAppendingPathComponent:SMART_FOLDER_NAME];
    NSURL *smartFolderDarkOutputDirectory = [folderPathUrl URLByAppendingPathComponent:SMART_FOLDER_DARK_NAME];

    NSFileManager *fileManager = [NSFileManager defaultManager];

    NSArray *dirs = @[folderOutputDirectory , folderDarkOutputDirectory, smartFolderOutputDirectory, smartFolderDarkOutputDirectory];

    for (NSURL *dir in dirs) {
        NSError *error = nil;
        BOOL success = [fileManager createDirectoryAtURL:dir withIntermediateDirectories:YES attributes:nil error:&error];
        if (!success) {
            NSLog(@"Error creating directory at URL %@: %@", dir, error);
            [catalog release];
            return;
        }
    }

    /* Iterate through every asset key; this includes sliced and unsliced images. There doesn't seem to be a way to directly export all of the images in a sliced ThemePixelRendition, so
     * we need to iterate through everything and just grab whatever shares the themeIdentifiers found above. I'll try to see if getting the slicing information is possible in the future, but this
     * works for now, since this function only has to run the first time the program is used.
    */


    [[catalog _themeStore].themeStore.allAssetKeys enumerateObjectsWithOptions:0 usingBlock:^(CUIRenditionKey * _Nonnull key, NSUInteger idx, BOOL * _Nonnull stop) {
        if ([key themeIdentifier] == folderKeyId || [key themeIdentifier] == folderDarkKeyId || [key themeIdentifier] == smartFolderKeyId || [key themeIdentifier] == smartFolderDarkKeyId) {
            CUIThemeRendition *rendition = [[catalog _themeStore] renditionWithKey:key.keyList];
            if (rendition.unslicedImage) { // Skip over the renditions that are named Folder, FolderDark, etc. We're only extracting images with the same identifier as these renditions.
                CGImageRef image = rendition.unslicedImage;
                NSURL *outputDirectory;
                if ([key themeIdentifier] == folderKeyId) {
                    outputDirectory = [folderPathUrl URLByAppendingPathComponent:FOLDER_NAME];
                } else if ([key themeIdentifier] == folderDarkKeyId) {
                    outputDirectory = [folderPathUrl URLByAppendingPathComponent:FOLDER_DARK_NAME];
                } else if ([key themeIdentifier] == smartFolderKeyId) {
                    outputDirectory = [folderPathUrl URLByAppendingPathComponent:SMART_FOLDER_NAME];
                } else {
                    outputDirectory = [folderPathUrl URLByAppendingPathComponent:SMART_FOLDER_DARK_NAME];
                }

                NSURL *outputFile = [outputDirectory URLByAppendingPathComponent:rendition.name];
                CFURLRef url = (__bridge CFURLRef)outputFile;

                CFStringRef format = kUTTypePNG;

                CGImageDestinationRef destination = CGImageDestinationCreateWithURL(url, format, 1, NULL);

                if (!destination) {
                    NSLog(@"Image destination could not be created.");
                    return;
                }

                CGImageDestinationAddImage(destination, image, nil);

                if (!CGImageDestinationFinalize(destination)) {
                    NSLog(@"Failed to write image to %@", url);
                    CFRelease(destination);
                    return;
                }

                CFRelease(destination);
            }
        }
    }];
}

void MacOSIconUtils::createICNSAndApply(const QList<QImage>& images, const QString &folderPathIn) {

    CFStringRef format = kUTTypeAppleICNS;

    CFMutableDataRef dataRef = CFDataCreateMutable(NULL, 0);

    CGImageDestinationRef destination = CGImageDestinationCreateWithData(dataRef, format, images.size(), NULL);
    if (!destination) {
        NSLog(@"Image destination could not be created.");
        CFRelease(dataRef);
        return;
    }

    for (int i = 0; i < images.size(); i++) {

        BOOL isHighRes = i % 2 != 0; // Icons are organized like 16x16, 16x16@2x, 32x32, 32x32@2x, etc...
        NSNumber *DPIdim;
        if (isHighRes) {
            DPIdim = @144.0;
        } else {
            DPIdim = @72.0;
        }

        QImage image = images.at(i);

        CGImageRef cgImage = image.toCGImage();

        NSDictionary *imageProperties = @{(__bridge id)kCGImagePropertyDPIHeight : DPIdim, (__bridge id)kCGImagePropertyDPIWidth : DPIdim};


        CGImageDestinationAddImage(destination, cgImage, (__bridge CFDictionaryRef)imageProperties);
        CFRelease(cgImage);

    }

    if (!CGImageDestinationFinalize(destination)) {
        NSLog(@"Error finalizing image destination");
    }


    NSString* folderPath = folderPathIn.toNSString();

    NSData *iconData = [NSData dataWithBytesNoCopy:CFDataGetMutableBytePtr(dataRef) length:CFDataGetLength(dataRef) freeWhenDone:NO];

    NSImage *icon = [[NSImage alloc] initWithData:iconData];

    [[NSWorkspace sharedWorkspace] setIcon:icon forFile:folderPath options:0];

    [icon release];
    CFRelease(destination);
    CFRelease(dataRef);
}

void MacOSIconUtils::resetFolderIconToDefault(const QString &folderPathIn) {

    NSString* folderPath = folderPathIn.toNSString();

    [[NSWorkspace sharedWorkspace] setIcon:nil forFile:folderPath options:0];

}














