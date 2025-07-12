use windows::core::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::{S_OK, E_FAIL};
use comic_archive::ImageArchive;
use std::path::Path;
use image::{DynamicImage, ImageFormat};
use image::GenericImageView;
use windows::core::GUID;

// Define CLSID once (hardcoded)
pub const CLSID_COMIC_THUMB_PROVIDER: GUID = GUID::from_u128(0x6e80958a_59b6_41b4_932b_64d3c9532235);
pub const CLSID_COMIC_THUMB_PROVIDER_STR: &str = "{6e80958a-59b6-41b4-932b-64d3c9532235}";

#[implement(IThumbnailProvider)]
pub struct ComicThumbnailProvider;

impl IThumbnailProvider_Impl for ComicThumbnailProvider {
    fn GetThumbnail(
        &self,
        cx: u32,
        phbmp: *mut HBITMAP,
        pdw_alpha: *mut WTS_ALPHATYPE,
    ) -> windows::core::Result<()> {
        // TODO: Get the file path from the COM context (not shown here)
        let archive_path = Path::new("B:/Explicit/EULA.cbz");
        let mut archive = ImageArchive::process(archive_path)
            .map_err(|_| windows::core::Error::from(E_FAIL))?;
        let image_list = archive.list_images();
        if image_list.is_empty() {
            return Err(windows::core::Error::from(E_FAIL));
        }
        let image_bytes = archive.read_image_by_name(&image_list[0])
            .map_err(|_| windows::core::Error::from(E_FAIL))?;

        let img = image::load_from_memory(&image_bytes)
            .map_err(|_| windows::core::Error::from(E_FAIL))?;
        let thumb = img.thumbnail(cx, cx);
        let rgba = thumb.to_rgba8();
        let (width, height) = thumb.dimensions();

        unsafe {
            let hdc = GetDC(None);
            let hbitmap = CreateBitmap(
                width as i32,
                height as i32,
                1,
                32,
                Some(rgba.as_ptr() as *const _),
            );
            ReleaseDC(None, hdc);

            if hbitmap.0 == 0 {
                return Err(windows::core::Error::from(E_FAIL));
            }

            *phbmp = hbitmap;
            *pdw_alpha = WTS_ALPHATYPE(2); // ARGB
        }

        Ok(())
    }
}

// Required DLL exports for COM registration
#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_OK
}

#[unsafe(no_mangle)]
pub extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> HRESULT {
    HRESULT(1)
}

#[unsafe(no_mangle)]
pub extern "system" fn DllRegisterServer() -> i32 {
    use winreg::enums::HKEY_CLASSES_ROOT;
    use winreg::RegKey;
    use std::env;

    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    let clsid = "{6e80958a-59b6-41b4-932b-64d3c9532235}";
    let dll_path = env::current_exe()
        .map(|p| p.with_extension("dll").to_string_lossy().to_string())
        .unwrap_or_default();

    let _ = hkcr.create_subkey(format!("CLSID\\{}", clsid))
        .and_then(|(clsid_key, _)| {
            clsid_key.set_value("", &"Comic Thumbnail Provider")?;
            clsid_key.create_subkey("InprocServer32")
                .and_then(|(inproc_key, _)| {
                    inproc_key.set_value("", &dll_path)?;
                    inproc_key.set_value("ThreadingModel", &"Apartment")
                })
        });

    for ext in &[".cbz", ".cbr", ".cb7"] {
        let _ = hkcr.create_subkey(format!(
            "{}\\shellex\\{{e357fccd-a995-4576-b01f-234630154e96}}",
            ext
        )).and_then(|(ext_key, _)| ext_key.set_value("", &clsid));
    }

    0 // S_OK
}

#[unsafe(no_mangle)]
pub extern "system" fn DllUnregisterServer() -> i32 {
    use winreg::enums::HKEY_CLASSES_ROOT;
    use winreg::RegKey;

    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let clsid = "{6e80958a-59b6-41b4-932b-64d3c9532235}";

    let _ = hkcr.delete_subkey_all(format!("CLSID\\{}", clsid));
    for ext in &[".cbz", ".cbr", ".cb7"] {
        let _ = hkcr.delete_subkey_all(format!(
            "{}\\shellex\\{{e357fccd-a995-4576-b01f-234630154e96}}",
            ext
        ));
    }

    0 // S_OK
}