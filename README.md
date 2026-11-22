# rxtop

A minimal terminal process resource manager written in Rust with Ratatui.

The process table shows each process name, PID, CPU usage, and memory use. It sorts by CPU usage and refreshes once per second. Search matches process names and PIDs.

## Controls

| Key | Action |
| --- | --- |
| Up / Down | Select a process |
| `/` | Search process names and PIDs |
| Enter | Keep the search filter |
| Escape | Clear the search filter |
| `k` | Request termination of the selected process |
| `q` | Quit |

## Run

Install from crates.io, then run rxtop:

```sh
cargo install rxtop
rxtop
```
