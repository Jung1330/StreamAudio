#![allow(non_snake_case, dead_code)]

use std::ffi::c_void;
use windows::core::{GUID, HRESULT, PCWSTR};
use windows::Win32::Media::Audio::{eCommunications, eConsole, eMultimedia, ERole};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

// CLSID_CPolicyConfigClient: {870af99c-171d-4f9e-af0d-e63df40c2bc9}
const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

// IID_IPolicyConfig: {f8679f50-850a-41cf-9c72-430f290290c8}
const IID_POLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

#[repr(C)]
struct IPolicyConfigVtbl {
    // IUnknown
    QueryInterface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    AddRef: unsafe extern "system" fn(*mut c_void) -> u32,
    Release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IPolicyConfig (indices 3..13)
    GetMixFormat: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut *mut c_void) -> HRESULT,
    GetDeviceFormat: unsafe extern "system" fn(*mut c_void, PCWSTR, i32, *mut *mut c_void) -> HRESULT,
    ResetDeviceFormat: unsafe extern "system" fn(*mut c_void, PCWSTR) -> HRESULT,
    SetDeviceFormat: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut c_void, *mut c_void) -> HRESULT,
    GetProcessingPeriod: unsafe extern "system" fn(*mut c_void, PCWSTR, i32, *mut i64, *mut i64) -> HRESULT,
    SetProcessingPeriod: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut i64) -> HRESULT,
    GetShareMode: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut c_void) -> HRESULT,
    SetShareMode: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut c_void) -> HRESULT,
    GetPropertyValue: unsafe extern "system" fn(*mut c_void, PCWSTR, *const c_void, *mut c_void) -> HRESULT,
    SetPropertyValue: unsafe extern "system" fn(*mut c_void, PCWSTR, *const c_void, *const c_void) -> HRESULT,
    SetDefaultEndpoint: unsafe extern "system" fn(*mut c_void, PCWSTR, ERole) -> HRESULT,
    SetEndpointVisibility: unsafe extern "system" fn(*mut c_void, PCWSTR, i32) -> HRESULT,
}

#[repr(C)]
struct IPolicyConfigRaw {
    vtbl: *const IPolicyConfigVtbl,
}

/// Sets the default Windows audio endpoint for Console, Multimedia, and Communications roles.
pub unsafe fn set_default_audio_endpoint(device_id: &str) -> Result<(), String> {
    let wide: Vec<u16> = device_id.encode_utf16().chain(std::iter::once(0)).collect();
    let pwstr = PCWSTR::from_raw(wide.as_ptr());

    let unknown: windows::core::IUnknown =
        CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)
            .map_err(|e| format!("CoCreateInstance(CPolicyConfigClient) failed: {e}"))?;

    let mut raw_ptr: *mut c_void = std::ptr::null_mut();
    let qi_hr = (windows::core::Interface::vtable(&unknown).QueryInterface)(
        windows::core::Interface::as_raw(&unknown),
        &IID_POLICY_CONFIG,
        &mut raw_ptr,
    );
    if qi_hr.is_err() || raw_ptr.is_null() {
        return Err(format!("QueryInterface(IPolicyConfig) failed: {qi_hr:?}"));
    }

    let policy = raw_ptr as *mut IPolicyConfigRaw;
    let vtbl = (*policy).vtbl;

    let mut last_err: Option<HRESULT> = None;
    for role in [eConsole, eMultimedia, eCommunications] {
        let hr = ((*vtbl).SetDefaultEndpoint)(raw_ptr, pwstr, role);
        if hr.is_err() {
            last_err = Some(hr);
        }
    }

    ((*vtbl).Release)(raw_ptr);

    if let Some(hr) = last_err {
        return Err(format!("SetDefaultEndpoint failed: {hr:?}"));
    }

    Ok(())
}
