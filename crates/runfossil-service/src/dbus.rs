#![forbid(unsafe_code)]
#![doc = "Minimal D-Bus wire protocol client for systemd and logind calls."]

use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;

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

fn align_up(offset: usize, alignment: usize) -> usize {
    (offset + alignment - 1) & !(alignment - 1)
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
