#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
extern char *qyj_toggle_hidden(void);

/// Copy an AX attribute from the test's Finder window without logging other user content.
static id attribute(AXUIElementRef element, CFStringRef key) {
    CFTypeRef value = NULL;
    AXUIElementCopyAttributeValue(element, key, &value);
    return CFBridgingRelease(value);
}
/// Finder list/icon views expose filenames through different AX attributes.
static BOOL containsHidden(AXUIElementRef element, int depth) {
    if (depth > 20) { return NO; }
    for (NSString *key in @[(id)kAXTitleAttribute, (id)kAXValueAttribute, (id)kAXDescriptionAttribute]) {
        id value = attribute(element, (__bridge CFStringRef)key);
        if ([value isKindOfClass:NSString.class] && [value containsString:@".qyj-hidden-sample"]) { return YES; }
    }
    for (id child in attribute(element, kAXChildrenAttribute)) {
        if (containsHidden((__bridge AXUIElementRef)child, depth + 1)) { return YES; }
    }
    return NO;
}
/// Wait for actual Finder rendering, rather than its potentially stale preference file.
static BOOL waitForHidden(AXUIElementRef window, BOOL expected) {
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:2];
    while (deadline.timeIntervalSinceNow > 0) {
        if (containsHidden(window, 0) == expected) { return YES; }
        [NSRunLoop.currentRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.05]];
    }
    return NO;
}
/// Opt-in integration test: toggle a real dotfile twice and keep Finder's PID and initial state.
int main(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        if (!AXIsProcessTrusted()) { fprintf(stderr, "Integration test requires existing Accessibility authorization.\n"); return 2; }
        NSRunningApplication *previous = NSWorkspace.sharedWorkspace.frontmostApplication;
        NSRunningApplication *finder = [NSRunningApplication runningApplicationsWithBundleIdentifier:@"com.apple.finder"].firstObject;
        pid_t originalPID = finder.processIdentifier;
        NSURL *directory = [NSURL fileURLWithPath:[NSTemporaryDirectory() stringByAppendingPathComponent:[@"qyj-hidden-test-" stringByAppendingString:NSUUID.UUID.UUIDString]] isDirectory:YES];
        NSFileManager *files = NSFileManager.defaultManager;
        [files createDirectoryAtURL:directory withIntermediateDirectories:YES attributes:nil error:nil];
        NSURL *sample = [directory URLByAppendingPathComponent:@".qyj-hidden-sample"];
        [@"test" writeToURL:sample atomically:YES encoding:NSUTF8StringEncoding error:nil];
        AXUIElementRef application = AXUIElementCreateApplication(originalPID);
        AXUIElementRef window = NULL;
        [NSWorkspace.sharedWorkspace openURL:directory];
        NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:3];
        while (!window && deadline.timeIntervalSinceNow > 0) {
            for (id candidate in attribute(application, kAXWindowsAttribute)) {
                if ([attribute((__bridge AXUIElementRef)candidate, kAXTitleAttribute) isEqual:directory.lastPathComponent]) {
                    window = (AXUIElementRef)CFRetain((__bridge AXUIElementRef)candidate); break;
                }
            }
            [NSRunLoop.currentRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.05]];
        }
        NSCAssert(window, @"test Finder window did not open");
        // Allow Finder to finish populating the fresh directory before recording its initial state.
        [NSRunLoop.currentRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.3]];
        NSDictionary *initialAttributes = [files attributesOfItemAtPath:sample.path error:nil];
        BOOL initial = containsHidden(window, 0);
        char *error = qyj_toggle_hidden();
        BOOL changed = !error && waitForHidden(window, !initial);
        free(error);
        BOOL restored = YES;
        if (changed) {
            error = qyj_toggle_hidden();
            restored = !error && waitForHidden(window, initial); free(error);
        }
        BOOL samePID = [NSRunningApplication runningApplicationsWithBundleIdentifier:@"com.apple.finder"].firstObject.processIdentifier == originalPID;
        NSDictionary *finalAttributes = [files attributesOfItemAtPath:sample.path error:nil];
        NSCAssert(finalAttributes && [initialAttributes[NSFilePosixPermissions] isEqual:finalAttributes[NSFilePosixPermissions]] && [initialAttributes[NSFileModificationDate] isEqual:finalAttributes[NSFileModificationDate]], @"hidden sample must retain its name, permissions and modification time");
        id close = attribute(window, kAXCloseButtonAttribute);
        if (close) { AXUIElementPerformAction((__bridge AXUIElementRef)close, kAXPressAction); }
        CFRelease(window); CFRelease(application);
        [files removeItemAtURL:directory error:nil];
        [previous activateWithOptions:0];
        NSLog(@"Finder hidden-file integration: changed=%d restored=%d samePID=%d PID=%d", changed, restored, samePID, originalPID);
        return changed && restored && samePID ? 0 : 1;
    }
}
