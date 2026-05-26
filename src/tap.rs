// Copyright 2026 variHQ OÜ
// SPDX-License-Identifier: BSD-3-Clause

//! `CGEventTap` that turns Option-drag-anywhere into a window move.

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use crate::ax::{self, AxElement};
use core_foundation::base::TCFType;
use core_foundation::mach_port::{CFMachPort, CFMachPortRef};
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventTapProxy, CGEventType, CallbackResult,
};
use core_graphics::geometry::CGPoint;

const DRAG_MODIFIERS: CGEventFlags = CGEventFlags::CGEventFlagAlternate;
const MODIFIER_MASK: CGEventFlags = CGEventFlags::from_bits_truncate(
    CGEventFlags::CGEventFlagCommand.bits()
        | CGEventFlags::CGEventFlagShift.bits()
        | CGEventFlags::CGEventFlagControl.bits()
        | CGEventFlags::CGEventFlagAlternate.bits(),
);

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
}

struct DragState {
    window: AxElement,
    offset_x: f64,
    offset_y: f64,
}

#[must_use = "dropping the CGEventTap un-installs the event tap"]
pub fn bind() -> CGEventTap<'static> {
    let port: Rc<OnceCell<CFMachPort>> = Rc::new(OnceCell::new());
    let port_cb = Rc::clone(&port);
    let state: RefCell<Option<DragState>> = RefCell::new(None);

    let events = vec![
        CGEventType::LeftMouseDown,
        CGEventType::LeftMouseDragged,
        CGEventType::LeftMouseUp,
    ];

    let tap = unsafe {
        CGEventTap::new_unchecked(
            CGEventTapLocation::Session,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            events,
            move |proxy, etype, event| dispatch(&state, &port_cb, proxy, etype, event),
        )
    }
    .expect("CGEventTapCreate failed - Accessibility permission missing?");

    let source = tap
        .mach_port()
        .create_runloop_source(0)
        .expect("CFMachPortCreateRunLoopSource failed");
    CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopCommonModes });

    let _ = port.set(tap.mach_port().clone());
    tap.enable();
    tap
}

fn dispatch(
    state: &RefCell<Option<DragState>>,
    port: &OnceCell<CFMachPort>,
    _proxy: CGEventTapProxy,
    etype: CGEventType,
    event: &CGEvent,
) -> CallbackResult {
    match etype {
        CGEventType::LeftMouseDown => on_mouse_down(state, event),
        CGEventType::LeftMouseDragged => on_mouse_dragged(state, event),
        CGEventType::LeftMouseUp => on_mouse_up(state),
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            if let Some(p) = port.get() {
                unsafe { CGEventTapEnable(p.as_concrete_TypeRef(), true) };
            }
            *state.borrow_mut() = None;
            CallbackResult::Keep
        }
        _ => CallbackResult::Keep,
    }
}

fn modifiers_match(flags: CGEventFlags) -> bool {
    (flags & MODIFIER_MASK) == DRAG_MODIFIERS
}

fn on_mouse_down(state: &RefCell<Option<DragState>>, event: &CGEvent) -> CallbackResult {
    if !modifiers_match(event.get_flags()) {
        return CallbackResult::Keep;
    }
    let cursor = event.location();
    let Some(window) = ax::window_at(cursor) else {
        return CallbackResult::Keep;
    };
    let Ok(origin) = ax::position(&window) else {
        return CallbackResult::Keep;
    };
    *state.borrow_mut() = Some(DragState {
        window,
        offset_x: cursor.x - origin.x,
        offset_y: cursor.y - origin.y,
    });
    CallbackResult::Drop
}

fn on_mouse_dragged(state: &RefCell<Option<DragState>>, event: &CGEvent) -> CallbackResult {
    let mut guard = state.borrow_mut();
    let Some(drag) = guard.as_ref() else {
        return CallbackResult::Keep;
    };
    let cursor = event.location();
    let target = CGPoint {
        x: cursor.x - drag.offset_x,
        y: cursor.y - drag.offset_y,
    };
    if ax::set_position(&drag.window, target).is_err() {
        *guard = None;
        return CallbackResult::Keep;
    }
    CallbackResult::Drop
}

fn on_mouse_up(state: &RefCell<Option<DragState>>) -> CallbackResult {
    if state.borrow_mut().take().is_some() {
        CallbackResult::Drop
    } else {
        CallbackResult::Keep
    }
}
