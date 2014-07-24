## Introduction

playform aspires to be an open-world sandbox game written in Rust.
Right now, it just renders a basic world and stops the player from colliding
with it.

Help is appreciated! I'm looking to incorporate parts of
[https://github.com/rlane/cubeland](https://github.com/rlane/cubeland)
once this is a little more developed.

## Making it work

It can be built with `cargo build`, which should grab dependencies
automatically. `playform` can then be run from the `target` directory.

The camera can be moved using `WASD` controls, as well as `Space` and `LShift`,
and the view can be rotated using the camera or the arrow keys.
Using the mouse to click on a block will cause it to disappear.

## Screenshots

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)
![screenshot 3](/../screenshots/screenshots/screenshot3.png?raw=true)
