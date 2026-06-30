use anyhow::Result;
use clap::Parser;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

use soltop::ui::App;
use soltop::{MonitorConfig, NetworkMonitor, TransportKind};

#[derive(Parser, Debug)]
#[command(name = "soltop")]
#[command(about = "Terminal UI for Solana programs monitoring", long_about = None)]
struct Args {
    /// Enable verbose performance statistics
    #[arg(short, long)]
    verbose: bool,

    /// RPC endpoint URL
    #[arg(
        long,
        default_value = "https://api.mainnet-beta.solana.com",
        help = "RPC endpoint URL"
    )]
    rpc_url: String,

    /// Hide system programs (Vote, ComputeBudget, System)
    #[arg(long)]
    hide_system: bool,

    /// Block source: `http` polling (default) or `ws` (blockSubscribe push)
    #[arg(long, value_enum, default_value_t = TransportKind::Http)]
    transport: TransportKind,

    /// WebSocket endpoint URL (defaults to ws(s):// derived from --rpc-url)
    #[arg(long)]
    ws_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Create configuration
    let config = MonitorConfig {
        rpc_url: args.rpc_url,
        window_duration: Duration::from_secs(5 * 60), // 5 minutes
        buffer_capacity: 750,
        poll_interval: Duration::from_millis(400),
        transport: args.transport,
        ws_url: args.ws_url,
    };

    // Create monitor
    let monitor = NetworkMonitor::new(config);

    // Get shared state reference for UI
    let network_state = monitor.get_state();

    // Spawn monitoring task in background
    tokio::spawn(async move {
        if let Err(e) = monitor.start().await {
            eprintln!("Monitor error: {}", e);
        }
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with the shared state
    let mut app = App::new(network_state);

    // Run the app
    let result = app.run(&mut terminal).await;

    // Cleanup: restore terminal
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Application error: {}", e);
    }

    Ok(())
}
