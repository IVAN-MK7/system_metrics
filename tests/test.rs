use std::{sync::mpsc, time::Duration};

use system_metrics::{TargetNetworkInterface, get_network_interfaces_names, get_system_stats};

#[test]
fn nework_interfaces_sample_test_json() {
    let network_names = get_network_interfaces_names();

    // Proceed even if no network interface has been found.
    println!("Available network interfaces:");
    for (index, name) in network_names.iter().enumerate() {
        println!("  [{}] '{}'", index, name);
    }

    // Setup a channel to listen for CTRL+C interrupts,
    let (tx, rx) = mpsc::channel();

    // and register the handler.
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })
    .ok();

    // Duration to measure the metrics over.
    let duration = Duration::from_secs(1);

    // Start the monitoring and retrieve the stats once it's done.
    let system_stats = get_system_stats(TargetNetworkInterface::FirstActive, duration, rx).unwrap();

    // Serialize to pretty JSON.
    let system_stats_json = match serde_json::to_string_pretty(&system_stats) {
        Ok(json) => json,
        Err(e) => panic!("Error serializing to JSON: {}", e),
    };

    println!("{system_stats_json}");
}
