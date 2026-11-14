#import <AppKit/AppKit.h>
#import <objc/runtime.h>
#import "../finder-extension/FinderSync.m"

@interface FakeController : NSObject
@property(nonatomic,strong) NSURL *targetedURL;
@property(nonatomic,strong) NSArray *selectedItemURLs;
@property(nonatomic,strong) NSSet *directoryURLs;
@end
@implementation FakeController
@end
static FakeController *controller;
static id fakeController(id self, SEL cmd) { return controller; }

@interface TestSync : QingRightSync
@property(nonatomic,strong) NSDictionary *fixture;
@property(nonatomic,strong) NSDictionary *captured;
@end
@implementation TestSync
/// Supply a deterministic read model while exercising the real menu construction code.
- (NSDictionary *)snapshot { return self.fixture; }
/// Capture Finder action identity without launching other apps in a logic test.
- (void)runAction:(NSMenuItem *)sender { self.captured = self.requests[@(sender.tag)]; }
@end

/// Verify menu copying preserves payload lookup and container menus ignore stale selections.
int main(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        controller = [FakeController new]; controller.targetedURL = [NSURL fileURLWithPath:@"/tmp"];
        controller.selectedItemURLs = @[[NSURL fileURLWithPath:@"/tmp/old.txt"]];
        class_replaceMethod(object_getClass(FIFinderSyncController.class), @selector(defaultController), (IMP)fakeController, "@@:");
        TestSync *sync = [TestSync new];
        sync.fixture = @{@"menuItems":@[@{@"id":@"new-file",@"enabled":@YES},@{@"id":@"favorites",@"enabled":@YES},@{@"id":@"copy-path",@"enabled":@YES},@{@"id":@"copy-to",@"enabled":@YES},@{@"id":@"move-to",@"enabled":@YES},@{@"id":@"trash",@"enabled":@YES},@{@"id":@"new-folder",@"enabled":@YES}],@"fileTypes":@[@{@"id":@"builtin-docx",@"displayName":@"Word 文档",@"extension":@"docx"}],@"favorites":@[@{@"id":@"fav-1",@"displayName":@"中文目录"}]};
        NSMenu *menu = [sync menuForMenuKind:FIMenuKindContextualMenuForContainer];
        NSCAssert(menu.numberOfItems==9,@"expected three actions, separator and settings");
        NSMenuItem *newFile = [[menu itemAtIndex:0].submenu itemAtIndex:0];
        NSCAssert(newFile.image && newFile.image.size.width == 16,@"file type icon missing or oversized");
        NSCAssert([menu itemAtIndex:0].image != nil,@"top-level Finder icon missing");
        NSCAssert([[menu itemAtIndex:1].submenu itemAtIndex:0].image != nil,@"favorite folder icon missing");
        NSMenuItem *copy = [newFile copy]; [sync runAction:copy];
        NSCAssert([sync.captured[@"resourceId"] isEqual:@"builtin-docx"],@"copied menu lost file type id");
        NSCAssert([sync.captured[@"context"][@"selectedPaths"] count]==0,@"container used stale selection");
        [sync runAction:[[[menu itemAtIndex:1].submenu itemAtIndex:0] copy]];
        NSCAssert([sync.captured[@"resourceId"] isEqual:@"fav-1"],@"favorite lost stable id");
        NSCAssert(![menu itemAtIndex:3].enabled && ![menu itemAtIndex:5].enabled,@"container must not copy or trash itself");
        NSCAssert([menu itemAtIndex:3].image && [menu itemAtIndex:4].image && [menu itemAtIndex:5].image,@"file action icons missing");
        menu = [sync menuForMenuKind:FIMenuKindContextualMenuForItems];
        [sync runAction:[[[menu itemAtIndex:3].submenu itemAtIndex:0] copy]];
        NSCAssert([sync.captured[@"action"] isEqual:@"copy_items"] && [sync.captured[@"resourceId"] isEqual:@"fav-1"],@"copy destination lost identity");
        NSCAssert([menu itemAtIndex:5].enabled,@"selected file should support recoverable trash");
        NSCAssert([menu itemAtIndex:6].image != nil,@"new folder icon missing");
        [sync runAction:[[menu itemAtIndex:6] copy]];
        NSCAssert([sync.captured[@"action"] isEqual:@"new_folder"],@"new folder action lost");
        NSMutableArray *many = [NSMutableArray new];
        for (int index=0; index<100; index++) { [many addObject:@{@"id":[NSString stringWithFormat:@"fav-%d",index],@"displayName":[NSString stringWithFormat:@"目录 %d",index]}]; }
        NSMutableDictionary *benchmark = sync.fixture.mutableCopy;benchmark[@"favorites"] = many;sync.fixture = benchmark;
        NSMutableArray<NSNumber *> *times = [NSMutableArray new];
        for (int index=0; index<30; index++) {
            @autoreleasepool {
                CFAbsoluteTime start = CFAbsoluteTimeGetCurrent();
                [sync menuForMenuKind:FIMenuKindContextualMenuForItems];
                [times addObject:@((CFAbsoluteTimeGetCurrent()-start)*1000)];
            }
        }
        [times sortUsingSelector:@selector(compare:)];
        NSLog(@"100 favorites, 30 samples: menu build p95 %.2f ms",times[28].doubleValue);
        NSLog(@"Menu tests passed: copied tags, stable ids, container selection");
        // Optional visual QA renders the production menu in this test process, not in Finder.
        if (getenv("QYJ_MENU_PREVIEW")) {
            QingRightSync *preview = [QingRightSync new];
            NSMenu *rendered = [preview menuForMenuKind:FIMenuKindContextualMenuForItems];
            dispatch_after(dispatch_time(DISPATCH_TIME_NOW,NSEC_PER_SEC/2),dispatch_get_main_queue(),^{
                NSArray *windows = CFBridgingRelease(CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly,kCGNullWindowID));
                for (NSDictionary *window in windows) {
                    NSDictionary *bounds = window[(__bridge id)kCGWindowBounds];
                    if ([window[(__bridge id)kCGWindowOwnerPID] intValue]==getpid() && [bounds[@"Height"] doubleValue]>150 && [bounds[@"Width"] doubleValue]<600) {
                        NSTask *capture = [NSTask new];capture.executableURL = [NSURL fileURLWithPath:@"/usr/sbin/screencapture"];
                        capture.arguments = @[@"-x",@"-l",[window[(__bridge id)kCGWindowNumber] stringValue],@"output/menu-native.png"];
                        [capture launchAndReturnError:nil];[capture waitUntilExit];break;
                    }
                }
                [rendered cancelTracking];
            });
            NSRect frame = NSScreen.mainScreen.visibleFrame;
            [rendered popUpMenuPositioningItem:nil atLocation:NSMakePoint(NSMidX(frame),NSMidY(frame)+200) inView:nil];
        }
    }
    return 0;
}
