#![cfg(feature = "system-libopenconnect")]

use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
pub struct OpenconnectInfo {
    _private: [u8; 0],
}

pub type ValidatePeerCertCallback =
    unsafe extern "C" fn(privdata: *mut c_void, reason: *const c_char) -> c_int;
pub type WriteNewConfigCallback =
    unsafe extern "C" fn(privdata: *mut c_void, buf: *const c_char, buflen: c_int) -> c_int;
pub type ProcessAuthFormCallback =
    unsafe extern "C" fn(privdata: *mut c_void, form: *mut c_void) -> c_int;

unsafe extern "C" {
    pub fn openconnect_init_ssl() -> c_int;
    pub fn openconnect_get_version() -> *const c_char;

    pub fn openconnect_vpninfo_new(
        useragent: *const c_char,
        validate_peer_cert: Option<ValidatePeerCertCallback>,
        write_new_config: Option<WriteNewConfigCallback>,
        process_auth_form: Option<ProcessAuthFormCallback>,
        progress: *mut c_void,
        privdata: *mut c_void,
    ) -> *mut OpenconnectInfo;

    pub fn openconnect_vpninfo_free(vpninfo: *mut OpenconnectInfo);
    pub fn openconnect_parse_url(vpninfo: *mut OpenconnectInfo, url: *const c_char) -> c_int;
    pub fn openconnect_set_protocol(
        vpninfo: *mut OpenconnectInfo,
        protocol: *const c_char,
    ) -> c_int;
    pub fn openconnect_set_cafile(vpninfo: *mut OpenconnectInfo, cafile: *const c_char) -> c_int;
    pub fn openconnect_set_loglevel(vpninfo: *mut OpenconnectInfo, level: c_int);
    pub fn openconnect_set_system_trust(vpninfo: *mut OpenconnectInfo, val: u32);
    pub fn openconnect_disable_ipv6(vpninfo: *mut OpenconnectInfo) -> c_int;
    pub fn openconnect_disable_dtls(vpninfo: *mut OpenconnectInfo) -> c_int;
    pub fn openconnect_get_protocol(vpninfo: *mut OpenconnectInfo) -> *const c_char;
    pub fn openconnect_get_connect_url(vpninfo: *mut OpenconnectInfo) -> *const c_char;
}
