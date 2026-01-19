/// Ash doesn't support this extension yet, so we have to define it ourselves.

use ash::vk::{Bool32, StructureType};
use std::ffi::{c_void, CStr};
use std::marker::PhantomData;
use ash::vk::{TaggedStructure, ExtendsPhysicalDeviceFeatures2, ExtendsDeviceCreateInfo};

pub mod unified_image_layouts {
    use super::*;

    pub const NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_KHR_unified_image_layouts\0") };

    #[repr(C)]
    #[cfg_attr(feature = "debug", derive(Debug))]
    #[derive(Copy, Clone)]
    #[doc = "<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkPhysicalDeviceUnifiedImageLayoutsFeaturesKHR.html>"]
    #[must_use]
    pub struct PhysicalDeviceUnifiedImageLayoutsFeaturesKHR<'a> {
        pub s_type: StructureType,
        pub p_next: *mut c_void,
        pub unified_image_layouts: Bool32,
        pub unified_image_layouts_video: Bool32,
        pub _marker: PhantomData<&'a ()>,
    }
    unsafe impl Send for PhysicalDeviceUnifiedImageLayoutsFeaturesKHR<'_> {}
    unsafe impl Sync for PhysicalDeviceUnifiedImageLayoutsFeaturesKHR<'_> {}
    impl ::core::default::Default for PhysicalDeviceUnifiedImageLayoutsFeaturesKHR<'_> {
        #[inline]
        fn default() -> Self {
            Self {
                s_type: Self::STRUCTURE_TYPE,
                p_next: ::core::ptr::null_mut(),
                unified_image_layouts: Bool32::default(),
                unified_image_layouts_video: Bool32::default(),
                _marker: PhantomData,
            }
        }
    }
    unsafe impl<'a> TaggedStructure for PhysicalDeviceUnifiedImageLayoutsFeaturesKHR<'a> {
        const STRUCTURE_TYPE: StructureType = StructureType::from_raw(1000527000);
    }
    unsafe impl ExtendsPhysicalDeviceFeatures2 for PhysicalDeviceUnifiedImageLayoutsFeaturesKHR<'_> {}
    unsafe impl ExtendsDeviceCreateInfo for PhysicalDeviceUnifiedImageLayoutsFeaturesKHR<'_> {}
    impl<'a> PhysicalDeviceUnifiedImageLayoutsFeaturesKHR<'a> {
        #[inline]
        pub fn unified_image_layouts(mut self, unified_image_layouts: bool) -> Self {
            self.unified_image_layouts = unified_image_layouts.into();
            self
        }
    }
}
