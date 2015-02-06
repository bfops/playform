[![Build Status](https://travis-ci.org/bfops/playform.svg?branch=master)](https://travis-ci.org/bfops/playform)

## Introduction

Playform aspires to be an open-world sandbox game written in Rust, taking
inspiration from [Voxel Farm](http://procworld.blogspot.com/) and Minecraft.

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)

Help is great! PRs and [issues](https://github.com/bfops/playform/issues) are appreciated.

## Making it work

Install the Rust compiler and `cargo`, as well as `libpng`, `SDL2` and `SDL2_ttf`.
Run `cargo run`! `cargo run --release` for a slower build, but faster playform.

**Some dependencies may not build**. Look for forks that are updated for your `rustc`,
and then point your `~/.cargo/config` at them.

If you find `playform` itself won't build on the latest nightly, please open an issue or file a PR!

## How to play

  * Move: WASD
  * Jump: Space
  * Look around: Mouse

One mob spawns that will play "tag" with you: tag it and it will chase you until it tags you back.

## If things don't work

If things are broken, like compile errors, problems getting it to start, crashes, etc.
please consider opening an issue! If you can take the time to do it in a non-optimized
build with `RUST_BACKTRACE` set, it would be much appreciated.

If things are laggy, there are some constants scattered around that you can tweak;
there are LOD settings in `player.rs`, `terrain/terrain.rs`, `terrain/tree_placer.rs`,
`terrain/texture_generator.rs`, and probably other places. Also consider setting a (lower)
`max_load_distance` in `init/mod.rs`.
