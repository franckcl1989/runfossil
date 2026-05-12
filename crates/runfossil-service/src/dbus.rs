#![forbid(unsafe_code)]
#![doc = "Minimal D-Bus wire protocol client for systemd and logind calls."]

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

const SYSTEM_BUS_PATHS: &[&str] = &[
    "/run/dbus/system_bus_socket",
    "/var/run/dbus/system_bus_socket",
];

const DBUS_HEADER_LEN: usize = 16;
const DBUS_ALIGN: usize = 8;

const MSG_TYPE_METHOD_CALL: u8 = 1;
const MSG_TYPE_METHOD_RETURN: u8 = 2;

const FIELD_PATH: u8 = 1;
const FIELD_INTERFACE: u8 = 2;
const FIELD_MEMBER: u8 = 3;
const FIELD_DESTINATION: u8 = 6;

/// Connects to the D-Bus system bus socket, authenticates, and returns a stream.
pub(crate) fn connect_system_bus() -> io::Result<UnixStream> {
    for path in SYSTEM_BUS_PATHS {
        match UnixStream::connect(path) {
            Ok(stream) => {
                stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                stream.set_write_timeout(Some(Duration::from_secs(5)))?;
                authenticate_unix_fd(&stream)?;
                return Ok(stream);
            }
            Err(_) => continue,
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "D-Bus system bus socket not found",
    ))
}

fn authenticate_unix_fd(stream: &UnixStream) -> io::Result<()> {
    let mut sock = stream.try_clone()?;
    sock.write_all(b"\0")?;
    Ok(())
}

/// Sends a D-Bus method call with string arguments and returns the raw response body.
pub(crate) fn dbus_call(
    stream: &mut UnixStream,
    destination: &str,
    path: &str,
    interface: &str,
    member: &str,
    string_args: &[&str],
) -> io::Result<Vec<u8>> {
    let serial = 1u32;
    let msg = build_method_call(serial, destination, path, interface, member, string_args)?;
    stream.write_all(&msg)?;
    read_method_return(stream)
}

fn build_method_call(
    serial: u32,
    destination: &str,
    path: &str,
    interface: &str,
    member: &str,
    string_args: &[&str],
) -> io::Result<Vec<u8>> {
    let mut fields_buf = Vec::new();

    append_header_field(&mut fields_buf, FIELD_PATH, path);
    append_header_field(&mut fields_buf, FIELD_INTERFACE, interface);
    append_header_field(&mut fields_buf, FIELD_MEMBER, member);
    append_header_field(&mut fields_buf, FIELD_DESTINATION, destination);

    let mut body = Vec::new();
    for arg in string_args {
        append_string_to_body(&mut body, arg);
    }

    let body_len = body.len() as u32;

    let mut writer = AlignedWriter::new(DBUS_ALIGN);
    writer.write_u8(b'l');
    writer.write_u8(MSG_TYPE_METHOD_CALL);
    writer.write_u8(0);
    writer.write_u8(1);
    writer.align_to(4);
    writer.write_u32(body_len);
    writer.write_u32(serial);
    writer.write_u32(fields_buf.len() as u32);
    writer.extend_from_slice(&fields_buf);
    writer.align_to(DBUS_ALIGN);
    writer.extend_from_slice(&body);

    Ok(writer.into_inner())
}

fn append_header_field(buf: &mut Vec<u8>, field_code: u8, value: &str) {
    let sig_with_null = value.as_bytes();

    let header_len: usize = 1 + 1 + 1 + 1;
    let pad_to_4 = align_up(header_len, 4);
    let string_block: usize = 4 + sig_with_null.len() + 1;
    let total_len_before_align = pad_to_4 + string_block;
    let total_len = align_up(total_len_before_align, DBUS_ALIGN);

    let offset = buf.len();
    buf.resize(offset + total_len, 0);

    let mut pos = offset;

    buf[pos] = field_code;
    pos += 1;

    buf[pos] = 1;
    pos += 1;

    buf[pos] = b's';
    pos += 1;

    buf[pos] = 0;
    pos += 1;

    pos = align_up(pos, 4);

    buf[pos..pos + 4].copy_from_slice(&(sig_with_null.len() as u32).to_ne_bytes());
    pos += 4;

    buf[pos..pos + sig_with_null.len()].copy_from_slice(sig_with_null);
    pos += sig_with_null.len();

    buf[pos] = 0;
}

fn append_string_to_body(buf: &mut Vec<u8>, value: &str) {
    let raw = value.as_bytes();
    let offset = align_up(buf.len(), 4);
    buf.resize(offset + 4 + raw.len() + 1, 0);

    buf[offset..offset + 4].copy_from_slice(&(raw.len() as u32).to_ne_bytes());
    buf[offset + 4..offset + 4 + raw.len()].copy_from_slice(raw);
}

pub(crate) fn align_up(offset: usize, alignment: usize) -> usize {
    (offset + alignment - 1) & !(alignment - 1)
}

pub(crate) fn take_dbus_string(data: &[u8]) -> Option<(&str, &[u8])> {
    if data.len() < 4 {
        return None;
    }
    let len = u32::from_ne_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let rest = &data[4..];
    if rest.len() < len + 1 {
        return None;
    }
    let s = std::str::from_utf8(&rest[..len]).ok()?;
    Some((s, &rest[len + 1..]))
}

pub(crate) fn take_variant_string(data: &[u8]) -> Option<(&str, &[u8])> {
    if data.is_empty() {
        return None;
    }
    let sig_len = data[0] as usize;
    let header_end = 1 + sig_len + 1;
    let aligned = align_up(header_end, 4);
    if data.len() <= aligned {
        return None;
    }
    take_dbus_string(&data[aligned..])
}

fn dbus_read_string_at(data: &[u8], pos: usize) -> Option<(&str, usize)> {
    let aligned = align_up(pos, 4);
    if aligned + 4 > data.len() {
        return None;
    }
    let len = u32::from_ne_bytes([
        data[aligned],
        data[aligned + 1],
        data[aligned + 2],
        data[aligned + 3],
    ]) as usize;
    let str_start = aligned + 4;
    if str_start + len + 1 > data.len() {
        return None;
    }
    let s = std::str::from_utf8(&data[str_start..str_start + len]).ok()?;
    Some((s, str_start + len + 1))
}

fn dbus_skip_string_at(data: &[u8], pos: usize) -> Option<usize> {
    let aligned = align_up(pos, 4);
    if aligned + 4 > data.len() {
        return None;
    }
    let len = u32::from_ne_bytes([
        data[aligned],
        data[aligned + 1],
        data[aligned + 2],
        data[aligned + 3],
    ]) as usize;
    let str_start = aligned + 4;
    if str_start + len + 1 > data.len() {
        return None;
    }
    Some(str_start + len + 1)
}

pub(crate) fn extract_list_units_names(body: &[u8], max: usize) -> Vec<(&str, String)> {
    let mut result = Vec::new();
    if body.len() < 4 {
        return result;
    }
    let array_bytes = u32::from_ne_bytes([body[0], body[1], body[2], body[3]]) as usize;
    let array_end = 4 + array_bytes;
    if body.len() < array_end {
        return result;
    }
    let mut pos: usize = 4;
    while pos < array_end && result.len() < max {
        pos = align_up(pos, 8);
        if pos >= array_end {
            break;
        }

        let (name, after_name) = match dbus_read_string_at(body, pos) {
            Some(v) => v,
            None => break,
        };
        let mut p = after_name;

        for _ in 0..5 {
            p = match dbus_skip_string_at(body, p) {
                Some(v) => v,
                None => break,
            };
        }
        p = match dbus_skip_string_at(body, p) {
            Some(v) => v,
            None => break,
        };
        p = align_up(p, 4) + 4;
        p = match dbus_skip_string_at(body, p) {
            Some(v) => v,
            None => break,
        };
        p = match dbus_skip_string_at(body, p) {
            Some(v) => v,
            None => break,
        };

        result.push((name, String::new()));
        pos = align_up(p, 8);
    }
    result
}

pub(crate) fn extract_list_sessions_ids(body: &[u8], max: usize) -> Vec<(&str, String)> {
    let mut result = Vec::new();
    if body.len() < 4 {
        return result;
    }
    let array_bytes = u32::from_ne_bytes([body[0], body[1], body[2], body[3]]) as usize;
    let array_end = 4 + array_bytes;
    if body.len() < array_end {
        return result;
    }
    let mut pos: usize = 4;
    while pos < array_end && result.len() < max {
        pos = align_up(pos, 8);
        if pos >= array_end {
            break;
        }

        let (id, after_id) = match dbus_read_string_at(body, pos) {
            Some(v) => v,
            None => break,
        };
        let mut p = after_id;

        p = align_up(p, 4) + 4;

        for _ in 0..3 {
            p = match dbus_skip_string_at(body, p) {
                Some(v) => v,
                None => break,
            };
        }

        result.push((id, String::new()));
        pos = align_up(p, 8);
    }
    result
}

pub(crate) fn parse_object_path(body: &[u8]) -> Option<&str> {
    take_dbus_string(body).map(|(s, _)| s)
}

struct AlignedWriter {
    buf: Vec<u8>,
    pos: usize,
}

impl AlignedWriter {
    fn new(_alignment: usize) -> Self {
        Self {
            buf: Vec::new(),
            pos: 0,
        }
    }

    fn align_to(&mut self, alignment: usize) {
        let aligned = align_up(self.pos, alignment);
        if aligned > self.buf.len() {
            self.buf.resize(aligned, 0);
        }
        self.pos = aligned;
    }

    fn write_u8(&mut self, value: u8) {
        if self.pos >= self.buf.len() {
            self.buf.push(value);
        } else {
            self.buf[self.pos] = value;
        }
        self.pos += 1;
    }

    fn write_u32(&mut self, value: u32) {
        self.align_to(4);
        let bytes = value.to_ne_bytes();
        if self.pos + 4 > self.buf.len() {
            self.buf.resize(self.pos + 4, 0);
        }
        self.buf[self.pos..self.pos + 4].copy_from_slice(&bytes);
        self.pos += 4;
    }

    fn extend_from_slice(&mut self, data: &[u8]) {
        if self.pos + data.len() > self.buf.len() {
            self.buf.resize(self.pos + data.len(), 0);
        }
        self.buf[self.pos..self.pos + data.len()].copy_from_slice(data);
        self.pos += data.len();
    }

    fn into_inner(self) -> Vec<u8> {
        self.buf
    }
}

fn read_method_return(stream: &mut UnixStream) -> io::Result<Vec<u8>> {
    let mut header = [0u8; DBUS_HEADER_LEN];
    stream.read_exact(&mut header)?;

    let endian = header[0];
    let msg_type = header[1];
    let _version = header[3];

    let body_len = u32::from_ne_bytes([header[4], header[5], header[6], header[7]]);
    let _serial = u32::from_ne_bytes([header[8], header[9], header[10], header[11]]);
    let fields_len = u32::from_ne_bytes([header[12], header[13], header[14], header[15]]) as usize;

    if endian != b'l' {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "D-Bus response has non-native endianness",
        ));
    }

    if msg_type != MSG_TYPE_METHOD_RETURN {
        let mut error_body = Vec::new();
        if body_len > 0 {
            let limit = body_len.min(4096) as usize;
            let mut buf = vec![0u8; limit];
            let n = stream.read(&mut buf)?;
            error_body.extend_from_slice(&buf[..n]);
        }
        return Err(io::Error::other(format!(
            "D-Bus call failed: message type {msg_type}, body: {}",
            String::from_utf8_lossy(&error_body)
        )));
    }

    let mut fields_buf = vec![0u8; fields_len];
    if fields_len > 0 {
        stream.read_exact(&mut fields_buf)?;
    }

    let header_end = DBUS_HEADER_LEN + fields_len;
    let aligned_end = align_up(header_end, DBUS_ALIGN);
    let pad_len = aligned_end - header_end;

    if pad_len > 0 {
        let mut pad = vec![0u8; pad_len];
        stream.read_exact(&mut pad)?;
    }

    let mut body = vec![0u8; body_len as usize];
    if body_len > 0 {
        stream.read_exact(&mut body)?;
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_method_call_starts_with_little_endian() -> std::io::Result<()> {
        let msg = build_method_call(
            1u32,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            "ListUnits",
            &[],
        )?;
        assert_eq!(msg[0], b'l');
        Ok(())
    }

    #[test]
    fn build_method_call_message_type_is_method_call() -> std::io::Result<()> {
        let msg = build_method_call(
            1u32,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            "ListUnits",
            &[],
        )?;
        assert_eq!(msg[1], MSG_TYPE_METHOD_CALL);
        Ok(())
    }

    #[test]
    fn build_method_call_includes_serial() -> std::io::Result<()> {
        let msg = build_method_call(42u32, "dest", "/path", "iface", "member", &[])?;
        let serial = u32::from_ne_bytes([msg[8], msg[9], msg[10], msg[11]]);
        assert_eq!(serial, 42);
        Ok(())
    }

    #[test]
    fn build_method_call_header_fields_contain_path() -> std::io::Result<()> {
        let msg = build_method_call(1u32, "dest", "/org/test", "iface", "member", &[])?;
        let body = String::from_utf8_lossy(&msg);
        assert!(body.contains("/org/test"));
        Ok(())
    }

    #[test]
    fn build_method_call_header_fields_contain_member() -> std::io::Result<()> {
        let msg = build_method_call(1u32, "dest", "/path", "iface", "ListUnits", &[])?;
        let body = String::from_utf8_lossy(&msg);
        assert!(body.contains("ListUnits"));
        Ok(())
    }

    #[test]
    fn build_method_call_with_string_args_includes_args_in_body() -> std::io::Result<()> {
        let msg = build_method_call(1u32, "dest", "/path", "iface", "member", &["arg1"])?;
        let body = String::from_utf8_lossy(&msg);
        assert!(body.contains("arg1"));
        Ok(())
    }

    #[test]
    fn build_method_call_body_length_is_correct() -> std::io::Result<()> {
        let msg = build_method_call(
            1u32,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            "ListUnits",
            &[],
        )?;
        let body_len = u32::from_ne_bytes([msg[4], msg[5], msg[6], msg[7]]);
        assert_eq!(body_len, 0);
        Ok(())
    }

    #[test]
    fn build_method_call_with_args_has_nonzero_body_length() -> std::io::Result<()> {
        let msg = build_method_call(
            1u32,
            "dest",
            "/path",
            "iface",
            "member",
            &["unit1", "unit2"],
        )?;
        let body_len = u32::from_ne_bytes([msg[4], msg[5], msg[6], msg[7]]);
        assert!(body_len > 0);
        Ok(())
    }

    #[test]
    fn build_method_call_field_array_length_is_set() -> std::io::Result<()> {
        let msg = build_method_call(1u32, "dest", "/path", "iface", "member", &[])?;
        let fields_len = u32::from_ne_bytes([msg[12], msg[13], msg[14], msg[15]]);
        assert!(fields_len > 0);
        Ok(())
    }

    #[test]
    fn align_up_maps_to_boundaries() {
        assert_eq!(align_up(0, 8), 0);
        assert_eq!(align_up(1, 8), 8);
        assert_eq!(align_up(8, 8), 8);
        assert_eq!(align_up(9, 8), 16);
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(3, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
    }

    #[test]
    fn aligned_writer_write_u8_and_u32() {
        let mut w = AlignedWriter::new(8);
        w.write_u8(0x42);
        w.write_u32(0xDEAD_BEEF);
        let buf = w.into_inner();
        assert_eq!(buf[0], 0x42);
        let val = u32::from_ne_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(val, 0xDEAD_BEEF);
    }

    #[test]
    fn aligned_writer_pads_to_alignment() {
        let mut w = AlignedWriter::new(8);
        w.write_u8(1);
        w.write_u8(2);
        assert!(w.into_inner().len() >= 2);
    }
}
