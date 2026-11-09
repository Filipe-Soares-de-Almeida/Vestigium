use vestigium_core::health::SecurityProduct;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::SecurityCenter::{
    IWSCProductList, WSC_SECURITY_PRODUCT_STATE_ON, WSC_SECURITY_PROVIDER_ANTIVIRUS, WSCProductList,
};

/// The antivirus products the Windows Security Center knows, through its COM API (no WMI);
/// `None` where the Security Center is missing, as on Windows Server.
pub fn antivirus() -> Option<Vec<SecurityProduct>> {
    // SAFETY: initializes COM for this thread; balanced below when this call owns it.
    let owned = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    let products = list();
    if owned {
        // SAFETY: balances the successful CoInitializeEx above, on the same thread.
        unsafe { CoUninitialize() };
    }
    products.ok()
}

fn list() -> windows::core::Result<Vec<SecurityProduct>> {
    // SAFETY: COM is initialized on this thread; the list is released when dropped.
    let list: IWSCProductList =
        unsafe { CoCreateInstance(&WSCProductList, None, CLSCTX_INPROC_SERVER)? };
    // SAFETY: calls on a live interface pointer.
    unsafe {
        list.Initialize(WSC_SECURITY_PROVIDER_ANTIVIRUS)?;
        (0..list.Count()?.max(0) as u32)
            .map(|i| {
                let item = list.get_Item(i)?;
                Ok(SecurityProduct {
                    name: item.ProductName()?.to_string(),
                    on: item.ProductState()? == WSC_SECURITY_PRODUCT_STATE_ON,
                })
            })
            .collect()
    }
}
