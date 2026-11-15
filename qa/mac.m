#import <Cocoa/Cocoa.h>
#import <ApplicationServices/ApplicationServices.h>
#import <Metal/Metal.h>
#import <unistd.h>
static CGPoint target;
static NSView *view;
static NSArray *paths;
static void mouse(CGEventType kind, CGPoint p) {
    CGEventRef e=CGEventCreateMouseEvent(NULL,kind,p,kCGMouseButtonLeft);
    CGEventPost(kCGHIDEventTap,e); CFRelease(e);
}
static void later(double s, void(^block)(void)) {
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW,(int64_t)(s*NSEC_PER_SEC)),dispatch_get_main_queue(),block);
}
@interface Source:NSView<NSDraggingSource>
@end
@implementation Source
-(void)drawRect:(NSRect)r {[[NSColor systemBlueColor] setFill]; NSRectFill(self.bounds);}
-(BOOL)acceptsFirstMouse:(NSEvent*)event {return YES;}
-(void)mouseDown:(NSEvent*)event {}
-(void)mouseDragged:(NSEvent*)event {
    NSMutableArray *items=[NSMutableArray array];
    for (NSString *path in paths) {
        NSDraggingItem *item=[[NSDraggingItem alloc] initWithPasteboardWriter:[NSURL fileURLWithPath:path]];
        [item setDraggingFrame:NSMakeRect(0,0,32,32) contents:[NSImage imageNamed:NSImageNameFolder]];
        [items addObject:item];
    }
    [self beginDraggingSessionWithItems:items event:event source:self];
}
-(NSDragOperation)draggingSession:(NSDraggingSession*)session sourceOperationMaskForDraggingContext:(NSDraggingContext)context {return NSDragOperationCopy;}
-(void)draggingSession:(NSDraggingSession*)session endedAtPoint:(NSPoint)p operation:(NSDragOperation)op {
    printf("AppKit drop operation %lu\n",(unsigned long)op); fflush(stdout); exit(op?0:3);
}
@end
int main(int argc,const char **argv) {
    @autoreleasepool {
        NSArray *args=NSProcessInfo.processInfo.arguments;
        if(argc<3) return 2;
        pid_t pid=atoi(argv[2]); CGRect bounds=CGRectZero;
        NSArray *windows=CFBridgingRelease(CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly,kCGNullWindowID));
        for(NSDictionary *w in windows){
            if([w[(id)kCGWindowOwnerPID] intValue]!=pid || [w[(id)kCGWindowLayer] intValue]!=0) continue;
            CGRect rect;
            if(CGRectMakeWithDictionaryRepresentation((__bridge CFDictionaryRef)w[(id)kCGWindowBounds],&rect) && rect.size.width>300){bounds=rect;break;}
        }
        if(CGRectIsEmpty(bounds)){fprintf(stderr,"No window for pid %d, Metal=%s\n",pid,MTLCreateSystemDefaultDevice().name.UTF8String?:"none");return 1;}
        if([args[1] isEqual:@"window"]){printf("%.0f %.0f %.0f %.0f\n",bounds.origin.x,bounds.origin.y,bounds.size.width,bounds.size.height);return 0;}
        if([args[1] isEqual:@"click"]){
            [[NSRunningApplication runningApplicationWithProcessIdentifier:pid] activateWithOptions:NSApplicationActivateIgnoringOtherApps];
            usleep(150000);
            CGPoint p=CGPointMake(CGRectGetMaxX(bounds)-65,CGRectGetMaxY(bounds)-32);
            mouse(kCGEventMouseMoved,p); mouse(kCGEventLeftMouseDown,p); usleep(80000); mouse(kCGEventLeftMouseUp,p);return 0;
        }
        if(argc<4)return 2;
        paths=[args subarrayWithRange:NSMakeRange(3,argc-3)];
        [NSApplication sharedApplication]; [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
        target=CGPointMake(CGRectGetMidX(bounds),CGRectGetMidY(bounds));
        CGPoint start=CGPointMake(target.x-200,target.y);
        CGFloat screenHeight=CGDisplayBounds(CGMainDisplayID()).size.height;
        NSWindow *window=[[NSWindow alloc]initWithContentRect:NSMakeRect(start.x-45,screenHeight-start.y-30,90,60) styleMask:NSWindowStyleMaskBorderless backing:NSBackingStoreBuffered defer:NO];
        window.level=NSFloatingWindowLevel; view=[[Source alloc] initWithFrame:NSMakeRect(0,0,90,60)]; window.contentView=view;
        [window orderFrontRegardless]; [NSApp activateIgnoringOtherApps:YES];
        later(0.6,^{mouse(kCGEventMouseMoved,start); mouse(kCGEventLeftMouseDown,start);});
        for(int i=1;i<=20;i++){CGPoint p=CGPointMake(start.x+10*i,start.y);later(0.7+0.05*i,^{mouse(kCGEventLeftMouseDragged,p);});}
        later(2.0,^{mouse(kCGEventLeftMouseUp,target);});
        later(7,^{fprintf(stderr,"AppKit drag session timed out\n");exit(4);});
        [NSApp run];
    }
    return 0;
}
