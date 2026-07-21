//! ICMP echo через Windows IP Helper (`IcmpSendEcho`). Возвращает RTT в мс
//! или -1 при недоступности/таймауте. Резолвит хост в первый IPv4-адрес.

use std::net::{Ipv4Addr, ToSocketAddrs};

const TIMEOUT_MS: u32 = 6000;

/// Пинг адреса (host или IP). -1, если не разрешился IPv4 или недоступен.
pub fn ping(address: &str) -> i32 {
    let Some(ipv4) = resolve_ipv4(address) else {
        return -1;
    };
    #[cfg(windows)]
    {
        icmp_echo(ipv4)
    }
    #[cfg(not(windows))]
    {
        let _ = ipv4;
        -1
    }
}

/// Первый IPv4 из резолвинга `host:0`.
fn resolve_ipv4(address: &str) -> Option<Ipv4Addr> {
    if let Ok(ip) = address.parse::<Ipv4Addr>() {
        return Some(ip);
    }
    (address, 0)
        .to_socket_addrs()
        .ok()?
        .find_map(|sa| match sa.ip() {
            std::net::IpAddr::V4(v4) => Some(v4),
            _ => None,
        })
}

#[cfg(windows)]
fn icmp_echo(ip: Ipv4Addr) -> i32 {
    use windows::Win32::NetworkManagement::IpHelper::{
        IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY,
    };

    unsafe {
        let Ok(handle) = IcmpCreateFile() else {
            return -1;
        };

        // Небольшой полезный груз запроса.
        let send_data: [u8; 32] = [0x61; 32];
        // Буфер ответа: reply + данные (+ запас, как требует API).
        let reply_size = std::mem::size_of::<ICMP_ECHO_REPLY>() + send_data.len() + 8;
        let mut reply_buf = vec![0u8; reply_size];

        let dest = u32::from_le_bytes(ip.octets()); // IPAddr в сетевом порядке (LE octets)

        let ret = IcmpSendEcho(
            handle,
            dest,
            send_data.as_ptr() as *const _,
            send_data.len() as u16,
            None,
            reply_buf.as_mut_ptr() as *mut _,
            reply_size as u32,
            TIMEOUT_MS,
        );

        let result = if ret > 0 {
            let reply = &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY);
            // Status 0 = IP_SUCCESS; RoundTripTime в мс.
            if reply.Status == 0 {
                reply.RoundTripTime as i32
            } else {
                -1
            }
        } else {
            -1
        };

        let _ = IcmpCloseHandle(handle);
        result
    }
}
