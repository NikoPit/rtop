# rtop

A minimal terminal process resource manager written in Rust with Ratatui.

The process table shows each process name, PID, CPU usage, and memory use. The summary shows total CPU usage and used memory. The table sorts by CPU usage and refreshes once per second.

## Controls

| Key | Action |
| --- | --- |
| Up / Down | Select a process |
| `k` | Request termination of the selected process |
| `q` | Quit |

## Run

The crates.io name `rtop` is already in use. Until this project is published under an available name, run it from this checkout:

```sh
cargo run --release
```
