# Desktop Platform Features Design

Date: 2026-02-15
Status: Approved

## Overview

Extend GPUI's Platform trait with 15 new desktop capabilities that competing frameworks (Tauri, Electron, Qt) provide. All features follow the established pattern: default no-op implementations on the Platform trait, platform-specific implementations per OS, clean public API on `App`.

Philosophy: GPUI is the platform layer. Raw capabilities, clean APIs, zero UI opinions. Developers compose however they want.

## Feature List

### Tier 1: System Events & Core Desktop Capabilities

1. System power events (sleep/wake/lock/unlock/shutdown)
2. Attention request (dock bounce / taskbar flash)
3. Dock badge
4. Taskbar progress bar (per-window)
5. Native context menus at arbitrary positions
6. System idle detection
7. Media key handling

### Tier 2: Quality-of-Life

8. Power save blocker
9. Network connectivity status + change events
10. Window positioner helpers (pure Rust utility)
11. Window state snapshot/restore (pure Rust data)
12. Native message dialogs (alert/confirm)

### Tier 3: Differentiators

13. OS info query
14. Biometric authentication (Touch ID / Windows Hello / fprintd)
15. Crash reporting hooks

---

## Section 1: System Events

Four features sharing a callback-based event delivery pattern.

### 1.1 System Power Events

```rust
pub enum SystemPowerEvent {
    Suspend,
    Resume,
    LockScreen,
    UnlockScreen,
    Shutdown,
}
```

**Platform trait:**
```rust
fn on_system_power_event(&self, _callback: Box<dyn FnMut(SystemPowerEvent)>) {}
```

**App API:**
```rust
fn on_system_power_event(&self, callback: impl FnMut(SystemPowerEvent, &mut App) + 'static)
```

**Implementations:**
- macOS: `NSWorkspace.shared.notificationCenter` observers for `willSleepNotification`, `didWakeNotification`, `sessionDidBecomeActiveNotification`, `sessionDidResignActiveNotification`, `willPowerOffNotification`
- Windows: `WM_POWERBROADCAST` with `PBT_APMSUSPEND`, `PBT_APMRESUMEAUTOMATIC`, `WM_WTSSESSION_CHANGE` with `WTS_SESSION_LOCK`/`WTS_SESSION_UNLOCK`. Register via `WTSRegisterSessionNotification`.
- Linux: `org.freedesktop.login1.Manager` D-Bus signals: `PrepareForSleep(true/false)`, `PrepareForShutdown(true)`. Session lock via `org.freedesktop.login1.Session.Lock`/`Unlock`.

### 1.2 Power Save Blocker

```rust
pub enum PowerSaveBlockerKind {
    PreventAppSuspension,
    PreventDisplaySleep,
}
```

**Platform trait:**
```rust
fn start_power_save_blocker(&self, _kind: PowerSaveBlockerKind) -> Option<u32> { None }
fn stop_power_save_blocker(&self, _id: u32) {}
```

**App API:**
```rust
fn start_power_save_blocker(&self, kind: PowerSaveBlockerKind) -> Option<u32>
fn stop_power_save_blocker(&self, id: u32)
```

**Implementations:**
- macOS: `IOPMAssertionCreateWithName` with `kIOPMAssertionTypeNoIdleSleep` (display) or `kIOPMAssertionTypePreventUserIdleSystemSleep` (app). Returns `IOPMAssertionID`. Release with `IOPMAssertionRelease`.
- Windows: `SetThreadExecutionState` with `ES_CONTINUOUS | ES_SYSTEM_REQUIRED` (app) or `ES_CONTINUOUS | ES_DISPLAY_REQUIRED` (display). Clear with `SetThreadExecutionState(ES_CONTINUOUS)`. Track IDs internally since Windows doesn't return handles.
- Linux: `org.freedesktop.login1.Manager.Inhibit` D-Bus call with `sleep` (app) or `idle` (display). Returns a file descriptor — close to release. Map fd to u32 ID internally.

### 1.3 System Idle Detection

**Platform trait:**
```rust
fn system_idle_time(&self) -> Option<Duration> { None }
```

**App API:**
```rust
fn system_idle_time(&self) -> Option<Duration>
```

**Implementations:**
- macOS: `CGEventSourceSecondsSinceLastEventType(kCGEventSourceStateCombinedSessionState, kCGAnyInputEventType)` returns `CFTimeInterval` (seconds as f64).
- Windows: `GetLastInputInfo` returns `LASTINPUTINFO.dwTime` (tick count). Compute `GetTickCount() - dwTime` and convert ms to Duration.
- Linux X11: `XScreenSaverQueryInfo` returns `XScreenSaverInfo.idle` in milliseconds. Requires linking `Xss`.
- Linux Wayland: `ext-idle-notify-v1` protocol. Create `ext_idle_notification_v1` with timeout, listen for `idled`/`resumed` events. For a polling API, track last activity time internally.

### 1.4 Network Connectivity Status

```rust
pub enum NetworkStatus {
    Online,
    Offline,
}
```

**Platform trait:**
```rust
fn network_status(&self) -> NetworkStatus { NetworkStatus::Online }
fn on_network_status_change(&self, _callback: Box<dyn FnMut(NetworkStatus)>) {}
```

**App API:**
```rust
fn network_status(&self) -> NetworkStatus
fn on_network_status_change(&self, callback: impl FnMut(NetworkStatus, &mut App) + 'static)
```

**Implementations:**
- macOS: `NWPathMonitor()` on a background queue. `path.status == .satisfied` means Online. Update handler fires on changes.
- Windows: `INetworkListManager::GetConnectivity` for current status. Implement `INetworkListManagerEvents` sink and `Advise` for change notifications.
- Linux: `org.freedesktop.NetworkManager` D-Bus property `State`. Value `70` = connected. Subscribe to `PropertiesChanged` signal for changes. Fallback for systems without NetworkManager: check `/sys/class/net/*/operstate`.

### 1.5 Media Key Handling

```rust
pub enum MediaKeyEvent {
    Play,
    Pause,
    PlayPause,
    Stop,
    NextTrack,
    PreviousTrack,
}
```

**Platform trait:**
```rust
fn on_media_key_event(&self, _callback: Box<dyn FnMut(MediaKeyEvent)>) {}
```

**App API:**
```rust
fn on_media_key_event(&self, callback: impl FnMut(MediaKeyEvent, &mut App) + 'static)
```

**Implementations:**
- macOS: `MPRemoteCommandCenter.shared()`. Register handlers on `playCommand`, `pauseCommand`, `togglePlayPauseCommand`, `stopCommand`, `nextTrackCommand`, `previousTrackCommand`. Must also set `MPNowPlayingInfoCenter.default().playbackState` for the system to route keys to our app.
- Windows: `SystemMediaTransportControls` via `SystemMediaTransportControlsInterop::GetForWindow`. Register `ButtonPressed` event handler. Map `SystemMediaTransportControlsButton` variants.
- Linux: Implement MPRIS2 D-Bus interface `org.mpris.MediaPlayer2.Player` at `/org/mpris/MediaPlayer2`. Listen for method calls: `Play`, `Pause`, `PlayPause`, `Stop`, `Next`, `Previous`.

---

## Section 2: Window & Dock Capabilities

### 2.1 Attention Request

```rust
pub enum AttentionType {
    Informational,
    Critical,
}
```

**Platform trait:**
```rust
fn request_user_attention(&self, _attention_type: AttentionType) {}
fn cancel_user_attention(&self) {}
```

**App API:**
```rust
fn request_user_attention(&self, attention_type: AttentionType)
fn cancel_user_attention(&self)
```

**Implementations:**
- macOS: `NSApp.requestUserAttention(NSInformationalRequest)` or `NSApp.requestUserAttention(NSCriticalRequest)`. Returns request ID. `NSApp.cancelUserAttentionRequest(id)`.
- Windows: `FlashWindowEx` with `FLASHW_TIMERNOFG` (informational, flash until foreground) or `FLASHW_ALL | FLASHW_TIMERNOFG` (critical). Cancel with `FLASHW_STOP`.
- Linux: `org.freedesktop.Notifications.Notify` with urgency hint `1` (normal) or `2` (critical). Or if using GTK: `gtk_window_set_urgency_hint`. For X11: set `_NET_WM_STATE_DEMANDS_ATTENTION` atom.

### 2.2 Dock Badge

**Platform trait:**
```rust
fn set_dock_badge(&self, _label: Option<&str>) {}
```

**App API:**
```rust
fn set_dock_badge(&self, label: Option<&str>)
```

**Implementations:**
- macOS: `NSApp.dockTile.badgeLabel = label` (nil clears).
- Windows: No native dock badge. Use tray tooltip as fallback, or `ITaskbarList3::SetOverlayIcon` with a dynamically rendered icon containing the badge text.
- Linux: `com.canonical.Unity.LauncherEntry` D-Bus interface. Set `count` property and `count-visible`. Works on Ubuntu (Unity/GNOME with extension). KDE: similar via `org.kde.StatusNotifierItem`. Fallback: no-op.

### 2.3 Taskbar Progress Bar (Per-Window)

```rust
pub enum ProgressBarState {
    None,
    Indeterminate,
    Normal(f64),
    Error(f64),
    Paused(f64),
}
```

**PlatformWindow trait:**
```rust
fn set_progress_bar(&self, _state: ProgressBarState) {}
```

**Window API:**
```rust
fn set_progress_bar(&self, state: ProgressBarState)
```

**Implementations:**
- macOS: No native taskbar progress. Render progress into `NSDockTile` by setting a custom `contentView` with an NSProgressIndicator. Or use dock badge as percentage text.
- Windows: `ITaskbarList3::SetProgressState` + `SetProgressValue`. Map states: `TBPF_NOPROGRESS`, `TBPF_INDETERMINATE`, `TBPF_NORMAL`, `TBPF_ERROR`, `TBPF_PAUSED`.
- Linux: `com.canonical.Unity.LauncherEntry` D-Bus. Set `progress` (0.0-1.0) and `progress-visible`. Same bus path as dock badge.

### 2.4 Native Context Menu

Reuses `TrayMenuItem` enum — no new types needed.

**Platform trait:**
```rust
fn show_context_menu(
    &self,
    _position: Point<Pixels>,
    _items: Vec<TrayMenuItem>,
    _callback: Box<dyn FnMut(SharedString)>,
) {}
```

**App API:**
```rust
fn show_context_menu(
    &self,
    position: Point<Pixels>,
    items: Vec<TrayMenuItem>,
    callback: impl FnMut(SharedString, &mut App) + 'static,
)
```

**Implementations:**
- macOS: Build `NSMenu` from items (reuse existing `build_menu` from tray.rs). `NSMenu.popUpContextMenu(_:with:for:)` at the given position. Action callbacks via the existing `handleTrayMenuItem:` delegate method pattern.
- Windows: `CreatePopupMenu`, build with `AppendMenuW` (reuse `WindowsTray::build_menu`). `TrackPopupMenu` at position. Handle `WM_COMMAND` with the returned menu item ID.
- Linux X11: Create an undecorated popup window at position, render menu items as GPUI elements. Or use `gtk_menu_popup_at_rect` if GTK is available.
- Linux Wayland: `xdg_popup` surface with menu content rendered via GPUI. Wayland has no global coordinate popup support, so position is relative to a parent surface.

### 2.5 Window Positioner Helpers

Pure Rust utility functions. No platform calls.

```rust
pub enum WindowPosition {
    Center,
    CenterOnDisplay(DisplayId),
    TrayCenter(Bounds<Pixels>),
    TopRight { margin: Pixels },
    BottomRight { margin: Pixels },
    TopLeft { margin: Pixels },
    BottomLeft { margin: Pixels },
}
```

**App API:**
```rust
fn compute_window_bounds(
    &self,
    size: Size<Pixels>,
    position: WindowPosition,
) -> Bounds<Pixels>
```

Logic:
- `Center`: get primary display bounds, center the size within it.
- `CenterOnDisplay(id)`: find display by ID, center within its bounds.
- `TrayCenter(tray_bounds)`: center horizontally below tray icon, pin to top of screen below menu bar.
- `TopRight/BottomRight/TopLeft/BottomLeft`: anchor to display corner with margin offset.

### 2.6 Window State Snapshot/Restore

Pure Rust data types. Developers choose when/where to persist.

```rust
pub struct WindowState {
    pub bounds: WindowBounds,
    pub display_id: Option<DisplayId>,
    pub fullscreen: bool,
}
```

**Window API:**
```rust
fn window_state(&self) -> WindowState
fn restore_window_state(&mut self, state: WindowState)
```

`window_state()` reads current bounds, display, fullscreen from the platform window.
`restore_window_state()` applies bounds (resize + move), optionally moves to the specified display, toggles fullscreen if needed.

---

## Section 3: Native Dialogs

### 3.1 Native Message Dialog

```rust
pub enum DialogKind {
    Info,
    Warning,
    Error,
}

pub struct DialogOptions {
    pub kind: DialogKind,
    pub title: SharedString,
    pub message: SharedString,
    pub detail: Option<SharedString>,
    pub buttons: Vec<SharedString>,
}
```

**Platform trait:**
```rust
fn show_dialog(&self, _options: DialogOptions) -> oneshot::Receiver<usize> {
    let (tx, rx) = oneshot::channel();
    tx.send(0).ok();
    rx
}
```

**App API:**
```rust
fn show_dialog(&self, options: DialogOptions) -> oneshot::Receiver<usize>
```

**Implementations:**
- macOS: `NSAlert` with `alertStyle` mapped from `DialogKind`. `messageText` = title, `informativeText` = message + detail. Buttons added via `addButtonWithTitle:` in order. Run with `runModal` on background thread, send result index.
- Windows: `TaskDialogIndirect` with `TD_INFORMATION_ICON` / `TD_WARNING_ICON` / `TD_ERROR_ICON`. `pszWindowTitle` = title, `pszMainInstruction` = message, `pszContent` = detail. Custom buttons via `TASKDIALOG_BUTTON` array.
- Linux: `xdg-desktop-portal` `org.freedesktop.portal.Notification` if available. Fallback: spawn `zenity --question` or `kdialog --yesno` subprocess with appropriate flags. Parse exit code for button index.

---

## Section 4: OS Info, Biometric Auth & Crash Hooks

### 4.1 OS Info Query

```rust
pub struct OsInfo {
    pub name: SharedString,
    pub version: SharedString,
    pub arch: SharedString,
    pub locale: SharedString,
    pub hostname: SharedString,
}
```

**Platform trait:**
```rust
fn os_info(&self) -> OsInfo {
    OsInfo {
        name: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        version: String::new().into(),
        locale: String::new().into(),
        hostname: String::new().into(),
    }
}
```

**App API:**
```rust
fn os_info(&self) -> OsInfo
```

**Implementations:**
- macOS: `NSProcessInfo.processInfo.operatingSystemVersionString` for version. `NSLocale.currentLocale.localeIdentifier` for locale. `gethostname(2)` for hostname.
- Windows: `RtlGetVersion` for OS version (avoids compatibility mode lies from `GetVersionEx`). `GetUserDefaultLocaleName` for locale. `GetComputerNameW` for hostname.
- Linux: Parse `/etc/os-release` for `PRETTY_NAME` (name) and `VERSION_ID` (version). `$LANG` or `$LC_ALL` environment variable for locale. `gethostname(2)` for hostname.

### 4.2 Biometric Authentication

```rust
pub enum BiometricKind {
    TouchId,
    WindowsHello,
    Fingerprint,
}

pub enum BiometricStatus {
    Available(BiometricKind),
    Unavailable,
}
```

**Platform trait:**
```rust
fn biometric_status(&self) -> BiometricStatus { BiometricStatus::Unavailable }
fn authenticate_biometric(
    &self,
    _reason: &str,
    _callback: Box<dyn FnOnce(bool)>,
) {
    // Default: immediate failure
}
```

**App API:**
```rust
fn biometric_status(&self) -> BiometricStatus
fn authenticate_biometric(&self, reason: &str, callback: impl FnOnce(bool) + 'static)
```

**Implementations:**
- macOS: Link `LocalAuthentication.framework`. `LAContext().canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics)` for status. `evaluatePolicy(_:localizedReason:reply:)` for auth. Reply block calls callback with success bool.
- Windows: `UserConsentVerifier.CheckAvailabilityAsync()` for status (`Available` = good). `RequestVerificationAsync(reason)` for auth. Result `Verified` = true.
- Linux: `org.freedesktop.Fprint.Manager.GetDevices` D-Bus call. If devices list non-empty, `Available(Fingerprint)`. Auth via `org.freedesktop.Fprint.Device.VerifyStart` + listen for `VerifyStatus` signal. Most Linux desktops won't have this, so `Unavailable` is the common default.

### 4.3 Crash Reporting Hooks

Not a Platform trait method — this is process-level.

```rust
pub struct CrashReport {
    pub message: String,
    pub backtrace: String,
    pub os_info: OsInfo,
    pub app_version: Option<String>,
}
```

**App API:**
```rust
fn set_crash_handler(
    &mut self,
    handler: impl Fn(CrashReport) + Send + Sync + 'static,
)
```

**Implementation (all platforms):**
- `std::panic::set_hook` to intercept panics.
- Capture `PanicInfo` message and location.
- Capture backtrace via `std::backtrace::Backtrace` (or `backtrace` crate for older MSRV).
- Call `platform.os_info()` for system context.
- Construct `CrashReport` and invoke the user's handler.
- After handler returns, re-raise or abort as appropriate.

---

## Cross-Platform Coverage Matrix

| Feature | macOS | Windows | Linux | Default |
|---------|-------|---------|-------|---------|
| Power events | NSWorkspace notifications | WM_POWERBROADCAST + WTS | logind D-Bus | no-op |
| Power save blocker | IOPMAssertion | SetThreadExecutionState | logind Inhibit D-Bus | None |
| System idle time | CGEventSource | GetLastInputInfo | X11: XScreenSaver / Wayland: ext-idle-notify | None |
| Network status | NWPathMonitor | INetworkListManager | NetworkManager D-Bus | Online |
| Media keys | MPRemoteCommandCenter | SystemMediaTransportControls | MPRIS2 D-Bus | no-op |
| Attention request | NSApp.requestUserAttention | FlashWindowEx | _NET_WM_STATE_DEMANDS_ATTENTION | no-op |
| Dock badge | NSDockTile.badgeLabel | ITaskbarList3 overlay icon | Unity LauncherEntry D-Bus | no-op |
| Progress bar | NSDockTile custom view | ITaskbarList3 | Unity LauncherEntry D-Bus | no-op |
| Context menu | NSMenu.popUpContextMenu | TrackPopupMenu | X11: popup window / Wayland: xdg_popup | no-op |
| Window positioner | Pure Rust | Pure Rust | Pure Rust | computed |
| Window state | Pure Rust | Pure Rust | Pure Rust | computed |
| Native dialog | NSAlert | TaskDialogIndirect | zenity/kdialog subprocess | returns 0 |
| OS info | NSProcessInfo/NSLocale | RtlGetVersion | /etc/os-release | basic fallback |
| Biometric auth | LAContext | UserConsentVerifier | fprintd D-Bus | Unavailable |
| Crash handler | std::panic::set_hook | std::panic::set_hook | std::panic::set_hook | no-op |

## File Organization

New files per platform:
- `crates/gpui/src/platform/mac/power.rs` — power events, power save blocker, idle detection
- `crates/gpui/src/platform/mac/network.rs` — network monitoring
- `crates/gpui/src/platform/mac/media_keys.rs` — media key handling
- `crates/gpui/src/platform/mac/dock.rs` — badge, attention request
- `crates/gpui/src/platform/mac/dialog.rs` — native message dialogs
- `crates/gpui/src/platform/mac/os_info.rs` — OS information
- `crates/gpui/src/platform/mac/biometric.rs` — Touch ID
- `crates/gpui/src/platform/windows/power.rs` — power events, blocker, idle
- `crates/gpui/src/platform/windows/network.rs` — network monitoring
- `crates/gpui/src/platform/windows/media_keys.rs` — media key handling
- `crates/gpui/src/platform/windows/dialog.rs` — TaskDialog
- `crates/gpui/src/platform/windows/os_info.rs` — OS information
- `crates/gpui/src/platform/windows/biometric.rs` — Windows Hello
- `crates/gpui/src/platform/linux/power.rs` — logind D-Bus
- `crates/gpui/src/platform/linux/network.rs` — NetworkManager D-Bus
- `crates/gpui/src/platform/linux/media_keys.rs` — MPRIS D-Bus
- `crates/gpui/src/platform/linux/dialog.rs` — zenity/kdialog
- `crates/gpui/src/platform/linux/os_info.rs` — /etc/os-release
- `crates/gpui/src/platform/linux/biometric.rs` — fprintd D-Bus

Shared types in:
- `crates/gpui/src/platform.rs` — enums, structs, Platform trait additions
- `crates/gpui/src/app.rs` — App API additions
- `crates/gpui/src/platform/crash.rs` — crash handler (pure Rust, cross-platform)
- `crates/gpui/src/platform/window_positioner.rs` — position helpers (pure Rust)
