# Installing Strudel Desktop on macOS

## "App is damaged" Error Fix

If you download the macOS build from GitHub Actions and see an error like:

> "Strudel.app" is damaged and can't be opened. You should move it to the Trash.

This is because the app isn't code-signed (requires a paid Apple Developer account). Here's how to fix it:

### Option 1: Remove Quarantine Attribute (Recommended)

Open Terminal and run:

```bash
xattr -cr /Applications/Strudel.app
```

Or if the app is in your Downloads:

```bash
xattr -cr ~/Downloads/Strudel.app
```

Then you can open the app normally.

### Option 2: Right-Click to Open

1. Right-click (or Control+click) on the app
2. Select "Open" from the menu
3. Click "Open" in the dialog that appears

This only works the first time - subsequent launches will work normally.

### Why This Happens

Apps downloaded from the internet are "quarantined" by macOS Gatekeeper. Unsigned apps (like CI builds) trigger this error. Code signing requires:

- Apple Developer account ($99/year)
- Code signing certificates
- App notarization

For open source development builds, the workaround above is standard practice.

### Future: Code Signing

If we add code signing in the future, users won't need these workarounds. This would require:

1. Setting up Apple Developer account
2. Storing certificates as GitHub Secrets
3. Adding signing/notarization to CI workflow

See: https://v2.tauri.app/distribute/sign/macos/

---

**Note:** If you're uncomfortable removing the quarantine attribute, you can build from source instead:

```bash
git clone https://github.com/nukleas/strudel-desktop.git
cd strudel-desktop
pnpm install
pnpm tauri:build
```

The locally-built app won't have quarantine attributes.
