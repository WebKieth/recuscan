#!/usr/bin/env bash
set -euo pipefail

: "${ANDROID_HOME:?}" "${BUILD_TOOLS_VERSION:?}" "${RUNNER_TEMP:?}"
: "${ANDROID_KEYSTORE_BASE64:?}" "${ANDROID_KEYSTORE_PASSWORD:?}"
: "${ANDROID_KEY_ALIAS:?}" "${ANDROID_KEY_PASSWORD:?}"

tools="$ANDROID_HOME/build-tools/$BUILD_TOOLS_VERSION"
keystore=$(mktemp "$RUNNER_TEMP/recuscan-keystore.XXXXXX")
trap 'rm -f "$keystore"' EXIT
chmod 600 "$keystore"
printf '%s' "$ANDROID_KEYSTORE_BASE64" | base64 --decode > "$keystore"
mkdir -p release

# Sign after Gradle builds: the generated Android project stays disposable.
for entry in arm64-v8a:arm64 armeabi-v7a:arm; do
  abi=${entry%%:*}
  flavor=${entry##*:}
  mapfile -t apks < <(find src-tauri/gen/android/app/build/outputs/apk -type f -path "*/$flavor/release/*" -name '*.apk')
  if [[ ${#apks[@]} -ne 1 ]]; then
    echo "::error::Expected one release APK for $abi, found ${#apks[@]}"
    exit 1
  fi
  output="release/RecuScan-$abi.apk"
  "$tools/zipalign" -P 16 -f 4 "${apks[0]}" "$output"
  "$tools/apksigner" sign --ks "$keystore" \
    --ks-key-alias "$ANDROID_KEY_ALIAS" \
    --ks-pass env:ANDROID_KEYSTORE_PASSWORD --key-pass env:ANDROID_KEY_PASSWORD \
    "$output"
  "$tools/apksigner" verify --verbose "$output"
  "$tools/zipalign" -c -P 16 4 "$output"
done

# apksigner may create sidecar .idsig files; only publish APKs and checksums.
cd release
sha256sum ./*.apk > SHA256SUMS.txt
rm -f -- ./*.idsig
