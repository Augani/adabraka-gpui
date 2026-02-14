# Murmur GPUI Extensions — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend adabraka-gpui with daemon-mode lifecycle, system tray, global hotkeys, overlay windows, click-through, active window detection, permissions, auto-launch, single instance, and notifications — all three platforms.

**Architecture:** Extend existing `Platform` trait (platform.rs:164) and `PlatformWindow` trait (platform.rs:460) with new methods. Each platform impl (MacPlatform, LinuxClient, WindowsPlatform, TestPlatform) gets corresponding implementations. New types go in `platform.rs`. New platform-specific code goes in dedicated files under each platform directory.

**Tech Stack:** Rust, cocoa/objc (macOS), wayland-client/x11rb (Linux), windows crate (Windows), ksni (Linux tray), notify-rust (Linux notifications)

---

## Task 1: Add New Types to Platform Trait

**Files:**
- Modify: `crates/gpui/src/platform.rs`

**Step 1: Add new enums and structs after WindowBackgroundAppearance (line ~1328)**

Add these types to `platform.rs` after the `WindowBackgroundAppearance` enum:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayIconEvent {
    LeftClick,
    RightClick,
    DoubleClick,
}

#[derive(Debug, Clone)]
pub enum TrayMenuItem {
    Action {
        label: SharedString,
        id: SharedString,
    },
    Separator,
    Submenu {
        label: SharedString,
        items: Vec<TrayMenuItem>,
    },
    Toggle {
        label: SharedString,
        checked: bool,
        id: SharedString,
    },
}

#[derive(Debug, Clone)]
pub struct FocusedWindowInfo {
    pub app_name: String,
    pub window_title: String,
    pub bundle_id: Option<String>,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoLaunchStatus {
    Enabled,
    Disabled,
    Unknown,
}
```

**Step 2: Extend WindowKind enum (line ~1262)**

Add `Overlay` variant:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WindowKind {
    Normal,
    PopUp,
    Floating,
    Overlay,
}
```

**Step 3: Add `mouse_passthrough` to WindowOptions (line ~1091)**

Add field to `WindowOptions` struct and its `Default` impl:

```rust
pub struct WindowOptions {
    // ... existing fields ...
    pub mouse_passthrough: bool,
}

// In Default impl, add:
// mouse_passthrough: false,
```

**Step 4: Add `mouse_passthrough` to WindowParams (line ~1148)**

Add field to the internal `WindowParams` struct:

```rust
pub(crate) struct WindowParams {
    // ... existing fields ...
    pub mouse_passthrough: bool,
}
```

**Step 5: Add new methods to Platform trait (line ~164)**

Add these methods to the `Platform` trait, after the `on_keyboard_layout_change` method (end of trait):

```rust
    // System tray
    fn set_tray_icon(&self, _icon: Option<&[u8]>) {}
    fn set_tray_menu(&self, _menu: Vec<TrayMenuItem>) {}
    fn set_tray_tooltip(&self, _tooltip: &str) {}
    fn on_tray_icon_event(&self, _callback: Box<dyn FnMut(TrayIconEvent)>) {}

    // Global hotkeys
    fn register_global_hotkey(&self, _id: u32, _keystroke: &Keystroke) -> Result<()> {
        Err(anyhow::anyhow!("Global hotkeys not supported on this platform"))
    }
    fn unregister_global_hotkey(&self, _id: u32) {}
    fn on_global_hotkey(&self, _callback: Box<dyn FnMut(u32)>) {}

    // Active window info (external apps)
    fn focused_window_info(&self) -> Option<FocusedWindowInfo> {
        None
    }

    // Permissions
    fn accessibility_status(&self) -> PermissionStatus {
        PermissionStatus::Granted
    }
    fn request_accessibility_permission(&self) {}

    fn microphone_status(&self) -> PermissionStatus {
        PermissionStatus::Granted
    }
    fn request_microphone_permission(&self, callback: Box<dyn FnOnce(bool)>) {
        callback(true);
    }

    // Auto-launch
    fn set_auto_launch(&self, _app_id: &str, _enabled: bool) -> Result<()> {
        Err(anyhow::anyhow!("Auto-launch not supported on this platform"))
    }
    fn is_auto_launch_enabled(&self, _app_id: &str) -> bool {
        false
    }

    // Notifications
    fn show_notification(&self, _title: &str, _body: &str) -> Result<()> {
        Err(anyhow::anyhow!("Notifications not supported on this platform"))
    }

    // App lifecycle
    fn set_keep_alive_without_windows(&self, _keep_alive: bool) {}
```

**Step 6: Add new methods to PlatformWindow trait (line ~460)**

Add these methods to `PlatformWindow`, after `update_ime_position`:

```rust
    fn show(&self) {}
    fn hide(&self) {}
    fn is_visible(&self) -> bool { true }
    fn set_mouse_passthrough(&self, _passthrough: bool) {}
```

**Step 7: Verify it compiles**

Run: `cargo check -p gpui 2>&1 | head -50`

The default implementations mean existing platform impls won't break. Fix any compile errors.

**Step 8: Commit**

```bash
git add crates/gpui/src/platform.rs
git commit -m "feat: Add platform trait methods and types for tray, hotkeys, overlay, permissions, notifications"
```

---

## Task 2: Wire New Types Through App/Window Context

**Files:**
- Modify: `crates/gpui/src/app.rs`
- Modify: `crates/gpui/src/window.rs`

**Step 1: Add tray/hotkey/lifecycle methods to App**

In `app.rs`, find the `impl App` block that contains `hide()` and `hide_other_apps()` (near line 987). Add new public methods:

```rust
    pub fn set_tray_icon(&self, icon: Option<&[u8]>) {
        self.platform.set_tray_icon(icon);
    }

    pub fn set_tray_menu(&self, menu: Vec<TrayMenuItem>) {
        self.platform.set_tray_menu(menu);
    }

    pub fn set_tray_tooltip(&self, tooltip: &str) {
        self.platform.set_tray_tooltip(tooltip);
    }

    pub fn on_tray_icon_event(&self, callback: impl FnMut(TrayIconEvent) + 'static) {
        self.platform.on_tray_icon_event(Box::new(callback));
    }

    pub fn register_global_hotkey(&self, id: u32, keystroke: &Keystroke) -> Result<()> {
        self.platform.register_global_hotkey(id, keystroke)
    }

    pub fn unregister_global_hotkey(&self, id: u32) {
        self.platform.unregister_global_hotkey(id);
    }

    pub fn on_global_hotkey(&self, callback: impl FnMut(u32) + 'static) {
        self.platform.on_global_hotkey(Box::new(callback));
    }

    pub fn focused_window_info(&self) -> Option<FocusedWindowInfo> {
        self.platform.focused_window_info()
    }

    pub fn accessibility_status(&self) -> PermissionStatus {
        self.platform.accessibility_status()
    }

    pub fn request_accessibility_permission(&self) {
        self.platform.request_accessibility_permission();
    }

    pub fn microphone_status(&self) -> PermissionStatus {
        self.platform.microphone_status()
    }

    pub fn request_microphone_permission(&self, callback: impl FnOnce(bool) + 'static) {
        self.platform.request_microphone_permission(Box::new(callback));
    }

    pub fn set_auto_launch(&self, app_id: &str, enabled: bool) -> Result<()> {
        self.platform.set_auto_launch(app_id, enabled)
    }

    pub fn is_auto_launch_enabled(&self, app_id: &str) -> bool {
        self.platform.is_auto_launch_enabled(app_id)
    }

    pub fn show_notification(&self, title: &str, body: &str) -> Result<()> {
        self.platform.show_notification(title, body)
    }

    pub fn set_keep_alive_without_windows(&self, keep_alive: bool) {
        self.platform.set_keep_alive_without_windows(keep_alive);
    }
```

**Step 2: Add window show/hide/passthrough methods to Window context**

In `window.rs`, find the `impl<'a> Window` or `impl WindowContext` block where `minimize_window` lives. Add:

```rust
    pub fn show_window(&self) {
        self.window.platform_window.borrow_on_main_thread().show();
    }

    pub fn hide_window(&self) {
        self.window.platform_window.borrow_on_main_thread().hide();
    }

    pub fn is_window_visible(&self) -> bool {
        self.window.platform_window.borrow_on_main_thread().is_visible()
    }

    pub fn set_mouse_passthrough(&self, passthrough: bool) {
        self.window.platform_window.borrow_on_main_thread().set_mouse_passthrough(passthrough);
    }
```

**Step 3: Wire mouse_passthrough through window open**

Find where `WindowOptions` is converted to `WindowParams` (in `window.rs` or `app.rs`). Add `mouse_passthrough` field to the conversion.

**Step 4: Export new types from lib.rs**

In `crates/gpui/src/lib.rs`, add exports for the new public types:

```rust
pub use platform::{
    TrayIconEvent, TrayMenuItem, FocusedWindowInfo, PermissionStatus, AutoLaunchStatus,
    // ... existing exports ...
};
```

**Step 5: Verify it compiles**

Run: `cargo check -p gpui 2>&1 | head -50`

**Step 6: Commit**

```bash
git add crates/gpui/src/app.rs crates/gpui/src/window.rs crates/gpui/src/lib.rs
git commit -m "feat: Wire tray, hotkey, permissions, notifications APIs through App and Window context"
```

---

## Task 3: Implement Keep-Alive Without Windows (All Platforms)

**Files:**
- Modify: `crates/gpui/src/platform/mac/platform.rs`
- Modify: `crates/gpui/src/platform/linux/platform.rs`
- Modify: `crates/gpui/src/platform/windows/platform.rs`
- Modify: `crates/gpui/src/platform/test/platform.rs`

### macOS

**Step 1: Add keep_alive state to MacPlatformState**

In `platform/mac/platform.rs`, find `MacPlatformState` struct. Add:

```rust
keep_alive_without_windows: bool,
```

Initialize to `false` in the constructor.

**Step 2: Implement set_keep_alive_without_windows**

```rust
fn set_keep_alive_without_windows(&self, keep_alive: bool) {
    self.0.lock().keep_alive_without_windows = keep_alive;
}
```

**Step 3: Override applicationShouldTerminateAfterLastWindowClosed**

Find the NSApplication delegate setup. If the delegate responds to `applicationShouldTerminateAfterLastWindowClosed:`, make it check the `keep_alive_without_windows` flag and return `NO` when true.

If this delegate method isn't set up yet, add it to the app delegate class declaration.

### Linux

**Step 4: Add keep_alive to LinuxCommon or LinuxClient implementations**

In `platform/linux/platform.rs`, find `LinuxCommon` struct. Add `keep_alive_without_windows: bool` field.

Implement the trait method on the platform to set this flag. Make sure the event loop doesn't exit when window count reaches 0 and this flag is true.

### Windows

**Step 5: Add keep_alive to WindowsPlatform**

In `platform/windows/platform.rs`, find `WindowsPlatformInner`. Add `keep_alive_without_windows: AtomicBool`.

When the last window closes, check this flag before posting `WM_QUIT`. Implement the trait method.

### Test

**Step 6: Stub on TestPlatform**

In `platform/test/platform.rs`, add:

```rust
fn set_keep_alive_without_windows(&self, _keep_alive: bool) {}
```

**Step 7: Verify and commit**

Run: `cargo check -p gpui 2>&1 | head -50`

```bash
git add crates/gpui/src/platform/
git commit -m "feat: Implement keep-alive-without-windows on all platforms"
```

---

## Task 4: System Tray — macOS

**Files:**
- Create: `crates/gpui/src/platform/mac/tray.rs`
- Modify: `crates/gpui/src/platform/mac/platform.rs`
- Modify: `crates/gpui/src/platform/mac/mod.rs`

**Step 1: Create tray.rs with MacTray struct**

Create `crates/gpui/src/platform/mac/tray.rs`:

```rust
use crate::platform::{TrayIconEvent, TrayMenuItem};
use cocoa::{
    appkit::{NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSSquareStatusItemLength, NSVariableStatusItemLength},
    base::{id, nil, YES, NO},
    foundation::{NSData, NSString, NSSize},
};
use objc::{class, msg_send, sel, sel_impl, rc::StrongPtr};
use std::cell::RefCell;

pub(crate) struct MacTray {
    status_item: StrongPtr,
    event_callback: RefCell<Option<Box<dyn FnMut(TrayIconEvent)>>>,
    menu_callback: RefCell<Option<Box<dyn FnMut(SharedString)>>>,
}

impl MacTray {
    pub fn new() -> Self {
        unsafe {
            let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            let status_item: id = msg_send![status_bar, statusItemWithLength: NSVariableStatusItemLength];
            let status_item = StrongPtr::retain(status_item);

            Self {
                status_item,
                event_callback: RefCell::new(None),
                menu_callback: RefCell::new(None),
            }
        }
    }

    pub fn set_icon(&self, icon_data: Option<&[u8]>) {
        unsafe {
            let button: id = msg_send![*self.status_item, button];
            match icon_data {
                Some(data) => {
                    let ns_data: id = msg_send![class!(NSData), dataWithBytes:data.as_ptr() length:data.len()];
                    let image: id = msg_send![class!(NSImage), alloc];
                    let image: id = msg_send![image, initWithData: ns_data];
                    let _: () = msg_send![image, setSize: NSSize::new(18.0, 18.0)];
                    let _: () = msg_send![image, setTemplate: YES];
                    let _: () = msg_send![button, setImage: image];
                }
                None => {
                    let _: () = msg_send![button, setImage: nil];
                }
            }
        }
    }

    pub fn set_tooltip(&self, tooltip: &str) {
        unsafe {
            let button: id = msg_send![*self.status_item, button];
            let ns_tooltip = cocoa::foundation::NSString::alloc(nil).init_str(tooltip);
            let _: () = msg_send![button, setToolTip: ns_tooltip];
        }
    }

    pub fn set_menu(&self, items: Vec<TrayMenuItem>) {
        unsafe {
            let menu: id = msg_send![class!(NSMenu), new];
            self.build_menu(menu, &items);
            let _: () = msg_send![*self.status_item, setMenu: menu];
        }
    }

    unsafe fn build_menu(&self, menu: id, items: &[TrayMenuItem]) {
        for item in items {
            match item {
                TrayMenuItem::Action { label, id: _ } => {
                    let title = cocoa::foundation::NSString::alloc(nil).init_str(label.as_ref());
                    let menu_item: id = msg_send![class!(NSMenuItem), alloc];
                    let menu_item: id = msg_send![menu_item, initWithTitle:title action:sel!(trayMenuAction:) keyEquivalent:cocoa::foundation::NSString::alloc(nil).init_str("")];
                    let _: () = msg_send![menu, addItem: menu_item];
                }
                TrayMenuItem::Separator => {
                    let separator: id = msg_send![class!(NSMenuItem), separatorItem];
                    let _: () = msg_send![menu, addItem: separator];
                }
                TrayMenuItem::Submenu { label, items: sub_items } => {
                    let title = cocoa::foundation::NSString::alloc(nil).init_str(label.as_ref());
                    let menu_item: id = msg_send![class!(NSMenuItem), alloc];
                    let menu_item: id = msg_send![menu_item, initWithTitle:title action:nil keyEquivalent:cocoa::foundation::NSString::alloc(nil).init_str("")];
                    let submenu: id = msg_send![class!(NSMenu), new];
                    self.build_menu(submenu, sub_items);
                    let _: () = msg_send![menu_item, setSubmenu: submenu];
                    let _: () = msg_send![menu, addItem: menu_item];
                }
                TrayMenuItem::Toggle { label, checked, id: _ } => {
                    let title = cocoa::foundation::NSString::alloc(nil).init_str(label.as_ref());
                    let menu_item: id = msg_send![class!(NSMenuItem), alloc];
                    let menu_item: id = msg_send![menu_item, initWithTitle:title action:sel!(trayMenuAction:) keyEquivalent:cocoa::foundation::NSString::alloc(nil).init_str("")];
                    let state: isize = if *checked { 1 } else { 0 };
                    let _: () = msg_send![menu_item, setState: state];
                    let _: () = msg_send![menu, addItem: menu_item];
                }
            }
        }
    }

    pub fn set_event_callback(&self, callback: Box<dyn FnMut(TrayIconEvent)>) {
        *self.event_callback.borrow_mut() = Some(callback);
    }
}
```

**Step 2: Integrate into MacPlatform**

Add `tray: RefCell<Option<MacTray>>` to `MacPlatformState`. Initialize lazily on first tray method call. Implement the Platform trait methods to delegate to MacTray.

**Step 3: Register mod in mac/mod.rs**

Add `mod tray;` to `crates/gpui/src/platform/mac/mod.rs`.

**Step 4: Verify and commit**

Run: `cargo check -p gpui 2>&1 | head -50`

```bash
git add crates/gpui/src/platform/mac/
git commit -m "feat: Implement system tray on macOS via NSStatusItem"
```

---

## Task 5: System Tray — Linux

**Files:**
- Create: `crates/gpui/src/platform/linux/tray.rs`
- Modify: `crates/gpui/src/platform/linux/platform.rs`
- Modify: `crates/gpui/Cargo.toml`

**Step 1: Add ksni dependency**

In `crates/gpui/Cargo.toml`, add under `[target.'cfg(any(target_os = "linux", target_os = "freebsd"))'.dependencies]`:

```toml
ksni = "0.2"
```

**Step 2: Create Linux tray implementation**

Create `crates/gpui/src/platform/linux/tray.rs` using `ksni` crate's `TrayService` for DBus StatusNotifierItem protocol. The `ksni::Tray` trait needs to be implemented with the menu items and icon data.

**Step 3: Wire into LinuxClient trait and implementations**

Add tray methods to `LinuxClient` trait. Implement on `WaylandClient` and `X11Client` (tray is desktop-protocol-independent on Linux, so both use the same `ksni` code). `HeadlessClient` returns no-ops.

**Step 4: Verify and commit**

Run: `cargo check -p gpui --features=x11 2>&1 | head -50`

```bash
git add crates/gpui/
git commit -m "feat: Implement system tray on Linux via ksni (DBus StatusNotifierItem)"
```

---

## Task 6: System Tray — Windows

**Files:**
- Create: `crates/gpui/src/platform/windows/tray.rs`
- Modify: `crates/gpui/src/platform/windows/platform.rs`

**Step 1: Create Windows tray implementation**

Create `crates/gpui/src/platform/windows/tray.rs` using Win32 `Shell_NotifyIconW` API via the `windows` crate. Use `NOTIFYICONDATAW` struct with `NIF_ICON | NIF_TIP | NIF_MESSAGE`.

Handle `WM_TRAYICON` (custom message) in the platform's message loop to dispatch click events.

**Step 2: Wire into WindowsPlatform**

Add tray state to `WindowsPlatformInner`. Implement trait methods. Handle tray messages in the Win32 message pump.

**Step 3: Verify and commit**

Run: `cargo check -p gpui 2>&1 | head -50` (on Windows, or cross-check)

```bash
git add crates/gpui/src/platform/windows/
git commit -m "feat: Implement system tray on Windows via Shell_NotifyIconW"
```

---

## Task 7: Global Hotkey — macOS

**Files:**
- Create: `crates/gpui/src/platform/mac/global_hotkey.rs`
- Modify: `crates/gpui/src/platform/mac/platform.rs`

**Step 1: Create macOS global hotkey implementation**

Create `crates/gpui/src/platform/mac/global_hotkey.rs`:

Use `NSEvent::addGlobalMonitorForEventsMatchingMask:handler:` for events when app is NOT focused, and `NSEvent::addLocalMonitorForEventsMatchingMask:handler:` for when it IS focused.

Map GPUI `Keystroke` modifiers to `NSEventModifierFlags` and keycode to `NSEvent.keyCode`.

Store registered hotkeys as `HashMap<u32, Keystroke>` and check incoming events against them.

When a match is found, call `dispatch_on_main_thread` to invoke the callback within the GPUI event loop.

**Step 2: Integrate into MacPlatform**

Add `global_hotkey: RefCell<Option<MacGlobalHotkey>>` to `MacPlatformState`. Implement the three Platform trait methods.

**Step 3: Verify and commit**

```bash
git add crates/gpui/src/platform/mac/
git commit -m "feat: Implement global hotkey on macOS via NSEvent monitors"
```

---

## Task 8: Global Hotkey — Linux

**Files:**
- Create: `crates/gpui/src/platform/linux/global_hotkey.rs`
- Modify: `crates/gpui/src/platform/linux/platform.rs`

**Step 1: X11 implementation**

For X11: Use `x11rb::protocol::xproto::grab_key` on the root window. Map Keystroke to X11 keysym + modifiers. Handle `KeyPress` events from the root window grab.

**Step 2: Wayland implementation**

For Wayland: Use `ashpd::desktop::global_shortcuts::GlobalShortcuts` (XDG Desktop Portal). This is the only portable way on Wayland. The `ashpd` crate is already a dependency.

**Step 3: Wire into LinuxClient**

Both `X11Client` and `WaylandClient` get their respective implementations. `HeadlessClient` returns error.

**Step 4: Verify and commit**

```bash
git add crates/gpui/src/platform/linux/
git commit -m "feat: Implement global hotkey on Linux (X11 grab + Wayland portal)"
```

---

## Task 9: Global Hotkey — Windows

**Files:**
- Create: `crates/gpui/src/platform/windows/global_hotkey.rs`
- Modify: `crates/gpui/src/platform/windows/platform.rs`

**Step 1: Create Windows global hotkey implementation**

Use `RegisterHotKey` Win32 API. Map Keystroke to virtual key code + `MOD_ALT | MOD_CONTROL | MOD_SHIFT | MOD_WIN` flags.

Handle `WM_HOTKEY` messages in the platform's message pump. The `wParam` contains the hotkey ID.

**Step 2: Wire into WindowsPlatform**

Implement trait methods. Call `UnregisterHotKey` in `unregister_global_hotkey`.

**Step 3: Verify and commit**

```bash
git add crates/gpui/src/platform/windows/
git commit -m "feat: Implement global hotkey on Windows via RegisterHotKey"
```

---

## Task 10: Single Instance Enforcement

**Files:**
- Create: `crates/gpui/src/platform/single_instance.rs`
- Modify: `crates/gpui/src/platform.rs` (re-export)
- Modify: `crates/gpui/src/lib.rs` (public export)

**Step 1: Create cross-platform SingleInstance**

Create `crates/gpui/src/platform/single_instance.rs`:

```rust
use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug)]
pub struct AlreadyRunning;

impl std::fmt::Display for AlreadyRunning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Another instance is already running")
    }
}

impl std::error::Error for AlreadyRunning {}

pub struct SingleInstance {
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
    _listener: std::os::unix::net::UnixListener,
    #[cfg(target_os = "windows")]
    _mutex: windows::Win32::Foundation::HANDLE,
}

impl SingleInstance {
    pub fn acquire(app_id: &str) -> Result<Self, AlreadyRunning> {
        Self::platform_acquire(app_id)
    }

    pub fn on_activate(&self, _callback: Box<dyn FnMut()>) {
        // Set up listener for activation messages
    }
}

pub fn send_activate_to_existing(app_id: &str) -> Result<()> {
    platform_send_activate(app_id)
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
fn socket_path(app_id: &str) -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(dir).join(format!("{}.sock", app_id))
}

// Unix implementation: Unix domain socket
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
impl SingleInstance {
    fn platform_acquire(app_id: &str) -> Result<Self, AlreadyRunning> {
        use std::os::unix::net::UnixListener;
        let path = socket_path(app_id);
        // Try to connect first — if successful, another instance is running
        if std::os::unix::net::UnixStream::connect(&path).is_ok() {
            return Err(AlreadyRunning);
        }
        // Remove stale socket
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).map_err(|_| AlreadyRunning)?;
        listener.set_nonblocking(true).ok();
        Ok(Self { _listener: listener })
    }
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "freebsd"))]
fn platform_send_activate(app_id: &str) -> Result<()> {
    use std::os::unix::net::UnixStream;
    let path = socket_path(app_id);
    let mut stream = UnixStream::connect(&path)?;
    stream.write_all(b"activate")?;
    Ok(())
}

// Windows implementation: Named mutex
#[cfg(target_os = "windows")]
impl SingleInstance {
    fn platform_acquire(app_id: &str) -> Result<Self, AlreadyRunning> {
        use windows::Win32::System::Threading::CreateMutexW;
        use windows::Win32::Foundation::GetLastError;
        use windows::core::HSTRING;
        let name = HSTRING::from(format!("Global\\{}", app_id));
        unsafe {
            let handle = CreateMutexW(None, true, &name).map_err(|_| AlreadyRunning)?;
            if GetLastError().is_err() {
                return Err(AlreadyRunning);
            }
            Ok(Self { _mutex: handle })
        }
    }
}

#[cfg(target_os = "windows")]
fn platform_send_activate(_app_id: &str) -> Result<()> {
    // Use named pipe or broadcast WM_COPYDATA
    Ok(())
}
```

**Step 2: Export from lib.rs**

Add `pub use platform::single_instance::{SingleInstance, AlreadyRunning, send_activate_to_existing};` to `lib.rs`.

**Step 3: Verify and commit**

```bash
git add crates/gpui/src/platform/single_instance.rs crates/gpui/src/lib.rs
git commit -m "feat: Add single-instance enforcement (Unix socket + Windows mutex)"
```

---

## Task 11: Window Overlay Level + Click-Through + Show/Hide — macOS

**Files:**
- Modify: `crates/gpui/src/platform/mac/window.rs`

**Step 1: Handle WindowKind::Overlay in window creation**

Find where `WindowKind` is mapped to `NSWindow.level` (around line 778-805). Add:

```rust
WindowKind::Overlay => {
    native_window.setLevel_(NSStatusWindowLevel); // 25, above floating panels and fullscreen
}
```

The constant `NSStatusWindowLevel` is 25 in AppKit. If not available, use raw: `let _: () = msg_send![native_window, setLevel: 25_isize];`

Also set `NSWindow.collectionBehavior` to include `NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorStationary | NSWindowCollectionBehaviorFullScreenAuxiliary` so it appears above fullscreen apps.

**Step 2: Implement mouse_passthrough**

During window creation, if `params.mouse_passthrough` is true:

```rust
let _: () = msg_send![native_window, setIgnoresMouseEvents: YES];
```

Implement `set_mouse_passthrough`:

```rust
fn set_mouse_passthrough(&self, passthrough: bool) {
    unsafe {
        let _: () = msg_send![self.native_window, setIgnoresMouseEvents: passthrough as BOOL];
    }
}
```

**Step 3: Implement show/hide**

```rust
fn show(&self) {
    unsafe {
        let _: () = msg_send![self.native_window, makeKeyAndOrderFront: nil];
    }
}

fn hide(&self) {
    unsafe {
        let _: () = msg_send![self.native_window, orderOut: nil];
    }
}

fn is_visible(&self) -> bool {
    unsafe {
        msg_send![self.native_window, isVisible]
    }
}
```

**Step 4: Verify and commit**

```bash
git add crates/gpui/src/platform/mac/window.rs
git commit -m "feat: Implement overlay level, click-through, show/hide on macOS"
```

---

## Task 12: Window Overlay + Click-Through + Show/Hide — Linux

**Files:**
- Modify: `crates/gpui/src/platform/linux/x11/window.rs`
- Modify: `crates/gpui/src/platform/linux/wayland/window.rs`

### X11

**Step 1: Overlay level**

For `WindowKind::Overlay`, set `_NET_WM_STATE_ABOVE` atom on the window via `x11rb`:

```rust
xcb_connection.change_property(
    PropMode::REPLACE,
    window_id,
    atoms._NET_WM_STATE,
    atoms.ATOM,
    32,
    1,
    &atoms._NET_WM_STATE_ABOVE.to_ne_bytes(),
)?;
```

Also set `_NET_WM_WINDOW_TYPE` to `_NET_WM_WINDOW_TYPE_DOCK` for overlay behavior.

**Step 2: Click-through (X11)**

Use `XShapeCombineRectangles` via `x11rb::protocol::shape`:

```rust
fn set_mouse_passthrough(&self, passthrough: bool) {
    if passthrough {
        // Set empty input shape
        shape::rectangles(conn, ShapeOp::SET, ShapeKind::INPUT, window_id, 0, 0, &[])?;
    } else {
        // Reset to full window
        shape::mask(conn, ShapeOp::SET, ShapeKind::INPUT, window_id, 0, 0, NONE)?;
    }
}
```

**Step 3: Show/hide (X11)**

```rust
fn show(&self) { xcb_connection.map_window(self.window_id); }
fn hide(&self) { xcb_connection.unmap_window(self.window_id); }
fn is_visible(&self) -> bool { self.visible.get() }
```

### Wayland

**Step 4: Overlay (Wayland)**

For `zwlr_layer_shell_v1`: request the overlay layer. This protocol is supported on Sway, Hyprland, and other wlroots compositors. Fall back to normal `xdg_toplevel` if layer shell isn't available.

**Step 5: Click-through (Wayland)**

Use `wl_surface::set_input_region` with an empty `wl_region`:

```rust
fn set_mouse_passthrough(&self, passthrough: bool) {
    if passthrough {
        let region = compositor.create_region();
        // Empty region = no input
        surface.set_input_region(Some(&region));
    } else {
        surface.set_input_region(None); // Reset to full surface
    }
    surface.commit();
}
```

**Step 6: Show/hide (Wayland)**

Attach null buffer to hide, reattach to show:

```rust
fn hide(&self) {
    self.surface.attach(None, 0, 0);
    self.surface.commit();
}
fn show(&self) {
    // Re-request frame and reattach buffer
    self.request_frame();
}
```

**Step 7: Verify and commit**

```bash
git add crates/gpui/src/platform/linux/
git commit -m "feat: Implement overlay, click-through, show/hide on Linux (X11 + Wayland)"
```

---

## Task 13: Window Overlay + Click-Through + Show/Hide — Windows

**Files:**
- Modify: `crates/gpui/src/platform/windows/window.rs`

**Step 1: Overlay level**

During window creation, for `WindowKind::Overlay`:

```rust
use windows::Win32::UI::WindowsAndMessaging::*;
SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
```

**Step 2: Click-through**

Add `WS_EX_TRANSPARENT | WS_EX_LAYERED` to extended window style:

```rust
fn set_mouse_passthrough(&self, passthrough: bool) {
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
    if passthrough {
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_TRANSPARENT.0 as i32 | WS_EX_LAYERED.0 as i32);
    } else {
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style & !(WS_EX_TRANSPARENT.0 as i32));
    }
}
```

**Step 3: Show/hide**

```rust
fn show(&self) { ShowWindow(hwnd, SW_SHOW); }
fn hide(&self) { ShowWindow(hwnd, SW_HIDE); }
fn is_visible(&self) -> bool { IsWindowVisible(hwnd).as_bool() }
```

**Step 4: Verify and commit**

```bash
git add crates/gpui/src/platform/windows/
git commit -m "feat: Implement overlay, click-through, show/hide on Windows"
```

---

## Task 14: Active Window Query — All Platforms

**Files:**
- Create: `crates/gpui/src/platform/mac/active_window.rs`
- Modify: `crates/gpui/src/platform/mac/platform.rs`
- Modify: `crates/gpui/src/platform/linux/platform.rs`
- Modify: `crates/gpui/src/platform/windows/platform.rs`

### macOS

**Step 1: Create active_window.rs**

```rust
use crate::platform::FocusedWindowInfo;
use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};

pub fn get_focused_window_info() -> Option<FocusedWindowInfo> {
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let frontmost_app: id = msg_send![workspace, frontmostApplication];
        if frontmost_app == nil {
            return None;
        }

        let app_name: id = msg_send![frontmost_app, localizedName];
        let app_name = nsstring_to_string(app_name)?;

        let bundle_id: id = msg_send![frontmost_app, bundleIdentifier];
        let bundle_id = nsstring_to_string(bundle_id);

        let pid: i32 = msg_send![frontmost_app, processIdentifier];

        // Window title requires Accessibility API
        let window_title = get_window_title_via_accessibility(pid).unwrap_or_default();

        Some(FocusedWindowInfo {
            app_name,
            window_title,
            bundle_id,
            pid: Some(pid as u32),
        })
    }
}

fn get_window_title_via_accessibility(pid: i32) -> Option<String> {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    // Create AXUIElement for the app, get focused window, read title attribute
    // Uses AXUIElementCreateApplication(pid) -> AXUIElementCopyAttributeValue(kAXFocusedWindowAttribute)
    // -> AXUIElementCopyAttributeValue(kAXTitleAttribute)
    // Returns None if accessibility permission not granted
    None // Implement with accessibility framework
}

unsafe fn nsstring_to_string(nsstring: id) -> Option<String> {
    if nsstring == nil {
        return None;
    }
    let bytes: *const std::ffi::c_char = msg_send![nsstring, UTF8String];
    if bytes.is_null() {
        return None;
    }
    Some(std::ffi::CStr::from_ptr(bytes).to_string_lossy().into_owned())
}
```

**Step 2: Implement on MacPlatform**

```rust
fn focused_window_info(&self) -> Option<FocusedWindowInfo> {
    active_window::get_focused_window_info()
}
```

### Linux

**Step 3: X11 active window query**

Use `x11rb` to read `_NET_ACTIVE_WINDOW` atom from root window, then `_NET_WM_NAME` and `_NET_WM_PID` from that window.

### Windows

**Step 4: Windows active window query**

```rust
fn focused_window_info(&self) -> Option<FocusedWindowInfo> {
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut title = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title);
        let window_title = String::from_utf16_lossy(&title[..len as usize]);

        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        // Get process name from PID
        let app_name = get_process_name(process_id).unwrap_or_default();

        Some(FocusedWindowInfo {
            app_name,
            window_title,
            bundle_id: None,
            pid: Some(process_id),
        })
    }
}
```

**Step 5: Verify and commit**

```bash
git add crates/gpui/src/platform/
git commit -m "feat: Implement active window query on all platforms"
```

---

## Task 15: Permissions — macOS

**Files:**
- Create: `crates/gpui/src/platform/mac/permissions.rs`
- Modify: `crates/gpui/src/platform/mac/platform.rs`

**Step 1: Create permissions.rs**

```rust
use crate::platform::PermissionStatus;

pub fn accessibility_status() -> PermissionStatus {
    unsafe {
        // AXIsProcessTrusted() returns bool
        let trusted: bool = AXIsProcessTrusted();
        if trusted {
            PermissionStatus::Granted
        } else {
            PermissionStatus::NotDetermined
        }
    }
}

pub fn request_accessibility_permission() {
    unsafe {
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;
        use core_foundation::boolean::CFBoolean;
        // AXIsProcessTrustedWithOptions with kAXTrustedCheckOptionPrompt = true
        // This opens System Settings > Privacy > Accessibility
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key, value)]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
    }
}

pub fn microphone_status() -> PermissionStatus {
    // AVCaptureDevice.authorizationStatus(for: .audio)
    // 0 = NotDetermined, 1 = Restricted (Denied), 2 = Denied, 3 = Authorized
    unsafe {
        let status: isize = msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: AVMediaTypeAudio];
        match status {
            0 => PermissionStatus::NotDetermined,
            3 => PermissionStatus::Granted,
            _ => PermissionStatus::Denied,
        }
    }
}

pub fn request_microphone_permission(callback: Box<dyn FnOnce(bool)>) {
    // AVCaptureDevice.requestAccess(for: .audio) { granted in ... }
    // Dispatch callback on main thread
}
```

**Step 2: Wire into MacPlatform**

Implement the `accessibility_status`, `request_accessibility_permission`, `microphone_status`, `request_microphone_permission` trait methods.

**Step 3: Verify and commit**

```bash
git add crates/gpui/src/platform/mac/
git commit -m "feat: Implement accessibility and microphone permission checks on macOS"
```

---

## Task 16: Auto-Launch — All Platforms

**Files:**
- Create: `crates/gpui/src/platform/mac/auto_launch.rs`
- Create: `crates/gpui/src/platform/linux/auto_launch.rs`
- Create: `crates/gpui/src/platform/windows/auto_launch.rs`

### macOS

**Step 1: macOS auto-launch via SMAppService**

```rust
pub fn set_auto_launch(app_id: &str, enabled: bool) -> Result<()> {
    // SMAppService.mainApp.register() for enable
    // SMAppService.mainApp.unregister() for disable
    // Requires macOS 13+, fall back to LSSharedFileList for older
}

pub fn is_auto_launch_enabled(app_id: &str) -> bool {
    // SMAppService.mainApp.status == .enabled
}
```

### Linux

**Step 2: Linux auto-launch via XDG autostart**

```rust
pub fn set_auto_launch(app_id: &str, enabled: bool) -> Result<()> {
    let autostart_dir = dirs::config_dir().unwrap().join("autostart");
    let desktop_file = autostart_dir.join(format!("{}.desktop", app_id));
    if enabled {
        std::fs::create_dir_all(&autostart_dir)?;
        let exe_path = std::env::current_exe()?;
        std::fs::write(&desktop_file, format!(
            "[Desktop Entry]\nType=Application\nName={}\nExec={}\nX-GNOME-Autostart-enabled=true\n",
            app_id, exe_path.display()
        ))?;
    } else {
        let _ = std::fs::remove_file(&desktop_file);
    }
    Ok(())
}
```

### Windows

**Step 3: Windows auto-launch via Registry**

```rust
pub fn set_auto_launch(app_id: &str, enabled: bool) -> Result<()> {
    use windows_registry::LOCAL_MACHINE;
    let key = windows_registry::CURRENT_USER
        .open("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run")?;
    if enabled {
        let exe_path = std::env::current_exe()?;
        key.set_string(app_id, &exe_path.to_string_lossy())?;
    } else {
        key.delete_value(app_id).ok();
    }
    Ok(())
}
```

**Step 4: Wire all three into their platform impls and verify**

```bash
git add crates/gpui/src/platform/
git commit -m "feat: Implement auto-launch on all platforms"
```

---

## Task 17: OS Notifications — All Platforms

**Files:**
- Create: `crates/gpui/src/notification.rs`
- Modify: `crates/gpui/src/platform/mac/platform.rs`
- Modify: `crates/gpui/src/platform/linux/platform.rs`
- Modify: `crates/gpui/src/platform/windows/platform.rs`
- Modify: `crates/gpui/Cargo.toml`

### macOS

**Step 1: macOS notifications via UNUserNotificationCenter**

```rust
fn show_notification(&self, title: &str, body: &str) -> Result<()> {
    unsafe {
        let center: id = msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        let content: id = msg_send![class!(UNMutableNotificationContent), new];
        let _: () = msg_send![content, setTitle: NSString::alloc(nil).init_str(title)];
        let _: () = msg_send![content, setBody: NSString::alloc(nil).init_str(body)];
        let request: id = msg_send![class!(UNNotificationRequest), requestWithIdentifier:NSString::alloc(nil).init_str(&uuid::Uuid::new_v4().to_string()) content:content trigger:nil];
        let _: () = msg_send![center, addNotificationRequest:request withCompletionHandler:nil];
    }
    Ok(())
}
```

### Linux

**Step 2: Add notify-rust dependency**

In `Cargo.toml` under Linux deps:

```toml
notify-rust = "4"
```

```rust
fn show_notification(&self, title: &str, body: &str) -> Result<()> {
    notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .show()?;
    Ok(())
}
```

### Windows

**Step 3: Windows toast notifications**

Use the `windows` crate's `ToastNotificationManager`:

```rust
fn show_notification(&self, title: &str, body: &str) -> Result<()> {
    // Use windows::UI::Notifications::ToastNotificationManager
    // Or fall back to balloon notification via Shell_NotifyIconW with NIF_INFO
    Ok(())
}
```

**Step 4: Verify and commit**

```bash
git add crates/gpui/
git commit -m "feat: Implement OS notifications on all platforms"
```

---

## Task 18: In-App Toast Component

**Files:**
- Create: `crates/gpui/src/elements/toast.rs`
- Modify: `crates/gpui/src/elements.rs` or `crates/gpui/src/lib.rs`

**Step 1: Create Toast element**

Build a GPUI view component that:
- Renders a styled container with text (title + optional body)
- Auto-dismisses after configurable duration (default 3s)
- Positions at top-right or bottom-right of window
- Supports fade-in/fade-out animation
- Can be stacked (multiple toasts at once)

Use the existing GPUI element system (div, styled, animation).

**Step 2: Export from lib.rs**

**Step 3: Verify and commit**

```bash
git add crates/gpui/src/elements/
git commit -m "feat: Add in-app Toast component with auto-dismiss"
```

---

## Task 19: Integration Example

**Files:**
- Create: `crates/gpui/examples/daemon_app.rs`

**Step 1: Create a comprehensive example**

Write an example that exercises all new features:
- Starts with no window (keep_alive_without_windows)
- Sets up system tray with icon and menu
- Registers a global hotkey
- On hotkey press: opens an overlay window (transparent, borderless, always-on-top)
- Shows a notification
- Demonstrates single instance
- Menu has "Settings" (opens normal window), "Quit" (exits)

**Step 2: Verify it compiles and runs**

Run: `cargo build --example daemon_app`

**Step 3: Commit**

```bash
git add crates/gpui/examples/daemon_app.rs
git commit -m "feat: Add daemon_app example demonstrating tray, hotkey, overlay, notifications"
```

---

## Task 20: Final Build Verification

**Step 1: Full build on all feature combinations**

```bash
cargo build -p gpui 2>&1 | tail -20
cargo build -p gpui --features=x11 2>&1 | tail -20
cargo build -p gpui --features=wayland 2>&1 | tail -20
cargo clippy -p gpui 2>&1 | tail -20
```

**Step 2: Run existing tests**

```bash
cargo test -p gpui 2>&1 | tail -20
```

**Step 3: Final commit if any fixups needed**

```bash
git add -A
git commit -m "fix: Build fixes and clippy warnings for GPUI extensions"
```

---

## Parallelization Guide

Tasks that can run in parallel:
- **Tasks 4, 5, 6** (system tray — one per platform)
- **Tasks 7, 8, 9** (global hotkey — one per platform)
- **Tasks 11, 12, 13** (overlay/click-through/show-hide — one per platform)
- **Task 14** (active window — all platforms but can split)
- **Task 16** (auto-launch — all platforms but can split)
- **Task 17** (notifications — all platforms but can split)

Tasks that must be sequential:
- **Task 1** (types) → everything else
- **Task 2** (wiring) → everything else
- **Task 3** (keep-alive) → Task 19 (example)
- **Tasks 4-18** → Task 19 (example)
- **Task 19** → Task 20 (final verification)
