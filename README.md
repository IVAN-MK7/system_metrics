# CLI tool for monitoring system resources usage, made with Rust 🦀

## Examples

#### Interactive CLI usage

```bash
cargo run --release -- --help
cargo run --release
```

```bash
cargo build --release
./target/release/system_metrics
```

#### Library usage

`./Cargo.toml`:

```toml
[dependencies]
system_metrics = { path = "../system_metrics" }
ctrlc = "3.5.2"
```

Rust code:

```rust
use std::{sync::mpsc, time::Duration};

use system_metrics::{TargetNetworkInterface, get_network_interfaces_names, get_system_stats};

// Set up a channel to listen for CTRL+C interrupts,
let (tx, rx) = mpsc::channel();

// and register the handler.
ctrlc::set_handler(move || {
    let _ = tx.send(());
})
.ok();

// Duration to measure the metrics over.
let duration = Duration::from_secs(3);

// Start the monitoring and retrieve the stats once it's done.
// Finish gathering early by pressing CTRL+C.
let system_stats = get_system_stats(TargetNetworkInterface::FirstActive, duration, rx).unwrap();
```
