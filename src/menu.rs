// Copyright 2026 variHQ OÜ
// SPDX-License-Identifier: BSD-3-Clause

//! Menu bar status item with a single Quit entry.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSVariableStatusItemLength,
};
use objc2_foundation::{MainThreadMarker, NSString, ns_string};

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "BlinkQuitTarget"]
    struct QuitTarget;

    impl QuitTarget {
        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            NSApplication::sharedApplication(mtm).terminate(None);
        }
    }
);

impl QuitTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm);
        unsafe { msg_send![this, init] }
    }
}

pub struct Menu {
    _status_item: Retained<NSStatusItem>,
    _quit_target: Retained<QuitTarget>,
}

#[must_use = "dropping the Menu removes the status-bar item"]
pub fn bind(mtm: MainThreadMarker) -> (Retained<NSApplication>, Menu) {
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let status_bar = NSStatusBar::systemStatusBar();
    let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

    if let Some(button) = status_item.button(mtm) {
        button.setTitle(ns_string!("◌"));
    }

    let quit_target = QuitTarget::new(mtm);

    let menu = NSMenu::new(mtm);
    let quit_item = NSMenuItem::new(mtm);
    quit_item.setTitle(&NSString::from_str("Quit Blink"));
    unsafe { quit_item.setTarget(Some(&quit_target)) };
    unsafe { quit_item.setAction(Some(sel!(quit:))) };
    quit_item.setKeyEquivalent(ns_string!("q"));
    menu.addItem(&quit_item);
    status_item.setMenu(Some(&menu));

    (
        app,
        Menu {
            _status_item: status_item,
            _quit_target: quit_target,
        },
    )
}
