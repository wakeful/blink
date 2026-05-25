// Copyright 2026 variHQ OÜ
// SPDX-License-Identifier: BSD-3-Clause

//! Safe wrappers over the macOS Accessibility (AX) C API.

use std::ffi::c_void;
use std::fmt;
use std::ptr;

use accessibility_sys::{
    AXError, AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue,
    AXUIElementCopyElementAtPosition, AXUIElementCreateSystemWide, AXUIElementRef,
    AXUIElementSetAttributeValue, AXValueCreate, AXValueGetValue, AXValueRef, error_string,
    kAXErrorSuccess, kAXParentAttribute, kAXPositionAttribute, kAXRoleAttribute,
    kAXTrustedCheckOptionPrompt, kAXValueTypeCGPoint, kAXWindowRole,
};
use core_foundation::base::{CFRelease, CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::CGPoint;

const MAX_PARENT_DEPTH: usize = 64;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct AxError(pub AXError);

impl fmt::Debug for AxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AxError({} / {})", self.0, error_string(self.0))
    }
}

impl fmt::Display for AxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(error_string(self.0))
    }
}

impl std::error::Error for AxError {}

pub type Result<T> = std::result::Result<T, AxError>;
pub struct AxElement(AXUIElementRef);

impl AxElement {
    unsafe fn from_owned(raw: AXUIElementRef) -> Self {
        debug_assert!(!raw.is_null());
        Self(raw)
    }

    const fn as_raw(&self) -> AXUIElementRef {
        self.0
    }
}

impl Drop for AxElement {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0 as CFTypeRef) };
    }
}

pub fn ensure_trusted() -> bool {
    let key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let val = CFBoolean::true_value();
    let dict = CFDictionary::from_CFType_pairs(&[(key, val.as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) }
}

pub fn window_at(point: CGPoint) -> Option<AxElement> {
    let system = system_wide()?;
    let hit = element_at(&system, point)?;

    std::iter::successors(Some(hit), parent)
        .take(MAX_PARENT_DEPTH)
        .find(|e| role_is(e, kAXWindowRole))
}

pub fn position(window: &AxElement) -> Result<CGPoint> {
    let value = copy_attr(window, kAXPositionAttribute)?;
    let mut point = CGPoint { x: 0.0, y: 0.0 };
    let ok = unsafe {
        AXValueGetValue(
            value.as_CFTypeRef() as AXValueRef,
            kAXValueTypeCGPoint,
            ptr::from_mut(&mut point).cast::<c_void>(),
        )
    };
    if ok {
        Ok(point)
    } else {
        Err(AxError(accessibility_sys::kAXErrorIllegalArgument))
    }
}

pub fn set_position(window: &AxElement, point: CGPoint) -> Result<()> {
    let value =
        unsafe { AXValueCreate(kAXValueTypeCGPoint, ptr::from_ref(&point).cast::<c_void>()) };
    if value.is_null() {
        return Err(AxError(accessibility_sys::kAXErrorFailure));
    }
    let key = CFString::new(kAXPositionAttribute);
    let err = unsafe {
        AXUIElementSetAttributeValue(
            window.as_raw(),
            key.as_concrete_TypeRef(),
            value as CFTypeRef,
        )
    };
    unsafe { CFRelease(value as CFTypeRef) };
    if err == kAXErrorSuccess {
        Ok(())
    } else {
        Err(AxError(err))
    }
}

fn system_wide() -> Option<AxElement> {
    let raw = unsafe { AXUIElementCreateSystemWide() };
    if raw.is_null() {
        None
    } else {
        Some(unsafe { AxElement::from_owned(raw) })
    }
}

fn element_at(root: &AxElement, point: CGPoint) -> Option<AxElement> {
    let mut hit: AXUIElementRef = ptr::null_mut();
    #[allow(clippy::cast_possible_truncation)]
    let (x, y) = (point.x as f32, point.y as f32);
    let err = unsafe { AXUIElementCopyElementAtPosition(root.as_raw(), x, y, &raw mut hit) };
    if err != kAXErrorSuccess || hit.is_null() {
        return None;
    }
    Some(unsafe { AxElement::from_owned(hit) })
}

fn parent(element: &AxElement) -> Option<AxElement> {
    let value = copy_attr(element, kAXParentAttribute).ok()?;
    let raw = value.as_CFTypeRef() as AXUIElementRef;
    std::mem::forget(value);
    Some(unsafe { AxElement::from_owned(raw) })
}

fn role_is(element: &AxElement, expected: &str) -> bool {
    let Ok(value) = copy_attr(element, kAXRoleAttribute) else {
        return false;
    };
    let raw = value.as_CFTypeRef() as CFStringRef;
    std::mem::forget(value);
    let cf = unsafe { CFString::wrap_under_create_rule(raw) };
    cf == expected
}

fn copy_attr(element: &AxElement, attr: &str) -> Result<CFType> {
    let key = CFString::new(attr);
    let mut out: CFTypeRef = ptr::null();
    let err = unsafe {
        AXUIElementCopyAttributeValue(element.as_raw(), key.as_concrete_TypeRef(), &raw mut out)
    };
    if err != kAXErrorSuccess {
        return Err(AxError(err));
    }
    if out.is_null() {
        return Err(AxError(accessibility_sys::kAXErrorNoValue));
    }
    Ok(unsafe { CFType::wrap_under_create_rule(out) })
}
