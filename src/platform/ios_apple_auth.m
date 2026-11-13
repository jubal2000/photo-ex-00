#import <Foundation/Foundation.h>
#import <UIKit/UIKit.h>
#import <AuthenticationServices/AuthenticationServices.h>
#import <CommonCrypto/CommonDigest.h>

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
#pragma clang diagnostic ignored "-Wunused-parameter"

typedef void (*PEAppleAuthCallback)(const char * _Nullable idToken,
                                   const char * _Nullable rawNonce,
                                   const char * _Nullable email,
                                   const char * _Nullable fullName,
                                   const char * _Nullable errorMsg,
                                   void *context);

@interface PEAppleAuthDelegate : NSObject <ASAuthorizationControllerDelegate, ASAuthorizationControllerPresentationContextProviding>
@property (nonatomic, assign) PEAppleAuthCallback callback;
@property (nonatomic, assign) void *context;
@property (nonatomic, copy) NSString *rawNonce;
@property (nonatomic, strong) id keepAlive;
@end

@implementation PEAppleAuthDelegate

- (ASPresentationAnchor)presentationAnchorForAuthorizationController:(ASAuthorizationController *)controller {
    UIWindow *window = nil;
    for (UIWindowScene *scene in [UIApplication sharedApplication].connectedScenes) {
        if ([scene isKindOfClass:[UIWindowScene class]]) {
            for (UIWindow *w in scene.windows) {
                if (w.isKeyWindow) { window = w; break; }
            }
        }
    }
    if (!window) {
        window = [UIApplication sharedApplication].windows.firstObject;
    }
    return window;
}

- (void)authorizationController:(ASAuthorizationController *)controller didCompleteWithAuthorization:(ASAuthorization *)authorization {
    if ([authorization.credential isKindOfClass:[ASAuthorizationAppleIDCredential class]]) {
        ASAuthorizationAppleIDCredential *cred = (ASAuthorizationAppleIDCredential *)authorization.credential;
        NSString *idToken = [[NSString alloc] initWithData:cred.identityToken encoding:NSUTF8StringEncoding];

        NSString *email = cred.email;
        NSString *fullName = nil;
        if (cred.fullName) {
            NSPersonNameComponentsFormatter *formatter = [[NSPersonNameComponentsFormatter alloc] init];
            fullName = [formatter stringFromPersonNameComponents:cred.fullName];
        }

        if (self.callback) {
            self.callback(idToken ? [idToken UTF8String] : NULL,
                          self.rawNonce ? [self.rawNonce UTF8String] : NULL,
                          email ? [email UTF8String] : NULL,
                          fullName ? [fullName UTF8String] : NULL,
                          NULL,
                          self.context);
            self.callback = NULL;
        }
    } else {
        if (self.callback) {
            self.callback(NULL, NULL, NULL, NULL, "unsupported credential type", self.context);
            self.callback = NULL;
        }
    }
    self.keepAlive = nil;
}

- (void)authorizationController:(ASAuthorizationController *)controller didCompleteWithError:(NSError *)error {
    if (self.callback) {
        const char *errStr = error.localizedDescription ? [error.localizedDescription UTF8String] : "Apple Sign-in failed";
        self.callback(NULL, NULL, NULL, NULL, errStr, self.context);
        self.callback = NULL;
    }
    self.keepAlive = nil;
}

@end

static NSString *pe_random_nonce(NSUInteger length) {
    uint8_t randomBytes[length];
    int result = SecRandomCopyBytes(kSecRandomDefault, length, randomBytes);
    if (result != 0) {
        return @"pe_nonce_fallback";
    }
    NSMutableString *hex = [NSMutableString stringWithCapacity:length * 2];
    for (NSUInteger i = 0; i < length; i++) {
        [hex appendFormat:@"%02x", randomBytes[i]];
    }
    return hex;
}

static NSString *pe_sha256(NSString *input) {
    const char *str = [input UTF8String];
    unsigned char result[CC_SHA256_DIGEST_LENGTH];
    CC_SHA256(str, (CC_LONG)strlen(str), result);
    NSMutableString *hex = [NSMutableString stringWithCapacity:CC_SHA256_DIGEST_LENGTH * 2];
    for (int i = 0; i < CC_SHA256_DIGEST_LENGTH; i++) {
        [hex appendFormat:@"%02x", result[i]];
    }
    return hex;
}

void pe_ios_apple_sign_in(PEAppleAuthCallback callback, void *context) {
    dispatch_async(dispatch_get_main_queue(), ^{
        if (@available(iOS 13.0, *)) {
            NSString *rawNonce = pe_random_nonce(32);
            NSString *hashedNonce = pe_sha256(rawNonce);

            ASAuthorizationAppleIDProvider *provider = [[ASAuthorizationAppleIDProvider alloc] init];
            ASAuthorizationAppleIDRequest *request = [provider createRequest];
            request.requestedScopes = @[ASAuthorizationScopeFullName, ASAuthorizationScopeEmail];
            request.nonce = hashedNonce;

            PEAppleAuthDelegate *delegate = [[PEAppleAuthDelegate alloc] init];
            delegate.callback = callback;
            delegate.context = context;
            delegate.rawNonce = rawNonce;
            delegate.keepAlive = delegate;

            ASAuthorizationController *controller = [[ASAuthorizationController alloc] initWithAuthorizationRequests:@[request]];
            controller.delegate = delegate;
            controller.presentationContextProvider = delegate;
            [controller performRequests];
        } else {
            if (callback) {
                callback(NULL, NULL, NULL, NULL, "Apple Sign-in requires iOS 13+", context);
            }
        }
    });
}

#pragma clang diagnostic pop
