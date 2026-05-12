#![doc = "Minimal signal handling for runfossil. Uses libc FFI for SIGINT/SIGTERM.\n\nThis crate intentionally does not use `#![forbid(unsafe_code)]` because\nLinux signal handling requires OS kernel interfaces that are inherently unsafe.\nAll unsafe code is confined to sigaction() and signal-safe operations.\nThe public API is entirely safe."]

use std::sync::atomic::{AtomicBool, Ordering};

static SIGNALLED: AtomicBool = AtomicBool::new(false);

extern "C" fn handler(_sig: libc::c_int) {
    SIGNALLED.store(true, Ordering::Release);
}

/// Install signal handlers for SIGINT and SIGTERM.
/// After calling this, `was_signalled()` will return true if a signal was received.
///
/// # Safety
///
/// This function is safe to call. The internal unsafe FFI is properly isolated.
pub fn install_handlers() {
    unsafe {
        libc::signal(libc::SIGINT, handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, handler as *const () as libc::sighandler_t);
    }
}

/// Check whether a signal (SIGINT or SIGTERM) has been received since
/// `install_handlers()` was called.
#[must_use]
pub fn was_signalled() -> bool {
    SIGNALLED.load(Ordering::Acquire)
}

/// Reset the signal flag. Useful for testing.
pub fn reset() {
    SIGNALLED.store(false, Ordering::Release);
}
