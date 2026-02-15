use crate::OsInfo;
use cocoa::base::id;
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::CStr;

pub fn get_os_info() -> OsInfo {
    unsafe {
        let process_info: id = msg_send![class!(NSProcessInfo), processInfo];
        let version_string: id = msg_send![process_info, operatingSystemVersionString];
        let version_cstr: *const std::ffi::c_char = msg_send![version_string, UTF8String];
        let version = CStr::from_ptr(version_cstr).to_string_lossy().to_string();

        let locale_class = class!(NSLocale);
        let current_locale: id = msg_send![locale_class, currentLocale];
        let locale_id: id = msg_send![current_locale, localeIdentifier];
        let locale_cstr: *const std::ffi::c_char = msg_send![locale_id, UTF8String];
        let locale = CStr::from_ptr(locale_cstr).to_string_lossy().to_string();

        let mut hostname_buf = [0u8; 256];
        libc::gethostname(
            hostname_buf.as_mut_ptr() as *mut libc::c_char,
            hostname_buf.len(),
        );
        let hostname = CStr::from_ptr(hostname_buf.as_ptr() as *const libc::c_char)
            .to_string_lossy()
            .to_string();

        OsInfo {
            name: "macOS".into(),
            version: version.into(),
            arch: std::env::consts::ARCH.into(),
            locale: locale.into(),
            hostname: hostname.into(),
        }
    }
}
