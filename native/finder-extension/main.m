#import <AppKit/AppKit.h>
extern int NSExtensionMain(int argc, const char *argv[]);
extern NSDictionary *QYJReadSnapshot(void);

/// A local diagnostic entry exercises the actual signed extension-to-host boundary.
/// Normal Finder launches always enter Apple's extension runtime.
int main(int argc, const char *argv[]) {
    if (argc == 2 && strcmp(argv[1], "--diagnostic-snapshot") == 0) {
        @autoreleasepool {
            NSDictionary *snapshot = QYJReadSnapshot();
            NSData *data = [NSJSONSerialization dataWithJSONObject:snapshot options:0 error:nil];
            puts([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
            return [snapshot[@"schemaVersion"] integerValue] == 1 ? 0 : 1;
        }
    }
    if (argc == 3 && strcmp(argv[1], "--diagnostic-action") == 0) {
        @autoreleasepool {
            [NSApplication sharedApplication];
            NSData *data = [[NSString stringWithUTF8String:argv[2]] dataUsingEncoding:NSUTF8StringEncoding];
            if (![NSJSONSerialization JSONObjectWithData:data options:0 error:nil]) { return 2; }
            NSString *encoded = [[data base64EncodedStringWithOptions:0] stringByReplacingOccurrencesOfString:@"+" withString:@"-"];
            encoded = [[encoded stringByReplacingOccurrencesOfString:@"/" withString:@"_"] stringByReplacingOccurrencesOfString:@"=" withString:@""];
            BOOL sent = [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:[@"qingyoujian://action?payload=" stringByAppendingString:encoded]]];
            // Keep the sender alive while the receiver validates its audit token and signature.
            NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:3];
            while ([deadline timeIntervalSinceNow] > 0) { [NSRunLoop.currentRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.1]]; }
            return sent ? 0 : 1;
        }
    }
    return NSExtensionMain(argc, argv);
}
