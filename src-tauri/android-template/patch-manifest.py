#!/usr/bin/env python3
"""Adds the permissions and components of the ceremony to the Android manifest.

`tauri android init` generates the manifest, so it is patched instead of being
replaced. Running the script twice changes nothing the second time.

Usage: patch-manifest.py path/to/AndroidManifest.xml
"""

import re
import sys

MARKER = "minute-of-silence-ceremony"

PERMISSIONS = """    <!-- {marker}:permissions -->
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
    <uses-permission android:name="android.permission.WAKE_LOCK" />
    <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_SPECIAL_USE" />
    <uses-permission android:name="android.permission.USE_FULL_SCREEN_INTENT" />
    <uses-permission android:name="android.permission.SCHEDULE_EXACT_ALARM" android:maxSdkVersion="32" />
    <uses-permission android:name="android.permission.USE_EXACT_ALARM" />

"""

COMPONENTS = """
        <!-- {marker}:components -->
        <receiver android:name=".CeremonyAlarmReceiver" android:exported="false" />

        <!-- Disabled until the autostart setting is switched on. -->
        <receiver
            android:name=".BootReceiver"
            android:enabled="false"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.BOOT_COMPLETED" />
                <action android:name="android.intent.action.MY_PACKAGE_REPLACED" />
            </intent-filter>
        </receiver>

        <service
            android:name=".CeremonyService"
            android:exported="false"
            android:foregroundServiceType="specialUse">
            <property
                android:name="android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE"
                android:value="daily-memorial-ceremony" />
        </service>
"""


def patch(text: str) -> str:
    if MARKER in text:
        return text

    if "<application" not in text or "</application>" not in text:
        raise SystemExit("The manifest has no <application> element")

    text = text.replace(
        "<application", PERMISSIONS.format(marker=MARKER) + "    <application", 1
    )
    text = text.replace("</application>", COMPONENTS.format(marker=MARKER) + "    </application>", 1)

    # Let the activity open over the lock screen when the alarm starts it.
    if "android:showWhenLocked" not in text:
        text = re.sub(
            r"<activity\b",
            '<activity android:showWhenLocked="true" android:turnScreenOn="true"',
            text,
            count=1,
        )
    return text


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    path = sys.argv[1]
    with open(path, encoding="utf-8") as manifest:
        original = manifest.read()
    patched = patch(original)
    if patched != original:
        with open(path, "w", encoding="utf-8") as manifest:
            manifest.write(patched)
        print(f"Patched {path}")
    else:
        print(f"{path} is already patched")


if __name__ == "__main__":
    main()
