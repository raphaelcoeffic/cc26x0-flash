# cc26x0-flash

A [probe-rs](https://probe.rs) flash algorithm for the TI CC26x0 (CC2640, CC2650):
128 KB of flash in 4 KB sectors, the last one holding the CCFG.

Erase and program call the flash functions of the boot ROM, found through the ROM API
table, as TI's driverlib and OpenOCD's cc26xx loader do. The flash cache is turned off
while the algorithm runs.

A chip erase erases every sector and then writes back the security part of the CCFG
(boot loader backdoor off, debug access enabled): an all-ones CCFG would lock out the
debugger at the next reset. The rest of the CCFG stays erased, so the boot ROM doesn't
start the image until a CCFG is programmed.

## Building

```sh
rustup target add thumbv7m-none-eabi
cargo build --release
```

## Testing on a chip

`cargo run --release` runs `target-gen test` on an attached chip (see `template.yaml`).
It erases the whole chip.

```sh
cargo install target-gen
cargo run --release -- --probe <VID:PID:serial> --test-address 0x10000
```

## Updating probe-rs

```sh
target-gen elf --fixed-load-address --name cc26x0-flash \
    target/thumbv7m-none-eabi/release/cc26x0-flash cc26x0.yaml
```

and copy the algorithm into `probe-rs/targets/CC13XX_CC26XX_Series.yaml`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
