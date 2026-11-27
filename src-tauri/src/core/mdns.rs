//! Local network mDNS / DNS-SD discovery for Minute of Silence devices.
//!
//! Discovers both:
//! - DNS-SD peers advertising `_khvylyna._tcp.local` (desktop apps, Android
//!   devices, and upgraded hardware devices) with PTR/SRV/TXT/A records.
//! - Legacy standalone ESP32 hardware devices advertising `khvylyna.local`
//!   via plain mDNS A-record responses and OS mDNS resolution.

use std::{
    collections::{BTreeMap, HashMap},
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use khvylyna_core::{DeviceKind, DiscoveredDevice};
use tokio::{net::UdpSocket, time};

/// Standard IPv4 mDNS multicast group address (RFC 6762).
pub const MDNS_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);

/// Standard mDNS UDP port.
pub const MDNS_PORT: u16 = 5353;

/// DNS-SD service type advertised by Minute of Silence peers.
pub const SERVICE_TYPE: &str = "_khvylyna._tcp.local";

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

/// DNS record type for pointer (`PTR`).
const DNS_TYPE_PTR: u16 = 12;

/// DNS record type for text attributes (`TXT`).
const DNS_TYPE_TXT: u16 = 16;

/// DNS record type for service location (`SRV`).
const DNS_TYPE_SRV: u16 = 33;

/// DNS class `IN` with the mDNS unicast-response (`QU`) bit set (RFC 6762 §5.4).
const MDNS_QCLASS_IN_QU: u16 = 0x8001;

/// Discover Minute of Silence devices (hardware, desktop, Android) on the local network.
///
/// Runs a raw UDP mDNS probe (both `_khvylyna._tcp.local` PTR and `khvylyna.local` A)
/// and the OS host resolver in parallel within [`DISCOVERY_TIMEOUT`], deduplicating
/// results by `(ip, port)` and preferring richer DNS-SD metadata when both match.
pub async fn discover_devices() -> crate::Result<Vec<DiscoveredDevice>> {
    discover_devices_with_timeout(DEFAULT_HOSTNAME, DISCOVERY_TIMEOUT).await
}

/// Discover devices advertising [`SERVICE_TYPE`] or `hostname` within `timeout`.
pub async fn discover_devices_with_timeout(
    hostname: &str,
    timeout: Duration,
) -> crate::Result<Vec<DiscoveredDevice>> {
    let (udp_result, os_ips) = tokio::join!(
        query_mdns_udp(hostname, timeout),
        lookup_os_mdns(hostname, timeout)
    );

    let mut by_endpoint: BTreeMap<(Ipv4Addr, u16), DiscoveredDevice> = BTreeMap::new();

    match udp_result {
        Ok(found) => {
            for device in found {
                if let Ok(ip) = device.ip.parse::<Ipv4Addr>() {
                    merge_device(&mut by_endpoint, ip, device);
                }
            }
        }
        Err(err) => log::warn!("UDP mDNS probe failed: {err}"),
    }

    for ip in os_ips {
        let legacy = DiscoveredDevice {
            hostname: hostname.to_string(),
            ip: ip.to_string(),
            port: DEFAULT_DEVICE_PORT,
            kind: DeviceKind::Hardware,
            name: None,
        };
        merge_device(&mut by_endpoint, ip, legacy);
    }

    Ok(by_endpoint.into_values().collect())
}

/// Insert `candidate` into `map`, preferring entries with richer DNS-SD metadata.
fn merge_device(
    map: &mut BTreeMap<(Ipv4Addr, u16), DiscoveredDevice>,
    ip: Ipv4Addr,
    candidate: DiscoveredDevice,
) {
    let key = (ip, candidate.port);
    match map.get(&key) {
        Some(existing) if device_richness(existing) >= device_richness(&candidate) => {}
        _ => {
            map.insert(key, candidate);
        }
    }
}

fn device_richness(device: &DiscoveredDevice) -> u8 {
    let mut score = 0;
    if device.kind != DeviceKind::Unknown {
        score += 1;
    }
    if device.name.is_some() {
        score += 2;
    }
    score
}

/// Send DNS-SD PTR and legacy A-record mDNS queries over UDP and collect matching devices.
async fn query_mdns_udp(
    hostname: &str,
    timeout: Duration,
) -> std::io::Result<Vec<DiscoveredDevice>> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;
    let dest = SocketAddrV4::new(MDNS_MULTICAST_ADDR, MDNS_PORT);

    let ptr_query = build_ptr_query(SERVICE_TYPE);
    let a_query = build_a_query(hostname);

    socket.send_to(&ptr_query, dest).await?;
    socket.send_to(&a_query, dest).await?;

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
                let packet = &buf[..len];
                found.extend(parse_dnssd_response(packet, SERVICE_TYPE));
                for ip in parse_a_response(packet, hostname) {
                    found.push(DiscoveredDevice {
                        hostname: hostname.to_string(),
                        ip: ip.to_string(),
                        port: DEFAULT_DEVICE_PORT,
                        kind: DeviceKind::Hardware,
                        name: None,
                    });
                }
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
    build_query(hostname, DNS_TYPE_A)
}

/// Build a wire-format mDNS PTR-record query packet for `service_type` with the QU bit set.
pub fn build_ptr_query(service_type: &str) -> Vec<u8> {
    build_query(service_type, DNS_TYPE_PTR)
}

fn build_query(name: &str, qtype: u16) -> Vec<u8> {
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

    encode_dns_name(&mut packet, name);
    packet.extend_from_slice(&qtype.to_be_bytes());
    packet.extend_from_slice(&MDNS_QCLASS_IN_QU.to_be_bytes());
    packet
}

/// Encode a dotted domain name into DNS wire-format length-prefixed labels terminated by `0x00`.
fn encode_dns_name(buf: &mut Vec<u8>, name: &str) {
    for label in name.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 {
            continue;
        }
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0x00);
}

/// Parse an mDNS/DNS response packet and extract all IPv4 `A` records matching `expected_hostname`.
pub fn parse_a_response(packet: &[u8], expected_hostname: &str) -> Vec<Ipv4Addr> {
    let Some(records) = parse_response_records(packet) else {
        return Vec::new();
    };
    let expected = expected_hostname.trim_end_matches('.');
    let mut results = Vec::new();

    for rr in records {
        if rr.rtype == DNS_TYPE_A && rr.rdata.len() == 4 && rr.name.eq_ignore_ascii_case(expected) {
            let ip = Ipv4Addr::new(rr.rdata[0], rr.rdata[1], rr.rdata[2], rr.rdata[3]);
            if !ip.is_unspecified() && !ip.is_loopback() {
                results.push(ip);
            }
        }
    }

    results
}

/// Parse a DNS-SD mDNS response packet and extract all resolved services matching `service_type`.
///
/// Correlates PTR, SRV, TXT, and A records across the Answer and Additional sections.
pub fn parse_dnssd_response(packet: &[u8], service_type: &str) -> Vec<DiscoveredDevice> {
    let Some(records) = parse_response_records(packet) else {
        return Vec::new();
    };

    let expected_service = service_type.trim_end_matches('.').to_ascii_lowercase();
    let mut instances: Vec<String> = Vec::new();
    let mut srvs: HashMap<String, (u16, String)> = HashMap::new();
    let mut txts: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut a_records: HashMap<String, Ipv4Addr> = HashMap::new();

    for rr in &records {
        let name_lower = rr.name.to_ascii_lowercase();
        match rr.rtype {
            DNS_TYPE_PTR if name_lower == expected_service => {
                if let Some((instance, _)) = read_dns_name(packet, rr.rdata_offset) {
                    let instance_lower = instance.to_ascii_lowercase();
                    if !instances.iter().any(|existing| existing == &instance_lower) {
                        instances.push(instance_lower);
                    }
                }
            }
            DNS_TYPE_SRV => {
                // SRV RDATA: priority (2) + weight (2) + port (2) + target (DNS name)
                if rr.rdata.len() >= 7 {
                    let port = u16::from_be_bytes([rr.rdata[4], rr.rdata[5]]);
                    if let Some((target, _)) = read_dns_name(packet, rr.rdata_offset + 6) {
                        srvs.insert(name_lower, (port, target));
                    }
                }
            }
            DNS_TYPE_TXT => {
                let attrs = parse_txt_rdata(rr.rdata);
                if !attrs.is_empty() {
                    txts.insert(name_lower, attrs);
                }
            }
            DNS_TYPE_A if rr.rdata.len() == 4 => {
                let ip = Ipv4Addr::new(rr.rdata[0], rr.rdata[1], rr.rdata[2], rr.rdata[3]);
                if !ip.is_unspecified() && !ip.is_loopback() {
                    a_records.insert(name_lower, ip);
                }
            }
            _ => {}
        }
    }

    // Also include SRV records whose instance name ends with `.{service_type}` even if PTR was omitted.
    let suffix = format!(".{expected_service}");
    for instance_name in srvs.keys() {
        if instance_name.ends_with(&suffix)
            && !instances.iter().any(|existing| existing == instance_name)
        {
            instances.push(instance_name.clone());
        }
    }

    let mut devices = Vec::new();
    for instance in instances {
        let Some((port, target_host)) = srvs.get(&instance) else {
            continue;
        };
        let Some(ip) = a_records.get(&target_host.to_ascii_lowercase()) else {
            continue;
        };

        let attrs = txts.get(&instance);
        let kind = attrs
            .and_then(|m| m.get("kind"))
            .map(|v| parse_device_kind(v))
            .unwrap_or(DeviceKind::Unknown);
        let name = attrs
            .and_then(|m| m.get("name"))
            .filter(|v| !v.trim().is_empty())
            .cloned();

        devices.push(DiscoveredDevice {
            hostname: target_host.clone(),
            ip: ip.to_string(),
            port: *port,
            kind,
            name,
        });
    }

    devices
}

/// Map a DNS-SD TXT `kind=` value to [`DeviceKind`].
fn parse_device_kind(value: &str) -> DeviceKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "hardware" | "esp32" => DeviceKind::Hardware,
        "desktop" => DeviceKind::Desktop,
        "android" => DeviceKind::Android,
        _ => DeviceKind::Unknown,
    }
}

/// Parse DNS TXT RDATA consisting of length-prefixed `key=value` character strings.
fn parse_txt_rdata(rdata: &[u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut offset = 0usize;
    while offset < rdata.len() {
        let len = rdata[offset] as usize;
        offset += 1;
        if offset + len > rdata.len() {
            break;
        }
        if let Ok(entry) = std::str::from_utf8(&rdata[offset..offset + len])
            && let Some((k, v)) = entry.split_once('=')
        {
            map.insert(k.to_ascii_lowercase(), v.to_string());
        }
        offset += len;
    }
    map
}

struct ParsedRecord<'a> {
    name: String,
    rtype: u16,
    rdata_offset: usize,
    rdata: &'a [u8],
}

/// Parse all resource records across Answer, Authority, and Additional sections of a DNS response.
fn parse_response_records(packet: &[u8]) -> Option<Vec<ParsedRecord<'_>>> {
    if packet.len() < 12 {
        return None;
    }

    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    // QR bit (0x8000) must be set for a response
    if (flags & 0x8000) == 0 {
        return None;
    }

    let qdcount = u16::from_be_bytes([packet[4], packet[5]]);
    let ancount = u16::from_be_bytes([packet[6], packet[7]]) as usize;
    let nscount = u16::from_be_bytes([packet[8], packet[9]]) as usize;
    let arcount = u16::from_be_bytes([packet[10], packet[11]]) as usize;
    let total_rrs = ancount.saturating_add(nscount).saturating_add(arcount);
    if total_rrs == 0 {
        return None;
    }

    let mut offset = 12;

    // Skip Question section
    for _ in 0..qdcount {
        let next = skip_dns_name(packet, offset)?;
        if next + 4 > packet.len() {
            return None;
        }
        offset = next + 4;
    }

    let mut records = Vec::with_capacity(total_rrs.min(64));
    for _ in 0..total_rrs {
        let Some((name, next)) = read_dns_name(packet, offset) else {
            break;
        };
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

        records.push(ParsedRecord {
            name,
            rtype,
            rdata_offset: rdata_start,
            rdata: &packet[rdata_start..rdata_end],
        });
        offset = rdata_end;
    }

    Some(records)
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

    fn push_rr(buf: &mut Vec<u8>, name: &str, rtype: u16, rdata: &[u8]) {
        encode_dns_name(buf, name);
        buf.extend_from_slice(&rtype.to_be_bytes());
        buf.extend_from_slice(&0x0001u16.to_be_bytes()); // CLASS IN
        buf.extend_from_slice(&120u32.to_be_bytes()); // TTL 120
        buf.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        buf.extend_from_slice(rdata);
    }

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
    fn build_ptr_query_encodes_service_type() {
        let query = build_ptr_query(SERVICE_TYPE);
        let len = query.len();
        assert_eq!(&query[len - 4..len - 2], &DNS_TYPE_PTR.to_be_bytes());
        assert_eq!(&query[len - 2..], &MDNS_QCLASS_IN_QU.to_be_bytes());
        let (decoded, _) = read_dns_name(&query, 12).expect("valid encoded name");
        assert_eq!(decoded, SERVICE_TYPE);
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
        push_rr(&mut resp, DEFAULT_HOSTNAME, DNS_TYPE_A, &[192, 168, 1, 105]);

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
        push_rr(&mut resp, "other.local", DNS_TYPE_A, &[192, 168, 1, 99]);
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

    #[test]
    fn parse_dnssd_response_extracts_peer_with_kind_and_name() {
        let instance = "office-pc._khvylyna._tcp.local";
        let host = "office-pc.local";

        let mut resp = vec![
            0x00, 0x00, // ID
            0x84, 0x00, // Flags: QR=1, AA=1
            0x00, 0x00, // QDCOUNT=0
            0x00, 0x01, // ANCOUNT=1 (PTR)
            0x00, 0x00, // NSCOUNT=0
            0x00, 0x03, // ARCOUNT=3 (SRV, TXT, A)
        ];

        let mut ptr_rdata = Vec::new();
        encode_dns_name(&mut ptr_rdata, instance);
        push_rr(&mut resp, SERVICE_TYPE, DNS_TYPE_PTR, &ptr_rdata);

        let mut srv_rdata = vec![0x00, 0x00, 0x00, 0x00]; // priority=0, weight=0
        srv_rdata.extend_from_slice(&49152u16.to_be_bytes());
        encode_dns_name(&mut srv_rdata, host);
        push_rr(&mut resp, instance, DNS_TYPE_SRV, &srv_rdata);

        let mut txt_rdata = Vec::new();
        for kv in ["kind=desktop", "name=Office Laptop", "ver=0.12.1"] {
            txt_rdata.push(kv.len() as u8);
            txt_rdata.extend_from_slice(kv.as_bytes());
        }
        push_rr(&mut resp, instance, DNS_TYPE_TXT, &txt_rdata);

        push_rr(&mut resp, host, DNS_TYPE_A, &[192, 168, 0, 50]);

        let devices = parse_dnssd_response(&resp, SERVICE_TYPE);
        assert_eq!(
            devices,
            vec![DiscoveredDevice {
                hostname: host.to_string(),
                ip: "192.168.0.50".to_string(),
                port: 49152,
                kind: DeviceKind::Desktop,
                name: Some("Office Laptop".to_string()),
            }]
        );
    }

    #[test]
    fn parse_dnssd_response_classifies_android_and_hardware_kinds() {
        assert_eq!(parse_device_kind("android"), DeviceKind::Android);
        assert_eq!(parse_device_kind("Android"), DeviceKind::Android);
        assert_eq!(parse_device_kind("hardware"), DeviceKind::Hardware);
        assert_eq!(parse_device_kind("esp32"), DeviceKind::Hardware);
        assert_eq!(parse_device_kind("desktop"), DeviceKind::Desktop);
        assert_eq!(parse_device_kind("toaster"), DeviceKind::Unknown);
    }

    #[test]
    fn merge_device_prefers_dnssd_metadata_over_legacy_a_record() {
        let mut map = BTreeMap::new();
        let ip = Ipv4Addr::new(192, 168, 1, 105);

        merge_device(
            &mut map,
            ip,
            DiscoveredDevice {
                hostname: DEFAULT_HOSTNAME.to_string(),
                ip: ip.to_string(),
                port: 80,
                kind: DeviceKind::Hardware,
                name: None,
            },
        );
        merge_device(
            &mut map,
            ip,
            DiscoveredDevice {
                hostname: DEFAULT_HOSTNAME.to_string(),
                ip: ip.to_string(),
                port: 80,
                kind: DeviceKind::Hardware,
                name: Some("Living Room".to_string()),
            },
        );

        assert_eq!(map.len(), 1);
        assert_eq!(map[&(ip, 80)].name.as_deref(), Some("Living Room"));
    }
}
