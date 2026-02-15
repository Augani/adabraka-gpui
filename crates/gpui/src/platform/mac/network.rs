use crate::NetworkStatus;
use std::ffi::c_void;

#[link(name = "SystemConfiguration", kind = "framework")]
unsafe extern "C" {
    fn SCNetworkReachabilityCreateWithName(
        allocator: *const c_void,
        nodename: *const u8,
    ) -> *const c_void;
    fn SCNetworkReachabilityGetFlags(target: *const c_void, flags: *mut u32) -> u8;
}

const K_SC_NETWORK_REACHABILITY_FLAGS_REACHABLE: u32 = 1 << 1;

pub(crate) fn network_status() -> NetworkStatus {
    unsafe {
        let host = b"captive.apple.com\0";
        let reachability = SCNetworkReachabilityCreateWithName(std::ptr::null(), host.as_ptr());
        if reachability.is_null() {
            return NetworkStatus::Offline;
        }

        let mut flags: u32 = 0;
        let ok = SCNetworkReachabilityGetFlags(reachability, &mut flags);

        core_foundation::base::CFRelease(reachability as core_foundation::base::CFTypeRef);

        if ok != 0 && (flags & K_SC_NETWORK_REACHABILITY_FLAGS_REACHABLE) != 0 {
            NetworkStatus::Online
        } else {
            NetworkStatus::Offline
        }
    }
}
