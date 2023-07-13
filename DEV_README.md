
The documentation assume a working Linux environment.

# Create executable

## Linux

```bash
cargo build --release
```
The executable is created in the `target/release` directory.

## Cross-compile to Windows

```bash
cargo install cross
cross build --target x86_64-pc-windows-gnu --release
```
The executable is created in the `target/x86_64-pc-windows-gnu/release` directory.
