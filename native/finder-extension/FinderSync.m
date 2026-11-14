#import <FinderSync/FinderSync.h>
#import <AppKit/AppKit.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

/// Shared by the extension runtime and its local diagnostic reader.
NSDictionary *QYJReadSnapshot(void) {
    CFPreferencesAppSynchronize(CFSTR("local.qingyoujian"));
    CFPropertyListRef raw = CFPreferencesCopyAppValue(CFSTR("MenuSnapshot"), CFSTR("local.qingyoujian"));
    id value = CFBridgingRelease(raw);
    if (![value isKindOfClass:NSString.class]) { return @{}; }
    NSDictionary *snapshot = [NSJSONSerialization JSONObjectWithData:[value dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil];
    return [snapshot[@"schemaVersion"] integerValue] == 1 ? snapshot : @{};
}

@interface QingRightSync : FIFinderSync
@property(nonatomic,strong) NSMutableDictionary<NSNumber *, NSDictionary *> *requests;
@property(nonatomic) NSInteger nextTag;
@property(nonatomic,strong) NSMutableDictionary<NSString *,NSImage *> *symbolCache;
@end
@implementation QingRightSync
/// Register mounted local roots without scanning any directory contents.
- (instancetype)init {
    self = [super init];
    if (self) {
        [self updateRoots];
        [NSWorkspace.sharedWorkspace.notificationCenter addObserver:self selector:@selector(updateRoots) name:NSWorkspaceDidMountNotification object:nil];
        [NSWorkspace.sharedWorkspace.notificationCenter addObserver:self selector:@selector(updateRoots) name:NSWorkspaceDidUnmountNotification object:nil];
    }
    return self;
}
/// Mounted roots are refreshed only on mount/unmount notifications.
- (void)updateRoots {
    NSMutableSet *roots = [NSMutableSet setWithObject:[NSURL fileURLWithPath:@"/"]];
    for (NSURL *url in [NSFileManager.defaultManager mountedVolumeURLsIncludingResourceValuesForKeys:nil options:NSVolumeEnumerationSkipHiddenVolumes]) { [roots addObject:url]; }
    FIFinderSyncController.defaultController.directoryURLs = roots;
}
/// Read a single immutable snapshot for each menu construction.
- (NSDictionary *)snapshot {
    return QYJReadSnapshot();
}
/// Flatten icons to one 16pt bitmap; Finder copies menu images and must not receive 1024px reps.
- (NSImage *)smallIcon:(NSImage *)image tint:(BOOL)tint {
    if (!image) { return nil; }
    NSBitmapImageRep *rep = [[NSBitmapImageRep alloc] initWithBitmapDataPlanes:NULL pixelsWide:32 pixelsHigh:32 bitsPerSample:8 samplesPerPixel:4 hasAlpha:YES isPlanar:NO colorSpaceName:NSDeviceRGBColorSpace bytesPerRow:0 bitsPerPixel:0];
    rep.size = NSMakeSize(16,16);
    [NSGraphicsContext saveGraphicsState];
    NSGraphicsContext.currentContext = [NSGraphicsContext graphicsContextWithBitmapImageRep:rep];
    [NSApp.effectiveAppearance performAsCurrentDrawingAppearance:^{
        [image drawInRect:NSMakeRect(0,0,16,16)];
        if (tint) { [NSColor.labelColor set]; NSRectFillUsingOperation(NSMakeRect(0,0,16,16),NSCompositingOperationSourceAtop); }
    }];
    [NSGraphicsContext restoreGraphicsState];
    NSImage *small = [[NSImage alloc] initWithSize:NSMakeSize(16,16)];[small addRepresentation:rep];return small;
}
/// Bake the current appearance's text color because Finder may drop the template-image flag.
- (NSImage *)symbol:(NSString *)name {
    if (self.symbolCache[name]) { return self.symbolCache[name]; }
    NSImage *image = [NSImage imageWithSystemSymbolName:name accessibilityDescription:nil] ?: [NSImage imageWithSystemSymbolName:@"doc" accessibilityDescription:nil];
    NSImage *small = [self smallIcon:image tint:YES];
    if (small) { self.symbolCache[name] = small; }
    return small;
}
/// File-type icons follow the user's default apps, including Office or WPS when installed.
- (NSImage *)typeIcon:(NSString *)extension {
    UTType *type = [UTType typeWithFilenameExtension:extension] ?: UTTypeData;
    return [self smallIcon:[NSWorkspace.sharedWorkspace iconForContentType:type] tint:NO];
}
/// Favorite ids, not paths embedded in a copied menu, select copy/move destinations.
- (NSMenu *)destinationMenu:(NSString *)action context:(NSDictionary *)context snapshot:(NSDictionary *)snapshot {
    NSMenu *menu = [NSMenu new];menu.autoenablesItems = NO;
    for (NSDictionary *favorite in snapshot[@"favorites"]) {
        NSMenuItem *item = [self item:favorite[@"displayName"] action:@selector(runAction:) data:@{@"action":action,@"context":context,@"resourceId":favorite[@"id"]}];
        item.image = [self symbol:@"folder"];[menu addItem:item];
    }
    if (menu.numberOfItems > 0) { [menu addItem:NSMenuItem.separatorItem]; }
    NSMenuItem *choose = [self item:@"选择其他目录…" action:@selector(runAction:) data:@{@"action":action,@"context":context}];
    choose.image = [self symbol:@"folder.badge.plus"];[menu addItem:choose];return menu;
}
/// Menu targets are self; tags are assigned by AppKit and the payload is attached to each item.
- (NSMenuItem *)item:(NSString *)title action:(SEL)action data:(id)data {
    NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:title action:action keyEquivalent:@""];
    item.image = [self symbol:@"gearshape"];
    item.target = self; item.tag = ++self.nextTag;
    if (data) { self.requests[@(item.tag)] = data; }
    return item;
}
/// Finder copies menus; store payloads in the extension and use preserved integer tags.
- (NSMenu *)menuForMenuKind:(FIMenuKind)kind {
    if (kind != FIMenuKindContextualMenuForContainer && kind != FIMenuKindContextualMenuForItems) { return nil; }
    self.requests = [NSMutableDictionary new]; self.symbolCache = [NSMutableDictionary new]; self.nextTag = 0;
    NSDictionary *snapshot = [self snapshot];
    NSMenu *menu = [NSMenu new]; menu.autoenablesItems = NO;
    NSDictionary *titles = @{@"new-folder":@"新建文件夹",@"copy-to":@"复制到",@"move-to":@"剪切到",@"trash":@"移到废纸篓",@"new-file":@"新建文件",@"copy-path":@"拷贝路径",@"terminal":@"在此打开终端",@"show-hidden":@"显示所有隐藏文件",@"hide-hidden":@"隐藏所有隐藏文件",@"favorites":@"常用目录",@"add-favorite":@"添加到常用目录"};
    NSDictionary *symbols = @{@"new-folder":@"folder.badge.plus",@"new-file":@"doc.badge.plus",@"copy-path":@"link",@"copy-to":@"doc.on.doc",@"move-to":@"scissors",@"trash":@"trash",@"terminal":@"terminal",@"show-hidden":@"eye",@"hide-hidden":@"eye.slash",@"favorites":@"star",@"add-favorite":@"star.circle"};
    NSDictionary *context = @{ @"menuKind":kind == FIMenuKindContextualMenuForContainer ? @"container":@"items", @"targetedPath": FIFinderSyncController.defaultController.targetedURL.path ?: NSNull.null, @"selectedPaths":[FIFinderSyncController.defaultController.selectedItemURLs valueForKey:@"path"] ?: @[] };
    if (kind == FIMenuKindContextualMenuForContainer) { NSMutableDictionary *clean = context.mutableCopy; clean[@"selectedPaths"] = @[]; context = clean; }
    for (NSDictionary *config in snapshot[@"menuItems"]) {
        if (![config[@"enabled"] boolValue]) { continue; }
        NSString *key = config[@"id"];
        if (!titles[key]) { continue; }
        NSMutableDictionary *payload = [@{@"context":context} mutableCopy];
        NSMenuItem *entry = [self item:titles[key] action:@selector(runAction:) data:payload];
        entry.image = [self symbol:symbols[key]];
        if ([key isEqual:@"new-folder"]) { payload[@"action"] = @"new_folder"; }
        else if ([key isEqual:@"new-file"]) {
            NSMenu *sub = [NSMenu new]; sub.autoenablesItems = NO;
            for (NSDictionary *type in snapshot[@"fileTypes"]) {
                NSMutableDictionary *request = payload.mutableCopy; request[@"action"] = @"new_file"; request[@"resourceId"] = type[@"id"];
                NSMenuItem *typed = [self item:type[@"displayName"] action:@selector(runAction:) data:request];typed.image = [self typeIcon:type[@"extension"]];[sub addItem:typed];
            }
            NSMutableDictionary *custom = payload.mutableCopy; custom[@"action"] = @"new_file";
            [sub addItem:NSMenuItem.separatorItem]; NSMenuItem *customItem = [self item:@"自定义文件名…" action:@selector(runAction:) data:custom];customItem.image = [self symbol:@"doc.badge.plus"];[sub addItem:customItem];
            entry.submenu = sub;
        } else if ([key isEqual:@"copy-to"] || [key isEqual:@"move-to"]) {
            entry.submenu = [self destinationMenu:[key isEqual:@"copy-to"] ? @"copy_items" : @"move_items" context:context snapshot:snapshot];
            entry.enabled = kind == FIMenuKindContextualMenuForItems && [context[@"selectedPaths"] count] > 0;
        } else if ([key isEqual:@"trash"]) {
            payload[@"action"] = @"trash_items";
            entry.enabled = kind == FIMenuKindContextualMenuForItems && [context[@"selectedPaths"] count] > 0;
        } else if ([key isEqual:@"favorites"]) {
            NSMenu *sub = [NSMenu new]; sub.autoenablesItems = NO;
            for (NSDictionary *favorite in snapshot[@"favorites"]) {
                NSMenuItem *saved = [self item:favorite[@"displayName"] action:@selector(runAction:) data:@{@"action":@"open_favorite",@"resourceId":favorite[@"id"]}];saved.image = [self symbol:@"folder"];[sub addItem:saved];
            }
            if (sub.numberOfItems == 0) { NSMenuItem *empty = [self item:@"暂无常用目录" action:nil data:nil]; empty.enabled = NO; [sub addItem:empty]; }
            [sub addItem:NSMenuItem.separatorItem]; [sub addItem:[self item:@"管理常用目录…" action:@selector(runAction:) data:@{@"action":@"show_settings"}]];
            entry.submenu = sub;
        } else if ([key isEqual:@"copy-path"]) { entry.action = @selector(copyPaths:); }
        else if ([key isEqual:@"terminal"]) { payload[@"action"] = @"open_terminal"; }
        else if ([key isEqual:@"add-favorite"]) {
            payload[@"action"] = @"add_favorite";
            if (kind == FIMenuKindContextualMenuForContainer) { entry.title = @"添加当前目录到常用目录"; }
        } else { payload[@"action"] = @"set_hidden_visible"; payload[@"visible"] = @([key isEqual:@"show-hidden"]); }
        BOOL hasContext = [context[@"targetedPath"] isKindOfClass:NSString.class] || [context[@"selectedPaths"] count] == 1;
        BOOL selectionAction = [@[@"copy-path",@"copy-to",@"move-to",@"trash"] containsObject:key];
        if (!hasContext && !(selectionAction && [context[@"selectedPaths"] count] > 0) && ![key isEqual:@"favorites"] && ![key hasSuffix:@"hidden"]) { entry.enabled = NO; }
        [menu addItem:entry];
    }
    [menu addItem:NSMenuItem.separatorItem]; [menu addItem:[self item:@"轻右键设置…" action:@selector(runAction:) data:@{@"action":@"show_settings"}]];
    return menu;
}
/// Preserve raw POSIX paths, one line per selection, without reading file contents.
- (void)copyPaths:(NSMenuItem *)sender {
    NSDictionary *context = self.requests[@(sender.tag)][@"context"];
    NSArray *paths = context[@"selectedPaths"];
    if (paths.count == 0 && [context[@"targetedPath"] isKindOfClass:NSString.class]) { paths = @[context[@"targetedPath"]]; }
    if (paths.count == 0) { return; }
    [NSPasteboard.generalPasteboard clearContents];
    [NSPasteboard.generalPasteboard setString:[paths componentsJoinedByString:@"\n"] forType:NSPasteboardTypeString];
}
/// Send the attached request, not menu positions or localized display names.
- (void)runAction:(NSMenuItem *)sender {
    NSMutableDictionary *payload = [self.requests[@(sender.tag)] mutableCopy];
    payload[@"protocolVersion"] = @1; payload[@"requestId"] = NSUUID.UUID.UUIDString;
    NSData *data = [NSJSONSerialization dataWithJSONObject:payload options:0 error:nil];
    NSString *encoded = [[data base64EncodedStringWithOptions:0] stringByReplacingOccurrencesOfString:@"+" withString:@"-"];
    encoded = [[encoded stringByReplacingOccurrencesOfString:@"/" withString:@"_"] stringByReplacingOccurrencesOfString:@"=" withString:@""];
    NSURL *url = [NSURL URLWithString:[@"qingyoujian://action?payload=" stringByAppendingString:encoded]];
    if (url) { [NSWorkspace.sharedWorkspace openURL:url]; }
}
@end
