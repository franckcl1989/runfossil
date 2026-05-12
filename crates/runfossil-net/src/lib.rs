#![forbid(unsafe_code)]
#![doc = "Native netlink collector for runfossil. Captures link, address, route, and neighbor state without external commands."]

use std::io::{self, Read};
use std::path::Path;
use std::process;

use socket2::{Domain, Protocol, Socket, Type};

use runfossil_core::{ManifestStatus, ObjectKind, SourceSlug};
use runfossil_store::{ManifestEntry, ObjectLimits, SnapshotStore, StoreError};

const AF_NETLINK: i32 = 16;
const NETLINK_ROUTE: i32 = 0;

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

struct NlDump {
    family: &'static str,
    msg_type: u16,
    body_bytes: usize,
}

/// Returns the source slug owned by this collector crate.
#[must_use]
pub const fn source() -> SourceSlug {
    SourceSlug::Netlink
}

/// Collects native netlink state for link, address, route, and neighbor
/// families. Each dump is written as raw binary to `raw/netlink/<family>.dump`.
pub fn collect_netlink(store: &mut SnapshotStore) -> Result<(), StoreError> {
    // P0-P2 core families: always attempted
    let dumps = [
        NlDump {
            family: "link",
            msg_type: RTM_GETLINK,
            body_bytes: 16,
        },
        NlDump {
            family: "addr",
            msg_type: RTM_GETADDR,
            body_bytes: 8,
        },
        NlDump {
            family: "route",
            msg_type: RTM_GETROUTE,
            body_bytes: 12,
        },
        NlDump {
            family: "neigh",
            msg_type: RTM_GETNEIGH,
            body_bytes: 12,
        },
    ];
    // P3 extended families: auto-probed, marked unsupported if socket is unavailable
    let extended: &[NlDump] = &[
        NlDump {
            family: "sockdiag",
            msg_type: 20,
            body_bytes: 8,
        }, // SOCK_DIAG
        NlDump {
            family: "conntrack",
            msg_type: 0,
            body_bytes: 4,
        }, // NETLINK_NETFILTER
        NlDump {
            family: "xfrm",
            msg_type: 0,
            body_bytes: 8,
        }, // XFRM state
    ];

    let mut sock = match open_netlink_socket() {
        Ok(s) => s,
        Err(error) => {
            for dump in &dumps {
                let entry = ManifestEntry::new(
                    format!("netlink.{}.dump", dump.family),
                    SourceSlug::Netlink,
                    dump.family,
                    "dump",
                    ObjectKind::NativeDump,
                    ManifestStatus::Unsupported,
                )
                .with_reason(format!("failed to open netlink socket: {error}"))
                .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0));

                store.record_object(entry)?;
            }
            return Ok(());
        }
    };

    let pid = process::id();

    // Collect core families first
    for dump in &dumps {
        let out_path = format!("raw/netlink/{}.dump", dump.family);
        match nl_dump(&mut sock, dump.msg_type, dump.body_bytes, pid) {
            Ok(data) => {
                let bytes = data.len() as u64;
                store.write_raw_file(Path::new(&out_path), &data)?;
                store.record_object(
                    ManifestEntry::new(
                        format!("netlink.{}.dump", dump.family),
                        SourceSlug::Netlink,
                        dump.family,
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
                store.record_object(
                    ManifestEntry::new(
                        format!("netlink.{}.dump", dump.family),
                        SourceSlug::Netlink,
                        dump.family,
                        "dump",
                        ObjectKind::NativeDump,
                        ManifestStatus::IoError,
                    )
                    .with_reason(format!("netlink dump failed: {error}"))
                    .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0)),
                )?;
            }
        }
    }

    // Auto-probe extended families
    for dump in extended {
        let out_path = format!("raw/netlink/{}.dump", dump.family);

        match nl_dump(&mut sock, dump.msg_type, dump.body_bytes, pid) {
            Ok(data) => {
                let bytes = data.len() as u64;
                store.write_raw_file(Path::new(&out_path), &data)?;

                let entry = ManifestEntry::new(
                    format!("netlink.{}.dump", dump.family),
                    SourceSlug::Netlink,
                    dump.family,
                    "dump",
                    ObjectKind::NativeDump,
                    ManifestStatus::Captured,
                )
                .with_path(out_path)
                .with_bytes(bytes)
                .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0));

                store.record_object(entry)?;
            }
            Err(error) => {
                let entry = ManifestEntry::new(
                    format!("netlink.{}.dump", dump.family),
                    SourceSlug::Netlink,
                    dump.family,
                    "dump",
                    ObjectKind::NativeDump,
                    ManifestStatus::IoError,
                )
                .with_reason(format!("netlink dump failed: {error}"))
                .with_limits(ObjectLimits::new(MAX_DUMP_BYTES, 2000, 1, 0));

                store.record_object(entry)?;
            }
        }
    }

    Ok(())
}

fn open_netlink_socket() -> io::Result<Socket> {
    let domain = Domain::from(AF_NETLINK);
    let socket_type = Type::DGRAM;
    let protocol = Some(Protocol::from(NETLINK_ROUTE));
    Socket::new(domain, socket_type, protocol)
}

fn nl_dump(sock: &mut Socket, msg_type: u16, body_bytes: usize, pid: u32) -> io::Result<Vec<u8>> {
    let msg = build_nl_request(msg_type, body_bytes, pid);
    sock.send(&msg)?;

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

            if nl_type == 3 {
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

fn build_nl_request(msg_type: u16, body_bytes: usize, pid: u32) -> Vec<u8> {
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
        let msg = build_nl_request(RTM_GETLINK, 16, 1);
        assert_eq!(msg.len(), 16 + 16);
    }

    #[test]
    fn build_nl_request_length_field_is_first_four_bytes() {
        let msg = build_nl_request(RTM_GETLINK, 16, 1);
        let total_len = u32::from_ne_bytes([msg[0], msg[1], msg[2], msg[3]]);
        assert_eq!(total_len as usize, msg.len());
    }

    #[test]
    fn build_nl_request_msg_type_in_header() {
        let msg = build_nl_request(RTM_GETADDR, 8, 1);
        let msg_type = u16::from_ne_bytes([msg[4], msg[5]]);
        assert_eq!(msg_type, RTM_GETADDR);
    }

    #[test]
    fn build_nl_request_flags_include_request_and_dump() {
        let msg = build_nl_request(RTM_GETROUTE, 12, 1);
        let flags = u16::from_ne_bytes([msg[6], msg[7]]);
        assert_eq!(flags, NLM_F_REQUEST | NLM_F_DUMP);
    }

    #[test]
    fn build_nl_request_includes_pid() {
        let msg = build_nl_request(RTM_GETLINK, 16, 42);
        let pid = u32::from_ne_bytes([msg[12], msg[13], msg[14], msg[15]]);
        assert_eq!(pid, 42);
    }

    #[test]
    fn build_nl_request_sequence_is_one() {
        let msg = build_nl_request(RTM_GETLINK, 16, 1);
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
            let msg = build_nl_request(*msg_type, *body_bytes, 1);
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
}
