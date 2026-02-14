# GPUI Extensions for Murmur — Design Document

Date: 2026-02-14

## Overview

Extend adabraka-gpui with daemon-mode capabilities, system tray, global hotkeys, overlay windows, platform integration, and notifications to support the Murmur voice-to-text application.

## Architecture

Extend the existing `Platform` and `PlatformWindow` traits with new methods. This is consistent with how GPUI already handles credentials, clipboard, screen capture, and menus. All three platforms (macOS, Linux, Windows) are targeted equally.

## Already Implemented (Verified)

- `WindowBackgroundAppearance::Transparent` — per-pixel alpha
- `WindowKind::Floating` — basic always-on-top
- `WindowDecorations::Client` — borderless windows
- Multi-window with per-window options
- Keychain/credentials on all 3 platforms
- Canvas element + custom drawing (paths, quads, beziers)
- Animation system (spring/tween)
- Async executor with background tasks
- Thread-safe state updates via `cx.notify()`

## Phase 1 — Daemon Mode

### 1.1 Headless App Lifecycle

Add `keep_alive_without_windows: bool` to `AppOptions`. When true, the event loop persists with zero windows open.

Platform implementations:
- macOS: `applicationShouldTerminateAfterLastWindowClosed:` returns `NO`
- Linux: Skip quit logic when window count reaches 0
- Windows: Don't post `WM_QUIT` when last window closes

### 1.2 System Tray

New `Platform` trait methods:
```rust
fn set_tray_icon(&self, icon: Option<Vec<u8>>)
fn set_tray_menu(&self, menu: Vec<TrayMenuItem>)
fn set_tray_tooltip(&self, tooltip: &str)
fn on_tray_icon_event(&self, callback: Box<dyn FnMut(TrayIconEvent)>)
```

New types:
```rust
pub enum TrayIconEvent { LeftClick, RightClick, DoubleClick }
pub enum TrayMenuItem {
    Action { label: SharedString, id: SharedString },
    Separator,
    Submenu { label: SharedString, items: Vec<TrayMenuItem> },
    Toggle { label: SharedString, checked: bool, id: SharedString },
}
```

Platform implementations:
- macOS: Modernize existing `status_item.rs` → `tray.rs`, use `NSStatusItem` + `NSMenu`
- Linux: `ksni` crate (DBus StatusNotifierItem protocol)
- Windows: `Shell_NotifyIconW` Win32 API

### 2.1 Global Hotkey

New `Platform` trait methods:
```rust
fn register_global_hotkey(&self, id: u32, keystroke: &Keystroke) -> Result<()>
fn unregister_global_hotkey(&self, id: u32)
fn on_global_hotkey(&self, callback: Box<dyn FnMut(u32)>)
```

Platform implementations:
- macOS: `NSEvent.addGlobalMonitorForEvents` + `addLocalMonitorForEvents`
- Linux X11: `XGrabKey` on root window
- Linux Wayland: `org.freedesktop.portal.GlobalShortcuts` (XDG Portal)
- Windows: `RegisterHotKey` Win32 API

### 3.6 Single Instance

Pre-platform utility:
```rust
pub struct SingleInstance { /* ... */ }
impl SingleInstance {
    pub fn acquire(app_id: &str) -> Result<Self, AlreadyRunning>
    pub fn on_activate(&self, callback: Box<dyn FnMut()>)
}
pub fn send_activate_to_existing(app_id: &str) -> Result<()>
```

Platform implementations:
- macOS/Linux: Unix domain socket
- Windows: Named mutex + named pipe

## Phase 2 — Overlay Window

### 1.3 Always-on-Top (Strengthen)

Add `WindowKind::Overlay` — renders above fullscreen apps.
- macOS: `NSWindow.level = .statusBar`
- Linux X11: `_NET_WM_STATE_ABOVE`
- Linux Wayland: `zwlr_layer_shell_v1` overlay layer
- Windows: `HWND_TOPMOST`

### 1.6 Click-Through Window

New field: `WindowOptions.mouse_passthrough: bool`
New `PlatformWindow` method: `fn set_mouse_passthrough(&self, passthrough: bool)`
- macOS: `NSWindow.ignoresMouseEvents = true`
- Linux X11: `XShapeCombineRectangles` with empty `ShapeInput`
- Linux Wayland: `wl_surface.set_input_region` with empty region
- Windows: `WS_EX_TRANSPARENT` extended style

### 1.9 Window Show/Hide

New `PlatformWindow` methods:
```rust
fn show(&self)
fn hide(&self)
fn is_visible(&self) -> bool
```

- macOS: `orderOut:` / `makeKeyAndOrderFront:`
- Linux X11: `XUnmapWindow` / `XMapWindow`
- Linux Wayland: Surface attach/detach
- Windows: `ShowWindow(SW_HIDE/SW_SHOW)`

## Phase 3 — Context Detection

### 3.1 Active Window Query

New `Platform` method:
```rust
fn focused_window_info(&self) -> Option<FocusedWindowInfo>
```

```rust
pub struct FocusedWindowInfo {
    pub app_name: String,
    pub window_title: String,
    pub bundle_id: Option<String>,
    pub pid: Option<u32>,
}
```

- macOS: `NSWorkspace.shared.frontmostApplication` + Accessibility API
- Linux X11: `_NET_ACTIVE_WINDOW` → `_NET_WM_NAME` + `_NET_WM_PID`
- Linux Wayland: `wlr-foreign-toplevel-management-v1`
- Windows: `GetForegroundWindow()` → `GetWindowText()` + module name

### 3.2 Accessibility Permissions (macOS)

```rust
fn accessibility_status(&self) -> PermissionStatus
fn request_accessibility_permission(&self)
```

Uses `AXIsProcessTrusted()`. Linux/Windows return `Granted`.

### 3.3 Microphone Permissions

```rust
fn microphone_status(&self) -> PermissionStatus
fn request_microphone_permission(&self, callback: Box<dyn FnOnce(bool)>)
```

- macOS: `AVCaptureDevice.authorizationStatus`
- Windows: UWP privacy API
- Linux: Returns `Granted`

## Phase 4 — Polish

### 3.5 Auto-Launch

```rust
fn set_auto_launch(&self, enabled: bool) -> Result<()>
fn is_auto_launch_enabled(&self) -> bool
```

- macOS: `SMAppService` (macOS 13+)
- Linux: XDG autostart desktop entry
- Windows: Registry `HKCU\...\Run`

### 4.3 Notifications

**OS-native:**
```rust
fn show_notification(&self, title: &str, body: &str, icon: Option<&[u8]>) -> Result<()>
```

- macOS: `UNUserNotificationCenter`
- Linux: `notify-rust`
- Windows: Toast notifications

**In-app toast:** Framework-level `Toast` component with auto-dismiss.

## New Dependencies

| Crate | Purpose | Platform |
|-------|---------|----------|
| `ksni` | System tray (StatusNotifierItem) | Linux |
| `notify-rust` | Desktop notifications | Linux |

macOS/Windows use existing `cocoa`/`objc`/`windows` deps.

## File Plan

New files:
- `crates/gpui/src/platform/mac/tray.rs`
- `crates/gpui/src/platform/linux/tray.rs`
- `crates/gpui/src/platform/windows/tray.rs`
- `crates/gpui/src/platform/mac/global_hotkey.rs`
- `crates/gpui/src/platform/linux/global_hotkey.rs`
- `crates/gpui/src/platform/windows/global_hotkey.rs`
- `crates/gpui/src/platform/single_instance.rs`
- `crates/gpui/src/platform/mac/permissions.rs`
- `crates/gpui/src/platform/mac/auto_launch.rs`
- `crates/gpui/src/platform/linux/auto_launch.rs`
- `crates/gpui/src/platform/windows/auto_launch.rs`
- `crates/gpui/src/notification.rs`
- `crates/gpui/src/elements/toast.rs`

Modified files:
- `crates/gpui/src/platform.rs` (trait extensions + types)
- `crates/gpui/src/platform/{mac,linux,windows}/platform.rs`
- `crates/gpui/src/platform/{mac,linux/x11,linux/wayland,windows}/window.rs`
- `crates/gpui/src/app.rs` (expose APIs)
- `crates/gpui/src/window.rs` (show/hide, click-through)
- `crates/gpui/Cargo.toml` (deps)
