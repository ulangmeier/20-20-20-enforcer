# 20-20-20 Enforcer

A lightweight Windows system-tray app that enforces the **20-20-20 rule** to reduce eye strain:

> Every **20 minutes**, look at something **20 feet away** for **20 seconds**.

## What it does

- Sits silently in the system tray with an eye icon.
- Every 20 minutes it covers the screen with a black fullscreen overlay for 20 seconds, prompting you to rest your eyes.
- The tray tooltip counts down to the next break (or shows the remaining break time).
- Automatically registers itself to run at Windows startup.
- Right-click the tray icon → **Quit** to exit.

## Usage

Just run `twenty-twenty-twenty.exe`. No configuration needed.

### Test mode

Pass `--test` to use shortened timers (10-second intervals, 5-second breaks) for quick verification:

```
twenty-twenty-twenty.exe --test
```

## Building from source

Requires [Rust](https://rustup.rs/) and the MSVC toolchain on Windows.

```
cargo build --release
```

The compiled binary will be at `target/release/twenty-twenty-twenty.exe`.

## License

This project is released into the public domain under the [Unlicense](LICENSE).
