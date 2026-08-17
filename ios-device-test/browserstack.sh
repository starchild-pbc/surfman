#!/bin/bash

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <ipa> <artifact-directory>" >&2
  exit 2
fi

: "${BROWSERSTACK_USERNAME:?BROWSERSTACK_USERNAME is required}"
: "${BROWSERSTACK_ACCESS_KEY:?BROWSERSTACK_ACCESS_KEY is required}"

ipa_path="$1"
artifact_dir="$2"
api_root="https://api-cloud.browserstack.com/app-automate"
webdriver_root="https://hub-cloud.browserstack.com/wd/hub"
pass_marker="SURFMAN_IOS_DEVICE_TEST_PASS"
session_id=""
session_closed=false
test_passed=false

mkdir -p "$artifact_dir"

finish() {
  local exit_code=$?
  trap - EXIT
  if [[ "$test_passed" != true && -n "$session_id" ]]; then
    if [[ "$session_closed" != true ]]; then
      curl --silent --show-error \
        --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
        --request DELETE "$webdriver_root/session/$session_id" >/dev/null || true
    fi
    curl --silent --show-error \
      --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
      --request PUT \
      --header 'Content-Type: application/json' \
      --data '{"status":"failed","reason":"The surfman iOS device test failed."}' \
      "$api_root/sessions/$session_id.json" >/dev/null || true
  fi
  exit "$exit_code"
}
trap finish EXIT

upload_response="$(curl --silent --show-error --fail-with-body \
  --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
  --request POST \
  --form "file=@$ipa_path" \
  "$api_root/upload")"
app_url="$(jq -er '.app_url' <<<"$upload_response")"

build_name="${BROWSERSTACK_BUILD_NAME:-surfman iOS device test}"
session_payload="$(jq -n \
  --arg app "$app_url" \
  --arg build "$build_name" \
  '{capabilities: {alwaysMatch: {
    platformName: "iOS",
    "appium:deviceName": "iPhone 15",
    "appium:platformVersion": "17",
    "appium:automationName": "XCUITest",
    "appium:app": $app,
    "bstack:options": {
      projectName: "surfman",
      buildName: $build,
      sessionName: "surfman iOS device test",
      deviceLogs: true,
      debug: true,
      idleTimeout: 300
    }
  }}}')"
session_response="$(curl --silent --show-error --fail-with-body \
  --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
  --request POST \
  --header 'Content-Type: application/json' \
  --data "$session_payload" \
  "$webdriver_root/session")"
session_id="$(jq -er '.value.sessionId // .sessionId' <<<"$session_response")"
printf '%s\n' "$session_id" >"$artifact_dir/session-id.txt"

sleep 20
screenshot_response="$(curl --silent --show-error --fail-with-body \
  --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
  "$webdriver_root/session/$session_id/screenshot")"
jq -er '.value' <<<"$screenshot_response" | openssl base64 -d -A \
  >"$artifact_dir/screenshot.png"

curl --silent --show-error --fail-with-body \
  --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
  --request DELETE "$webdriver_root/session/$session_id" >/dev/null
session_closed=true

for _ in $(seq 1 30); do
  session_detail="$(curl --silent --show-error --fail-with-body \
    --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
    "$api_root/sessions/$session_id.json")"
  session_status="$(jq -er '.automation_session.status // .status' <<<"$session_detail")"
  if [[ "$session_status" == done ]]; then
    break
  fi
  if [[ "$session_status" == failed || "$session_status" == timeout ]]; then
    echo "BrowserStack session ended with status $session_status." >&2
    exit 1
  fi
  sleep 5
done
if [[ "${session_status:-}" != done ]]; then
  echo "BrowserStack session did not finish before the timeout." >&2
  exit 1
fi
jq '{automation_session: (.automation_session | {
  hashed_id,
  status,
  reason,
  device,
  os,
  os_version,
  build_hashed_id,
  build_name,
  project_name,
  name,
  browser_url
})}' <<<"$session_detail" >"$artifact_dir/session.json"

session_url="$(jq -er \
  '.automation_session.browser_url // .automation_session.public_url // .browser_url // .public_url' \
  <<<"$session_detail")"
printf '%s\n' "$session_url" >"$artifact_dir/session-url.txt"

device_logs_url="$(jq -er \
  '.automation_session.device_logs_url // .device_logs_url' \
  <<<"$session_detail")"
device_log_status=""
for _ in $(seq 1 12); do
  device_log_status="$(curl --silent --show-error \
    --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
    --output "$artifact_dir/device.log" \
    --write-out '%{http_code}' \
    "$device_logs_url")"
  if [[ "$device_log_status" == 200 ]]; then
    break
  fi
  if [[ "$device_log_status" != 404 ]]; then
    echo "BrowserStack device log request returned HTTP $device_log_status." >&2
    exit 1
  fi
  sleep 5
done
if [[ "$device_log_status" != 200 ]]; then
  echo "BrowserStack device log was not ready before the timeout." >&2
  exit 1
fi
grep -F "$pass_marker" "$artifact_dir/device.log" >"$artifact_dir/pass-marker.txt"

crash_logs_url="$(jq -r \
  '.automation_session.crash_logs_url // .crash_logs_url // empty' \
  <<<"$session_detail")"
if [[ -z "$crash_logs_url" ]]; then
  crash_logs_url="${device_logs_url%/devicelogs}/crashlogs"
fi
crash_status="$(curl --silent --show-error \
  --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
  --output "$artifact_dir/crash.log" \
  --write-out '%{http_code}' \
  "$crash_logs_url")"
if [[ "$crash_status" == 200 ]]; then
  echo "BrowserStack reported an application crash." >&2
  exit 1
fi
if [[ "$crash_status" != 404 ]]; then
  echo "BrowserStack crash log request returned HTTP $crash_status." >&2
  exit 1
fi

curl --silent --show-error --fail-with-body \
  --user "$BROWSERSTACK_USERNAME:$BROWSERSTACK_ACCESS_KEY" \
  --request PUT \
  --header 'Content-Type: application/json' \
  --data '{"status":"passed","reason":"The device log contains the surfman PASS marker."}' \
  "$api_root/sessions/$session_id.json" >/dev/null

test_passed=true
printf 'BrowserStack session: %s\n' "$session_id"
printf 'Device log marker: %s\n' "$pass_marker"
printf 'Session URL: %s\n' "$session_url"
