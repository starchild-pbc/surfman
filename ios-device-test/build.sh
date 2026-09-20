#!/bin/bash

set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
build_dir="${IOS_DEVICE_TEST_BUILD_DIR:-$repo_root/target/ios-device-test}"
rust_target_dir="$build_dir/rust"
payload_dir="$build_dir/Payload"
app_dir="$payload_dir/SurfmanIosDeviceTest.app"
binary_path="$app_dir/SurfmanIosDeviceTest"
plist_path="$app_dir/Info.plist"
ipa_path="$build_dir/surfman-ios-device-test.ipa"

mkdir -p "$build_dir"
rm -rf "$rust_target_dir" "$payload_dir" "$ipa_path"
mkdir -p "$app_dir"

IPHONEOS_DEPLOYMENT_TARGET=15.0 \
  CARGO_TARGET_DIR="$rust_target_dir" cargo build \
  --manifest-path "$script_dir/Cargo.toml" \
  --release \
  --target aarch64-apple-ios

xcrun --sdk iphoneos clang \
  -arch arm64 \
  -miphoneos-version-min=15.0 \
  -fobjc-arc \
  -Wno-deprecated-declarations \
  "$script_dir/main.m" \
  "$rust_target_dir/aarch64-apple-ios/release/libsurfman_ios_device_test.a" \
  -framework CoreFoundation \
  -framework CoreVideo \
  -framework Foundation \
  -framework IOSurface \
  -framework OpenGLES \
  -framework UIKit \
  -o "$binary_path"

plutil -create xml1 "$plist_path"
plutil -insert CFBundleDevelopmentRegion -string en "$plist_path"
plutil -insert CFBundleExecutable -string SurfmanIosDeviceTest "$plist_path"
plutil -insert CFBundleIdentifier -string org.mozilla.surfman-ios-device-test "$plist_path"
plutil -insert CFBundleInfoDictionaryVersion -string 6.0 "$plist_path"
plutil -insert CFBundleName -string SurfmanIosDeviceTest "$plist_path"
plutil -insert CFBundlePackageType -string APPL "$plist_path"
plutil -insert CFBundleShortVersionString -string 1.0 "$plist_path"
plutil -insert CFBundleVersion -string 1 "$plist_path"
plutil -insert LSRequiresIPhoneOS -bool true "$plist_path"
plutil -insert MinimumOSVersion -string 15.0 "$plist_path"
plutil -insert UIDeviceFamily -xml '<array><integer>1</integer></array>' "$plist_path"
plutil -insert UILaunchScreen -xml '<dict/>' "$plist_path"

codesign --force --sign - --timestamp=none "$app_dir"
(
  cd "$build_dir"
  /usr/bin/zip -qry "$ipa_path" Payload
)

printf '%s\n' "$ipa_path"
