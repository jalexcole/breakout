#[doc(hidden)]
#[repr(transparent)]
#[derive(::core::cmp::PartialEq, ::core::cmp::Eq, ::core::fmt::Debug, ::core::clone::Clone)]
pub struct IDisplayRequest(::windows_core::IUnknown);
unsafe impl ::windows_core::Interface for IDisplayRequest {
    type Vtable = IDisplayRequest_Vtbl;
}
unsafe impl ::windows_core::ComInterface for IDisplayRequest {
    const IID: ::windows_core::GUID = ::windows_core::GUID::from_u128(0xe5732044_f49f_4b60_8dd4_5e7e3a632ac0);
}
#[repr(C)]
#[doc(hidden)]
pub struct IDisplayRequest_Vtbl {
    pub base__: ::windows_core::IInspectable_Vtbl,
    pub RequestActive: unsafe extern "system" fn(this: *mut ::core::ffi::c_void) -> ::windows_core::HRESULT,
    pub RequestRelease: unsafe extern "system" fn(this: *mut ::core::ffi::c_void) -> ::windows_core::HRESULT,
}
#[repr(transparent)]
#[derive(::core::cmp::PartialEq, ::core::cmp::Eq, ::core::fmt::Debug, ::core::clone::Clone)]
pub struct DisplayRequest(::windows_core::IUnknown);
impl DisplayRequest {
    pub fn new() -> ::windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }
    fn IActivationFactory<R, F: FnOnce(&::windows_core::imp::IGenericFactory) -> ::windows_core::Result<R>>(callback: F) -> ::windows_core::Result<R> {
        static SHARED: ::windows_core::imp::FactoryCache<DisplayRequest, ::windows_core::imp::IGenericFactory> = ::windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
    pub fn RequestActive(&self) -> ::windows_core::Result<()> {
        let this = self;
        unsafe { (::windows_core::Interface::vtable(this).RequestActive)(::windows_core::Interface::as_raw(this)).ok() }
    }
    pub fn RequestRelease(&self) -> ::windows_core::Result<()> {
        let this = self;
        unsafe { (::windows_core::Interface::vtable(this).RequestRelease)(::windows_core::Interface::as_raw(this)).ok() }
    }
}
impl ::windows_core::RuntimeType for DisplayRequest {
    const SIGNATURE: ::windows_core::imp::ConstBuffer = ::windows_core::imp::ConstBuffer::from_slice(b"rc(Windows.System.Display.DisplayRequest;{e5732044-f49f-4b60-8dd4-5e7e3a632ac0})");
}
unsafe impl ::windows_core::Interface for DisplayRequest {
    type Vtable = IDisplayRequest_Vtbl;
}
unsafe impl ::windows_core::ComInterface for DisplayRequest {
    const IID: ::windows_core::GUID = <IDisplayRequest as ::windows_core::ComInterface>::IID;
}
impl ::windows_core::RuntimeName for DisplayRequest {
    const NAME: &'static str = "Windows.System.Display.DisplayRequest";
}
::windows_core::imp::interface_hierarchy!(DisplayRequest, ::windows_core::IUnknown, ::windows_core::IInspectable);
