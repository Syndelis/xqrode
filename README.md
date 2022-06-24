# Wayland Screencopy Utilites

This repository contains a library (gazo) and binary tools (gazo-cli and qrode) written in Rust. Gazo can be used as a library or cli tool to capture the screen output of Wayland compositors implementing the wlr screencopy protocol. Qrode uses gazo to capture a region of the screen and decode QR codes; the QR code is then opened as a link, if that fails it is opened as a text file.

If anything is unclear or not working, please open an issue.

Please remember these projects are still in early development and there will be bugs. One current limitation of qrode is that it tends to fail to decode non-standard QR codes. Any problems with decoding QR codes should be reported to the <a href = "https://github.com/WanzenBug/rqrr" target = "_blank">rqrr</a> crate (please don't bother them too much and make sure not to open duplicate issues).

## Installation

### Manual

Clone the repository using git and cd into it.

```bash
git clone https://gitlab.com/redArch/wayland-screencopy-utilities
cd wayland-screencopy-utilities
```

Qrode and gazo-cli can be built using Cargo.

```bash
cargo build --release --target-dir qrode # or gazo-cli
```

The built binary can be found in "./target/release/".

### AUR

Qrode and gazo are available in the AUR as "qrode-git" and "gazo-git", respectively.

```bash
yay -S qrode-git
yay -S gazo-git
```

## Usage

Qrode works best with <a href = "https://github.com/emersion/slurp" target = "_blank">slurp</a>. See below for typical usage.

```bash
qrode -g "$(slurp)"
```

Gazo-cli partially emulates grim in terms of its arguments, but it is not a complete drop in replacement (although that may or may not change). See below for example uses.

```bash
gazo screenshot.png # capture all outputs and save in screenshot.png

gazo -o "DP-1" screenshot.png # capture the output named "DP-1"

gazo -g "$(slurp)" screenshot.png # capture the region specified by slurp
```

Some notes about gazo: it only supports PNG images currently (this may or may not change); there is no level flag for PNG compression as the library used does not have an option for setting the compression level; and Wayland output scaling is handled, but a scaling factor cannot be set like grim (scaling factor is 1 for all images for now).

Both qrode and gazo have a help option if you get stuck or want to see other options that may not be listed here.

```bash
qrode -h # print the qrode help
gazo -h  # print the gazo help
```
## Acknowledgments
These projects depend heavily on other projects, and without them it would not have been possible for me to create qrode and gazo. A non-exhaustive list of helpful projects I would like to thank:

- The <a href = "https://smithay.github.io/" target = "_blan">Smithay project</a> wayland-client and wayland-protocols crates
- The <a href = "https://github.com/WanzenBug/rqrr" target = "_blank">rqrr</a> crate, used to decode QR codes for qrode
- The <a href = "https://github.com/brion/mtpng" target = "_blank">mtpng</a> crate, used to encode image data into the PNG format extremely quickly
- The <a href = "https://github.com/clap-rs/clap" target = "_blank">clap</a> crate, used to parse arguments from the command line
- The <a href = "https://github.com/razrfalcon/memmap2-rs" target = "_blank">memmap2</a> crate, used to map memory backed files for easy manipulation

## Contributing
If you'd like to contribute, please open an issue. This repository is still in early development, so there will probably be a lot of major changes that will make collaborating difficult.

## License
[GPL-3.0](https://choosealicense.com/licenses/gpl-3.0/)

## Donations

If you like this project and want to support me, you can <a href = "https://www.buymeacoffee.com/redarch3" target = "_blank">buy me a coffee</a>.