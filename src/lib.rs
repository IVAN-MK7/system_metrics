use std::error::Error;
use std::fmt;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sysinfo::{Networks, System};

#[derive(Debug, Clone)]
pub enum SystemStatsError {
    /// Returned when the monitoring duration is effectively zero,
    /// to prevent the metrics from being devided by zero for the averages' calculations.
    InsufficientTimeElapsed,
}

impl fmt::Display for SystemStatsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemStatsError::InsufficientTimeElapsed => {
                write!(f, "Not enough time elapsed for the system metrics gathering.")
            }
        }
    }
}

impl Error for SystemStatsError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SystemStats {
    /// Average CPU usage in percentage (%), with one decimal place.
    cpu_usage_percent: f32,
    /// RAM usage at the start of the monitoring.
    ram_used_mb: u64,
    /// Total system RAM.
    ram_total_mb: u64,
    /// Network stats, omitted if the network interface has not been found.
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<NetworkStats>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Average upload speed in Kilobits/s, with one decimal place.
    upload_kbps: f32,
    /// Average download speed in Kilobits/s, with one decimal place.
    download_kbps: f32,
}

/// Network interface selection method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetNetworkInterface {
    /// Automatically picks the first interface that is actively transferring data.
    FirstActive,
    /// Selects an interface by its alphabetically sorted index.
    Index(usize),
    /// Selects an interface by its exact name (e.g.: "eth0").
    Name(String),
}

/// Returns the `SystemStats` metrics gathered over the provided `duration`,
/// or early if `rx` gets triggered.
///
///
/// # Example
///
/// ```
/// use std::{sync::mpsc::{self, Sender, Receiver}, time::Duration};
///
/// use system_metrics::{TargetNetworkInterface, get_system_stats};
///
/// // Set up a channel to listen for CTRL+C interrupts,
/// let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();
///
/// // and register the handler.
/// ctrlc::set_handler(move || {
///     let _res: Result<(), mpsc::SendError<()>> = tx.send(());
/// })
/// .ok();
///
/// // Start the monitoring and retrieve the stats once it's done.
/// let system_stats = get_system_stats(TargetNetworkInterface::FirstActive, Duration::from_secs(1), rx).unwrap();
/// ```
pub fn get_system_stats(
    network_interface: TargetNetworkInterface,
    duration: Duration,
    rx: Receiver<()>,
) -> Result<SystemStats, SystemStatsError> {
    // Initialize system and network objects.
    let mut system = System::new_all();
    let mut networks = Networks::new_with_refreshed_list();

    // Define a baseline for the metrics.
    system.refresh_cpu_usage();
    networks.refresh(true);

    let start_time = Instant::now();

    // Wait until either the duration elapses or a cancellation signal is sent through the channel.
    let _ = rx.recv_timeout(duration);

    let elapsed = start_time.elapsed();

    // Refresh to collect the usage occurred over the elapsed time.
    system.refresh_cpu_usage();
    system.refresh_memory();
    networks.refresh(true);

    // CPU usage %.
    let cpu_usage_percent = system.global_cpu_usage();

    // RAM usage in MB.
    let ram_used_mb = system.used_memory() / (1024 * 1024);
    let ram_total_mb = system.total_memory() / (1024 * 1024);

    // Single network interface throughput, in kilobits/s.
    let mut upload_kbps = 0.0;
    let mut download_kbps = 0.0;
    let mut network_found = false;

    // Prevent division by zero if the receiver woke up immediately.
    let time_divisor = match elapsed.as_secs_f32() {
        s if s > 0.0 => s,
        _ => return Err(SystemStatsError::InsufficientTimeElapsed),
    };

    // Sort the network interfaces' names to ensure predictable indexing.
    let mut network_names: Vec<&String> = networks.keys().collect();
    network_names.sort();

    match network_interface {
        TargetNetworkInterface::FirstActive => {
            for (_, data) in &networks {
                let tx_bytes = data.transmitted();
                let rx_bytes = data.received();

                if tx_bytes > 0 || rx_bytes > 0 {
                    upload_kbps = ((tx_bytes as f32 * 8.0) / 1024.0) / time_divisor;
                    download_kbps = ((rx_bytes as f32 * 8.0) / 1024.0) / time_divisor;
                    network_found = true;
                    break;
                }
            }
        }
        TargetNetworkInterface::Index(index) => {
            if let Some(&name) = network_names.get(index) {
                if let Some((_, data)) = networks.iter().find(|(n, _)| *n == name) {
                    upload_kbps = ((data.transmitted() as f32 * 8.0) / 1000.0) / time_divisor;
                    download_kbps = ((data.received() as f32 * 8.0) / 1000.0) / time_divisor;
                    network_found = true;
                }
            } else {
                eprintln!("Warning: network interface index {} is out of bounds.", index);
            }
        }
        TargetNetworkInterface::Name(target_name) => {
            if let Some((_, data)) = networks.iter().find(|(n, _)| **n == target_name) {
                upload_kbps = ((data.transmitted() as f32 * 8.0) / 1000.0) / time_divisor;
                download_kbps = ((data.received() as f32 * 8.0) / 1000.0) / time_divisor;
                network_found = true;
            } else {
                eprintln!("Warning: network interface name '{}' not found.", target_name);
            }
        }
    }

    let network_stats = if network_found {
        Some(NetworkStats { upload_kbps: (upload_kbps * 10.0).round() / 10.0, download_kbps: (download_kbps * 10.0).round() / 10.0 })
    } else {
        None
    };

    Ok(SystemStats { cpu_usage_percent: (cpu_usage_percent * 10.0).round() / 10.0, ram_used_mb, ram_total_mb, network: network_stats })
}

pub fn get_network_interfaces_names() -> Vec<String> {
    let networks = Networks::new_with_refreshed_list();
    let mut network_names: Vec<String> = networks.keys().map(|n| n.to_string()).collect();
    network_names.sort();
    network_names
}
