#![forbid(unsafe_code)]
#![doc = "Network and netlink collector skeleton for runfossil."]

use runfossil_core::SourceSlug;

/// Returns the source slug owned by this collector crate.
#[must_use]
pub const fn source() -> SourceSlug {
    SourceSlug::Netlink
}
