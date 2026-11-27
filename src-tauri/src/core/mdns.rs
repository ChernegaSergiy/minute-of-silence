//! Local network mDNS / DNS-SD discovery and peer responder for Minute of Silence.
//!
//! All peers (ESP32 hardware devices, desktop apps, and Android apps) advertise
//! and discover each other via the unified DNS-SD service `_khvylyna._tcp.local`
//! carrying PTR, SRV, TXT (`kind=`, `name=`, `ver=`, `proto=`), and A records.

use std::{
    collections::{BTreeMap, HashMap},
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use khvylyna_core::{DeviceKind, DiscoveredDevice};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::{net::UdpSocket, time};

/// Standard IPv4 mDNS multicast group address (RFC 6762).
pub const MDNS_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);

/// Standard mDNS UDP port.
pub const MDNS_PORT: u16 = 5353;

/// DNS-SD service type advertised by Minute of Silence peers.
pub const SERVICE_TYPE: &str = "_khvylyna._tcp.local";

/// Default peer port advertised by desktop/Android instances over DNS-SD.
pub const DEFAULT_PEER_PORT: u16 = 49152;

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

/// DNS wildcard record type (`ANY`).
const DNS_TYPE_ANY: u16 = 255;

/// DNS class `IN` with the mDNS unicast-response (`QU`) bit set (RFC 6762 §5.4).
const MDNS_QCLASS_IN_QU: u16 = 0x8001;

/// DNS class `IN` (Internet).
const DNS_CLASS_IN: u16 = 0x0001;

/// Default mDNS resource record TTL in seconds.
const MDNS_TTL_SECS: u32 = 120;

/// Return the [`DeviceKind`] corresponding to the current build target.
pub fn current_device_kind() -> DeviceKind {
    if cfg!(target_os = "android") {
        DeviceKind::Android
    } else {
        DeviceKind::Desktop
    }
}

/// Convert [`DeviceKind`] into its DNS-SD TXT `kind=` string value.
fn device_kind_str(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Hardware => "hardware",
        DeviceKind::Desktop => "desktop",
        DeviceKind::Android => "android",
        DeviceKind::Unknown => "unknown",
    }
}

/// Determine a human-readable display name for this local instance.
pub fn local_device_name() -> String {
    let env_name = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    env_name.unwrap_or_else(|| {
        if cfg!(target_os = "android") {
            "Minute of Silence Android".to_string()
        } else {
            "Minute of Silence Desktop".to_string()
        }
    })
}

/// Sanitize a display name into a valid single DNS label (ASCII alphanumeric + `-`, max 48 bytes).
fn sanitize_dns_label(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if (ch == '-' || ch == '_' || ch == ' ') && !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        if out.len() >= 48 {
            break;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "khvylyna-peer".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Return the local IPv4 address of the interface that reaches `peer`.
///
/// A UDP `connect` only consults the routing table and sends nothing, so an on-link route to
/// the peer is enough; no default route is required.
pub fn local_ipv4_towards(peer: Ipv4Addr) -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(SocketAddrV4::new(peer, MDNS_PORT)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_unspecified() => Some(ip),
        _ => None,
    }
}

/// Return `true` if `ip` is assigned to one of this machine's network interfaces.
///
/// Binding only succeeds for addresses that belong to a local interface.
pub fn is_local_ipv4(ip: Ipv4Addr) -> bool {
    std::net::UdpSocket::bind(SocketAddrV4::new(ip, 0)).is_ok()
}

#[cfg(target_os = "android")]
struct AndroidMulticastGuard;

#[cfg(target_os = "android")]
impl AndroidMulticastGuard {
    fn acquire() -> std::io::Result<Self> {
        crate::platform::android::device::call::<()>("acquireMdnsMulticast", serde_json::json!({}))
            .map_err(std::io::Error::other)?;
        Ok(Self)
    }
}

#[cfg(target_os = "android")]
impl Drop for AndroidMulticastGuard {
    fn drop(&mut self) {
        if let Err(error) = crate::platform::android::device::call::<()>(
            "releaseMdnsMulticast",
            serde_json::json!({}),
        ) {
            log::warn!("Failed to release the Android mDNS multicast lock: {error}");
        }
    }
}

pub(crate) struct PreparedResponder {
    socket: UdpSocket,
    #[cfg(target_os = "android")]
    multicast_guard: AndroidMulticastGuard,
}

pub(crate) fn prepare_responder() -> std::io::Result<PreparedResponder> {
    let socket = UdpSocket::from_std(bind_mdns_socket(MDNS_PORT)?)?;
    socket.join_multicast_v4(MDNS_MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED)?;
    #[cfg(target_os = "android")]
    let multicast_guard = AndroidMulticastGuard::acquire()?;

    Ok(PreparedResponder {
        socket,
        #[cfg(target_os = "android")]
        multicast_guard,
    })
}

pub(crate) async fn run_responder(
    responder: PreparedResponder,
    app_version: String,
) -> std::io::Result<()> {
    let PreparedResponder {
        socket,
        #[cfg(target_os = "android")]
            multicast_guard: _multicast_guard,
    } = responder;

    let display_name = local_device_name();
    let base_label = sanitize_dns_label(&display_name);
    let kind = current_device_kind();

    log::info!(
        "mDNS peer responder active: {} ({}) on {}",
        display_name,
        device_kind_str(kind),
        SERVICE_TYPE
    );

    let mut buf = [0u8; 1500];
    let multicast_dest = SocketAddr::V4(SocketAddrV4::new(MDNS_MULTICAST_ADDR, MDNS_PORT));

    loop {
        let (len, src) = match socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(err) => return Err(err),
        };
        let packet = &buf[..len];
        if !is_dnssd_query_for(packet, SERVICE_TYPE) {
            continue;
        }

        let IpAddr::V4(src_ip) = src.ip() else {
            continue;
        };
        let Some(local_ip) = local_ipv4_towards(src_ip).filter(|ip| !ip.is_loopback()) else {
            continue;
        };

        let octets = local_ip.octets();
        let instance_label = format!("{base_label}-{:02x}{:02x}", octets[2], octets[3]);
        let response = build_dnssd_response(
            &instance_label,
            &display_name,
            kind,
            local_ip,
            DEFAULT_PEER_PORT,
            &app_version,
        );

        let dest = if src.port() != MDNS_PORT {
            src
        } else {
            multicast_dest
        };
        if let Err(err) = socket.send_to(&response, dest).await {
            log::debug!("mDNS responder send error: {err}");
        }
    }
}

fn bind_mdns_socket(port: u16) -> std::io::Result<std::net::UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port).into())?;
    socket.set_nonblocking(true)?;
    Ok(socket.into())
}

/// Return `true` if `packet` is a DNS query asking for `service_type` (PTR or ANY).
pub fn is_dnssd_query_for(packet: &[u8], service_type: &str) -> bool {
    if packet.len() < 12 {
        return false;
    }

    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    // Must be a query (QR bit == 0)
    if (flags & 0x8000) != 0 {
        return false;
    }

    let qdcount = u16::from_be_bytes([packet[4], packet[5]]);
    let expected = service_type.trim_end_matches('.');
    let mut offset = 12;

    for _ in 0..qdcount {
        let Some((name, next)) = read_dns_name(packet, offset) else {
            return false;
        };
        if next + 4 > packet.len() {
            return false;
        }
        let qtype = u16::from_be_bytes([packet[next], packet[next + 1]]);
        if name.eq_ignore_ascii_case(expected) && (qtype == DNS_TYPE_PTR || qtype == DNS_TYPE_ANY) {
            return true;
        }
        offset = next + 4;
    }

    false
}

/// Build a wire-format mDNS DNS-SD response advertising this peer on [`SERVICE_TYPE`].
pub fn build_dnssd_response(
    instance_label: &str,
    display_name: &str,
    kind: DeviceKind,
    ip: Ipv4Addr,
    port: u16,
    app_version: &str,
) -> Vec<u8> {
    let instance_fqdn = format!("{instance_label}.{SERVICE_TYPE}");
    let host_fqdn = format!("{instance_label}.local");

    let mut packet = Vec::with_capacity(256);
    packet.extend_from_slice(&[
        0x00, 0x00, // Transaction ID = 0
        0x84, 0x00, // Flags: QR=1, AA=1
        0x00, 0x00, // QDCOUNT = 0
        0x00, 0x01, // ANCOUNT = 1 (PTR)
        0x00, 0x00, // NSCOUNT = 0
        0x00, 0x03, // ARCOUNT = 3 (SRV, TXT, A)
    ]);

    // PTR: _khvylyna._tcp.local -> <instance>._khvylyna._tcp.local
    let mut ptr_rdata = Vec::new();
    encode_dns_name(&mut ptr_rdata, &instance_fqdn);
    append_rr(&mut packet, SERVICE_TYPE, DNS_TYPE_PTR, &ptr_rdata);

    // SRV: <instance>._khvylyna._tcp.local -> (0, 0, port, <instance>.local)
    let mut srv_rdata = vec![0x00, 0x00, 0x00, 0x00];
    srv_rdata.extend_from_slice(&port.to_be_bytes());
    encode_dns_name(&mut srv_rdata, &host_fqdn);
    append_rr(&mut packet, &instance_fqdn, DNS_TYPE_SRV, &srv_rdata);

    // TXT: kind=..., name=..., ver=..., proto=...
    let mut txt_rdata = Vec::new();
    let kind_entry = format!("kind={}", device_kind_str(kind));
    let name_entry = format!("name={display_name}");
    let ver_entry = format!("ver={app_version}");
    let proto_entry = format!("proto={}", khvylyna_pairing::protocol::VERSION);
    for entry in [
        kind_entry.as_str(),
        name_entry.as_str(),
        ver_entry.as_str(),
        proto_entry.as_str(),
    ] {
        let bytes = entry.as_bytes();
        let len = bytes.len().min(255);
        txt_rdata.push(len as u8);
        txt_rdata.extend_from_slice(&bytes[..len]);
    }
    append_rr(&mut packet, &instance_fqdn, DNS_TYPE_TXT, &txt_rdata);

    // A: <instance>.local -> IPv4
    append_rr(&mut packet, &host_fqdn, DNS_TYPE_A, &ip.octets());

    packet
}

fn append_rr(buf: &mut Vec<u8>, name: &str, rtype: u16, rdata: &[u8]) {
    encode_dns_name(buf, name);
    buf.extend_from_slice(&rtype.to_be_bytes());
    buf.extend_from_slice(&DNS_CLASS_IN.to_be_bytes());
    buf.extend_from_slice(&MDNS_TTL_SECS.to_be_bytes());
    buf.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
    buf.extend_from_slice(rdata);
}

/// Discover Minute of Silence devices (hardware, desktop, Android) on the local network.
///
/// Sends a DNS-SD PTR query for [`SERVICE_TYPE`] to `224.0.0.251:5353` and collects
/// responses within [`DISCOVERY_TIMEOUT`], deduplicating by `(ip, port)` and excluding
/// this local instance.
pub async fn discover_devices() -> crate::Result<Vec<DiscoveredDevice>> {
    discover_devices_with_timeout(DISCOVERY_TIMEOUT).await
}

/// Discover devices advertising [`SERVICE_TYPE`] within `timeout`.
pub async fn discover_devices_with_timeout(
    timeout: Duration,
) -> crate::Result<Vec<DiscoveredDevice>> {
    let found = query_dnssd_udp(timeout).await?;
    let mut by_endpoint: BTreeMap<(Ipv4Addr, u16), DiscoveredDevice> = BTreeMap::new();

    for device in found {
        if let Ok(ip) = device.ip.parse::<Ipv4Addr>() {
            if device.port == DEFAULT_PEER_PORT && is_local_ipv4(ip) {
                continue;
            }
            by_endpoint.entry((ip, device.port)).or_insert(device);
        }
    }

    Ok(by_endpoint.into_values().collect())
}

/// Send a DNS-SD PTR mDNS query over UDP and collect matching devices until `timeout`.
async fn query_dnssd_udp(timeout: Duration) -> std::io::Result<Vec<DiscoveredDevice>> {
    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;
    let dest = SocketAddrV4::new(MDNS_MULTICAST_ADDR, MDNS_PORT);
    let ptr_query = build_ptr_query(SERVICE_TYPE);

    socket.send_to(&ptr_query, dest).await?;

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
                found.extend(parse_dnssd_response(&buf[..len], SERVICE_TYPE));
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

/// Build a wire-format mDNS PTR-record query packet for `service_type` with the QU bit set.
pub fn build_ptr_query(service_type: &str) -> Vec<u8> {
    let mut packet = Vec::with_capacity(64);
    packet.extend_from_slice(&[
        0x00, 0x00, // Transaction ID (0 in mDNS)
        0x00, 0x00, // Flags: standard query
        0x00, 0x01, // QDCOUNT = 1
        0x00, 0x00, // ANCOUNT = 0
        0x00, 0x00, // NSCOUNT = 0
        0x00, 0x00, // ARCOUNT = 0
    ]);

    encode_dns_name(&mut packet, service_type);
    packet.extend_from_slice(&DNS_TYPE_PTR.to_be_bytes());
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
        let protocol_ok = attrs.and_then(|m| m.get("proto")).is_some_and(|value| {
            value.trim().parse::<u8>().ok() == Some(khvylyna_pairing::protocol::VERSION)
        });
        if !protocol_ok {
            continue;
        }
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
    fn mdns_sockets_can_share_a_port() -> std::io::Result<()> {
        let first = bind_mdns_socket(0)?;
        let second = bind_mdns_socket(first.local_addr()?.port())?;
        assert_eq!(first.local_addr()?.port(), second.local_addr()?.port());
        Ok(())
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
    fn parse_dnssd_response_ignores_query_packets() {
        let query = build_ptr_query(SERVICE_TYPE);
        assert!(parse_dnssd_response(&query, SERVICE_TYPE).is_empty());
    }

    #[test]
    fn parse_dnssd_response_handles_malformed_and_cyclic_pointers_safely() {
        assert!(parse_dnssd_response(&[0x00, 0x01, 0x84], SERVICE_TYPE).is_empty());

        let cyclic = vec![
            0x00, 0x00, 0x84, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x0C,
            0x00, 0x0C, 0x00, 0x01, 0x00, 0x00, 0x00, 0x78, 0x00, 0x02, 0xC0, 0x0C,
        ];
        assert!(parse_dnssd_response(&cyclic, SERVICE_TYPE).is_empty());
    }

    #[test]
    fn parse_dnssd_response_classifies_all_device_kinds() {
        assert_eq!(parse_device_kind("android"), DeviceKind::Android);
        assert_eq!(parse_device_kind("Android"), DeviceKind::Android);
        assert_eq!(parse_device_kind("hardware"), DeviceKind::Hardware);
        assert_eq!(parse_device_kind("esp32"), DeviceKind::Hardware);
        assert_eq!(parse_device_kind("desktop"), DeviceKind::Desktop);
        assert_eq!(parse_device_kind("toaster"), DeviceKind::Unknown);
    }

    #[test]
    fn dnssd_response_round_trips_for_android_desktop_and_hardware() {
        for (label, display, kind, ip, port) in [
            (
                "pixel-8",
                "Pixel 8",
                DeviceKind::Android,
                Ipv4Addr::new(192, 168, 1, 77),
                DEFAULT_PEER_PORT,
            ),
            (
                "office-pc",
                "Office Laptop",
                DeviceKind::Desktop,
                Ipv4Addr::new(192, 168, 0, 50),
                DEFAULT_PEER_PORT,
            ),
            (
                "khvylyna",
                "Khvylyna ESP32",
                DeviceKind::Hardware,
                Ipv4Addr::new(192, 168, 1, 105),
                80,
            ),
        ] {
            let packet = build_dnssd_response(label, display, kind, ip, port, "0.12.1");
            let devices = parse_dnssd_response(&packet, SERVICE_TYPE);
            assert_eq!(
                devices,
                vec![DiscoveredDevice {
                    hostname: format!("{label}.local"),
                    ip: ip.to_string(),
                    port,
                    kind,
                    name: Some(display.to_string()),
                }]
            );
        }
    }

    #[test]
    fn incompatible_or_missing_protocol_txt_records_are_ignored() {
        let label = "incompatible";
        let ip = Ipv4Addr::new(192, 168, 1, 120);
        let original = build_dnssd_response(
            label,
            "Incompatible",
            DeviceKind::Desktop,
            ip,
            DEFAULT_PEER_PORT,
            "0.12.1",
        );
        assert_eq!(parse_dnssd_response(&original, SERVICE_TYPE).len(), 1);
        let offset = original
            .windows(6)
            .position(|window| window == b"proto=")
            .expect("proto TXT entry");

        let mut wrong_version = original.clone();
        wrong_version[offset + 6] = b'0' + khvylyna_pairing::protocol::VERSION + 1;
        assert!(parse_dnssd_response(&wrong_version, SERVICE_TYPE).is_empty());

        let mut non_numeric = original.clone();
        non_numeric[offset + 6] = b'x';
        assert!(parse_dnssd_response(&non_numeric, SERVICE_TYPE).is_empty());

        let mut missing = original;
        missing[offset..offset + 6].copy_from_slice(b"other=");
        assert!(parse_dnssd_response(&missing, SERVICE_TYPE).is_empty());
    }

    #[test]
    fn is_dnssd_query_for_matches_ptr_query_and_ignores_responses() {
        let ptr_query = build_ptr_query(SERVICE_TYPE);
        assert!(is_dnssd_query_for(&ptr_query, SERVICE_TYPE));

        let other_query = build_ptr_query("_http._tcp.local");
        assert!(!is_dnssd_query_for(&other_query, SERVICE_TYPE));

        let response = build_dnssd_response(
            "workstation",
            "Workstation",
            DeviceKind::Desktop,
            Ipv4Addr::new(10, 0, 0, 5),
            DEFAULT_PEER_PORT,
            "0.12.1",
        );
        assert!(!is_dnssd_query_for(&response, SERVICE_TYPE));
    }

    #[test]
    fn local_ipv4_helpers_recognise_loopback_and_reject_foreign_addresses() {
        assert_eq!(
            local_ipv4_towards(Ipv4Addr::LOCALHOST),
            Some(Ipv4Addr::LOCALHOST)
        );
        assert!(is_local_ipv4(Ipv4Addr::LOCALHOST));
        assert!(!is_local_ipv4(Ipv4Addr::new(192, 0, 2, 1)));
    }
}
