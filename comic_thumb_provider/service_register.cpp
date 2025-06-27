// cbz_thumbnail_provider.cpp
#include <windows.h>
#include <shlwapi.h>
#include <shobjidl.h>
#include <strsafe.h>
#include <wincodec.h>

#pragma comment(lib, "shlwapi.lib")
#pragma comment(lib, "windowscodecs.lib")

// {F3A9F6D8-4E96-4C2B-A3B0-9A3E2F4C1C6E} - generate your own GUID for your CLSID!
const CLSID CLSID_CbzThumbnailProvider = 
{ 0xf3a9f6d8, 0x4e96, 0x4c2b, { 0xa3, 0xb0, 0x9a, 0x3e, 0x2f, 0x4c, 0x1c, 0x6e } };

class CbzThumbnailProvider : public IInitializeWithStream, public IThumbnailProvider, public IUnknown
{
    LONG _refCount;
    IStream* _pStream;

public:
    CbzThumbnailProvider() : _refCount(1), _pStream(nullptr) {}
    ~CbzThumbnailProvider() { if (_pStream) _pStream->Release(); }

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppv)
    {
        if (!ppv) return E_POINTER;
        if (riid == IID_IUnknown || riid == IID_IInitializeWithStream)
        {
            *ppv = static_cast<IInitializeWithStream*>(this);
        }
        else if (riid == IID_IThumbnailProvider)
        {
            *ppv = static_cast<IThumbnailProvider*>(this);
        }
        else
        {
            *ppv = nullptr;
            return E_NOINTERFACE;
        }
        AddRef();
        return S_OK;
    }

    STDMETHODIMP_(ULONG) AddRef() { return InterlockedIncrement(&_refCount); }
    STDMETHODIMP_(ULONG) Release()
    {
        ULONG ref = InterlockedDecrement(&_refCount);
        if (ref == 0) delete this;
        return ref;
    }

    // IInitializeWithStream
    STDMETHODIMP Initialize(IStream* pStream, DWORD /*grfMode*/)
    {
        if (_pStream)
        {
            _pStream->Release();
            _pStream = nullptr;
        }
        if (!pStream) return E_INVALIDARG;
        pStream->AddRef();
        _pStream = pStream;
        return S_OK;
    }

    // IThumbnailProvider
    STDMETHODIMP GetThumbnail(UINT cx, HBITMAP* phbmp, WTS_ALPHATYPE* pdwAlpha)
    {
        if (!phbmp || !pdwAlpha) return E_INVALIDARG;
        // For demo: return a simple solid color bitmap thumbnail.

        HDC hdc = GetDC(NULL);
        if (!hdc) return E_FAIL;

        HDC memDC = CreateCompatibleDC(hdc);
        if (!memDC)
        {
            ReleaseDC(NULL, hdc);
            return E_FAIL;
        }

        HBITMAP hBitmap = CreateCompatibleBitmap(hdc, cx, cx);
        if (!hBitmap)
        {
            DeleteDC(memDC);
            ReleaseDC(NULL, hdc);
            return E_FAIL;
        }

        SelectObject(memDC, hBitmap);

        // Fill bitmap with red color for demo
        HBRUSH brush = CreateSolidBrush(RGB(255, 0, 0));
        RECT rc = { 0, 0, (LONG)cx, (LONG)cx };
        FillRect(memDC, &rc, brush);
        DeleteObject(brush);

        DeleteDC(memDC);
        ReleaseDC(NULL, hdc);

        *phbmp = hBitmap;
        *pdwAlpha = WTS_ALPHATYPE::WTSAT_ARGB;

        return S_OK;
    }
};

// Factory and DLL entrypoints
class CbzThumbnailProviderFactory : public IClassFactory
{
    LONG _refCount;

public:
    CbzThumbnailProviderFactory() : _refCount(1) {}

    STDMETHODIMP QueryInterface(REFIID riid, void** ppv)
    {
        if (!ppv) return E_POINTER;
        if (riid == IID_IUnknown || riid == IID_IClassFactory)
        {
            *ppv = static_cast<IClassFactory*>(this);
        }
        else
        {
            *ppv = nullptr;
            return E_NOINTERFACE;
        }
        AddRef();
        return S_OK;
    }

    STDMETHODIMP_(ULONG) AddRef() { return InterlockedIncrement(&_refCount); }
    STDMETHODIMP_(ULONG) Release()
    {
        ULONG ref = InterlockedDecrement(&_refCount);
        if (ref == 0) delete this;
        return ref;
    }

    STDMETHODIMP CreateInstance(IUnknown* pUnkOuter, REFIID riid, void** ppv)
    {
        if (pUnkOuter) return CLASS_E_NOAGGREGATION;
        CbzThumbnailProvider* pProvider = new (std::nothrow) CbzThumbnailProvider();
        if (!pProvider) return E_OUTOFMEMORY;
        HRESULT hr = pProvider->QueryInterface(riid, ppv);
        pProvider->Release();
        return hr;
    }

    STDMETHODIMP LockServer(BOOL fLock)
    {
        if (fLock)
            InterlockedIncrement(&_refCount);
        else
            InterlockedDecrement(&_refCount);
        return S_OK;
    }
};

HMODULE g_hModule = nullptr;
LONG g_lockCount = 0;

STDAPI DllGetClassObject(REFCLSID rclsid, REFIID riid, LPVOID* ppv)
{
    if (rclsid == CLSID_CbzThumbnailProvider)
    {
        static CbzThumbnailProviderFactory factory;
        return factory.QueryInterface(riid, ppv);
    }
    return CLASS_E_CLASSNOTAVAILABLE;
}

STDAPI DllCanUnloadNow()
{
    return (g_lockCount == 0) ? S_OK : S_FALSE;
}

BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID /*lpReserved*/)
{
    if (ul_reason_for_call == DLL_PROCESS_ATTACH)
    {
        g_hModule = hModule;
        DisableThreadLibraryCalls(hModule);
    }
    return TRUE;
}

// Registration helper - register CLSID and thumbnail provider handler
HRESULT RegisterServer()
{
    wchar_t modulePath[MAX_PATH];
    if (!GetModuleFileNameW(g_hModule, modulePath, MAX_PATH))
        return HRESULT_FROM_WIN32(GetLastError());

    // Registry keys under:
    // HKCR\.cbz\shellex\{e357fccd-a995-4576-b01f-234630154e96} = {your CLSID}
    // {e357fccd-a995-4576-b01f-234630154e96} is the thumbnail provider shell extension CLSID

    wchar_t clsidString[39];
    StringFromGUID2(CLSID_CbzThumbnailProvider, clsidString, 39);

    wchar_t regPath[MAX_PATH];

    // Create key: HKCR\.cbz\shellex\{e357fccd-a995-4576-b01f-234630154e96}
    StringCchPrintfW(regPath, MAX_PATH, L".cbz\\shellex\\{e357fccd-a995-4576-b01f-234630154e96}");
    HKEY hKey;
    LONG res = RegCreateKeyExW(HKEY_CLASSES_ROOT, regPath, 0, nullptr, 0, KEY_WRITE, nullptr, &hKey, nullptr);
    if (res != ERROR_SUCCESS) return HRESULT_FROM_WIN32(res);

    res = RegSetValueExW(hKey, nullptr, 0, REG_SZ, (const BYTE*)clsidString, (DWORD)((wcslen(clsidString) + 1) * sizeof(wchar_t)));
    RegCloseKey(hKey);
    if (res != ERROR_SUCCESS) return HRESULT_FROM_WIN32(res);

    // Register CLSID\{your CLSID}
    StringCchPrintfW(regPath, MAX_PATH, L"CLSID\\%s", clsidString);
    res = RegCreateKeyExW(HKEY_CLASSES_ROOT, regPath, 0, nullptr, 0, KEY_WRITE, nullptr, &hKey, nullptr);
    if (res != ERROR_SUCCESS) return HRESULT_FROM_WIN32(res);

    // Set default value to descriptive name
    const wchar_t* description = L"CBZ Thumbnail Provider";
    res = RegSetValueExW(hKey, nullptr, 0, REG_SZ, (const BYTE*)description, (DWORD)((wcslen(description) + 1) * sizeof(wchar_t)));
    if (res != ERROR_SUCCESS) {
        RegCloseKey(hKey);
        return HRESULT_FROM_WIN32(res);
    }

    // InprocServer32 subkey
    HKEY hInprocKey;
    res = RegCreateKeyExW(hKey, L"InprocServer32", 0, nullptr, 0, KEY_WRITE, nullptr, &hInprocKey, nullptr);
    if (res != ERROR_SUCCESS) {
        RegCloseKey(hKey);
        return HRESULT_FROM_WIN32(res);
    }

    // Set default value to DLL path
    res = RegSetValueExW(hInprocKey, nullptr, 0, REG_SZ, (const BYTE*)modulePath, (DWORD)((wcslen(modulePath) + 1) * sizeof(wchar_t)));

    // Set ThreadingModel = Apartment
    const wchar_t* threadingModel = L"Apartment";
    RegSetValueExW(hInprocKey, L"ThreadingModel", 0, REG_SZ, (const BYTE*)threadingModel, (DWORD)((wcslen(threadingModel) + 1) * sizeof(wchar_t)));

    RegCloseKey(hInprocKey);
    RegCloseKey(hKey);

    return S_OK;
}

HRESULT UnregisterServer()
{
    // Remove registry keys created in RegisterServer()
    // (simple version - error checking omitted for brevity)

    wchar_t clsidString[39];
    StringFromGUID2(CLSID_CbzThumbnailProvider, clsidString, 39);

    wchar_t regPath[MAX_PATH];
    StringCchPrintfW(regPath, MAX_PATH, L".cbz\\shellex\\{e357fccd-a995-4576-b01f-234630154e96}");
    RegDeleteKeyW(HKEY_CLASSES_ROOT, regPath);

    StringCchPrintfW(regPath, MAX_PATH, L"CLSID\\%s\\InprocServer32", clsidString);
    RegDeleteKeyW(HKEY_CLASSES_ROOT, regPath);

    StringCchPrintfW(regPath, MAX_PATH, L"CLSID\\%s", clsidString);
    RegDeleteKeyW(HKEY_CLASSES_ROOT, regPath);

    return S_OK;
}

STDAPI DllRegisterServer()
{
    return RegisterServer();
}

STDAPI DllUnregisterServer()
{
    return UnregisterServer();
}

