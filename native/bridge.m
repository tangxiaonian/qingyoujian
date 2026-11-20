#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#import <Security/Security.h>
#import <Carbon/Carbon.h>
#import <ServiceManagement/ServiceManagement.h>

static void (*actionCallback)(const char *);
static BOOL hostReady = NO;
static NSMutableArray<NSString *> *pendingActions;
static BOOL loginLaunch = NO;
@class QYJEvents;
static QYJEvents *eventReceiver;
static NSStatusItem *menuBarItem;

/// AppKit can deliver launch events before Tauri setup; keep already-authenticated requests until ready.
static void deliver(NSString *value) {
    if (hostReady && actionCallback) { actionCallback(value.UTF8String); }
    else { if (!pendingActions) { pendingActions = [NSMutableArray new]; } [pendingActions addObject:value]; }
}

/// Called after Tauri has installed its state and application handle.
void qyj_host_ready(void) {
    hostReady = YES;
    NSArray *pending = pendingActions.copy;
    [pendingActions removeAllObjects];
    for (NSString *value in pending) { deliver(value); }
}

/// Only the extension embedded in this exact local build may submit Finder actions.
static BOOL trustedSender(NSAppleEventDescriptor *event) {
    NSData *token = [event attributeDescriptorForKeyword:keySenderAuditTokenAttr].data;
    NSURL *extension = [NSBundle.mainBundle.builtInPlugInsURL URLByAppendingPathComponent:@"QingRightExtension.appex"];
    if (!token || !extension) { return NO; }
    SecCodeRef sender = NULL;
    SecStaticCodeRef bundled = NULL;
    CFDictionaryRef info = NULL;
    SecRequirementRef requirement = NULL;
    BOOL valid = NO;
    if (SecCodeCopyGuestWithAttributes(NULL, (__bridge CFDictionaryRef)@{(__bridge id)kSecGuestAttributeAudit: token}, 0, &sender) == errSecSuccess &&
        SecStaticCodeCreateWithPath((__bridge CFURLRef)extension, 0, &bundled) == errSecSuccess &&
        SecCodeCopySigningInformation(bundled, 0, &info) == errSecSuccess) {
        NSData *hash = ((__bridge NSDictionary *)info)[(__bridge id)kSecCodeInfoUnique];
        NSMutableString *hex = [NSMutableString string];
        const unsigned char *bytes = hash.bytes;
        for (NSUInteger i = 0; i < hash.length; i++) { [hex appendFormat:@"%02x", bytes[i]]; }
        NSString *rule = [NSString stringWithFormat:@"cdhash H\"%@\"", hex];
        if (hash && SecRequirementCreateWithString((__bridge CFStringRef)rule, 0, &requirement) == errSecSuccess) {
            valid = SecCodeCheckValidity(sender, 0, requirement) == errSecSuccess;
        }
    }
    if (sender) { CFRelease(sender); }
    if (bundled) { CFRelease(bundled); }
    if (info) { CFRelease(info); }
    if (requirement) { CFRelease(requirement); }
    return valid;
}

@interface QYJEvents : NSObject
- (void)handle:(NSAppleEventDescriptor *)event reply:(NSAppleEventDescriptor *)reply;
- (void)openSettings:(NSAppleEventDescriptor *)event reply:(NSAppleEventDescriptor *)reply;
- (void)showFromStatus:(id)sender;
- (void)quitFromStatus:(id)sender;
@end
@implementation QYJEvents
/// Plain application-open events show settings; only URL actions require a trusted sender.
- (void)openSettings:(NSAppleEventDescriptor *)event reply:(NSAppleEventDescriptor *)reply {
    loginLaunch = [event paramDescriptorForKeyword:keyAELaunchedAsLogInItem].booleanValue || [event paramDescriptorForKeyword:keyAEPropData].enumCodeValue == keyAELaunchedAsLogInItem;
    if (!loginLaunch) { deliver(@"qingyoujian-internal:settings"); }
}
/// A tray item is an explicit local user action and does not accept file-operation payloads.
- (void)showFromStatus:(id)sender { deliver(@"qingyoujian-internal:settings"); }
/// Explicit quit bypasses the background-window-close preference.
- (void)quitFromStatus:(id)sender { deliver(@"qingyoujian-internal:quit"); }
/// Keep authentication at the Apple Event boundary, before Rust sees the request.
- (void)handle:(NSAppleEventDescriptor *)event reply:(NSAppleEventDescriptor *)reply {
    if (!trustedSender(event)) { NSLog(@"轻右键 rejected an untrusted action sender"); return; }
    NSString *url = [event paramDescriptorForKeyword:keyDirectObject].stringValue;
    if (url) { deliver(url); }
}
@end

/// Install before the macOS event loop begins so cold and warm URL delivery share one handler.
void qyj_install_handler(void (*callback)(const char *)) {
    actionCallback = callback;
    // A normal launch has no URL action. Use AppKit's launch reason instead of a timer,
    // so a cold Finder action never flashes a settings WebView.
    static dispatch_once_t observation;
    dispatch_once(&observation, ^{
        // NSApplication initializes its own URL handlers after main begins. Reinstall at
        // will-finish-launching, before macOS delivers a cold launch URL to its delegate.
        [NSNotificationCenter.defaultCenter addObserverForName:NSApplicationWillFinishLaunchingNotification object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *notification) {
            (void)notification;
            qyj_install_handler(actionCallback);
        }];
        [NSNotificationCenter.defaultCenter addObserverForName:NSApplicationDidFinishLaunchingNotification object:nil queue:NSOperationQueue.mainQueue usingBlock:^(NSNotification *notification) {
            if ([notification.userInfo[NSApplicationLaunchIsDefaultLaunchKey] boolValue]) {
                dispatch_async(dispatch_get_main_queue(), ^{ if (!loginLaunch) { deliver(@"qingyoujian-internal:settings"); } });
            }
        }];
    });
    if (!eventReceiver) { eventReceiver = [QYJEvents new]; }
    [[NSAppleEventManager sharedAppleEventManager] setEventHandler:eventReceiver andSelector:@selector(handle:reply:) forEventClass:kInternetEventClass andEventID:kAEGetURL];
    [[NSAppleEventManager sharedAppleEventManager] setEventHandler:eventReceiver andSelector:@selector(openSettings:reply:) forEventClass:kCoreEventClass andEventID:kAEOpenApplication];
}

/// Publish one complete snapshot; the Finder extension reads only this preference domain.
bool qyj_publish(const char *json) {
    CFPreferencesSetAppValue(CFSTR("MenuSnapshot"), (__bridge CFStringRef)[NSString stringWithUTF8String:json], CFSTR("local.qingyoujian"));
    return CFPreferencesAppSynchronize(CFSTR("local.qingyoujian"));
}

/// Native naming/rename prompt; NULL means cancelled, caller releases the returned string.
char *qyj_prompt(const char *title, const char *suggestion, const char *message) {
    NSAlert *alert = [NSAlert new];
    alert.messageText = [NSString stringWithUTF8String:title];
    alert.informativeText = [NSString stringWithUTF8String:message];
    [alert addButtonWithTitle:@"确定"]; [alert addButtonWithTitle:@"取消"];
    NSTextField *field = [[NSTextField alloc] initWithFrame:NSMakeRect(0,0,340,26)];
    field.stringValue = [NSString stringWithUTF8String:suggestion];
    alert.accessoryView = field; alert.window.initialFirstResponder = field;
    [NSRunLoop.currentRunLoop performInModes:@[NSModalPanelRunLoopMode] block:^{ [NSApp activateIgnoringOtherApps:YES]; [alert.window makeFirstResponder:field]; }];
    [NSApp activateIgnoringOtherApps:YES];
    if ([alert runModal] != NSAlertFirstButtonReturn) { return NULL; }
    return strdup(field.stringValue.UTF8String);
}

/// Show a Chinese action failure without starting a WebView.
void qyj_alert(const char *message) {
    NSAlert *alert = [NSAlert new]; alert.messageText = @"轻右键";
    alert.informativeText = [NSString stringWithUTF8String:message];
    [alert addButtonWithTitle:@"好"]; [NSApp activateIgnoringOtherApps:YES]; [alert runModal];
}

/// User-selected folders are returned as paths; the dialog never accepts files.
char *qyj_choose_folder(void) {
    NSOpenPanel *panel = [NSOpenPanel openPanel];
    panel.canChooseFiles = NO; panel.canChooseDirectories = YES; panel.allowsMultipleSelection = NO;
    panel.prompt = @"选择目录"; [NSApp activateIgnoringOtherApps:YES];
    if ([panel runModal] != NSModalResponseOK) { return NULL; }
    return strdup(panel.URL.path.UTF8String);
}

/// Missing favorites offer explicit recovery; no automatic search or folder deletion.
int qyj_missing_folder(void) {
    NSAlert *alert = [NSAlert new]; alert.messageText = @"常用目录暂时不可用";
    alert.informativeText = @"目录可能已移动、删除，或所在磁盘尚未连接。移除收藏不会删除实际文件。";
    [alert addButtonWithTitle:@"重新定位"]; [alert addButtonWithTitle:@"移除收藏"]; [alert addButtonWithTitle:@"取消"];
    [NSApp activateIgnoringOtherApps:YES]; return (int)[alert runModal] - (int)NSAlertFirstButtonReturn;
}

/// Use Finder's live shortcut; its persisted preference can lag behind the visible state.
char *qyj_toggle_hidden(void) {
    if (!AXIsProcessTrusted()) {
        NSAlert *alert = [NSAlert new];
        alert.messageText = @"允许轻右键切换隐藏文件";
        alert.informativeText = @"此功能通过 Finder 自带的 ⌘⇧. 快捷键立即刷新，无需重启 Finder。请在系统设置 → 隐私与安全性 → 辅助功能中允许轻右键，然后重新点击菜单。其他功能无需此授权。";
        [alert addButtonWithTitle:@"前往授权"]; [alert addButtonWithTitle:@"取消"];
        [NSApp activateIgnoringOtherApps:YES];
        if ([alert runModal] == NSAlertFirstButtonReturn) {
            AXIsProcessTrustedWithOptions((__bridge CFDictionaryRef)@{(__bridge id)kAXTrustedCheckOptionPrompt: @YES});
            [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:@"x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"]];
        }
        return NULL;
    }
    NSRunningApplication *finder = [NSRunningApplication runningApplicationsWithBundleIdentifier:@"com.apple.finder"].firstObject;
    if (!finder) { return strdup("Finder 当前没有运行，请打开一个 Finder 窗口后重试。"); }
    [finder activateWithOptions:0];
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:0.8];
    while (!finder.active && deadline.timeIntervalSinceNow > 0) {
        [NSRunLoop.currentRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.02]];
    }
    if (!finder.active) { return strdup("无法激活 Finder，请关闭 Finder 中的对话框后重试。"); }
    CGEventRef down = CGEventCreateKeyboardEvent(NULL, kVK_ANSI_Period, true);
    CGEventRef up = CGEventCreateKeyboardEvent(NULL, kVK_ANSI_Period, false);
    if (!down || !up) {
        if (down) { CFRelease(down); }
        if (up) { CFRelease(up); }
        return strdup("无法创建 Finder 快捷键事件，请重试。");
    }
    CGEventSetFlags(down, kCGEventFlagMaskCommand | kCGEventFlagMaskShift);
    CGEventSetFlags(up, kCGEventFlagMaskCommand | kCGEventFlagMaskShift);
    // Target Finder explicitly so the shortcut cannot be delivered to an unrelated app.
    CGEventPostToPid(finder.processIdentifier, down);
    CGEventPostToPid(finder.processIdentifier, up);
    CFRelease(down); CFRelease(up);
    return NULL;
}

/// Reuse system launch APIs rather than executing paths as shell code.
bool qyj_open_folder(const char *path) {
    return [NSWorkspace.sharedWorkspace openURL:[NSURL fileURLWithPath:[NSString stringWithUTF8String:path] isDirectory:YES]];
}
void qyj_reveal(const char *path) {
    [NSWorkspace.sharedWorkspace activateFileViewerSelectingURLs:@[[NSURL fileURLWithPath:[NSString stringWithUTF8String:path]]]];
}
/// Select completed ZIP outputs together without opening a new application or editor.
void qyj_reveal_many(const char *json) {
    NSData *data = [[NSString stringWithUTF8String:json] dataUsingEncoding:NSUTF8StringEncoding];
    NSArray *paths = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    NSMutableArray<NSURL *> *urls = [NSMutableArray new];
    for (NSString *path in paths) { [urls addObject:[NSURL fileURLWithPath:path]]; }
    if (urls.count) { [NSWorkspace.sharedWorkspace activateFileViewerSelectingURLs:urls]; }
}
bool qyj_is_package(const char *path) { return [NSWorkspace.sharedWorkspace isFilePackageAtPath:[NSString stringWithUTF8String:path]]; }
void qyj_accessory(bool accessory) { [NSApp setActivationPolicy:accessory ? NSApplicationActivationPolicyAccessory : NSApplicationActivationPolicyRegular]; }

/// Foundation copies directories and symlinks without reading entire files into Rust memory.
char *qyj_transfer(const char *source, const char *destination, bool move) {
    @autoreleasepool {
        NSURL *src = [NSURL fileURLWithPath:[NSString stringWithUTF8String:source]];
        NSURL *dest = [NSURL fileURLWithPath:[NSString stringWithUTF8String:destination]];
        NSError *error = nil;
        BOOL success = move ? [NSFileManager.defaultManager moveItemAtURL:src toURL:dest error:&error] : [NSFileManager.defaultManager copyItemAtURL:src toURL:dest error:&error];
        return success ? NULL : strdup((error.localizedDescription ?: @"文件操作失败。请检查权限和目标目录。").UTF8String);
    }
}

/// Use the system Trash API; never implement deletion with unlink or rm.
char *qyj_trash(const char *source, char **result) {
    @autoreleasepool {
        NSURL *src = [NSURL fileURLWithPath:[NSString stringWithUTF8String:source]];
        NSURL *dest = nil; NSError *error = nil;
        if (![NSFileManager.defaultManager trashItemAtURL:src resultingItemURL:&dest error:&error]) { return strdup((error.localizedDescription ?: @"无法移到废纸篓。").UTF8String); }
        *result = dest ? strdup(dest.path.UTF8String) : NULL;
        return NULL;
    }
}

/// Successful batches use a transient, non-activating panel; failures require readable feedback.
void qyj_feedback(const char *message, bool failure) {
    if (failure) { qyj_alert(message); return; }
    NSScreen *screen = NSScreen.mainScreen;
    NSRect area = screen.visibleFrame;
    NSPanel *panel = [[NSPanel alloc] initWithContentRect:NSMakeRect(NSMaxX(area)-370, NSMaxY(area)-95, 340, 60) styleMask:NSWindowStyleMaskBorderless | NSWindowStyleMaskNonactivatingPanel backing:NSBackingStoreBuffered defer:NO];
    panel.level = NSFloatingWindowLevel; panel.backgroundColor = NSColor.windowBackgroundColor;
    panel.contentView.wantsLayer = YES; panel.contentView.layer.cornerRadius = 12;
    NSTextField *label = [NSTextField wrappingLabelWithString:[NSString stringWithUTF8String:message]];
    label.frame = NSMakeRect(16,10,308,40);label.font = [NSFont systemFontOfSize:13];
    [panel.contentView addSubview:label];[panel orderFrontRegardless];
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 2*NSEC_PER_SEC),dispatch_get_main_queue(),^{[panel close];});
}

/// The status item belongs to the native host, not to an always-running WebView.
void qyj_set_tray_visible(bool visible) {
    if (!visible) {
        if (menuBarItem) { [NSStatusBar.systemStatusBar removeStatusItem:menuBarItem]; menuBarItem = nil; }
        return;
    }
    if (menuBarItem) { return; }
    if (!eventReceiver) {eventReceiver=[QYJEvents new];}
    menuBarItem = [NSStatusBar.systemStatusBar statusItemWithLength:NSVariableStatusItemLength];
    NSImage *icon = [NSImage imageWithSystemSymbolName:@"cursorarrow" accessibilityDescription:@"轻右键"];
    icon.size = NSMakeSize(16,16);icon.template = YES;menuBarItem.button.image = icon;menuBarItem.button.toolTip = @"轻右键";
    NSMenu *menu = [NSMenu new];
    NSMenuItem *settings = [menu addItemWithTitle:@"打开轻右键设置…" action:@selector(showFromStatus:) keyEquivalent:@""];settings.target = eventReceiver;
    [menu addItem:NSMenuItem.separatorItem];
    NSMenuItem *quit = [menu addItemWithTitle:@"退出轻右键" action:@selector(quitFromStatus:) keyEquivalent:@""];quit.target = eventReceiver;
    menuBarItem.menu = menu;
}
/// Native visual tests inspect their own status item without touching persisted preferences.
bool qyj_tray_visible(void) { return menuBarItem != nil; }

/// Report the OS authorization state instead of treating a stored preference as registration proof.
int qyj_login_status(void) { return (int)SMAppService.mainAppService.status; }
/// Login registration changes only after an explicit settings action.
char *qyj_set_login(bool enabled) {
    NSError *error = nil;
    BOOL success = enabled ? [SMAppService.mainAppService registerAndReturnError:&error] : [SMAppService.mainAppService unregisterAndReturnError:&error];
    return success ? NULL : strdup((error.localizedDescription ?: @"无法修改登录项，请在系统设置中检查授权。").UTF8String);
}

/// Open the native authorization page without changing its settings.
void qyj_open_login_settings(void) { [SMAppService openSystemSettingsLoginItems]; }
