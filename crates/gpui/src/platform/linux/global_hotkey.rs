#![allow(dead_code)]

use anyhow::Result;
use collections::HashMap;

use crate::Keystroke;

pub struct LinuxGlobalHotkey {
    registered: HashMap<u32, Keystroke>,
}

impl LinuxGlobalHotkey {
    pub fn new() -> Self {
        Self {
            registered: HashMap::default(),
        }
    }

    pub fn register(&mut self, id: u32, keystroke: &Keystroke) -> Result<()> {
        self.registered.insert(id, keystroke.clone());
        Ok(())
    }

    pub fn unregister(&mut self, id: u32) {
        self.registered.remove(&id);
    }
}

#[cfg(feature = "x11")]
pub mod x11 {
    use super::*;
    use std::rc::Rc;
    use x11rb::protocol::xproto::{self, ConnectionExt as _};
    use x11rb::xcb_ffi::XCBConnection;

    pub struct X11GlobalHotkey {
        inner: LinuxGlobalHotkey,
    }

    impl X11GlobalHotkey {
        pub fn new() -> Self {
            Self {
                inner: LinuxGlobalHotkey::new(),
            }
        }

        pub fn register(
            &mut self,
            id: u32,
            keystroke: &Keystroke,
            _xcb: &Rc<XCBConnection>,
            _root_window: xproto::Window,
        ) -> Result<()> {
            self.inner.register(id, keystroke)
        }

        pub fn unregister(
            &mut self,
            id: u32,
            _xcb: &Rc<XCBConnection>,
            _root_window: xproto::Window,
        ) {
            self.inner.unregister(id);
        }
    }
}

#[cfg(feature = "wayland")]
pub mod wayland {
    use super::*;

    pub struct WaylandGlobalHotkey {
        inner: LinuxGlobalHotkey,
    }

    impl WaylandGlobalHotkey {
        pub fn new() -> Self {
            Self {
                inner: LinuxGlobalHotkey::new(),
            }
        }

        pub fn register(&mut self, id: u32, keystroke: &Keystroke) -> Result<()> {
            self.inner.register(id, keystroke)
        }

        pub fn unregister(&mut self, id: u32) {
            self.inner.unregister(id);
        }
    }
}
