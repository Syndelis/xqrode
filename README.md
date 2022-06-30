# XQRode

This repository is a re-distribution of [QRode](https://gitlab.com/redArch/wayland-screencopy-utilities) with X11 support (doesn't support Wayland, check the original if you need it).

In order to do so, I've substituted Will's "gazo" library, responsible for capturing the screen, with a re-distribution of neXromancers' [shotgun](https://github.com/neXromancers/shotgun). "shotgun" is available only as a binary with no internal library crate, so I've taken the liberty of stripping out the necessary parts for this software to work and bundled them as [libshotgun](/libshotgun/).

below you'll find most of the original README file from the original QRode (apart from wayland/x11 changes).

---

Please remember these projects are still in early development and there will be bugs. One current limitation of qrode is that it tends to fail to decode non-standard QR codes. Any problems with decoding QR codes should be reported to the <a href = "https://github.com/WanzenBug/rqrr" target = "_blank">rqrr</a> crate (please don't bother them too much and make sure not to open duplicate issues).

## Installation

```bash
cargo install --path qrode
```

The built binary can be found in "./target/release/".

## Usage

Qrode works best with [xrectsel](https://github.com/ropery/xrectsel). See below for typical usage.

```bash
xqrode -g "$(xrectsel)"
```

![Example usage of qrode in a GIF](/xqrode_example.gif)

## Acknowledgments
These projects depend heavily on other projects, and without them it would not have been possible for me to create qrode and gazo. A non-exhaustive list of helpful projects I would like to thank:

- [QRode] The <a href = "https://smithay.github.io/" target = "_blan">Smithay project</a> wayland-client and wayland-protocols crates
- [QRode] The <a href = "https://github.com/WanzenBug/rqrr" target = "_blank">rqrr</a> crate, used to decode QR codes for qrode
- [QRode] The <a href = "https://github.com/brion/mtpng" target = "_blank">mtpng</a> crate, used to encode image data into the PNG format extremely quickly
- [QRode] The <a href = "https://github.com/clap-rs/clap" target = "_blank">clap</a> crate, used to parse arguments from the command line
- [QRode] The <a href = "https://github.com/razrfalcon/memmap2-rs" target = "_blank">memmap2</a> crate, used to map memory backed files for easy manipulation
- [XQRode] The [shotgun](https://github.com/neXromancers/shotgun) binary crate, used to capture X11 screen areas
- [XQRode] Of course, the original [QRode](https://gitlab.com/redArch/wayland-screencopy-utilities), which not only serves as the base of this project but also was of great inspiration for me

## Donations

If you like this project, please support the original created by Will by <a href = "https://www.buymeacoffee.com/redarch3" target = "_blank">buying him a coffee</a>.

---

## Footnote: Licenses, Crediting and Open Source Code

This is a first for me when it comes to publishing a repository where most of the code was written by other people. I've tried to respect their work by crediting them wherever I could and by honouring their linceses (to the extent I understand). If you think I've taken the wrong approach at something, please let me know and I'll fix it up as soon as possible.

I also want to make it clear that I do not claim ownership of anything in this project. Even the lines of code I wrote were heavily inspired by their work. All I wanted to do is make a fantastic little tool available for more people.