//! Wake-on-LAN packet building and parsing.

/// Wake-on-LAN constants.
pub const DEFAULT_PORT: u16 = 9;
pub const HEADER_SIZE: usize = 6;
pub const MAC_SIZE: usize = 6;
pub const MAC_REPEAT: usize = 16;
pub const PACKET_SIZE: usize = HEADER_SIZE + MAC_SIZE * MAC_REPEAT;
/// The largest SecureOn password appended to a magic packet.
pub const MAX_PASSWORD_SIZE: usize = MAC_SIZE;
/// The largest accepted packet: a magic packet plus a SecureOn password.
pub const MAX_ACCEPTED_SIZE: usize = PACKET_SIZE + MAX_PASSWORD_SIZE;

/// A MAC-48 address.
pub type Mac = [u8; MAC_SIZE];

/// A Wake-on-LAN packet.
pub type Packet = [u8; PACKET_SIZE];

/// Wake-on-LAN errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WolError {
    #[error("invalid MAC-48 address")]
    InvalidMac,
    #[error("invalid Wake-on-LAN packet size")]
    InvalidSize,
    #[error("invalid Wake-on-LAN packet header")]
    InvalidHeader,
}

/// Formats a MAC address as colon-separated lowercase hex, e.g. "00:11:22:33:44:55".
pub fn mac_to_string(mac: &Mac) -> String {
    mac.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Builds a Wake-on-LAN packet with the given MAC address.
pub fn build_packet(mac: Mac) -> Packet {
    let mut packet = [0xffu8; PACKET_SIZE];
    for chunk in packet[HEADER_SIZE..].as_chunks_mut::<MAC_SIZE>().0 {
        *chunk = mac;
    }
    packet
}

/// Parses a Wake-on-LAN packet and returns the MAC address contained in it.
///
/// Only the first [`PACKET_SIZE`] bytes are validated. Up to
/// [`MAX_PASSWORD_SIZE`] trailing bytes are accepted so that packets carrying a
/// SecureOn password (4 or 6 bytes) can still be relayed unchanged.
pub fn parse_packet(packet: &[u8]) -> Result<Mac, WolError> {
    if packet.len() < PACKET_SIZE || packet.len() > MAX_ACCEPTED_SIZE {
        return Err(WolError::InvalidSize);
    }
    let packet = &packet[..PACKET_SIZE];

    // The header must be the 0xFF byte repeated 6 times.
    if packet[..HEADER_SIZE] != [0xff; HEADER_SIZE] {
        return Err(WolError::InvalidHeader);
    }

    // The MAC address should be the same MAC address repeated 16 times.
    let (macs, _) = packet[HEADER_SIZE..].as_chunks::<MAC_SIZE>();
    let mac = macs[0];
    if macs.iter().any(|&candidate| candidate != mac) {
        return Err(WolError::InvalidMac);
    }

    Ok(mac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_parse_roundtrip() {
        let mac: Mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        let packet = build_packet(mac);
        assert_eq!(packet.len(), PACKET_SIZE);
        assert_eq!(parse_packet(&packet).unwrap(), mac);
    }

    #[test]
    fn parse_packet_rejects_wrong_size() {
        let short = [0u8; PACKET_SIZE - 1];
        assert_eq!(parse_packet(&short), Err(WolError::InvalidSize));

        let too_long = vec![0xffu8; MAX_ACCEPTED_SIZE + 1];
        assert_eq!(parse_packet(&too_long), Err(WolError::InvalidSize));
    }

    #[test]
    fn parse_packet_accepts_secure_on_password() {
        let mac: Mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];

        // SecureOn passwords are 4 or 6 bytes appended after the 16 MAC repeats.
        let mut packet = build_packet(mac).to_vec();
        packet.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(parse_packet(&packet).unwrap(), mac);

        packet.extend_from_slice(&[0x01, 0x02]);
        assert_eq!(parse_packet(&packet).unwrap(), mac);
    }

    #[test]
    fn parse_packet_rejects_bad_header() {
        let mut packet = build_packet([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        packet[0] = 0x00;
        assert_eq!(parse_packet(&packet), Err(WolError::InvalidHeader));
    }

    #[test]
    fn parse_packet_rejects_mismatched_repeat() {
        let mut packet = build_packet([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        packet[HEADER_SIZE + MAC_SIZE * 5] = 0x99;
        assert_eq!(parse_packet(&packet), Err(WolError::InvalidMac));
    }

    #[test]
    fn mac_displays_colon_separated_hex() {
        let mac: Mac = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
        assert_eq!(mac_to_string(&mac), "00:11:22:33:44:55");
    }
}
