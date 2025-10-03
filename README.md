# mtm

Keep your screen awake by occasionally nudging the mouse pointer. The program waits a random amount of time between a lower and upper bound, then wiggles the cursor by a few pixels so the system stays active.

## Usage

```powershell
cargo run --release -- --lower 45s --upper 2m15s --distance 20 --verbose
```

### Arguments

- `--lower` / `-l`: Shortest interval between wiggles. Accepts values such as `30s`, `4m2s`, or `1h5m`. Default: `45s`.
- `--upper` / `-u`: Longest interval between wiggles. Same format as `--lower`. Must be greater than or equal to `--lower`. Default: `90s`.
- `--distance` / `-d`: Maximum pixel distance for each wiggle (the pointer moves there and back). Default: `15`.
- `--verbose` / `-v`: Print wait intervals and mouse-movement details each cycle.

Press `Ctrl+C` to stop the program.

## Notes

- Durations support hours (`h`), minutes (`m`), seconds (`s`), and milliseconds (`ms`). You can chain multiple units together, e.g. `1m30s`.
- The program moves the pointer out and then back, so it returns to its original position after each wiggle.
