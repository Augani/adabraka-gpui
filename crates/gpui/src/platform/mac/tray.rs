use crate::platform::TrayMenuItem;
use cocoa::{
    appkit::NSApplication,
    base::{id, nil, NO, YES},
    foundation::{NSData, NSSize, NSString},
};
use objc::{class, msg_send, rc::StrongPtr, sel, sel_impl};
use std::ffi::c_void;

pub(crate) struct MacTray {
    status_item: StrongPtr,
}

impl MacTray {
    pub fn new() -> Self {
        unsafe {
            let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            let length: f64 = -1.0;
            let status_item: id = msg_send![status_bar, statusItemWithLength: length];
            let status_item = StrongPtr::retain(status_item);
            let _: () = msg_send![*status_item, setVisible: YES];

            let button: id = msg_send![*status_item, button];
            if button != nil {
                let default_title = NSString::alloc(nil).init_str("App");
                let _: () = msg_send![button, setTitle: default_title];
            }

            Self { status_item }
        }
    }

    pub fn set_icon(&self, icon_data: Option<&[u8]>) {
        unsafe {
            let button: id = msg_send![*self.status_item, button];
            if button == nil {
                return;
            }
            match icon_data {
                Some(data) => {
                    let ns_data: id = NSData::dataWithBytes_length_(
                        nil,
                        data.as_ptr() as *const c_void,
                        data.len() as u64,
                    );
                    let image: id = msg_send![class!(NSImage), alloc];
                    let image: id = msg_send![image, initWithData: ns_data];
                    if image != nil {
                        let _: () = msg_send![image, setSize: NSSize::new(18.0, 18.0)];
                        let _: () = msg_send![image, setTemplate: YES];
                        let _: () = msg_send![button, setImage: image];
                        let empty = NSString::alloc(nil).init_str("");
                        let _: () = msg_send![button, setTitle: empty];
                    }
                }
                None => {
                    let _: () = msg_send![button, setImage: nil];
                }
            }
        }
    }

    pub fn set_title(&self, title: &str) {
        unsafe {
            let button: id = msg_send![*self.status_item, button];
            if button == nil {
                return;
            }
            let ns_title = NSString::alloc(nil).init_str(title);
            let _: () = msg_send![button, setTitle: ns_title];
        }
    }

    pub fn set_tooltip(&self, tooltip: &str) {
        unsafe {
            let button: id = msg_send![*self.status_item, button];
            if button == nil {
                return;
            }
            let ns_tooltip = NSString::alloc(nil).init_str(tooltip);
            let _: () = msg_send![button, setToolTip: ns_tooltip];
        }
    }

    pub fn set_menu(&self, items: Vec<TrayMenuItem>) {
        unsafe {
            let menu: id = msg_send![class!(NSMenu), new];
            let _: () = msg_send![menu, setAutoenablesItems: NO];
            build_menu(menu, &items);
            let _: () = msg_send![*self.status_item, setMenu: menu];
        }
    }
}

impl Drop for MacTray {
    fn drop(&mut self) {
        unsafe {
            let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            let _: () = msg_send![status_bar, removeStatusItem: *self.status_item];
        }
    }
}

unsafe fn get_app_delegate() -> id {
    let app: id = msg_send![class!(NSApplication), sharedApplication];
    msg_send![app, delegate]
}

unsafe fn configure_actionable_item(menu_item: id, item_id: &str) {
    let delegate = get_app_delegate();
    if delegate != nil {
        let _: () = msg_send![menu_item, setTarget: delegate];
        let _: () = msg_send![menu_item, setAction: sel!(handleTrayMenuItem:)];
        let represented = NSString::alloc(nil).init_str(item_id);
        let _: () = msg_send![menu_item, setRepresentedObject: represented];
        let _: () = msg_send![menu_item, setEnabled: YES];
    }
}

unsafe fn build_menu(menu: id, items: &[TrayMenuItem]) {
    unsafe {
        for item in items {
            match item {
                TrayMenuItem::Action { label, id } => {
                    let title = NSString::alloc(nil).init_str(label.as_ref());
                    let menu_item: id = msg_send![class!(NSMenuItem), alloc];
                    let empty = NSString::alloc(nil).init_str("");
                    let menu_item: id =
                        msg_send![menu_item, initWithTitle:title action:nil keyEquivalent:empty];
                    configure_actionable_item(menu_item, id.as_ref());
                    let _: () = msg_send![menu, addItem: menu_item];
                }
                TrayMenuItem::Separator => {
                    let separator: id = msg_send![class!(NSMenuItem), separatorItem];
                    let _: () = msg_send![menu, addItem: separator];
                }
                TrayMenuItem::Submenu {
                    label,
                    items: sub_items,
                } => {
                    let title = NSString::alloc(nil).init_str(label.as_ref());
                    let menu_item: id = msg_send![class!(NSMenuItem), alloc];
                    let empty = NSString::alloc(nil).init_str("");
                    let menu_item: id =
                        msg_send![menu_item, initWithTitle:title action:nil keyEquivalent:empty];
                    let submenu: id = msg_send![class!(NSMenu), new];
                    build_menu(submenu, sub_items);
                    let _: () = msg_send![menu_item, setSubmenu: submenu];
                    let _: () = msg_send![menu, addItem: menu_item];
                }
                TrayMenuItem::Toggle { label, checked, id } => {
                    let title = NSString::alloc(nil).init_str(label.as_ref());
                    let menu_item: id = msg_send![class!(NSMenuItem), alloc];
                    let empty = NSString::alloc(nil).init_str("");
                    let menu_item: id =
                        msg_send![menu_item, initWithTitle:title action:nil keyEquivalent:empty];
                    configure_actionable_item(menu_item, id.as_ref());
                    let state: isize = if *checked { 1 } else { 0 };
                    let _: () = msg_send![menu_item, setState: state];
                    let _: () = msg_send![menu, addItem: menu_item];
                }
            }
        }
    }
}
