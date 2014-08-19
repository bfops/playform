## Introduction

playform aspires to be an open-world sandbox game written in Rust.
Right now, it renders a basic world and stops the player from colliding
with it, as well as allowing the player to place and destroy blocks.

Help is appreciated! I'm looking to incorporate parts of
[https://github.com/rlane/cubeland](https://github.com/rlane/cubeland)
once this is a little more developed.

## Making it work

It can be built with `cargo build`, which should grab dependencies
automatically. `playform` can then be run from the `target` directory.

The player can be moved using `WASD` controls, as well as `Space` and `LShift`,
and the view can be rotated using the mouse or the arrow keys.
Using the mouse to click on a block will cause it to disappear, and
right-clicking will place a block of dirt.

One mob spawns that will play a tag-lke game with you: once you touch it, it
will follow you until it touches you, at which point it will stop again.

## Screenshots

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)
![screenshot 3](/../screenshots/screenshots/screenshot3.png?raw=true)
![screenshot 4](/../screenshots/screenshots/screenshot4.png?raw=true)
