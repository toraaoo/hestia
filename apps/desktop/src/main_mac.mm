#if defined(__APPLE__)
#import <Cocoa/Cocoa.h>
#include "include/cef_app.h"
#include "include/cef_application_mac.h"
#include "include/wrapper/cef_library_loader.h"
#include "core/app/main_util.h"
#include <hestia/logging.h>

@interface AppApplication : NSApplication <CefAppProtocol>
@property(nonatomic, assign) BOOL handlingSendEvent;
@end

@implementation AppApplication
- (BOOL)isHandlingSendEvent { return _handlingSendEvent; }
- (void)setHandlingSendEvent:(BOOL)v { _handlingSendEvent = v; }
- (void)sendEvent:(NSEvent*)event {
    CefScopedSendingEvent sendingEventScoper;
    [super sendEvent:event];
}
@end

int main(int argc, char* argv[]) {
    hestia::init_logging();

    CefMainArgs main_args(argc, argv);
    CefScopedLibraryLoader loader;
    if (!loader.LoadInMain()) return 1;

    @autoreleasepool {
        auto cmd = desktop::app::CreateCommandLine(main_args);
        auto app = desktop::app::CreateApp(desktop::app::GetProcessType(cmd));

        [AppApplication sharedApplication];

        CefSettings settings;
        settings.no_sandbox = true;  // macOS sandbox via helper bundles
        const std::string exe = desktop::app::GetExecutableDirectory();
        CefString(&settings.root_cache_path) = exe + "/cache";

        if (!CefInitialize(main_args, settings, app, nullptr))
            return CefGetExitCode();

        CefRunMessageLoop();
        CefShutdown();
    }
    return 0;
}
#endif
