[![Build Status](https://travis-ci.org/bfops/playform.svg?branch=master)](https://travis-ci.org/bfops/playform)

## Introduction

playform aspires to be an open-world sandbox game written in Rust, taking
inspiration from [Voxel Farm](http://procworld.blogspot.com/) and Minecraft.

Help is great! PRs and [issues](https://github.com/bfops/playform/issues) are appreciated.

## Making it work

Have a `rustc` and `cargo` built no earlier than January 5th, 2015.
Install `libpng`, `SDL2` and `SDL2_ttf`.
Run `cargo run`! Consider setting `RUST_BACKTRACE=1` and `RUST_LOG=info` when you run playform.

**Some dependencies may not build**. Look for forks that are updated for your `rustc`,
and then point your `~/.cargo/config` at them.

## How to play

  * Move: WASD
  * Jump: Space
  * Look around: Mouse
  * Toggle block outline rendering: L
  * Save line-of-sight: M

One mob spawns that will play "tag" with you: tag it and it will chase you until it tags you back.
Watch out though, if you move too far away, it'll fall off the world.

## If things don't work

If things are broken, like compile errors, problems getting it to start, crashes, etc.
please consider opening an issue!

If things are laggy, there are some constants scattered around that you can tweak;
the settings in `surroundings_loader.rs` (especially `LOAD_DISTANCE`) probably matter the most.

## Screenshots

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)
