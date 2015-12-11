[![Build Status](https://travis-ci.org/bfops/playform.svg?branch=master)](https://travis-ci.org/bfops/playform)

## Introduction

**FYI: Playform is on hiatus. I'm taking a detour into learning about Unity and Cubiquity. As it stands, I'm not convinced of Rust's current viability as a language for game development, since it requires that all the game development wheels be reinvented in C or Rust. If/when Rust gets stronger support for using C++ libraries, I'll think about spinning Playform up again (or a similar project). I'm still happy to merge PRs, talk with anybody about the design, about how it works, about voxels in general, etc, but I don't plan to invest much more coding time in the near future.**

Playform aspires to be an open-world sandbox game written in Rust, taking
inspiration from [Voxel Farm](http://procworld.blogspot.com/) and Minecraft. The dev blog is available [here](http://playformdev.blogspot.com/).

It's currently.. well, very much a WIP. As
[michaelwu's rust-bindgen fork](https://github.com/michaelwu/rust-bindgen/tree/sm-hacks) supports C++
better and better, part of the plan is to start using seasoned C++
libraries like [bullet physics](https://github.com/bulletphysics/bullet3).

Help is great! PRs and [issues](https://github.com/bfops/playform/issues)
are appreciated. Featureful changes should go into the client-voxels branch, which is a major refactoring. It hasn't been merged into master because it degrades performance a lot right now.

Some picture things:

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)
![screenshot 3](/../screenshots/screenshots/screenshot3.png?raw=true)
![screenshot 4](/../screenshots/screenshots/screenshot4.png?raw=true)

## Making it work

Make sure you have:

  * The **2015-10-28 nightly build** of the Rust compiler and cargo. It's possible that other versions will work.
  * OpenGL 3.3+
  * libpng
  * SDL2
  * SDL2_ttf
  * libnanomsg

At any point, `--release` can be appended onto `cargo build` or `cargo run` for a slower
build, but a much more optimized binary.

Run the Playform server using `cargo run` in the `server` folder. It takes one parameter:
the listen URL for the server. It defaults to running locally: `ipc:///tmp/server.ipc`.

The client can be run similarly with `cargo run` in the `client` folder. It takes two
parameters: the listen URL of the client and the listen URL of the server. They
both default to running locally (`ipc:///tmp/client.ipc` for the client URL).

**Some dependencies might not build**. Look for forks that are updated for
your `rustc`, and then point your `~/.cargo/config` at them.

If you find `playform` itself won't build on the latest `rustc`, please open an issue or file a PR!

## Performance

I mostly work on non-performance stuff because it's more fun, so Playform runs passably on my pretty good computer. If it's too slow for you, try tweaking `MAX_LOAD_DISTANCE` in `client/src/client.rs`. 

## How to play

  * Move: WASD
  * Jump: Space
  * Look around: Mouse
  * Place a tree: Left mouse button
  * Sphere eraser tool: Right mouse button
  * Toggle HUD: H

One mob spawns that will play "tag" with you: tag it and it will chase you until it tags you back. If you get too far away from it, it'll probably get lost and fall through the planet. It's a little needy.

## If things don't work

If things are broken, like compile errors, problems getting it to start, crashes, etc.
please consider opening an issue! If you can take the time to do it in a non-optimized
build with `RUST_BACKTRACE=1` and `RUST_LOG=debug` set, it would be much appreciated.
