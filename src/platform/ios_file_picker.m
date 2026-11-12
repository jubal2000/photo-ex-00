#import <UIKit/UIKit.h>
#import <WebKit/WebKit.h>
#import <objc/runtime.h>

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
#pragma clang diagnostic ignored "-Wunused-parameter"

// -----------------------------------------------------------------------------
// Native Image Picker for direct button click (No <input type="file"> needed)
// -----------------------------------------------------------------------------
typedef void (*PEImagePickCallback)(const char * _Nullable dataUrl, void *context);

@interface PENativeImagePicker : NSObject <UIImagePickerControllerDelegate, UINavigationControllerDelegate>
@property (nonatomic, assign) PEImagePickCallback callback;
@property (nonatomic, assign) void *context;
@property (nonatomic, strong) id keepAlive;
@end

@implementation PENativeImagePicker

- (void)finishWithDataUrl:(NSString *)dataUrl {
    if (self.callback) {
        if (dataUrl) {
            self.callback([dataUrl UTF8String], self.context);
        } else {
            self.callback(NULL, self.context);
        }
        self.callback = NULL;
    }
    self.keepAlive = nil;
}

- (void)imagePickerController:(UIImagePickerController *)picker didFinishPickingMediaWithInfo:(NSDictionary<UIImagePickerControllerInfoKey,id> *)info {
    UIImage *image = info[UIImagePickerControllerEditedImage];
    if (!image) {
        image = info[UIImagePickerControllerOriginalImage];
    }

    NSString *dataUrl = nil;
    if (image) {
        NSData *jpegData = UIImageJPEGRepresentation(image, 0.85);
        if (jpegData) {
            NSString *b64 = [jpegData base64EncodedStringWithOptions:0];
            dataUrl = [NSString stringWithFormat:@"data:image/jpeg;base64,%@", b64];
        }
    }

    [picker dismissViewControllerAnimated:YES completion:^{
        [self finishWithDataUrl:dataUrl];
    }];
}

- (void)imagePickerControllerDidCancel:(UIImagePickerController *)picker {
    [picker dismissViewControllerAnimated:YES completion:^{
        [self finishWithDataUrl:nil];
    }];
}

@end

static UIViewController *PEGetTopViewController(void) {
    UIWindow *keyWindow = nil;
    for (UIScene *scene in [UIApplication sharedApplication].connectedScenes) {
        if (scene.activationState == UISceneActivationStateForegroundActive && [scene isKindOfClass:[UIWindowScene class]]) {
            UIWindowScene *windowScene = (UIWindowScene *)scene;
            for (UIWindow *w in windowScene.windows) {
                if (w.isKeyWindow) {
                    keyWindow = w;
                    break;
                }
            }
            if (keyWindow) break;
            if (!keyWindow && windowScene.windows.count > 0) {
                keyWindow = windowScene.windows.firstObject;
            }
        }
    }
    if (!keyWindow) {
        keyWindow = [UIApplication sharedApplication].keyWindow;
    }
    if (!keyWindow && [UIApplication sharedApplication].windows.count > 0) {
        keyWindow = [UIApplication sharedApplication].windows.firstObject;
    }

    UIViewController *topVC = keyWindow.rootViewController;
    while (topVC.presentedViewController) {
        topVC = topVC.presentedViewController;
    }
    return topVC;
}

__attribute__((used))
void pe_pick_image_ios(void *context, PEImagePickCallback callback) {
    NSLog(@"[PE_IOS] pe_pick_image_ios called!");
    dispatch_async(dispatch_get_main_queue(), ^{
        UIViewController *rootVC = PEGetTopViewController();
        if (!rootVC) {
            NSLog(@"[PE_IOS] pe_pick_image_ios failed to find rootVC");
            if (callback) callback(NULL, context);
            return;
        }

        PENativeImagePicker *helper = [[PENativeImagePicker alloc] init];
        helper.callback = callback;
        helper.context = context;
        helper.keepAlive = helper;

        UIAlertController *alert = [UIAlertController alertControllerWithTitle:@"사진 선택"
                                                                       message:nil
                                                                preferredStyle:UIAlertControllerStyleActionSheet];

        [alert addAction:[UIAlertAction actionWithTitle:@"사진 보관함"
                                                 style:UIAlertActionStyleDefault
                                               handler:^(UIAlertAction * _Nonnull action) {
            UIImagePickerController *picker = [[UIImagePickerController alloc] init];
            picker.delegate = helper;
            picker.sourceType = UIImagePickerControllerSourceTypePhotoLibrary;
            picker.modalPresentationStyle = UIModalPresentationFullScreen;
            [rootVC presentViewController:picker animated:YES completion:nil];
        }]];

        if ([UIImagePickerController isSourceTypeAvailable:UIImagePickerControllerSourceTypeCamera]) {
            [alert addAction:[UIAlertAction actionWithTitle:@"카메라 촬영"
                                                     style:UIAlertActionStyleDefault
                                                   handler:^(UIAlertAction * _Nonnull action) {
                UIImagePickerController *picker = [[UIImagePickerController alloc] init];
                picker.delegate = helper;
                picker.sourceType = UIImagePickerControllerSourceTypeCamera;
                picker.modalPresentationStyle = UIModalPresentationFullScreen;
                [rootVC presentViewController:picker animated:YES completion:nil];
            }]];
        }

        [alert addAction:[UIAlertAction actionWithTitle:@"취소"
                                                 style:UIAlertActionStyleCancel
                                               handler:^(UIAlertAction * _Nonnull action) {
            [helper finishWithDataUrl:nil];
        }]];

        // iPad popover support
        if (alert.popoverPresentationController) {
            CGRect bounds = rootVC.view.bounds;
            CGFloat midX = bounds.origin.x + bounds.size.width * 0.5;
            CGFloat midY = bounds.origin.y + bounds.size.height * 0.5;
            alert.popoverPresentationController.sourceView = rootVC.view;
            alert.popoverPresentationController.sourceRect = CGRectMake(midX, midY, 0, 0);
            alert.popoverPresentationController.permittedArrowDirections = 0;
        }

        [rootVC presentViewController:alert animated:YES completion:nil];
    });
}

// -----------------------------------------------------------------------------
// Fallback: WebKit UIDelegate hook for traditional <input type="file">
// -----------------------------------------------------------------------------
@interface PEFilePickerHelper : NSObject <UIImagePickerControllerDelegate, UINavigationControllerDelegate>
@property (nonatomic, copy) void (^completionHandler)(NSArray<NSURL *> * _Nullable);
@property (nonatomic, strong) id keepAlive;
@end

@implementation PEFilePickerHelper

- (void)imagePickerController:(UIImagePickerController *)picker didFinishPickingMediaWithInfo:(NSDictionary<UIImagePickerControllerInfoKey,id> *)info {
    UIImage *image = info[UIImagePickerControllerEditedImage];
    if (!image) {
        image = info[UIImagePickerControllerOriginalImage];
    }

    NSURL *fileUrl = nil;
    if (image) {
        NSData *data = UIImageJPEGRepresentation(image, 0.85);
        NSString *fileName = [NSString stringWithFormat:@"pe_upload_%ld.jpg", (long)[[NSDate date] timeIntervalSince1970]];
        NSString *tempPath = [NSTemporaryDirectory() stringByAppendingPathComponent:fileName];
        [data writeToFile:tempPath atomically:YES];
        fileUrl = [NSURL fileURLWithPath:tempPath];
    } else {
        fileUrl = info[UIImagePickerControllerImageURL];
    }

    [picker dismissViewControllerAnimated:YES completion:^{
        if (self.completionHandler) {
            if (fileUrl) {
                self.completionHandler(@[fileUrl]);
            } else {
                self.completionHandler(nil);
            }
            self.completionHandler = nil;
        }
        self.keepAlive = nil;
    }];
}

- (void)imagePickerControllerDidCancel:(UIImagePickerController *)picker {
    [picker dismissViewControllerAnimated:YES completion:^{
        if (self.completionHandler) {
            self.completionHandler(nil);
            self.completionHandler = nil;
        }
        self.keepAlive = nil;
    }];
}

@end

static void WryRunOpenPanel(id self, SEL _cmd, id webView, id parameters, id frame, void (^completionHandler)(NSArray<NSURL *> * _Nullable)) {
    dispatch_async(dispatch_get_main_queue(), ^{
        UIViewController *rootVC = PEGetTopViewController();
        if (!rootVC) {
            if (completionHandler) completionHandler(nil);
            return;
        }

        PEFilePickerHelper *helper = [[PEFilePickerHelper alloc] init];
        helper.completionHandler = completionHandler;
        helper.keepAlive = helper;

        UIAlertController *alert = [UIAlertController alertControllerWithTitle:@"사진 선택"
                                                                       message:nil
                                                                preferredStyle:UIAlertControllerStyleActionSheet];

        [alert addAction:[UIAlertAction actionWithTitle:@"사진 보관함"
                                                 style:UIAlertActionStyleDefault
                                               handler:^(UIAlertAction * _Nonnull action) {
            UIImagePickerController *picker = [[UIImagePickerController alloc] init];
            picker.delegate = helper;
            picker.sourceType = UIImagePickerControllerSourceTypePhotoLibrary;
            picker.modalPresentationStyle = UIModalPresentationFullScreen;
            [rootVC presentViewController:picker animated:YES completion:nil];
        }]];

        if ([UIImagePickerController isSourceTypeAvailable:UIImagePickerControllerSourceTypeCamera]) {
            [alert addAction:[UIAlertAction actionWithTitle:@"카메라 촬영"
                                                     style:UIAlertActionStyleDefault
                                                   handler:^(UIAlertAction * _Nonnull action) {
                UIImagePickerController *picker = [[UIImagePickerController alloc] init];
                picker.delegate = helper;
                picker.sourceType = UIImagePickerControllerSourceTypeCamera;
                picker.modalPresentationStyle = UIModalPresentationFullScreen;
                [rootVC presentViewController:picker animated:YES completion:nil];
            }]];
        }

        [alert addAction:[UIAlertAction actionWithTitle:@"취소"
                                                 style:UIAlertActionStyleCancel
                                               handler:^(UIAlertAction * _Nonnull action) {
            if (helper.completionHandler) {
                helper.completionHandler(nil);
                helper.completionHandler = nil;
            }
            helper.keepAlive = nil;
        }]];

        // iPad popover support
        if (alert.popoverPresentationController) {
            CGRect bounds = rootVC.view.bounds;
            CGFloat midX = bounds.origin.x + bounds.size.width * 0.5;
            CGFloat midY = bounds.origin.y + bounds.size.height * 0.5;
            alert.popoverPresentationController.sourceView = rootVC.view;
            alert.popoverPresentationController.sourceRect = CGRectMake(midX, midY, 0, 0);
            alert.popoverPresentationController.permittedArrowDirections = 0;
        }

        [rootVC presentViewController:alert animated:YES completion:nil];
    });
}

static void InjectOpenPanelToClass(Class cls) {
    if (!cls) return;
    SEL sel = sel_registerName("webView:runOpenPanelWithParameters:initiatedByFrame:completionHandler:");
    if (!class_respondsToSelector(cls, sel)) {
        class_addMethod(cls, sel, (IMP)WryRunOpenPanel, "v@:@@@?");
    }
}

static void (*orig_setUIDelegate)(id self, SEL _cmd, id<WKUIDelegate> delegate) = NULL;

static void swizzled_setUIDelegate(id self, SEL _cmd, id<WKUIDelegate> delegate) {
    if (delegate) {
        Class delCls = object_getClass(delegate);
        InjectOpenPanelToClass(delCls);
    }
    if (orig_setUIDelegate) {
        orig_setUIDelegate(self, _cmd, delegate);
    }
}

__attribute__((used))
void init_ios_file_picker(void) {
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        Class wryCls = objc_getClass("WryWebViewUIDelegate");
        if (wryCls) {
            InjectOpenPanelToClass(wryCls);
        }

        Class wkCls = objc_getClass("WKWebView");
        if (wkCls) {
            SEL sel = sel_registerName("setUIDelegate:");
            Method m = class_getInstanceMethod(wkCls, sel);
            if (m) {
                orig_setUIDelegate = (void (*)(id, SEL, id<WKUIDelegate>))method_getImplementation(m);
                method_setImplementation(m, (IMP)swizzled_setUIDelegate);
            }
        }
    });
}

#pragma clang diagnostic pop
