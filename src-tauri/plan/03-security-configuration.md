# Security & Configuration Improvements

## Overview

Enhance security settings and complete application metadata for production readiness.

## Current Issues

1. **Content Security Policy (CSP)** is set to `null` (disabled)
2. **Filesystem scope** is overly permissive (`$HOME/**`)
3. **Bundle metadata** is incomplete (no description, copyright, etc.)
4. **No asset security configuration**

## Security Improvements

### 1. Configure Content Security Policy

**Current configuration in `tauri.conf.json`:**

```json
{
  "app": {
    "security": {
      "csp": null
    }
  }
}
```

**Recommended configuration:**

```json
{
  "app": {
    "security": {
      "csp": {
        "default-src": "'self'",
        "script-src": [
          "'self'",
          "'unsafe-inline'",
          "'unsafe-eval'"
        ],
        "style-src": [
          "'self'",
          "'unsafe-inline'"
        ],
        "img-src": [
          "'self'",
          "data:",
          "blob:",
          "https:"
        ],
        "font-src": [
          "'self'",
          "data:"
        ],
        "connect-src": [
          "'self'",
          "ws://localhost:*",
          "wss://localhost:*",
          "http://localhost:*",
          "https://strudel.cc"
        ],
        "media-src": [
          "'self'",
          "data:",
          "blob:",
          "https:"
        ],
        "worker-src": [
          "'self'",
          "blob:"
        ]
      },
      "dangerousRemoteDomainIpcAccess": [],
      "assetProtocol": {
        "enable": true,
        "scope": ["$APPDATA/**", "$RESOURCE/**"]
      }
    }
  }
}
```

### 2. Refine Filesystem Permissions

**Current configuration:**

```json
{
  "plugins": {
    "fs": {
      "all": true,
      "scope": ["$HOME/**", "$HOME", "$HOME/*"]
    }
  }
}
```

**Problems:**
- `"all": true` grants every filesystem permission
- `$HOME/**` allows reading/writing anywhere in home directory
- Too permissive for a music live-coding tool

**Recommended configuration:**

```json
{
  "plugins": {
    "fs": {
      "scope": [
        "$APPDATA/**",
        "$APPCONFIG/**",
        "$APPLOCALDATA/**",
        "$APPLOG/**",
        {
          "path": "$HOME/Documents/Strudel/**"
        },
        {
          "path": "$HOME/Music/Strudel/**"
        }
      ],
      "readDir": true,
      "readFile": true,
      "writeFile": true,
      "createDir": true,
      "removeDir": false,
      "removeFile": true,
      "renameFile": true,
      "copyFile": true,
      "exists": true
    }
  }
}
```

This configuration:
- Removes `"all": true` in favor of explicit permissions
- Limits scope to app-specific directories
- Allows reading/writing in user's Documents and Music folders under Strudel subdirectory
- Disables directory removal (removeDir: false) for safety

### 3. Configure Dialog Plugin Permissions

**Current configuration:**

```json
{
  "plugins": {
    "dialog": {
      "all": true
    }
  }
}
```

**Recommended configuration:**

```json
{
  "plugins": {
    "dialog": {
      "open": true,
      "save": true,
      "message": true,
      "confirm": true,
      "ask": true
    }
  }
}
```

### 4. Add Clipboard Security

The app uses `tauri-plugin-clipboard-manager`. Configure it properly:

```json
{
  "plugins": {
    "clipboard": {
      "readText": true,
      "writeText": true,
      "readImage": false,
      "writeImage": false
    }
  }
}
```

## Configuration Improvements

### 1. Complete Bundle Metadata

**Update `tauri.conf.json`:**

```json
{
  "productName": "Strudel (nukleas fork)",
  "version": "0.1.0",
  "identifier": "com.nukleas.strudel",
  "bundle": {
    "active": true,
    "category": "AudioVideo",
    "copyright": "Copyright © 2024 Strudel Contributors. Licensed under AGPL-3.0",
    "shortDescription": "Live coding music in the browser - Desktop Edition",
    "longDescription": "Strudel is a live coding pattern language for making music. It's a port of TidalCycles to JavaScript that runs in the browser using the Web Audio API. This desktop app adds MIDI and OSC support for integration with external music software.",
    "externalBin": [],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "targets": "all",
    "macOS": {
      "entitlements": null,
      "exceptionDomain": "",
      "frameworks": [],
      "providerShortName": null,
      "signingIdentity": null,
      "minimumSystemVersion": "10.13"
    },
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": "",
      "wix": {
        "language": "en-US"
      }
    },
    "linux": {
      "deb": {
        "depends": []
      }
    }
  }
}
```

### 2. Window Security Configuration

**Update window configuration:**

```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "Strudel REPL",
        "width": 1800,
        "height": 1200,
        "resizable": true,
        "fullscreen": false,
        "visible": true,
        "transparent": false,
        "decorations": true,
        "alwaysOnTop": false,
        "contentProtected": false,
        "skipTaskbar": false,
        "theme": "Auto",
        "titleBarStyle": "Visible",
        "hiddenTitle": false,
        "acceptFirstMouse": false,
        "tabbingIdentifier": null,
        "additionalBrowserArgs": "",
        "userAgent": null,
        "webviewInstallMode": {
          "type": "DownloadBootstrapper"
        },
        "proxyUrl": null
      }
    ]
  }
}
```

### 3. Build Configuration

**Update build settings:**

```json
{
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:4321",
    "frontendDist": "../website/dist",
    "withGlobalTauri": false,
    "runner": null
  }
}
```

## Environment-Specific Configuration

Consider using different configurations for development vs production:

### Development Config (`tauri.conf.dev.json`)

```json
{
  "app": {
    "security": {
      "csp": null
    }
  },
  "plugins": {
    "fs": {
      "all": true,
      "scope": ["$HOME/**"]
    }
  }
}
```

### Production Config (`tauri.conf.json`)

Use the tightened security settings described above.

## Additional Security Measures

### 1. Disable Dangerous Features in Production

```rust
// In main.rs
#[cfg(not(debug_assertions))]
fn should_allow_navigation(_url: &str) -> bool {
    false  // Prevent navigation in production
}

#[cfg(debug_assertions)]
fn should_allow_navigation(_url: &str) -> bool {
    true  // Allow in development
}
```

### 2. Validate IPC Commands

Add validation to commands:

```rust
#[tauri::command]
pub async fn sendmidi(
    messagesfromjs: Vec<MessageFromJS>,
    state: tauri::State<'_, AsyncInputTransmit>
) -> Result<(), String> {
    // Validate input
    if messagesfromjs.len() > 1000 {
        return Err("Too many MIDI messages in one batch".to_string());
    }

    for msg in &messagesfromjs {
        if msg.message.len() > 256 {
            return Err("MIDI message too large".to_string());
        }
    }

    // ... rest of implementation
}
```

### 3. Rate Limiting

Implement rate limiting for commands:

```rust
use std::time::{Duration, Instant};
use std::sync::Mutex;

pub struct RateLimiter {
    last_call: Mutex<Instant>,
    min_interval: Duration,
}

impl RateLimiter {
    fn check(&self) -> Result<(), String> {
        let mut last = self.last_call.lock().unwrap();
        if last.elapsed() < self.min_interval {
            return Err("Rate limit exceeded".to_string());
        }
        *last = Instant::now();
        Ok(())
    }
}
```

## Benefits

1. **Enhanced Security**: Proper CSP and permission scoping
2. **Better UX**: Complete metadata shows in installers and app lists
3. **Production Ready**: Suitable for distribution
4. **Audit Trail**: Clear permission boundaries
5. **Future Proof**: Easier to maintain and update

## Implementation Checklist

- [ ] Update CSP configuration in tauri.conf.json
- [ ] Refine filesystem scope
- [ ] Configure dialog plugin permissions explicitly
- [ ] Complete bundle metadata
- [ ] Add clipboard security settings
- [ ] Test app with new security settings
- [ ] Verify file operations still work
- [ ] Document any breaking changes for users
- [ ] Add input validation to commands
- [ ] Consider rate limiting for high-frequency commands

## Testing

After implementing these changes:

1. Test file save/load operations
2. Verify MIDI/OSC still function
3. Check clipboard operations
4. Ensure dialogs open correctly
5. Confirm no CSP violations in console
6. Test on all target platforms (Windows, macOS, Linux)

## References

- [Tauri Security Best Practices](https://v2.tauri.app/security/)
- [Content Security Policy Guide](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP)
- [Tauri Configuration Reference](https://v2.tauri.app/reference/config/)
