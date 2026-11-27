//! Local network mDNS discovery for standalone Minute of Silence hardware devices.
//!
//! Queries for `khvylyna.local` A records using both a direct UDP mDNS probe
//! (RFC 6762, unicast-response bit set) and the operating system's resolver
//! concurrently, returning a deduplicated list of discovered devices.

use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use khvylyna_core::{DeviceKind, DiscoveredDevice};
use tokio::{net::UdpSocket, time};

/// Standard IPv4 mDNS multicast group address (RFC 6762).
pub const MDNS_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);

/// Standard mDNS UDP port.
pub const MDNS_PORT: u16 = 5353;

/// Default hostname advertised by standalone Minute of Silence hardware devices.
pub const DEFAULT_HOSTNAME: &str = "khvylyna.local";

/// Default HTTP communication port exposed by the hardware device.
pub const DEFAULT_DEVICE_PORT: u16 = 80;

/// Total time window to wait for mDNS responses on the local network.
pub const DISCOVERY_TIMEOUT: Duration = Duration::from_millis(1500);

/// Maximum number of DNS compression pointer hops allowed while decoding a name.
const MAX_POINTER_HOPS: usize = 16;

/// DNS record type for IPv4 address (`A`).
const DNS_TYPE_A: u16 = 1;

/// DNS class `IN` with the mDNS unicast-response (`QU`) bit set (RFC 6762 §5.4).
const MDNS_QCLASS_IN_QU: u16 = 0x8001;

/// Discover standalone Minute of Silence devices on the local network.
///
/// Runs a raw UDP mDNS query to `224.0.0.251:5353` and the OS host resolver
/// in parallel within [`DISCOVERY_TIMEOUT`], deduplicating IPv4 addresses.
pub async fn discover_devices() -> crate::Result<Vec<DiscoveredDevice>> {
    discover_devices_with_timeout(DEFAULT_HOSTNAME, DISCOVERY_TIMEOUT).await
}

/// Discover devices advertising `hostname` within `timeout`.
pub async fn discover_devices_with_timeout(
    hostname: &str,
    timeout: Duration,
) -> crate::Result<Vec<DiscoveredDevice>> {
    let (udp_result, os_ips) = tokio::join!(
        query_mdns_udp(hostname, timeout),
        lookup_os_mdns(hostname, timeout)
    );

    let mut ips = BTreeSet::new();
    match udp_result {
        Ok(found) => ips.extend(found),
        Err(err) => log::warn!("UDP mDNS probe failed: {err}"),
    }
    ips.extend(os_ips);

    let devices = ips
        .into_iter()
        .map(|ip| DiscoveredDevice {
            hostname: hostname.to_string(),
            ip: ip.to_string(),
            port: DEFAULT_DEVICE_PORT,
            kind: DeviceKind::Hardware,
            name: None,
        })
        .collect();

    Ok(devices)
}

/// Send an mDNS A-record query over UDP and collect matching IPv4 responses until `timeout`.
async fn query_mdns_udp(hostname: &str, timeout: Duration) -> std::io::Result<Vec<Ipv4Addr>> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;
    let query = build_a_query(hostname);
    let dest = SocketAddrV4::new(MDNS_MULTICAST_ADDR, MDNS_PORT);

    socket.send_to(&query, dest).await?;

    let mut found = Vec::new();
    let mut buf = [0u8; 1500];
    let deadline = time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        match time::timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, _src))) => {
                let ips = parse_a_response(&buf[..len], hostname);
                found.extend(ips);
            }
            Ok(Err(err)) => {
                log::debug!("mDNS recv error: {err}");
                break;
            }
            Err(_) => break,
        }
    }

    Ok(found)
}

/// Attempt resolution via the operating system's mDNS/DNS resolver (Bonjour, Avahi, Windows mDNS).
async fn lookup_os_mdns(hostname: &str, timeout: Duration) -> Vec<Ipv4Addr> {
    let target = format!("{hostname}:{DEFAULT_DEVICE_PORT}");
    match time::timeout(timeout, tokio::net::lookup_host(target)).await {
        Ok(Ok(addrs)) => addrs
            .filter_map(|addr: SocketAddr| match addr.ip() {
                IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_unspecified() => Some(v4),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Build a wire-format mDNS A-record query packet for `hostname` with the QU bit set.
pub fn build_a_query(hostname: &str) -> Vec<u8> {
    let mut packet = Vec::with_capacity(64);
    // Header: ID=0, Flags=0 (standard query), QDCOUNT=1, ANCOUNT=0, NSCOUNT=0, ARCOUNT=0
    packet.extend_from_slice(&[
        0x00, 0x00, // Transaction ID (0 in mDNS)
        0x00, 0x00, // Flags: standard query
        0x00, 0x01, // QDCOUNT = 1
        0x00, 0x00, // ANCOUNT = 0
        0x00, 0x00, // NSCOUNT = 0
        0x00, 0x00, // ARCOUNT = 0
    ]);

    for label in hostname.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 {
            continue;
        }
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0x00); // Root label terminator

    packet.extend_from_slice(&DNS_TYPE_A.to_be_bytes());
    packet.extend_from_slice(&MDNS_QCLASS_IN_QU.to_be_bytes());
    packet
}

/// Parse an mDNS/DNS response packet and extract all IPv4 `A` records matching `expected_hostname`.
pub fn parse_a_response(packet: &[u8], expected_hostname: &str) -> Vec<Ipv4Addr> {
    if packet.len() < 12 {
        return Vec::new();
    }

    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    // QR bit (0x8000) must be set for a response
    if (flags & 0x8000) == 0 {
        return Vec::new();
    }

    let qdcount = u16::from_be_bytes([packet[4], packet[5]]);
    let ancount = u16::from_be_bytes([packet[6], packet[7]]);
    if ancount == 0 {
        return Vec::new();
    }

    let mut offset = 12;

    // Skip Question section
    for _ in 0..qdcount {
        let Some(next) = skip_dns_name(packet, offset) else {
            return Vec::new();
        };
        // Skip QTYPE (2) + QCLASS (2)
        if next + 4 > packet.len() {
            return Vec::new();
        }
        offset = next + 4;
    }

    let expected = expected_hostname.trim_end_matches('.');
    let mut results = Vec::new();

    // Parse Answer section
    for _ in 0..ancount {
        let Some((name, next)) = read_dns_name(packet, offset) else {
            break;
        };
        // TYPE (2) + CLASS (2) + TTL (4) + RDLENGTH (2) = 10 bytes
        if next + 10 > packet.len() {
            break;
        }

        let rtype = u16::from_be_bytes([packet[next], packet[next + 1]]);
        let rdlength = u16::from_be_bytes([packet[next + 8], packet[next + 9]]) as usize;
        let rdata_start = next + 10;
        let rdata_end = rdata_start.saturating_add(rdlength);
        if rdata_end > packet.len() {
            break;
        }

        if rtype == DNS_TYPE_A && rdlength == 4 && name.eq_ignore_ascii_case(expected) {
            let ip = Ipv4Addr::new(
                packet[rdata_start],
                packet[rdata_start + 1],
                packet[rdata_start + 2],
                packet[rdata_start + 3],
            );
            if !ip.is_unspecified() && !ip.is_loopback() {
                results.push(ip);
            }
        }

        offset = rdata_end;
    }

    results
}

/// Skip a DNS name starting at `offset`, returning the offset immediately after it.
fn skip_dns_name(packet: &[u8], mut offset: usize) -> Option<usize> {
    let mut visited = 0usize;
    while offset < packet.len() {
        let len = packet[offset];
        if len == 0 {
            return Some(offset + 1);
        }
        if (len & 0xC0) == 0xC0 {
            // Compression pointer consumes 2 bytes in the current stream and terminates the name
            return (offset + 2 <= packet.len()).then_some(offset + 2);
        }
        if (len & 0xC0) != 0 {
            return None;
        }
        offset = offset.checked_add(1 + len as usize)?;
        visited += 1;
        if visited > 128 {
            return None;
        }
    }
    None
}

/// Decode a DNS domain name starting at `offset`, following compression pointers safely.
///
/// Returns `(decoded_name, next_offset_in_stream)`.
fn read_dns_name(packet: &[u8], start: usize) -> Option<(String, usize)> {
    let mut labels = Vec::new();
    let mut offset = start;
    let mut next_offset = None;
    let mut hops = 0usize;

    loop {
        if offset >= packet.len() {
            return None;
        }
        let len = packet[offset];
        if len == 0 {
            let after = offset + 1;
            return Some((labels.join("."), next_offset.unwrap_or(after)));
        }
        if (len & 0xC0) == 0xC0 {
            if offset + 1 >= packet.len() {
                return None;
            }
            hops += 1;
            if hops > MAX_POINTER_HOPS {
                return None;
            }
            if next_offset.is_none() {
                next_offset = Some(offset + 2);
            }
            let ptr = (((len & 0x3F) as usize) << 8) | (packet[offset + 1] as usize);
            if ptr >= packet.len() {
                return None;
            }
            offset = ptr;
            continue;
        }
        if (len & 0xC0) != 0 {
            return None;
        }

        let label_start = offset + 1;
        let label_end = label_start.checked_add(len as usize)?;
        if label_end > packet.len() {
            return None;
        }
        let label = std::str::from_utf8(&packet[label_start..label_end]).ok()?;
        labels.push(label);
        offset = label_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_a_query_encodes_khvylyna_local() {
        let query = build_a_query(DEFAULT_HOSTNAME);
        let expected: Vec<u8> = vec![
            0x00, 0x00, // ID
            0x00, 0x00, // Flags
            0x00, 0x01, // QDCOUNT=1
            0x00, 0x00, // ANCOUNT=0
            0x00, 0x00, // NSCOUNT=0
            0x00, 0x00, // ARCOUNT=0
            0x08, b'k', b'h', b'v', b'y', b'l', b'y', b'n', b'a', // "khvylyna"
            0x05, b'l', b'o', b'c', b'a', b'l', // "local"
            0x00, // terminator
            0x00, 0x01, // QTYPE = A
            0x80, 0x01, // QCLASS = IN | QU
        ];
        assert_eq!(query, expected);
    }

    #[test]
    fn parse_a_response_extracts_ipv4_uncompressed() {
        let mut resp = vec![
            0x00, 0x00, // ID
            0x84, 0x00, // Flags: QR=1, AA=1
            0x00, 0x00, // QDCOUNT=0
            0x00, 0x01, // ANCOUNT=1
            0x00, 0x00, // NSCOUNT=0
            0x00, 0x00, // ARCOUNT=0
        ];
        resp.extend_from_slice(&[
            0x08, b'k', b'h', b'v', b'y', b'l', b'y', b'n', b'a', 0x05, b'l', b'o', b'c', b'a',
            b'l', 0x00,
        ]);
        resp.extend_from_slice(&[
            0x00, 0x01, // TYPE A
            0x80, 0x01, // CLASS IN + cache flush
            0x00, 0x00, 0x00, 0x78, // TTL 120
            0x00, 0x04, // RDLENGTH 4
            192, 168, 1, 105, // IPv4
        ]);

        let ips = parse_a_response(&resp, DEFAULT_HOSTNAME);
        assert_eq!(ips, vec![Ipv4Addr::new(192, 168, 1, 105)]);
    }

    #[test]
    fn parse_a_response_handles_dns_compression_pointer() {
        let mut resp = vec![
            0x00, 0x00, // ID
            0x84, 0x00, // Flags: QR=1, AA=1
            0x00, 0x01, // QDCOUNT=1
            0x00, 0x01, // ANCOUNT=1
            0x00, 0x00, // NSCOUNT=0
            0x00, 0x00, // ARCOUNT=0
        ];
        // Question at offset 12 (0x0C)
        resp.extend_from_slice(&[
            0x08, b'K', b'h', b'v', b'y', b'l', b'y', b'n', b'a', 0x05, b'l', b'o', b'c', b'a',
            b'l', 0x00, 0x00, 0x01, 0x00, 0x01,
        ]);
        // Answer pointing back to offset 0x0C
        resp.extend_from_slice(&[
            0xC0, 0x0C, // Pointer to "Khvylyna.local"
            0x00, 0x01, // TYPE A
            0x00, 0x01, // CLASS IN
            0x00, 0x00, 0x00, 0x78, // TTL
            0x00, 0x04, // RDLENGTH 4
            10, 0, 0, 42, // IPv4
        ]);

        let ips = parse_a_response(&resp, DEFAULT_HOSTNAME);
        assert_eq!(ips, vec![Ipv4Addr::new(10, 0, 0, 42)]);
    }

    #[test]
    fn parse_a_response_ignores_query_packets() {
        let query = build_a_query(DEFAULT_HOSTNAME);
        assert!(parse_a_response(&query, DEFAULT_HOSTNAME).is_empty());
    }

    #[test]
    fn parse_a_response_ignores_unrelated_hostname() {
        let mut resp = vec![
            0x00, 0x00, 0x84, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        ];
        resp.extend_from_slice(&[
            0x05, b'o', b't', b'h', b'e', b'r', 0x05, b'l', b'o', b'c', b'a', b'l', 0x00, 0x00,
            0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00, 0x04, 192, 168, 1, 99,
        ]);
        assert!(parse_a_response(&resp, DEFAULT_HOSTNAME).is_empty());
    }

    #[test]
    fn parse_a_response_handles_malformed_and_cyclic_pointers_safely() {
        // Too short
        assert!(parse_a_response(&[0x00, 0x01, 0x84], DEFAULT_HOSTNAME).is_empty());

        // Self-referencing pointer loop at offset 12
        let cyclic = vec![
            0x00, 0x00, 0x84, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x0C,
            0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00, 0x04, 192, 168, 1, 1,
        ];
        assert!(parse_a_response(&cyclic, DEFAULT_HOSTNAME).is_empty());
    }
}
