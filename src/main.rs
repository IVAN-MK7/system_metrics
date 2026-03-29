use std::{error::Error, sync::mpsc, time::Duration};

use clap::Parser;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use system_metrics::{TargetNetworkInterface, get_network_interfaces_names, get_system_stats};

/// A CLI tool to monitor system resources usage and print it as JSON.
///
/// An interactive prompt will be displayed for the not provided arguments.
///
/// OUTPUT FORMAT:
/// {
///   "cpu_usage_percent": 14.2,  // Average CPU usage percentage (1 decimal place)
///   "ram_used_mb": 6241,        // Initial Used RAM in MegaBytes
///   "ram_total_mb": 16384,      // Total physical RAM in MegaBytes
///   "network": {                // Omitted if no network interface is found
///     "upload_kbps": 12.5,      // Average upload speed in Kilobits/s (1 decimal place)
///     "download_kbps": 405.2    // Average download speed in Kilobits/s (1 decimal place)
///   }
/// }
///
/// EXAMPLES:
///   system_metrics                                   // Interactive mode
///   system_metrics --list                            // List available interfaces
///   system_metrics --network first --duration 7      // First active, 7 seconds
///   system_metrics -n index:0 -d 1                   // Index 0, 1 second
///   system_metrics -n name:eth0                      // Specific name, prompts for time
#[derive(Parser, Debug)]
#[command(author, version, about, verbatim_doc_comment)]
struct Cli {
    /// List all available network interfaces and exit.
    #[arg(short, long, verbatim_doc_comment)]
    list: bool,

    /// Target network interface to monitor.
    ///
    /// Valid formats:
    /// - 'first'         : Sorts alphabetically, picks the first that performed network traffic.
    /// - 'index:<num>'   : Sorts alphabetically, picks by index (e.g.: index:0).
    /// - 'name:<string>' : Picks by exact network interface name (e.g.: name:eth0).
    #[arg(short, long, verbatim_doc_comment)]
    network: Option<String>,

    /// Duration to measure the metrics over, in seconds.
    #[arg(short, long, verbatim_doc_comment)]
    duration: Option<u64>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if cli.list {
        let network_names = get_network_interfaces_names();

        println!("Available network interfaces:");
        for (index, name) in network_names.iter().enumerate() {
            println!("  [{}] '{}'", index, name);
        }
        return Ok(());
    }

    let duration = {
        let duration_secs = match cli.duration {
            Some(d) => d,
            None => Input::<u64>::new().with_prompt("Enter duration to measure (in seconds)").default(1).interact()?,
        };
        Duration::from_secs(duration_secs)
    };

    let network_interface = {
        if let Some(net_arg) = cli.network {
            if net_arg == "first" {
                TargetNetworkInterface::FirstActive
            } else if let Some(index_str) = net_arg.strip_prefix("index:") {
                let index: usize = index_str.parse().map_err(|_| "Invalid index format")?;
                TargetNetworkInterface::Index(index)
            } else if let Some(name) = net_arg.strip_prefix("name:") {
                TargetNetworkInterface::Name(name.to_string())
            } else {
                return Err("Invalid network argument format. Use 'first', 'index:<N>', or 'name:<STR>'".into());
            }
        } else {
            // No CLI argument provided, display an interactive prompt.
            let network_names = get_network_interfaces_names();

            // Proceed even if no network interface has been found.
            let mut options = vec!["Auto (first with network traffic)".to_string()];
            for (index, name) in network_names.iter().enumerate() {
                options.push(format!("Index {}: {}", index, name));
            }

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Choose the network interface to monitor")
                .default(0)
                .items(&options)
                .interact()?;

            if selection == 0 { TargetNetworkInterface::FirstActive } else { TargetNetworkInterface::Index(selection - 1) }
        }
    };

    // Setup a channel to listen for CTRL+C interrupts,
    let (tx, rx) = mpsc::channel();

    // and register the handler.
    ctrlc::set_handler(move || {
        let _res: Result<(), mpsc::SendError<()>> = tx.send(());
        println!("CTRL+C detected. Stopping data gathering early...")
    })
    .ok();

    println!("Gathering metrics... (press CTRL+C to finish early)");

    // Start the monitoring and retrieve the stats once it's done.
    let system_stats = get_system_stats(network_interface, duration, rx)?;

    // Serialize to pretty JSON.
    let system_stats_json = match serde_json::to_string_pretty(&system_stats) {
        Ok(json) => json,
        Err(e) => return Err(format!("Error serializing to JSON: {}", e).into()),
    };

    println!("{system_stats_json}");

    Ok(())
}
