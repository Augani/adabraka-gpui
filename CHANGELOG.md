# Changelog

## 0.5.1 (2026-02-17)

### Performance

- **DirectX pipeline state caching** — skip redundant `set_pipeline_state` calls when consecutive batches use the same pipeline, saving ~6 D3D11 API calls per batch in text-heavy UIs
- **Cross-window text layout cache** — global LRU cache on `TextSystem` prevents re-shaping text that another window already shaped

## 0.5.0 (2026-02-15)

### Desktop Platform Features

Added 15 new cross-platform capabilities to the `Platform` trait, with implementations for macOS, Windows, and Linux:

**System Integration**
- **System power events** — subscribe to suspend/resume/lock/unlock/shutdown notifications (macOS: `NSWorkspace`, Windows: `WM_POWERBROADCAST`, Linux: stub for D-Bus logind)
- **Power save blocker** — prevent display sleep or app suspension (macOS: `IOPMAssertionCreateWithName`, Windows: `SetThreadExecutionState`, Linux: `dbus-send` screensaver inhibit + `systemd-inhibit`)
- **System idle time** — query time since last user input (macOS: `CGEventSourceSecondsSinceLastEventType`, Windows: `GetLastInputInfo`, Linux: X11 screensaver extension)
- **Network status** — query online/offline connectivity (macOS: `NWPathMonitor`, Windows: `INetworkListManager`, Linux: `/sys/class/net/*/operstate`)
- **OS info** — query OS name, version, arch, locale, hostname
- **Biometric authentication** — Touch ID (macOS), Windows Hello, with availability detection

**Window Management**
- **User attention** — request/cancel taskbar attention (macOS: dock bounce, Windows: `FlashWindowEx`, Linux: X11 EWMH `_NET_WM_STATE_DEMANDS_ATTENTION`)
- **Progress bar** — set taskbar/dock progress state (macOS: `NSDockTile`, Windows: `ITaskbarList3`)
- **Window state save/restore** — `WindowState` struct for persisting window bounds, display, and fullscreen state
- **Window positioner** — `WindowPosition` enum for semantic positioning (center, tray-relative, corners)

**UI & Input**
- **Native dialogs** — modal alert/confirm dialogs with customizable buttons (macOS: `NSAlert`, Windows: `TaskDialogIndirect`, Linux: `zenity`/`kdialog`)
- **Context menus** — show native context menus at a position (macOS: `NSMenu`, Windows: `TrackPopupMenu`)
- **Media keys** — intercept play/pause/stop/next/previous hardware keys (macOS: `MPRemoteCommandCenter`, Windows: `WM_APPCOMMAND`, Linux: XF86 keysym interception)
- **Dock badge** — set dock icon badge label (macOS only)

**App API**
- New `App` and `Window` convenience methods for all platform features
- `app.os_info()`, `app.network_status()`, `app.start_power_save_blocker()`, `app.show_dialog()`, etc.
- `window.set_progress_bar()`, `window.request_user_attention()`, etc.

### Improvements

- **Tray panel mode** — position windows relative to the tray icon
- **Menu icons** — support icon data on tray menu items
- **Global hotkey normalization** — consistent key string handling across platforms
- **Platform features demo** — new `platform_features_demo` example exercising all new APIs

### Fixes

- Pin `core-text` to `=21.0.0` to prevent `core-graphics` version conflict on macOS
- Resolve all clippy warnings across the codebase
- Fix safety and correctness issues in platform feature implementations

## 0.4.1 (2026-02-14)

- Documentation and README updates for crates.io

## 0.4.0 (2026-02-14)

### Initial Murmur Extensions

- System tray with icon, tooltip, and menu
- Global hotkeys with platform-native registration
- Overlay windows (always-on-top, click-through)
- Window show/hide toggling
- Active window and focused window info queries
- Accessibility and microphone permission checks (macOS)
- Auto-launch at login
- Single-instance enforcement
- Desktop notifications
- Toast component for in-app notifications
- Keep-alive-without-windows daemon mode
- Element transforms (rotate, scale, transform-origin)
- Multi-stop, radial, and conic gradients
- Per-element blend modes
