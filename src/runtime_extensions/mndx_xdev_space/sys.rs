use std::ffi::c_void;

use openxr as xr;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct XrXDevListMNDX(u64);
pub type XrXDevIdMNDX = u64;

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct CustomStructureType(i32);
impl CustomStructureType {
    pub const XR_TYPE_SYSTEM_XDEV_SPACE_PROPERTIES_MNDX: CustomStructureType = Self(1000444001);
    pub const XR_TYPE_CREATE_XDEV_LIST_INFO_MNDX: CustomStructureType = Self(1000444002);
    pub const XR_TYPE_GET_XDEV_INFO_MNDX: CustomStructureType = Self(1000444003);
    pub const XR_TYPE_XDEV_PROPERTIES_MNDX: CustomStructureType = Self(1000444004);
    pub const XR_TYPE_CREATE_XDEV_SPACE_INFO_MNDX: CustomStructureType = Self(1000444005);
}

impl Into<xr::sys::StructureType> for CustomStructureType {
    fn into(self) -> xr::sys::StructureType {
        unsafe { std::mem::transmute(self) }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct XrSystemXDevSpacePropertiesMNDX {
    ty: xr::sys::StructureType,
    next: *mut c_void,
    supports_xdev_space: xr::sys::Bool32,
}

impl Default for XrSystemXDevSpacePropertiesMNDX {
    fn default() -> Self {
        Self {
            ty: CustomStructureType::XR_TYPE_SYSTEM_XDEV_SPACE_PROPERTIES_MNDX.into(),
            next: std::ptr::null_mut(),
            supports_xdev_space: xr::sys::FALSE,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct XrCreateXDevListInfoMNDX {
    ty: xr::sys::StructureType,
    next: *mut c_void,
}

impl Default for XrCreateXDevListInfoMNDX {
    fn default() -> Self {
        Self {
            ty: CustomStructureType::XR_TYPE_CREATE_XDEV_LIST_INFO_MNDX.into(),
            next: std::ptr::null_mut(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct XrGetXDevInfoMNDX {
    ty: xr::sys::StructureType,
    next: *mut c_void,
    pub id: XrXDevIdMNDX,
}

impl Default for XrGetXDevInfoMNDX {
    fn default() -> Self {
        Self {
            ty: CustomStructureType::XR_TYPE_GET_XDEV_INFO_MNDX.into(),
            next: std::ptr::null_mut(),
            id: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct XrXDevPropertiesMNDX {
    ty: xr::sys::StructureType,
    next: *mut c_void,
    name: [i8; 256],
    serial: [i8; 256],
    can_create_space: xr::sys::Bool32,
}

impl XrXDevPropertiesMNDX {
    pub fn name(&self) -> String {
        let name = unsafe { std::ffi::CStr::from_ptr(self.name.as_ptr()) };

        name.to_string_lossy().to_string()
    }

    pub fn serial(&self) -> String {
        let serial = unsafe { std::ffi::CStr::from_ptr(self.serial.as_ptr()) };

        serial.to_string_lossy().to_string()
    }

    pub fn can_create_space(&self) -> bool {
        self.can_create_space != openxr::sys::FALSE
    }
}

impl Default for XrXDevPropertiesMNDX {
    fn default() -> Self {
        Self {
            ty: CustomStructureType::XR_TYPE_XDEV_PROPERTIES_MNDX.into(),
            next: std::ptr::null_mut(),
            name: [0; 256],
            serial: [0; 256],
            can_create_space: xr::sys::FALSE,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct XrCreateXDevSpaceInfoMNDX {
    ty: xr::sys::StructureType,
    next: *mut c_void,
    pub xdev_list: XrXDevListMNDX,
    pub id: XrXDevIdMNDX,
    offset: xr::sys::Posef,
}

impl XrCreateXDevSpaceInfoMNDX {
    pub fn new(xdev_list: XrXDevListMNDX, id: XrXDevIdMNDX, offset: xr::Posef) -> Self {
        Self {
            ty: CustomStructureType::XR_TYPE_CREATE_XDEV_SPACE_INFO_MNDX.into(),
            next: std::ptr::null_mut(),
            xdev_list,
            id,
            offset,
        }
    }
}

impl Default for XrCreateXDevSpaceInfoMNDX {
    fn default() -> Self {
        Self {
            ty: CustomStructureType::XR_TYPE_CREATE_XDEV_SPACE_INFO_MNDX.into(),
            next: std::ptr::null_mut(),
            xdev_list: XrXDevListMNDX(0),
            id: 0,
            offset: xr::sys::Posef::IDENTITY,
        }
    }
}

pub type XrCreateXdevListMndx = unsafe extern "system" fn(
    session: xr::sys::Session,
    create_info: *const XrCreateXDevListInfoMNDX,
    xdev_list: *mut XrXDevListMNDX,
) -> xr::sys::Result;

pub type XrGetXdevListGenerationNumberMndx = unsafe extern "system" fn(
    xdev_list: XrXDevListMNDX,
    out_generation: *mut u64,
) -> xr::sys::Result;

pub type XrEnumerateXdevsMndx = unsafe extern "system" fn(
    xdev_list: XrXDevListMNDX,
    count_input: u32,
    count_output: *mut u32,
    xdevs: *mut XrXDevIdMNDX,
) -> xr::sys::Result;

pub type XrGetXdevPropertiesMndx = unsafe extern "system" fn(
    xdev_list: XrXDevListMNDX,
    info: *const XrGetXDevInfoMNDX,
    properties: *mut XrXDevPropertiesMNDX,
) -> xr::sys::Result;

pub type XrDestroyXdevListMndx =
    unsafe extern "system" fn(xdev_list: XrXDevListMNDX) -> xr::sys::Result;

pub type XrCreateXdevSpaceMndx = unsafe extern "system" fn(
    session: xr::sys::Session,
    create_info: *const XrCreateXDevSpaceInfoMNDX,
    space: *mut xr::sys::Space,
) -> xr::sys::Result;

#[derive(Debug, Copy, Clone)]
pub struct XdevSpaceExtension {
    create_xdev_list_fn: Option<XrCreateXdevListMndx>,
    get_xdev_list_generation_number_fn: Option<XrGetXdevListGenerationNumberMndx>,
    enumerate_xdevs_fn: Option<XrEnumerateXdevsMndx>,
    get_xdev_properties_fn: Option<XrGetXdevPropertiesMndx>,
    destroy_xdev_list_fn: Option<XrDestroyXdevListMndx>,
    create_xdev_space_fn: Option<XrCreateXdevSpaceMndx>,
}

macro_rules! xr_bind {
    ($instance:expr, $name:expr, $function:expr) => {
        let res = xr::sys::get_instance_proc_addr(
            $instance,
            std::ffi::CStr::from_bytes_until_nul($name)
                .unwrap()
                .as_ptr(),
            std::mem::transmute(std::ptr::addr_of_mut!($function)),
        );
        if res != xr::sys::Result::SUCCESS {
            return Err(res);
        }
    };
}

macro_rules! xr_call {
    ($function:expr, $($args:expr),*) => {
        if $function.is_none() {
            return Err(xr::sys::Result::ERROR_EXTENSION_NOT_PRESENT);
        }

        let res = unsafe { $function.unwrap()($($args),*) };

        if res != xr::sys::Result::SUCCESS {
            return Err(res);
        }

        return Ok(());
    };
}

impl XdevSpaceExtension {
    pub fn new(instance: xr::sys::Instance) -> xr::Result<Self> {
        unsafe {
            let mut s = Self {
                create_xdev_list_fn: None,
                get_xdev_list_generation_number_fn: None,
                enumerate_xdevs_fn: None,
                get_xdev_properties_fn: None,
                destroy_xdev_list_fn: None,
                create_xdev_space_fn: None,
            };

            xr_bind!(instance, b"xrCreateXDevListMNDX\0", s.create_xdev_list_fn);

            xr_bind!(
                instance,
                b"xrGetXDevListGenerationNumberMNDX\0",
                s.get_xdev_list_generation_number_fn
            );

            xr_bind!(instance, b"xrEnumerateXDevsMNDX\0", s.enumerate_xdevs_fn);

            xr_bind!(
                instance,
                b"xrGetXDevPropertiesMNDX\0",
                s.get_xdev_properties_fn
            );

            xr_bind!(instance, b"xrDestroyXDevListMNDX\0", s.destroy_xdev_list_fn);

            xr_bind!(instance, b"xrCreateXDevSpaceMNDX\0", s.create_xdev_space_fn);

            Ok(s)
        }
    }

    pub fn create_xdev_list(
        &self,
        session: xr::sys::Session,
        create_info: *const XrCreateXDevListInfoMNDX,
        xdev_list: &mut XrXDevListMNDX,
    ) -> xr::Result<()> {
        xr_call!(self.create_xdev_list_fn, session, create_info, xdev_list);
    }

    pub fn enumerate_xdevs(
        &self,
        xdev_list: XrXDevListMNDX,
        count_input: u32,
        count_output: *mut u32,
        xdevs: *mut XrXDevIdMNDX,
    ) -> xr::Result<()> {
        xr_call!(
            self.enumerate_xdevs_fn,
            xdev_list,
            count_input,
            count_output,
            xdevs
        );
    }

    pub fn get_xdev_properties(
        &self,
        xdev_list: XrXDevListMNDX,
        info: *const XrGetXDevInfoMNDX,
        properties: *mut XrXDevPropertiesMNDX,
    ) -> xr::Result<()> {
        xr_call!(self.get_xdev_properties_fn, xdev_list, info, properties);
    }

    pub fn destroy_xdev_list(&self, xdev_list: XrXDevListMNDX) -> xr::Result<()> {
        xr_call!(self.destroy_xdev_list_fn, xdev_list);
    }

    pub fn create_xdev_space(
        &self,
        session: xr::sys::Session,
        create_info: *const XrCreateXDevSpaceInfoMNDX,
        space: *mut xr::sys::Space,
    ) -> xr::Result<()> {
        xr_call!(self.create_xdev_space_fn, session, create_info, space);
    }
}
