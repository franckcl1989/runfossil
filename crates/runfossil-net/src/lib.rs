#![forbid(unsafe_code)]
#![doc = "Native netlink collector for runfossil. Captures link, address, route, neighbor, sockdiag, conntrack, and xfrm state without external commands."]

use std::io::{self, Read};
use std::path::Path;
use std::process;
use std::time::Duration;

use socket2::{Domain, Protocol, Socket, Type};

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

const AF_NETLINK: i32 = 16;
const NETLINK_ROUTE: i32 = 0;
const NETLINK_SOCK_DIAG: i32 = 4;
const NETLINK_NETFILTER: i32 = 12;
const NETLINK_XFRM: i32 = 6;

const NLM_F_REQUEST: u16 = 0x01;
const NLM_F_DUMP: u16 = 0x300;
const NLM_F_MULTI: u16 = 0x02;
const NLMSG_DONE: u16 = 3;

const RTM_GETLINK: u16 = 18;
const RTM_GETADDR: u16 = 22;
const RTM_GETROUTE: u16 = 26;
const RTM_GETNEIGH: u16 = 30;

const NLMSG_HDRLEN: u32 = 16;
const MAX_DUMP_BYTES: u64 = 4_194_304;

const AF_INET: u8 = 2;
const AF_UNSPEC: u8 = 0;

/// Returns the source slug owned by this collector crate.
#[must_use]
pub const fn source() -> SourceSlug {
    SourceSlug::Netlink
}

/// Collects native netlink state for link, address, route, neighbor, and
/// conditional extended families. Core families (P0-P2) are always attempted
/// on NETLINK_ROUTE. Extended families (P3) open their own protocol sockets
/// and report unsupported when the kernel does not provide the family.
pub fn collect_netlink(store: &SnapshotStore) -> Result<(), StoreError> {
    let pid = process::id();

    collect_route_families(store, pid)?;
    collect_extended_families(store, pid)?;

    Ok(())
}

fn record_unsupported(store: &SnapshotStore, family: &str, reason: &str) -> Result<(), StoreError> {
    store.record_object(
        ManifestEntry::new(
            format!("netlink.{family}.dump"),
            SourceSlug::Netlink,
            family,
            "dump",
            ObjectKind::NativeDump,
            ManifestStatus::Unsupported,
        )
        .with_reason(reason.to_string())
        .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0)),
    )
}

fn record_io_error(store: &SnapshotStore, family: &str, reason: &str) -> Result<(), StoreError> {
    store.record_object(
        ManifestEntry::new(
            format!("netlink.{family}.dump"),
            SourceSlug::Netlink,
            family,
            "dump",
            ObjectKind::NativeDump,
            ManifestStatus::IoError,
        )
        .with_reason(reason.to_string())
        .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0)),
    )
}

fn collect_route_families(store: &SnapshotStore, pid: u32) -> Result<(), StoreError> {
    let families = [
        ("link", RTM_GETLINK, 16usize),
        ("addr", RTM_GETADDR, 8),
        ("route", RTM_GETROUTE, 12),
        ("neigh", RTM_GETNEIGH, 12),
    ];

    let sock = match open_netlink_socket(NETLINK_ROUTE) {
        Ok(s) => s,
        Err(error) => {
            for &(family, _, _) in &families {
                record_unsupported(
                    store,
                    family,
                    &format!("failed to open NETLINK_ROUTE socket: {error}"),
                )?;
            }
            return Ok(());
        }
    };

    dump_family_set(store, sock, &families, pid)?;

    Ok(())
}

fn dump_family_set(
    store: &SnapshotStore,
    mut sock: Socket,
    families: &[(&str, u16, usize)],
    pid: u32,
) -> Result<(), StoreError> {
    for &(family, msg_type, body_bytes) in families {
        let out_path = format!("raw/netlink/{family}.dump");
        match route_dump(&mut sock, msg_type, body_bytes, pid) {
            Ok(data) => {
                let bytes = data.len() as u64;
                store.write_raw_file(Path::new(&out_path), &data)?;
                store.record_object(
                    ManifestEntry::new(
                        format!("netlink.{family}.dump"),
                        SourceSlug::Netlink,
                        family,
                        "dump",
                        ObjectKind::NativeDump,
                        ManifestStatus::Captured,
                    )
                    .with_path(out_path)
                    .with_bytes(bytes)
                    .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0)),
                )?;
            }
            Err(error) => {
                record_io_error(store, family, &format!("dump failed: {error}"))?;
            }
        }
    }
    Ok(())
}

fn collect_extended_families(store: &SnapshotStore, pid: u32) -> Result<(), StoreError> {
    try_sockdiag(store, pid)?;
    try_conntrack(store, pid)?;
    try_xfrm(store, pid)?;
    Ok(())
}

// ── SOCK_DIAG (NETLINK_SOCK_DIAG) ─────────────────────────────────────────

fn try_sockdiag(store: &SnapshotStore, pid: u32) -> Result<(), StoreError> {
    let mut sock = match open_netlink_socket(NETLINK_SOCK_DIAG) {
        Ok(s) => s,
        Err(error) => {
            return record_unsupported(
                store,
                "sockdiag",
                &format!("NETLINK_SOCK_DIAG not available: {error}"),
            );
        }
    };

    let family = "sockdiag";
    let out_path = format!("raw/netlink/{family}.dump");

    match sockdiag_dump(&mut sock, pid) {
        Ok(data) => {
            let bytes = data.len() as u64;
            store.write_raw_file(Path::new(&out_path), &data)?;
            store.record_object(
                ManifestEntry::new(
                    format!("netlink.{family}.dump"),
                    SourceSlug::Netlink,
                    family,
                    "dump",
                    ObjectKind::NativeDump,
                    ManifestStatus::Captured,
                )
                .with_path(out_path)
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0)),
            )?;
        }
        Err(error) => {
            record_io_error(store, family, &format!("sockdiag dump failed: {error}"))?;
        }
    }
    Ok(())
}

fn build_sockdiag_request(pid: u32) -> Vec<u8> {
    let msg_type: u16 = 20;
    let body = sockdiag_body();
    let total_len = NLMSG_HDRLEN as usize + body.len();
    let mut msg = vec![0u8; total_len];

    msg[0..4].copy_from_slice(&(total_len as u32).to_ne_bytes());
    msg[4..6].copy_from_slice(&msg_type.to_ne_bytes());
    msg[6..8].copy_from_slice(&(NLM_F_REQUEST | NLM_F_DUMP).to_ne_bytes());
    msg[8..12].copy_from_slice(&1u32.to_ne_bytes());
    msg[12..16].copy_from_slice(&pid.to_ne_bytes());
    msg[NLMSG_HDRLEN as usize..].copy_from_slice(&body);

    msg
}

fn sockdiag_body() -> Vec<u8> {
    let sdiag_states: u32 = 0xFFFF_FFFF;
    let idiag_ino: u32 = 0;
    let idiag_cookie: [u64; 2] = [0, 0];

    let mut body = vec![AF_INET, 0u8, 0u8, 0u8];
    body.extend_from_slice(&sdiag_states.to_ne_bytes());
    body.extend_from_slice(&idiag_ino.to_ne_bytes());
    body.extend_from_slice(&idiag_cookie[0].to_ne_bytes());
    body.extend_from_slice(&idiag_cookie[1].to_ne_bytes());

    body
}

fn sockdiag_dump(sock: &mut Socket, pid: u32) -> io::Result<Vec<u8>> {
    let msg = build_sockdiag_request(pid);
    sock.send(&msg)?;
    read_netlink_response(sock)
}

// ── CONNTRACK (NETLINK_NETFILTER) ─────────────────────────────────────────

fn try_conntrack(store: &SnapshotStore, pid: u32) -> Result<(), StoreError> {
    let mut sock = match open_netlink_socket(NETLINK_NETFILTER) {
        Ok(s) => s,
        Err(error) => {
            return record_unsupported(
                store,
                "conntrack",
                &format!("NETLINK_NETFILTER not available: {error}"),
            );
        }
    };

    let family = "conntrack";
    let out_path = format!("raw/netlink/{family}.dump");

    match conntrack_dump(&mut sock, pid) {
        Ok(data) => {
            let bytes = data.len() as u64;
            store.write_raw_file(Path::new(&out_path), &data)?;
            store.record_object(
                ManifestEntry::new(
                    format!("netlink.{family}.dump"),
                    SourceSlug::Netlink,
                    family,
                    "dump",
                    ObjectKind::NativeDump,
                    ManifestStatus::Captured,
                )
                .with_path(out_path)
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0)),
            )?;
        }
        Err(error) => {
            record_io_error(store, family, &format!("conntrack dump failed: {error}"))?;
        }
    }
    Ok(())
}

fn build_conntrack_request(pid: u32) -> Vec<u8> {
    let subsys_id: u8 = 1;
    let cmd: u8 = 1;
    let msg_type: u16 = ((subsys_id as u16) << 8) | (cmd as u16);

    let body = conntrack_body();
    let total_len = NLMSG_HDRLEN as usize + body.len();
    let mut msg = vec![0u8; total_len];

    msg[0..4].copy_from_slice(&(total_len as u32).to_ne_bytes());
    msg[4..6].copy_from_slice(&msg_type.to_ne_bytes());
    msg[6..8].copy_from_slice(&(NLM_F_REQUEST | NLM_F_DUMP).to_ne_bytes());
    msg[8..12].copy_from_slice(&1u32.to_ne_bytes());
    msg[12..16].copy_from_slice(&pid.to_ne_bytes());
    msg[NLMSG_HDRLEN as usize..].copy_from_slice(&body);

    msg
}

fn conntrack_body() -> Vec<u8> {
    let res_id: u16 = 0;

    let mut body = vec![AF_INET, 0u8];
    body.extend_from_slice(&res_id.to_be_bytes());

    body
}

fn conntrack_dump(sock: &mut Socket, pid: u32) -> io::Result<Vec<u8>> {
    let msg = build_conntrack_request(pid);
    sock.send(&msg)?;
    read_netlink_response(sock)
}

// ── XFRM (NETLINK_XFRM) ───────────────────────────────────────────────────

fn try_xfrm(store: &SnapshotStore, pid: u32) -> Result<(), StoreError> {
    let mut sock = match open_netlink_socket(NETLINK_XFRM) {
        Ok(s) => s,
        Err(error) => {
            return record_unsupported(
                store,
                "xfrm",
                &format!("NETLINK_XFRM not available: {error}"),
            );
        }
    };

    let msg_type: u16 = 0;

    let family = "xfrm";
    let out_path = format!("raw/netlink/{family}.dump");

    match route_dump(&mut sock, msg_type, xfrm_body().len(), pid) {
        Ok(data) => {
            let bytes = data.len() as u64;
            store.write_raw_file(Path::new(&out_path), &data)?;
            store.record_object(
                ManifestEntry::new(
                    format!("netlink.{family}.dump"),
                    SourceSlug::Netlink,
                    family,
                    "dump",
                    ObjectKind::NativeDump,
                    ManifestStatus::Captured,
                )
                .with_path(out_path)
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0)),
            )?;
        }
        Err(error) => {
            record_io_error(store, family, &format!("xfrm dump failed: {error}"))?;
        }
    }
    Ok(())
}

fn xfrm_body() -> Vec<u8> {
    let xfrm_family: u8 = AF_UNSPEC;
    let xfrm_pad1: u8 = 0;
    let xfrm_pad2: u16 = 0;

    let mut body = Vec::with_capacity(4);
    body.push(xfrm_family);
    body.push(xfrm_pad1);
    body.extend_from_slice(&xfrm_pad2.to_ne_bytes());

    body
}

// ── Shared netlink helpers ───────────────────────────────────────────────

fn open_netlink_socket(protocol: i32) -> io::Result<Socket> {
    let domain = Domain::from(AF_NETLINK);
    let socket_type = Type::DGRAM;
    let sock = Socket::new(domain, socket_type, Some(Protocol::from(protocol)))?;
    sock.set_read_timeout(Some(Duration::from_secs(5)))?;
    Ok(sock)
}

fn read_netlink_response(sock: &mut Socket) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; 65536];
    let mut all_data = Vec::new();
    let mut done = false;

    while !done && (all_data.len() as u64) < MAX_DUMP_BYTES {
        let n = sock.read(&mut buf)?;
        if n == 0 {
            break;
        }

        let mut offset = 0;
        while offset + NLMSG_HDRLEN as usize <= n {
            let len = u32::from_ne_bytes([
                buf[offset],
                buf[offset + 1],
                buf[offset + 2],
                buf[offset + 3],
            ]) as usize;

            if len < NLMSG_HDRLEN as usize || offset + len > n {
                break;
            }

            let nl_type = u16::from_ne_bytes([buf[offset + 4], buf[offset + 5]]);
            let nl_flags = u16::from_ne_bytes([buf[offset + 6], buf[offset + 7]]);

            if nl_type == NLMSG_DONE {
                done = true;
                break;
            }

            all_data.extend_from_slice(&buf[offset..offset + len]);

            if nl_flags & NLM_F_MULTI == 0 {
                done = true;
            }

            offset += len;
        }

        if offset < n {
            let remaining = n - offset;
            all_data.extend_from_slice(&buf[offset..offset + remaining]);
        }
    }

    Ok(all_data)
}

fn route_dump(
    sock: &mut Socket,
    msg_type: u16,
    body_bytes: usize,
    pid: u32,
) -> io::Result<Vec<u8>> {
    let msg = build_rtnetlink_request(msg_type, body_bytes, pid);
    sock.send(&msg)?;
    read_netlink_response(sock)
}

fn build_rtnetlink_request(msg_type: u16, body_bytes: usize, pid: u32) -> Vec<u8> {
    let total_len = NLMSG_HDRLEN as usize + body_bytes;
    let mut msg = vec![0u8; total_len];

    msg[0..4].copy_from_slice(&(total_len as u32).to_ne_bytes());
    msg[4..6].copy_from_slice(&msg_type.to_ne_bytes());
    msg[6..8].copy_from_slice(&(NLM_F_REQUEST | NLM_F_DUMP).to_ne_bytes());
    msg[8..12].copy_from_slice(&1u32.to_ne_bytes());
    msg[12..16].copy_from_slice(&pid.to_ne_bytes());

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_nl_request_correct_total_length() {
        let msg = build_rtnetlink_request(RTM_GETLINK, 16, 1);
        assert_eq!(msg.len(), 16 + 16);
    }

    #[test]
    fn build_nl_request_length_field_is_first_four_bytes() {
        let msg = build_rtnetlink_request(RTM_GETLINK, 16, 1);
        let total_len = u32::from_ne_bytes([msg[0], msg[1], msg[2], msg[3]]);
        assert_eq!(total_len as usize, msg.len());
    }

    #[test]
    fn build_nl_request_msg_type_in_header() {
        let msg = build_rtnetlink_request(RTM_GETADDR, 8, 1);
        let msg_type = u16::from_ne_bytes([msg[4], msg[5]]);
        assert_eq!(msg_type, RTM_GETADDR);
    }

    #[test]
    fn build_nl_request_flags_include_request_and_dump() {
        let msg = build_rtnetlink_request(RTM_GETROUTE, 12, 1);
        let flags = u16::from_ne_bytes([msg[6], msg[7]]);
        assert_eq!(flags, NLM_F_REQUEST | NLM_F_DUMP);
    }

    #[test]
    fn build_nl_request_includes_pid() {
        let msg = build_rtnetlink_request(RTM_GETLINK, 16, 42);
        let pid = u32::from_ne_bytes([msg[12], msg[13], msg[14], msg[15]]);
        assert_eq!(pid, 42);
    }

    #[test]
    fn build_nl_request_sequence_is_one() {
        let msg = build_rtnetlink_request(RTM_GETLINK, 16, 1);
        let seq = u32::from_ne_bytes([msg[8], msg[9], msg[10], msg[11]]);
        assert_eq!(seq, 1);
    }

    #[test]
    fn build_nl_request_all_four_msg_types() {
        for (msg_type, body_bytes) in &[
            (RTM_GETLINK, 16),
            (RTM_GETADDR, 8),
            (RTM_GETROUTE, 12),
            (RTM_GETNEIGH, 12),
        ] {
            let msg = build_rtnetlink_request(*msg_type, *body_bytes, 1);
            assert_eq!(msg.len(), 16 + body_bytes);
            let parsed_type = u16::from_ne_bytes([msg[4], msg[5]]);
            assert_eq!(parsed_type, *msg_type);
        }
    }

    #[test]
    fn nlmsg_done_and_multi_flags_are_distinct() {
        assert_ne!(NLMSG_DONE, NLM_F_MULTI);
        assert_ne!(NLMSG_DONE, 0);
    }

    #[test]
    fn build_sockdiag_request_has_correct_length() {
        let body = sockdiag_body();
        let msg = build_sockdiag_request(1);
        assert_eq!(msg.len(), NLMSG_HDRLEN as usize + body.len());
    }

    #[test]
    fn build_conntrack_request_has_correct_length() {
        let body = conntrack_body();
        let msg = build_conntrack_request(1);
        assert_eq!(msg.len(), NLMSG_HDRLEN as usize + body.len());
    }

    #[test]
    fn sockdiag_body_starts_with_af_inet() {
        let body = sockdiag_body();
        assert_eq!(body[0], AF_INET);
    }

    #[test]
    fn conntrack_body_starts_with_af_inet() {
        let body = conntrack_body();
        assert_eq!(body[0], AF_INET);
    }

    #[test]
    fn xfrm_body_starts_with_af_unspec() {
        let body = xfrm_body();
        assert_eq!(body[0], AF_UNSPEC);
    }
}
