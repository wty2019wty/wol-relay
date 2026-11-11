//! IPv4 network and interface resolution helpers.

use std::fmt;
use std::net::{Ipv4Addr, UdpSocket};

use crate::wol;

/// The size of the buffer used to read WOL packets.
pub const MAX_PACKET_SIZE: usize = 1024;

/// An IPv4 network with its prefix length. Host bits are masked off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Network {
    network: Ipv4Addr,
    prefix: u8,
}

/// Network parsing errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NetworkError {
    #[error("invalid CIDR {input:?}")]
    InvalidCidr { input: String },
    #[error("IPv6 not supported: {input:?}")]
    IPv6NotSupported { input: String },
    #[error("invalid prefix length: {input:?}")]
    InvalidPrefix { input: String },
}

impl Ipv4Network {
    /// Creates a network from an address and prefix length, masking off host bits.
    /// Returns `None` if the prefix length is greater than 32.
    pub fn new(addr: Ipv4Addr, prefix: u8) -> Option<Self> {
        if prefix > 32 {
            return None;
        }
        Some(Self {
            network: Ipv4Addr::from(u32::from(addr) & mask(prefix)),
            prefix,
        })
    }

    /// The network address with host bits masked off.
    pub fn network(&self) -> Ipv4Addr {
        self.network
    }

    /// The prefix length (0 to 32).
    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    /// Reports whether the given IP address is contained in this network.
    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        u32::from(ip) & mask(self.prefix) == u32::from(self.network)
    }

    /// The broadcast address of this network.
    pub fn broadcast(&self) -> Ipv4Addr {
        Ipv4Addr::from(u32::from(self.network) | !mask(self.prefix))
    }
}

impl fmt::Display for Ipv4Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix)
    }
}

/// The netmask for a prefix length (0 to 32).
fn mask(prefix: u8) -> u32 {
    u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0)
}

/// Parses a CIDR string into an IPv4 network. IPv6 CIDRs are rejected.
pub fn parse_cidr(s: &str) -> Result<Ipv4Network, NetworkError> {
    let input = || s.to_string();

    let Some((addr_part, prefix_part)) = s.split_once('/') else {
        return Err(NetworkError::InvalidCidr { input: input() });
    };

    if addr_part.contains(':') {
        return Err(NetworkError::IPv6NotSupported { input: input() });
    }

    let addr: Ipv4Addr = addr_part
        .parse()
        .map_err(|_| NetworkError::InvalidCidr { input: input() })?;

    prefix_part
        .parse()
        .ok()
        .and_then(|prefix| Ipv4Network::new(addr, prefix))
        .ok_or_else(|| NetworkError::InvalidPrefix { input: input() })
}

/// Converts a netmask to a prefix length. Returns `None` for non-contiguous masks.
pub fn netmask_to_prefix(mask: Ipv4Addr) -> Option<u8> {
    let mask = u32::from(mask);
    let prefix = mask.leading_ones() as u8;
    if prefix == 32 || (mask << prefix) == 0 {
        Some(prefix)
    } else {
        None
    }
}

/// 检查给定名字的网络接口是否存在（无论是否配置了 IP 地址）。
#[cfg(unix)]
fn interface_exists(name: &str) -> bool {
    std::ffi::CString::new(name)
        .map(|name| unsafe { libc::if_nametoindex(name.as_ptr()) } != 0)
        .unwrap_or(false)
}

/// 检查给定名字的网络接口是否存在（无论是否配置了 IP 地址）。
///
/// Windows 版：`if-addrs` 报告的接口名是接口别名（FriendlyName，如
/// "Loopback Pseudo-Interface 1"），对应 `ConvertInterfaceAliasToLuid` 的名字空间；
/// 按别名查 LUID 成功即表示接口存在，与是否配置了 IP 地址无关。
#[cfg(windows)]
fn interface_exists(name: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::NO_ERROR;
    use windows_sys::Win32::NetworkManagement::IpHelper::ConvertInterfaceAliasToLuid;
    use windows_sys::Win32::NetworkManagement::Ndis::NET_LUID_LH;

    // 别名是 UTF-16 字符串，且可能包含非 ASCII 字符（如"以太网"）。
    let alias: Vec<u16> = std::ffi::OsStr::new(name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut luid = std::mem::MaybeUninit::<NET_LUID_LH>::uninit();
    unsafe {
        ConvertInterfaceAliasToLuid(alias.as_ptr(), luid.as_mut_ptr()) == NO_ERROR
    }
}

/// Expands a CLI argument into one or more IPv4 networks.
///
/// 含 "/" 的参数按 CIDR 解析，其余按网络接口名解析。接口名不会包含 "/"
/// （Linux 接口名与 Windows 的接口别名 FriendlyName 都不含 "/"），
/// 因此该启发式是无歧义的。
pub fn resolve_arg(arg: &str) -> anyhow::Result<Vec<Ipv4Network>> {
    if arg.contains('/') {
        let network = parse_cidr(arg)?;
        return Ok(vec![network]);
    }

    let ifaces = if_addrs::get_if_addrs()
        .map_err(|err| anyhow::anyhow!("resolve interface {arg:?}: {err}"))?;
    if !interface_exists(arg) {
        anyhow::bail!("resolve interface {arg:?}: not found");
    }

    Ok(ifaces
        .iter()
        .filter(|iface| iface.name == arg)
        .filter_map(|iface| match &iface.addr {
            if_addrs::IfAddr::V4(addr) => addr_to_network(addr.ip, addr.netmask),
            _ => None,
        })
        .collect())
}

/// An interface address and its netmask as a network, or `None` when the netmask is
/// unknown or unusable.
///
/// A 0.0.0.0 netmask means getifaddrs does not know it (macOS secondary addresses);
/// treating it as a real /0 would monitor every IPv4 source, so those addresses are
/// skipped. Non-contiguous netmasks are skipped too.
fn addr_to_network(ip: Ipv4Addr, netmask: Ipv4Addr) -> Option<Ipv4Network> {
    if netmask == Ipv4Addr::UNSPECIFIED {
        return None;
    }
    netmask_to_prefix(netmask).and_then(|prefix| Ipv4Network::new(ip, prefix))
}

/// Returns all IPv4 addresses assigned to local network interfaces.
///
/// Used for self-detection so loop prevention works regardless of whether targets were
/// specified as interface names or CIDRs.
pub fn local_ipv4_addrs() -> anyhow::Result<Vec<Ipv4Addr>> {
    let ifaces = if_addrs::get_if_addrs()?;
    Ok(ifaces
        .iter()
        .filter_map(|iface| match &iface.addr {
            if_addrs::IfAddr::V4(addr) => Some(addr.ip),
            _ => None,
        })
        .collect())
}

/// Checks if the given IP address is contained in any of the provided networks.
pub fn is_ip_in_any(ip: Ipv4Addr, networks: &[Ipv4Network]) -> bool {
    networks.iter().any(|network| network.contains(ip))
}

/// Creates a UDP socket for sending WOL packets, with the OS assigning the source port.
/// Broadcast sends are enabled.
pub fn sender_socket() -> anyhow::Result<UdpSocket> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|err| anyhow::anyhow!("cannot create sender socket: {err}"))?;
    socket.set_broadcast(true).map_err(|err| {
        anyhow::anyhow!("cannot enable broadcast on sender socket: {err}")
    })?;
    Ok(socket)
}

/// Sends a Wake-on-LAN packet to the broadcast address of the given network.
pub fn send_wol_packet(
    socket: &UdpSocket,
    network: Ipv4Network,
    mac: wol::Mac,
) -> std::io::Result<()> {
    let broadcast_ip = network.broadcast();
    socket.send_to(&wol::build_packet(mac), (broadcast_ip, wol::DEFAULT_PORT))?;
    Ok(())
}

/// True for socket errors a blocking call may recover from without giving up.
pub fn is_transient(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::Interrupted
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(addr: &str, prefix: u8) -> Ipv4Network {
        Ipv4Network::new(addr.parse().unwrap(), prefix).unwrap()
    }

    #[test]
    fn parse_cidr_valid() {
        assert_eq!(parse_cidr("10.0.0.0/24").unwrap(), net("10.0.0.0", 24));
        assert_eq!(parse_cidr("10.0.0.5/24").unwrap(), net("10.0.0.0", 24));
        assert_eq!(
            parse_cidr("192.168.2.5/32").unwrap(),
            net("192.168.2.5", 32)
        );
        assert_eq!(parse_cidr("172.16.0.0/16").unwrap(), net("172.16.0.0", 16));
    }

    #[test]
    fn parse_cidr_masks_host_bits() {
        let parsed = parse_cidr("10.0.0.5/24").unwrap();
        assert_eq!(parsed.network(), "10.0.0.0".parse::<Ipv4Addr>().unwrap());
    }

    #[test]
    fn parse_cidr_invalid() {
        assert!(matches!(
            parse_cidr("not-a-cidr"),
            Err(NetworkError::InvalidCidr { .. })
        ));
        assert!(matches!(
            parse_cidr("::1/128"),
            Err(NetworkError::IPv6NotSupported { .. })
        ));
        assert!(matches!(
            parse_cidr("10.0.0.1/129"),
            Err(NetworkError::InvalidPrefix { .. })
        ));
        assert!(matches!(
            parse_cidr("10.0.0.1/abc"),
            Err(NetworkError::InvalidPrefix { .. })
        ));
        assert!(matches!(
            parse_cidr("10.0.0.1"),
            Err(NetworkError::InvalidCidr { .. })
        ));
    }

    #[test]
    fn display_masks_host_bits() {
        assert_eq!(
            parse_cidr("10.0.0.5/24").unwrap().to_string(),
            "10.0.0.0/24"
        );
    }

    #[test]
    fn broadcast() {
        let ip = |s: &str| s.parse::<Ipv4Addr>().unwrap();
        assert_eq!(net("192.168.1.1", 24).broadcast(), ip("192.168.1.255"));
        assert_eq!(net("10.0.0.0", 8).broadcast(), ip("10.255.255.255"));
        assert_eq!(net("192.168.2.5", 32).broadcast(), ip("192.168.2.5"));
    }

    #[test]
    fn netmask_to_prefix_cases() {
        let mask = |s: &str| s.parse::<Ipv4Addr>().unwrap();
        assert_eq!(netmask_to_prefix(mask("255.255.255.0")), Some(24));
        assert_eq!(netmask_to_prefix(mask("255.0.0.0")), Some(8));
        assert_eq!(netmask_to_prefix(mask("0.0.0.0")), Some(0));
        assert_eq!(netmask_to_prefix(mask("255.255.255.255")), Some(32));
        assert_eq!(netmask_to_prefix(mask("255.0.255.0")), None); // non-contiguous
    }

    #[test]
    fn is_ip_in_any_cases() {
        let networks = vec![net("192.168.1.1", 24), net("10.0.0.1", 24)];
        let ip = |s: &str| s.parse::<Ipv4Addr>().unwrap();
        let cases = [
            ("192.168.1.0", true),
            ("192.168.1.1", true),
            ("192.168.1.100", true),
            ("192.168.1.255", true),
            ("192.168.0.0", false),
            ("10.0.0.0", true),
            ("10.0.0.1", true),
            ("10.0.0.100", true),
            ("10.0.0.255", true),
            ("10.0.1.0", false),
            ("172.16.0.1", false),
            ("0.0.0.0", false),
        ];
        for (s, expected) in cases {
            assert_eq!(is_ip_in_any(ip(s), &networks), expected, "{s}");
        }
    }

    #[test]
    fn local_ipv4_addrs_includes_loopback() {
        let ips = local_ipv4_addrs().unwrap();
        assert!(!ips.is_empty(), "expected at least loopback");
        assert!(ips.contains(&"127.0.0.1".parse().unwrap()), "{ips:?}");
    }

    #[test]
    fn resolve_arg_cidr() {
        let networks = resolve_arg("127.0.0.1/8").unwrap();
        assert_eq!(networks, vec![net("127.0.0.0", 8)]);
    }

    #[test]
    fn resolve_arg_loopback_interface() {
        // Loopback interface name is platform-specific (lo0 on macOS, lo on Linux).
        let ifaces = if_addrs::get_if_addrs().unwrap();
        let lo = ifaces
            .iter()
            .find(|i| i.is_loopback())
            .expect("loopback interface");
        let networks = resolve_arg(&lo.name).unwrap();
        assert!(
            networks
                .iter()
                .any(|n| n.contains("127.0.0.1".parse().unwrap())),
            "{networks:?}"
        );
    }

    #[test]
    fn resolve_arg_rejects_unknown_interface() {
        assert!(resolve_arg("no-such-interface-xyz").is_err());
    }

    #[test]
    fn resolve_arg_skips_addresses_with_unknown_netmask() {
        // Loopback interface name is platform-specific (lo0 on macOS, lo on
        // Linux); every resolved network must have a known netmask, since a
        // /0 network would monitor every IPv4 source.
        let ifaces = if_addrs::get_if_addrs().unwrap();
        let lo = ifaces
            .iter()
            .find(|i| i.is_loopback())
            .expect("loopback interface");
        let networks = resolve_arg(&lo.name).expect("loopback interface must resolve");
        assert!(
            networks.iter().all(|n| n.prefix() > 0),
            "unexpected /0 network: {networks:?}"
        );
    }

    #[test]
    fn addr_to_network_skips_unknown_and_non_contiguous_netmasks() {
        let ip = |s: &str| s.parse::<Ipv4Addr>().unwrap();
        assert_eq!(addr_to_network(ip("127.0.2.2"), ip("0.0.0.0")), None);
        assert_eq!(addr_to_network(ip("10.0.0.5"), ip("255.0.255.0")), None);
        assert_eq!(
            addr_to_network(ip("10.0.0.5"), ip("255.255.255.0")),
            Some(net("10.0.0.0", 24))
        );
    }

    #[test]
    #[cfg(unix)]
    fn resolve_arg_accepts_existing_interface_without_addresses() {
        // An existing interface with no IP address must resolve to an empty
        // list, not an error (Go: net.InterfaceByName succeeds and Addrs()
        // is empty). gif0 exists on macOS without addresses; skip the test
        // on platforms where it does not exist.
        //
        // 仅 Unix：该场景是 macOS 的 gif0，且这里直接调用 libc::if_nametoindex。
        if unsafe { libc::if_nametoindex(c"gif0".as_ptr()) } == 0 {
            eprintln!("skipping: no gif0 interface");
            return;
        }
        let networks = resolve_arg("gif0").expect("existing interface must resolve");
        assert!(networks.is_empty(), "{networks:?}");
    }

    #[test]
    fn sender_socket_sets_broadcast() {
        let socket = sender_socket().unwrap();
        assert!(socket.broadcast().unwrap(), "SO_BROADCAST must be enabled");
    }

    #[test]
    fn send_wol_packet_sends_to_loopback_host() {
        let socket = sender_socket().unwrap();
        let network = net("127.0.0.1", 32);
        let mac: wol::Mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        send_wol_packet(&socket, network, mac)
            .expect("unicast send to a loopback host must succeed");
    }

    #[test]
    fn transient_errors_are_wouldblock_timeout_and_interrupted() {
        assert!(is_transient(&std::io::Error::from(
            std::io::ErrorKind::WouldBlock
        )));
        assert!(is_transient(&std::io::Error::from(
            std::io::ErrorKind::TimedOut
        )));
        assert!(is_transient(&std::io::Error::from(
            std::io::ErrorKind::Interrupted
        )));
        assert!(!is_transient(&std::io::Error::from(
            std::io::ErrorKind::ConnectionRefused
        )));
        assert!(!is_transient(&std::io::Error::other("hard failure")));
    }
}
