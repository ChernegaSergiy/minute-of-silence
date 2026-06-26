#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>

bool macos_detect_system_theme(void) {
    NSString *style = [[NSUserDefaults standardUserDefaults] stringForKey:@"AppleInterfaceStyle"];
    if (style) {
        return [style.lowercaseString containsString:@"dark"];
    }
    if (@available(macOS 10.14, *)) {
        NSAppearance *appearance = NSAppearance.currentAppearance;
        if (appearance) {
            return [[appearance bestMatchFromAppearancesWithNames:@[
                NSAppearanceNameDarkAqua, NSAppearanceNameAqua
            ]] isEqualToString:NSAppearanceNameDarkAqua];
        }
    }
    return false;
}
