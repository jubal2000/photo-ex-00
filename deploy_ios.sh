#!/bin/bash
set -e

SIGNING_ID="Apple Development: kim ju hyun (2LQZGYX35H)"
DEVICE_ID="23BED1A8-D403-579F-A0FE-3999D486F618"
BUNDLE_ID="com.jhfactory.poca00"
BUNDLE="target/dx/photo_exchanger/release/ios/PhotoExchanger.app"
ICON_DIR="assets/icons"
ENTITLEMENTS="/tmp/pe_entitlements.plist"

echo "▶ Loading env..."
set -a && source .env && set +a

echo "▶ Building and signing..."
dx build --platform ios --release --device --codesign --apple-team-id "$SIGNING_ID"

echo "▶ Extracting entitlements (before modifying bundle)..."
codesign -d --entitlements :- "$BUNDLE" > "$ENTITLEMENTS"

echo "▶ Injecting icons..."
for f in "$ICON_DIR"/AppIcon-*.png; do
    cp "$f" "$BUNDLE/"
done

echo "▶ Patching Info.plist..."
python3 - <<'PYEOF'
import plistlib

path = "target/dx/photo_exchanger/release/ios/PhotoExchanger.app/Info.plist"
with open(path, "rb") as f:
    pl = plistlib.load(f)

pl["CFBundleIcons"] = {
    "CFBundlePrimaryIcon": {
        "CFBundleIconFiles": [
            "AppIcon-20", "AppIcon-29", "AppIcon-40",
            "AppIcon-60", "AppIcon-76", "AppIcon-83.5", "AppIcon-1024",
        ],
        "UIPrerenderedIcon": False,
    }
}
pl["CFBundleIcons~ipad"] = {
    "CFBundlePrimaryIcon": {
        "CFBundleIconFiles": [
            "AppIcon-20", "AppIcon-29", "AppIcon-40",
            "AppIcon-76", "AppIcon-83.5", "AppIcon-1024",
        ],
        "UIPrerenderedIcon": False,
    }
}
pl["CFBundleURLTypes"] = [
    {
        "CFBundleURLName": "com.jhfactory.poca00",
        "CFBundleURLSchemes": ["poca00"],
    }
]
pl["NSPhotoLibraryUsageDescription"] = "포토카드 및 프로필 사진을 등록하기 위해 사진 보관함에 접근합니다."
pl["NSCameraUsageDescription"] = "포토카드 및 프로필 사진을 촬영하여 등록하기 위해 카메라에 접근합니다."
pl["NSPhotoLibraryAddUsageDescription"] = "사진을 저장하기 위해 사진 보관함에 접근합니다."

with open(path, "wb") as f:
    plistlib.dump(pl, f)
print("  Info.plist updated")
PYEOF

echo "▶ Checking for explicit poca00 provisioning profile with Apple Sign In..."
PROV_DIR="$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"
SPECIFIC_PROV=""
for f in "$PROV_DIR"/*.mobileprovision; do
    if [ -f "$f" ]; then
        has_id=$(security cms -D -i "$f" 2>/dev/null | grep -i "com.jhfactory.poca00" || true)
        if [ -n "$has_id" ]; then
            SPECIFIC_PROV="$f"
            break
        fi
    fi
done

if [ -n "$SPECIFIC_PROV" ]; then
    echo "  Found explicit poca00 profile: $(basename "$SPECIFIC_PROV")"
    cp "$SPECIFIC_PROV" "$BUNDLE/embedded.mobileprovision"
    python3 - <<'PYEOF'
import plistlib
path = "/tmp/pe_entitlements.plist"
with open(path, "rb") as f:
    pl = plistlib.load(f)
pl["com.apple.developer.applesignin"] = ["Default"]
with open(path, "wb") as f:
    plistlib.dump(pl, f)
print("  Injected com.apple.developer.applesignin entitlement")
PYEOF
else
    echo "  Using wildcard profile (Sign in with Apple capability requires explicit poca00 profile)"
fi

echo "▶ Re-signing with entitlements..."
codesign --force --sign "$SIGNING_ID" \
    --entitlements "$ENTITLEMENTS" \
    "$BUNDLE"

echo "▶ Installing to device..."
xcrun devicectl device install app \
    --device "$DEVICE_ID" \
    "$BUNDLE"

echo "▶ Launching..."
xcrun devicectl device process launch \
    --device "$DEVICE_ID" \
    "$BUNDLE_ID"

echo "✓ Done"
