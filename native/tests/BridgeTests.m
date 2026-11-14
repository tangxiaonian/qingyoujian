#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
extern char *qyj_prompt(const char *, const char *, const char *);
extern void qyj_set_tray_visible(bool);
extern bool qyj_tray_visible(void);
extern void qyj_install_handler(void (*callback)(const char *));
extern void qyj_host_ready(void);
static int openedSettings = 0;
/// Capture only the test process's synthetic launch event.
static void captureAction(const char *action) { if (strcmp(action,"qingyoujian-internal:settings")==0) { openedSettings++; } }
@interface QYJEvents : NSObject
- (void)openSettings:(NSAppleEventDescriptor *)event reply:(NSAppleEventDescriptor *)reply;
@end

/// Drive our own AppKit prompt from its modal loop; no Finder/UI scripting permission is needed.
static void finishPrompt(NSInteger result, NSString *text) {
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC / 4), dispatch_get_main_queue(), ^{
        NSWindow *window = NSApp.modalWindow;
        if (text) {
            for (NSView *view in window.contentView.subviews) {
                if ([view isKindOfClass:NSTextField.class] && [(NSTextField *)view isEditable]) { [(NSTextField *)view setStringValue:text]; }
            }
        }
        [NSApp stopModalWithCode:result];
    });
}
/// Verify confirmed and cancelled native prompts return distinct results and can be freed by Rust.
int main(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        qyj_install_handler(captureAction);qyj_host_ready();
        QYJEvents *events = [QYJEvents new];
        NSAppleEventDescriptor *login = [NSAppleEventDescriptor appleEventWithEventClass:kCoreEventClass eventID:kAEOpenApplication targetDescriptor:nil returnID:kAutoGenerateReturnID transactionID:kAnyTransactionID];
        [login setParamDescriptor:[NSAppleEventDescriptor descriptorWithBoolean:YES] forKeyword:keyAELaunchedAsLogInItem];
        [events openSettings:login reply:nil];NSCAssert(openedSettings==0,@"login must stay silent");
        NSAppleEventDescriptor *normal = [NSAppleEventDescriptor appleEventWithEventClass:kCoreEventClass eventID:kAEOpenApplication targetDescriptor:nil returnID:kAutoGenerateReturnID transactionID:kAnyTransactionID];
        [events openSettings:normal reply:nil];NSCAssert(openedSettings==1,@"manual open must show settings");
        qyj_set_tray_visible(true);NSCAssert(qyj_tray_visible(),@"tray enable failed");
        qyj_set_tray_visible(false);NSCAssert(!qyj_tray_visible(),@"tray disable failed");
        finishPrompt(NSAlertFirstButtonReturn, nil);
        char *accepted = qyj_prompt("新建文件", "中文 空格.docx", "仅用于自动化测试，不会创建文件。");
        NSCAssert(accepted && strcmp(accepted, "中文 空格.docx") == 0, @"native prompt did not preserve the UTF-8 name");
        free(accepted);
        finishPrompt(NSAlertSecondButtonReturn, nil);
        char *cancelled = qyj_prompt("取消测试", "不创建.txt", "此对话框会自动取消。");
        NSCAssert(cancelled == NULL, @"cancel must not be treated as a filename");
        NSLog(@"Bridge tests passed: UTF-8 input, cancellation, login/manual launch and tray switches");
    }
    return 0;
}
