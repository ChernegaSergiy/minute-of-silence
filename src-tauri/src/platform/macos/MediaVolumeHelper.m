#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>
#import <CoreAudio/CoreAudio.h>
#import <dlfcn.h>

// MediaRemote private framework — used by media keys (F7/F8/F9).
// Works with Chrome, Safari, Spotify, Apple Music, VLC, IINA, etc.
static BOOL mediaRemoteAvailable = NO;
static void (*MRMediaRemoteSendCommand)(int command, id userInfo);

static void initMediaRemote(void) {
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        void *handle = dlopen("/System/Library/PrivateFrameworks/MediaRemote.framework/MediaRemote", RTLD_NOW);
        if (handle) {
            MRMediaRemoteSendCommand = dlsym(handle, "MRMediaRemoteSendCommand");
            if (MRMediaRemoteSendCommand) {
                mediaRemoteAvailable = YES;
            }
        }
    });
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

// kMRPause = 1, kMRPlay = 0
#define kMRPause 1
#define kMRPlay 0

char* macos_pause_all(void) {
    @autoreleasepool {
        initMediaRemote();
        if (mediaRemoteAvailable) {
            MRMediaRemoteSendCommand(kMRPause, nil);
            return strdup("__mr__");
        }
        return strdup("");
    }
}

void macos_resume_players(const char *bundleIDsCsv) {
    @autoreleasepool {
        initMediaRemote();
        if (mediaRemoteAvailable) {
            MRMediaRemoteSendCommand(kMRPlay, nil);
        }
    }
}

void macos_free_string(char *ptr) {
    free(ptr);
}
