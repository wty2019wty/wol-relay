//! Wake-on-LAN relay that listens for WOL packets on specified network interfaces and
//! relays them to other networks.
//!
//! Entry point: parses the CLI, sets up logging, spawns the relay reader thread and the
//! signal thread, and waits for shutdown.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::process::exit;
#[cfg(windows)]
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
#[cfg(windows)]
use std::time::Duration;

use clap::Parser;
#[cfg(windows)]
use windows_sys::Win32::System::Console::{
    CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler,
};

use wol_relay::cli::Cli;
use wol_relay::logging;
use wol_relay::network::{
    self, Ipv4Network, MAX_PACKET_SIZE, is_ip_in_any, local_ipv4_addrs, resolve_arg,
    send_wol_packet, sender_socket,
};
use wol_relay::wol;

/// Why the process is stopping.
enum Stop {
    /// SIGTERM/SIGINT received; the signal number is logged and the process exits
    /// cleanly.
    Signal(i32),
    /// The reader thread hit an unrecoverable error; exit nonzero so the supervisor
    /// restarts the whole instance.
    ReaderDied,
}

/// Logs a structured error record and exits with status 1.
///
/// Takes the same arguments as `log::error!`: optional `key = value` fields, then
/// `;` and the message. Must only be used after the logger is initialized.
macro_rules! fatal {
    ($($arg:tt)+) => {{
        log::error!($($arg)+);
        exit(1)
    }};
}

/// Spawn a named thread; a spawn failure is fatal.
fn spawn_thread(name: &str, f: impl FnOnce() + Send + 'static) {
    if let Err(err) = thread::Builder::new().name(name.to_owned()).spawn(f) {
        fatal!(thread = name, error:display = err; "failed to spawn thread");
    }
}

/// 将 SIGTERM/SIGINT 转发到停止通道（Unix）。
#[cfg(unix)]
fn install_signal_handlers(stop_tx: mpsc::Sender<Stop>) {
    let signals = signal_hook::iterator::Signals::new([
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGINT,
    ]);
    let mut signals = match signals {
        Ok(signals) => signals,
        Err(err) => fatal!(error:display = err; "failed to install signal handlers"),
    };

    spawn_thread("signals", move || {
        for sig in signals.forever() {
            let _ = stop_tx.send(Stop::Signal(sig));
        }
    });
}

/// 控制台事件处理函数只能是普通的 `extern "system"` 函数指针，无法携带闭包环境，
/// 因此把停止通道的发送端放进全局槽位。
#[cfg(windows)]
static STOP_TX: Mutex<Option<mpsc::Sender<Stop>>> = Mutex::new(None);

/// 将控制台控制事件转换成对应的 CRT 信号编号后转发给主线程（Windows）。
///
/// Windows 没有 Unix 信号，控制台控制事件承担同样的角色：Ctrl+C/Ctrl+Break
/// 对应 SIGINT，控制台关闭/注销/系统关机对应 SIGTERM。处理函数运行在系统创建
/// 的独立线程上，直接把消息发给主线程即可。
#[cfg(windows)]
fn install_signal_handlers(stop_tx: mpsc::Sender<Stop>) {
    *STOP_TX.lock().unwrap_or_else(|err| err.into_inner()) = Some(stop_tx);

    if unsafe { SetConsoleCtrlHandler(Some(handle_console_event), 1) } == 0 {
        let err = std::io::Error::last_os_error();
        fatal!(error:display = err; "failed to install signal handlers");
    }
}

/// 控制台控制事件的回调：转发事件后返回 1（已处理，不再传给后续处理函数）。
#[cfg(windows)]
unsafe extern "system" fn handle_console_event(event: u32) -> i32 {
    // CRT 的信号编号与 Unix 保持一致（SIGINT = 2，SIGTERM = 15），
    // 让日志里的 signal 字段在各平台上口径统一。
    let (sig, grace) = match event {
        CTRL_C_EVENT | CTRL_BREAK_EVENT => (2, false),
        _ => (15, true),
    };

    {
        let stop_tx = STOP_TX.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(stop_tx) = stop_tx.as_ref() {
            let _ = stop_tx.send(Stop::Signal(sig));
        }
    }

    if grace {
        // 控制台关闭/注销/系统关机只给几秒钟收尾时间：稍作等待让主线程记录
        // 日志并干净退出，处理函数返回后系统会直接结束进程。
        thread::sleep(Duration::from_secs(2));
    }

    1
}

/// Block until shutdown is requested; never returns normally.
///
/// * `stop_rx` — receives a signal notification or a reader death.
fn wait_for_stop(stop_rx: mpsc::Receiver<Stop>) -> ! {
    match stop_rx.recv() {
        Ok(Stop::Signal(sig)) => {
            log::info!(signal = sig; "stopping on signal");
            exit(0);
        }

        Ok(Stop::ReaderDied) | Err(_) => fatal!("stopping: reader thread failed"),
    }
}

/// Resolve every CLI target to its monitored IPv4 networks, dropping duplicates while
/// preserving order.
///
/// Aborts on the first unknown target.
fn resolve_networks(targets: &[String]) -> Vec<Ipv4Network> {
    let mut networks = Vec::new();
    for target in targets {
        let resolved = match resolve_arg(target) {
            Ok(resolved) => resolved,
            Err(err) => {
                fatal!(
                    argument = target.as_str(),
                    error:display = err;
                    "failed to resolve argument"
                )
            }
        };

        // An interface with several addresses (e.g. loopback aliases) can resolve to
        // the same network more than once; each network only needs one relay.
        for network in resolved {
            if !networks.contains(&network) {
                networks.push(network);
            }
        }
    }

    if networks.is_empty() {
        fatal!(arguments:serde = targets; "no valid networks found");
    }

    networks
}

/// Relays a WOL packet for `mac` to every monitored network other than the source's
/// own, logging the outcome for each.
fn relay_packet(
    sender: &UdpSocket,
    networks: &[Ipv4Network],
    source_ip: Ipv4Addr,
    mac: wol::Mac,
) {
    let target_mac = wol::mac_to_string(&mac);

    for network in networks {
        // Don't send the WOL packet to the same network as the source IP to avoid
        // broadcasting the packet a second time on the original network.
        if network.contains(source_ip) {
            continue;
        }

        match send_wol_packet(sender, *network, mac) {
            Ok(()) => log::info!(
                source_ip:display = source_ip,
                target_network:display = network,
                target_mac = target_mac;
                "relayed packet"
            ),

            Err(err) if network::is_transient(&err) => log::debug!(
                source_ip:display = source_ip,
                target_network:display = network,
                target_mac = target_mac,
                error:display = err;
                "skipped relay: send queue full"
            ),

            Err(err) => log::warn!(
                source_ip:display = source_ip,
                target_network:display = network,
                target_mac = target_mac,
                error:display = err;
                "failed to relay packet"
            ),
        }
    }
}

/// Blocks forever reading WOL datagrams on `listener` and relaying them to every
/// monitored network other than the source's own. On a hard receive error it logs,
/// sends `Stop::ReaderDied` on `stop`, and returns.
///
/// * `listener` — the relay's receive socket (bound to `0.0.0.0`).
/// * `sender` — the relay's send socket (broadcast enabled).
/// * `networks` — monitored networks; sources outside them are skipped.
/// * `self_ips` — this host's addresses; packets from them are skipped.
/// * `stop` — signalled once before returning on a hard receive error.
fn reader_loop(
    listener: UdpSocket,
    sender: UdpSocket,
    networks: Vec<Ipv4Network>,
    self_ips: Vec<Ipv4Addr>,
    stop: mpsc::Sender<Stop>,
) {
    let mut buffer = [0u8; MAX_PACKET_SIZE];

    loop {
        let (n, source) = match listener.recv_from(&mut buffer) {
            Ok(result) => result,

            Err(err) if network::is_transient(&err) => continue,

            Err(err) => {
                log::error!(error:display = err; "failed to receive packet");
                let _ = stop.send(Stop::ReaderDied);
                return;
            }
        };

        // An IPv4 socket only yields IPv4 sources; IPv6 is unreachable.
        let source_ip = match source {
            SocketAddr::V4(addr) => *addr.ip(),
            SocketAddr::V6(addr) => {
                log::debug!(
                    source_ip:display = addr.ip();
                    "skipped packet: IPv6 source"
                );
                continue;
            }
        };

        // Ignore packets from networks that we are not monitoring.
        if !is_ip_in_any(source_ip, &networks) {
            log::debug!(
                source_ip:display = source_ip;
                "skipped packet: unmonitored network"
            );
            continue;
        }

        // Skip packets from this host to prevent relay loops.
        if self_ips.contains(&source_ip) {
            log::debug!(
                source_ip:display = source_ip;
                "skipped packet: sent by this host"
            );
            continue;
        }

        let mac = match wol::parse_packet(&buffer[..n]) {
            Ok(mac) => mac,
            Err(err) => {
                log::warn!(
                    source_ip:display = source_ip,
                    packet_size = n as u64,
                    error:display = err;
                    "failed to parse packet"
                );
                continue;
            }
        };

        relay_packet(&sender, &networks, source_ip, mac);
    }
}

fn main() {
    let cli = Cli::parse();

    logging::init(cli.log_level, cli.log_format);

    let networks = resolve_networks(&cli.targets);

    let self_ips = match local_ipv4_addrs() {
        Ok(ips) => ips,
        Err(err) => {
            fatal!(error = format!("{err:#}"); "failed to list local addresses")
        }
    };

    let listener = match UdpSocket::bind(("0.0.0.0", wol::DEFAULT_PORT)) {
        Ok(listener) => listener,
        Err(err) => fatal!(
            port = wol::DEFAULT_PORT,
            error:display = err;
            "failed to bind listener"
        ),
    };

    let sender = match sender_socket() {
        Ok(sender) => sender,
        Err(err) => fatal!(error = format!("{err:#}"); "failed to create sender"),
    };

    let network_names: Vec<String> = networks.iter().map(|n| n.to_string()).collect();
    log::info!(
        port = wol::DEFAULT_PORT,
        networks:serde = network_names;
        "listening for packets"
    );

    let (stop_tx, stop_rx) = mpsc::channel::<Stop>();

    spawn_thread("reader", {
        let stop_tx = stop_tx.clone();
        move || reader_loop(listener, sender, networks, self_ips, stop_tx)
    });

    install_signal_handlers(stop_tx);
    wait_for_stop(stop_rx);
}
