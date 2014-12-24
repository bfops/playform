[![Build Status](https://travis-ci.org/bfops/playform.svg?branch=master)](https://travis-ci.org/bfops/playform)

## Introduction

playform aspires to be an open-world sandbox game written in Rust, taking
inspiration from [Voxel Farm](http://procworld.blogspot.com/) and Minecraft.

Our goal is not necessarily to make something full or flashy, but to make
something that people will want to fork.

Help is appreciated! You can hop over to the [issues page](https://github.com/bfops/playform/issues) to see what needs doing.

## Making it work

Have a `rustc` and `cargo` built no earlier than December 23rd, 2014.
Install `libpng`, `SDL2` and `SDL2_ttf`.
Run `cargo run`! Consider setting `RUST_BACKTRACE=1` and `RUST_LOG=info` when you run playform.

## How to play

  * Move: WASD
  * Jump: Space
  * Look around: Mouse
  * Toggle block outline rendering: L
  * Save line-of-sight: M

One mob spawns that will play "tag" with you: tag it and it will chase you until it tags you back.

## If things don't work

If things are broken, like compile errors, problems getting playform to start, or you get crashes, please consider opening an issue!

If things are laggy, there are some constants scattered around that you can tweak;
the settings in `surroundings_loader.rs` (especially `LOAD_DISTANCE`) will probably make the biggest difference.

## Screenshots

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)
