#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

/// Read process/window metadata for this application's integration checks, without input automation.
int main(void) {
    @autoreleasepool {
        NSMutableArray *result = [NSMutableArray new];
        NSArray *windows = CFBridgingRelease(CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, kCGNullWindowID));
        for (NSRunningApplication *app in NSWorkspace.sharedWorkspace.runningApplications) {
            if (![app.bundleIdentifier isEqual:@"local.qingyoujian"]) { continue; }
            NSMutableArray *visible = [NSMutableArray new];
            for (NSDictionary *window in windows) {
                if ([window[(__bridge id)kCGWindowOwnerPID] intValue] == app.processIdentifier && [window[(__bridge id)kCGWindowLayer] intValue] == 0) {
                    [visible addObject:@{@"windowId":window[(__bridge id)kCGWindowNumber],@"bounds":window[(__bridge id)kCGWindowBounds]}];
                }
            }
            [result addObject:@{@"pid":@(app.processIdentifier),@"activationPolicy":@(app.activationPolicy),@"active":@(app.active),@"visibleWindows":visible}];
        }
        NSData *data = [NSJSONSerialization dataWithJSONObject:result options:0 error:nil];
        puts([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
    }
    return 0;
}
