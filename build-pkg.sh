#!/bin/bash
set -e

APP_NAME="Claude Remote"
APP_PATH="app/src-tauri/target/release/bundle/macos/${APP_NAME}.app"
PKG_OUTPUT="app/src-tauri/target/release/bundle/${APP_NAME}.pkg"
VERSION=$(grep '"version"' app/src-tauri/tauri.conf.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')
IDENTIFIER="com.clauderemote.desktop"

echo "Building PKG installer for ${APP_NAME} v${VERSION}..."

if [ ! -d "$APP_PATH" ]; then
  echo "Error: ${APP_PATH} not found. Run 'npm run tauri build' first."
  exit 1
fi

WORK_DIR=$(mktemp -d)
SCRIPTS_DIR=$(mktemp -d)
trap 'rm -rf "${WORK_DIR}" "${SCRIPTS_DIR}"' EXIT

# Copy .app to staging
PAYLOAD_DIR="${WORK_DIR}/payload"
mkdir -p "${PAYLOAD_DIR}"
cp -R "$APP_PATH" "${PAYLOAD_DIR}/${APP_NAME}.app"

# Component plist: disable relocation so installer always puts in /Applications
COMPONENT_PLIST="${WORK_DIR}/component.plist"
cat > "${COMPONENT_PLIST}" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<dict>
		<key>BundleHasStrictIdentifier</key>
		<true/>
		<key>BundleIsRelocatable</key>
		<false/>
		<key>BundleIsVersionChecked</key>
		<true/>
		<key>BundleOverwriteAction</key>
		<string>upgrade</string>
		<key>RootRelativeBundlePath</key>
		<string>Claude Remote.app</string>
	</dict>
</array>
</plist>
PLIST

# Postinstall: open the app
cat > "${SCRIPTS_DIR}/postinstall" << 'SCRIPT'
#!/bin/bash
CURRENT_USER=$(stat -f "%Su" /dev/console)
su "$CURRENT_USER" -c 'open -a "/Applications/Claude Remote.app"' &
exit 0
SCRIPT
chmod +x "${SCRIPTS_DIR}/postinstall"

rm -f "${PKG_OUTPUT}"

# Build with --component-plist to disable relocation
pkgbuild \
  --root "${PAYLOAD_DIR}" \
  --component-plist "${COMPONENT_PLIST}" \
  --install-location "/Applications" \
  --identifier "${IDENTIFIER}" \
  --version "${VERSION}" \
  --scripts "${SCRIPTS_DIR}" \
  "${PKG_OUTPUT}"

echo ""
echo "PKG installer created: ${PKG_OUTPUT}"
echo "Size: $(du -h "${PKG_OUTPUT}" | cut -f1)"
