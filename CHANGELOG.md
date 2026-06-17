# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- RPC client now enforces a 5s connect timeout and 15s request timeout. An unreachable or silently-dropping `--rpc-url` previously hung the monitor indefinitely before any error could be shown; it now surfaces as a visible RPC error on the loading screen.
- Removed `println!`/`eprintln!` calls in the slot monitor that wrote to the same terminal ratatui draws on, corrupting the TUI (e.g. a stray "Consumer shutting down" line). Fatal producer/consumer errors now surface via the on-screen RPC error indicator instead.

### Added
- Regression test verifying the program statistics table scrolls to keep the selected row visible when navigating past the visible area.

## [0.1.0] - 2025-12-29

### Added
- Initial public release of soltop
- Real-time Solana program monitoring via terminal UI
- Interactive terminal UI powered by ratatui
- Network overview panel displaying:
  - Current slot and network slot with lag tracking
  - Uptime and monitoring window duration
  - Total TPS, transaction count, and average success rate
  - Total compute units consumed per second
- Program statistics table showing:
  - Program ID (with truncation toggle)
  - Transactions per second (TPS)
  - Total transaction count
  - Success rate percentage
  - Compute units per second
  - Average compute units per transaction
- Keyboard controls:
  - `q` - Quit application
  - `t` - Toggle program ID truncation
  - `u` - Toggle system program visibility
  - `w` - Toggle view mode (Live vs Window)
- Command-line options:
  - `--rpc-url` - Custom RPC endpoint URL
  - `--hide-system` - Hide system programs by default
  - `--verbose` - Enable verbose performance statistics
- System program filtering (Vote, ComputeBudget, System, Token programs)
- Ring buffer-based statistics aggregation with 5-minute rolling window
- Loading screen during initial data retrieval
- Theme support with Flatline color scheme
- Producer-consumer architecture for efficient slot processing
- Async/await implementation with tokio runtime

### Technical Details
- Rust 1.75+ required
- Linux x86_64 binary distribution
- RPC polling interval: 400ms per slot
- Statistics window: 5 minutes (750 slots)
- Memory usage: ~10-50MB
- CPU usage: <5%
- Network bandwidth: ~100KB/sec

### Known Limitations
- Platform support limited to Linux x86_64 (macOS and Windows support planned)
- No crates.io distribution yet (GitHub releases only)
- Keyboard navigation in program list not implemented
- Compute unit metrics may be incomplete for programs without verbose transaction logs

[Unreleased]: https://github.com/jacquesmats/soltop-oss/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jacquesmats/soltop-oss/releases/tag/v0.1.0
