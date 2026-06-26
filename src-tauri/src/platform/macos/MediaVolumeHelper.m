#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#import <CoreAudio/CoreAudio.h>

static NSString* runAppleScript(NSString *source) {
    NSAppleScript *script = [[NSAppleScript alloc] initWithSource:source];
    NSDictionary *error = nil;
    NSAppleEventDescriptor *result = [script executeAndReturnError:&error];
    return error ? nil : [result stringValue];
}

static AudioObjectID getDefaultOutputDevice(void) {
    AudioObjectID deviceID = kAudioObjectUnknown;
    UInt32 size = sizeof(deviceID);
    AudioObjectPropertyAddress address = {
        kAudioHardwarePropertyDefaultOutputDevice,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain
    };
    AudioObjectGetPropertyData(kAudioObjectSystemObject,
                               &address, 0, NULL, &size, &deviceID);
    return deviceID;
}

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

uint8_t macos_get_volume(void) {
    AudioObjectID deviceID = getDefaultOutputDevice();
    if (deviceID == kAudioObjectUnknown) return 255;

    Float32 volume = 0;
    UInt32 size = sizeof(volume);
    AudioObjectPropertyAddress address = {
        kAudioDevicePropertyVolumeScalar,
        kAudioDevicePropertyScopeOutput,
        kAudioObjectPropertyElementMain
    };

    OSStatus status = AudioObjectGetPropertyData(deviceID, &address, 0, NULL, &size, &volume);
    if (status != noErr) {
        address.mElement = kAudioObjectPropertyElementMaster;
        status = AudioObjectGetPropertyData(deviceID, &address, 0, NULL, &size, &volume);
    }

    return status == noErr ? (uint8_t)(volume * 100.0) : 255;
}

bool macos_set_volume(uint8_t level) {
    AudioObjectID deviceID = getDefaultOutputDevice();
    if (deviceID == kAudioObjectUnknown) return false;

    Float32 volume = (Float32)level / 100.0;
    UInt32 size = sizeof(volume);
    AudioObjectPropertyAddress address = {
        kAudioDevicePropertyVolumeScalar,
        kAudioDevicePropertyScopeOutput,
        kAudioObjectPropertyElementMain
    };

    OSStatus status = AudioObjectSetPropertyData(deviceID, &address, 0, NULL, size, &volume);
    if (status != noErr) {
        address.mElement = kAudioObjectPropertyElementMaster;
        status = AudioObjectSetPropertyData(deviceID, &address, 0, NULL, size, &volume);
    }

    return status == noErr;
}

int8_t macos_is_muted(void) {
    AudioObjectID deviceID = getDefaultOutputDevice();
    if (deviceID == kAudioObjectUnknown) return -1;

    UInt32 mute = 0;
    UInt32 size = sizeof(mute);
    AudioObjectPropertyAddress address = {
        kAudioDevicePropertyMute,
        kAudioDevicePropertyScopeOutput,
        kAudioObjectPropertyElementMain
    };

    OSStatus status = AudioObjectGetPropertyData(deviceID, &address, 0, NULL, &size, &mute);
    if (status != noErr) {
        address.mElement = kAudioObjectPropertyElementMaster;
        status = AudioObjectGetPropertyData(deviceID, &address, 0, NULL, &size, &mute);
    }

    return status == noErr ? (int8_t)(mute != 0 ? 1 : 0) : -1;
}

bool macos_set_mute(bool mute) {
    AudioObjectID deviceID = getDefaultOutputDevice();
    if (deviceID == kAudioObjectUnknown) return false;

    UInt32 muteVal = mute ? 1 : 0;
    UInt32 size = sizeof(muteVal);
    AudioObjectPropertyAddress address = {
        kAudioDevicePropertyMute,
        kAudioDevicePropertyScopeOutput,
        kAudioObjectPropertyElementMain
    };

    OSStatus status = AudioObjectSetPropertyData(deviceID, &address, 0, NULL, size, &muteVal);
    if (status != noErr) {
        address.mElement = kAudioObjectPropertyElementMaster;
        status = AudioObjectSetPropertyData(deviceID, &address, 0, NULL, size, &muteVal);
    }

    return status == noErr;
}

char* macos_pause_all(void) {
    @autoreleasepool {
        NSArray<NSRunningApplication *> *runningApps = NSWorkspace.sharedWorkspace.runningApplications;
        NSMutableArray<NSString *> *pausedBundleIDs = [NSMutableArray array];

        for (NSRunningApplication *app in runningApps) {
            NSString *bundleID = app.bundleIdentifier;
            if (!bundleID) continue;

            NSString *script = [NSString stringWithFormat:
                @"tell application id \"%@\"\n"
                 "    try\n"
                 "        if player state is playing then\n"
                 "            pause\n"
                 "            return \"paused\"\n"
                 "        end if\n"
                 "    end try\n"
                 "    return \"not_playing\"\n"
                 "end tell",
                bundleID];

            NSString *result = runAppleScript(script);
            if ([result isEqualToString:@"paused"]) {
                [pausedBundleIDs addObject:bundleID];
            }
        }

        NSString *joined = [pausedBundleIDs componentsJoinedByString:@","];
        return strdup(joined.UTF8String ?: "");
    }
}

void macos_resume_players(const char *bundleIDsCsv) {
    @autoreleasepool {
        NSString *csv = [NSString stringWithUTF8String:bundleIDsCsv];
        if (!csv) return;

        NSArray<NSString *> *bundleIDs = [csv componentsSeparatedByString:@","];
        NSSet<NSString *> *runningBundleIDs = [NSSet setWithArray:
            [[NSWorkspace.sharedWorkspace.runningApplications valueForKey:@"bundleIdentifier"] allObjects]];

        for (NSString *bundleID in bundleIDs) {
            if (bundleID.length == 0) continue;
            if (![runningBundleIDs containsObject:bundleID]) continue;

            NSString *script = [NSString stringWithFormat:
                @"tell application id \"%@\"\n"
                 "    try\n"
                 "        play\n"
                 "    end try\n"
                 "end tell",
                bundleID];
            runAppleScript(script);
        }
    }
}

void macos_free_string(char *ptr) {
    free(ptr);
}
