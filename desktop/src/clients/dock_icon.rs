/// Shows `png` as the app's icon in the Dock; the app runs unbundled, so macOS has none of its own.
#[cfg(target_os = "macos")]
pub fn set_dock_icon(png: &[u8]) {
    use objc2::AllocAnyThread as _;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{MainThreadMarker, NSData};

    // Off the main thread AppKit is not to be touched: the generic icon stays.
    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(png);
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
        return;
    };
    let app = NSApplication::sharedApplication(main_thread);
    // SAFETY: on the main thread, with an image AppKit just read from valid bytes.
    unsafe { app.setApplicationIconImage(Some(&image)) };
}

/// Other systems take a window's icon from elsewhere.
#[cfg(not(target_os = "macos"))]
pub fn set_dock_icon(_: &[u8]) {}
