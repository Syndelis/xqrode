# Wayland Screencopy Utilites

This repository contains a library (gazo) and binary tools (gazo and qrode) written in Rust. Gazo can be used as a library or cli tool to capture the screen output of Wayland compositors implementing the wlr screencopy protocol. Qrode uses gazo to capture a region of the screen and decode QR codes; the QR code is then opened as a link, if that fails it is opened as a text file.

## Installation

TODO

```bash
TODO
```

## Usage

Gazo partially emulates grim in terms of its arguments, but it is not a complete drop in replacement (although that may or may not change).

```
$ gazo -h
gazo 0.0.1
redArch <redarch@protonmail.com>
Screenshot tool for Wayland compositors

USAGE:
    gazo [OPTIONS] <OUTPUT_FILE>

ARGS:
    <OUTPUT_FILE>    Location to save the image. Image type is PNG.

OPTIONS:
    -c                   Include cursors in the screenshot.
    -g <GEOMETRY>        Set the region to capture
    -h, --help           Print help information
    -o <OUTPUT>          Set the output name to capture.
    -V, --version        Print version information
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[GPL-3.0](https://choosealicense.com/licenses/gpl-3.0/)
