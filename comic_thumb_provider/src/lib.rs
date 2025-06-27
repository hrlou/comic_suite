#![windows_subsystem = "windows"]

use std::{
    cell::Cell,
    ffi::c_void,
    ptr::null_mut,
};

use windows::{
    core::{GUID, HRESULT, Interface, Result, ComInterface, Type},
    Win32::{
        Foundation::{E_NOINTERFACE, S_OK},
        Graphics::Gdi::{
            CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, GetDC, ReleaseDC, SelectObject,
            HBITMAP,
        },
        System::Com::IStream,
        UI::Shell::{IInitializeWithStream, IThumbnailProvider, WTS_ALPHATYPE},
    },
};

pub struct ThumbnailProvider {
    ref_count: Cell<u32>,
    stream: Option<IStream>,
}

impl ThumbnailProvider {
    pub fn new() -> Self {
        Self {
            ref_count: Cell::new(1),
            stream: None,
        }
    }
}

// IUnknown methods

unsafe extern "system" fn query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    let this = this as *mut ThumbnailProvider;

    unsafe {
        if ppv.is_null() {
            return HRESULT::from_win32(0x80004003); // E_POINTER
        }

        *ppv = null_mut();

        if *riid == IThumbnailProvider::IID || *riid == IInitializeWithStream::IID || *riid == windows::core::IUnknown::IID {
            *ppv = this as *mut c_void;
            add_ref(this);
            S_OK
        } else {
            E_NOINTERFACE
        }
    }
}

unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    let this = this as *mut ThumbnailProvider;
    let count = (*this).ref_count.get() + 1;
    (*this).ref_count.set(count);
    count
}

unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    let this = this as *mut ThumbnailProvider;
    let count = (*this).ref_count.get() - 1;
    (*this).ref_count.set(count);
    if count == 0 {
        Box::from_raw(this);
        0
    } else {
        count
    }
}

// IInitializeWithStream::Initialize

unsafe extern "system" fn initialize(this: *mut c_void, stream: *mut c_void) -> HRESULT {
    let this = this as *mut ThumbnailProvider;
    if stream.is_null() {
        return HRESULT::from_win32(0x80070057); // E_INVALIDARG
    }
    let stream = IStream::from_raw(stream);

    (*this).stream = Some(stream);
    S_OK
}

// IThumbnailProvider::GetThumbnail

unsafe extern "system" fn get_thumbnail(
    this: *mut c_void,
    cx: u32,
    phbmp: *mut HBITMAP,
    pdw_alpha: *mut WTS_ALPHATYPE,
) -> HRESULT {
    if phbmp.is_null() || pdw_alpha.is_null() {
        return HRESULT::from_win32(0x80070057); // E_INVALIDARG
    }

    let hdc = GetDC(None);
    if hdc.is_null() {
        return HRESULT::from_win32(0x80004005); // E_FAIL
    }
    let memdc = CreateCompatibleDC(hdc);
    let hbmp = CreateCompatibleBitmap(hdc, cx as i32, cx as i32);
    if hbmp.0 == 0 {
        ReleaseDC(None, hdc);
        DeleteDC(memdc);
        return HRESULT::from_win32(0x80004005); // E_FAIL
    }
    SelectObject(memdc, hbmp);

    // Here you’d render your thumbnail content onto hbmp
    // For now, leave it blank or fill with background color if you want

    *phbmp = hbmp;
    *pdw_alpha = WTS_ALPHATYPE::WTSAT_ARGB;

    ReleaseDC(None, hdc);
    DeleteDC(memdc);

    S_OK
}

// VTable for ThumbnailProvider

#[repr(C)]
pub struct ThumbnailProviderVTable {
    pub query_interface: unsafe extern "system" fn(
        *mut c_void,
        *const GUID,
        *mut *mut c_void,
    ) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub initialize: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
    pub get_thumbnail: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *mut HBITMAP,
        *mut WTS_ALPHATYPE,
    ) -> HRESULT,
}

static VTABLE: ThumbnailProviderVTable = ThumbnailProviderVTable {
    query_interface,
    add_ref,
    release,
    initialize,
    get_thumbnail,
};

impl Interface for ThumbnailProvider {
    type Vtable = ThumbnailProviderVTable;
}

unsafe impl ComInterface for ThumbnailProvider {
    const IID: GUID = <IThumbnailProvider as ComInterface>::IID;
}

// Helper to get the VTable pointer

impl ThumbnailProvider {
    pub fn vtable(&self) -> *const ThumbnailProviderVTable {
        &VTABLE as *const _
    }
}

// Conversion to raw pointer for COM

impl From<Box<ThumbnailProvider>> for *mut c_void {
    fn from(b: Box<ThumbnailProvider>) -> Self {
        Box::into_raw(b) as *mut _
    }
}
