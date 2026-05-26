// Copyright 2026 variHQ OÜ
// SPDX-License-Identifier: BSD-3-Clause

//! Blink - hold left option and drag any macOS window from anywhere inside it.

mod ax;
mod menu;
mod tap;

use std::process::ExitCode;

use objc2_foundation::MainThreadMarker;

fn main() -> ExitCode {
    let mtm = MainThreadMarker::new().expect("main() must run on the main thread");

    if !ax::ensure_trusted() {
        eprintln!(
            "Blink: Accessibility permission not granted. \
             Grant it in System Settings - Privacy & Security - Accessibility, \
             then relaunch Blink."
        );
        return ExitCode::FAILURE;
    }

    let (app, _menu) = menu::bind(mtm);
    let _tap = tap::bind();
    app.run();
    ExitCode::SUCCESS
}
