[![Build Status](https://travis-ci.org/bfops/playform.svg?branch=master)](https://travis-ci.org/bfops/playform)

## Overview

Playform aspires to be an open-world sandbox game written in Rust, taking
inspiration from [Voxel Farm](http://procworld.blogspot.com/) and Minecraft. The "dev blog" is at [here](http://playformdev.blogspot.com/).

It's very much a WIP. Hopefully as the Rust ecosystem improves (and, in a perfect world, when Rust gets a story for linking with C++),
the hackiest parts of Playform can be outsourced to other libraries (physics and graphics APIs, threading, networking).

Help is great! PRs and [issues](https://github.com/bfops/playform/issues) are appreciated.

Some picture things:

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)
![screenshot 3](/../screenshots/screenshots/screenshot3.png?raw=true)
![screenshot 4](/../screenshots/screenshots/screenshot4.png?raw=true)

## Making it work

Make sure you have:

  * The **2015-01-18 nightly build** of the Rust compiler and cargo. There are probably other versions that work, but that's what I'm running.
  * OpenGL 3.3+
  * libpng
  * SDL2
  * SDL2\_ttf
  * libnanomsg

At any point, `--release` can be appended onto `cargo build` or `cargo run` for a slower
build, but a much more optimized binary.

Playform has a separate server and client, which can be built and run in `server/bin` and `client/bin`,
but there's also a server+client (singleplayer) bundled binary that builds in the root directory.

## Performance

It's not great. It would be great to get Playform running well on a variety of PCs, but I only have mine.

## How to play

  * Move: WASD
  * Jump: Space
  * Look around: Mouse
  * Tree tool: Left mouse button
  * Dig tool: Right mouse button
  * Toggle HUD: H

One mob spawns that will play "tag" with you: tag it and it will chase you until it tags you back. If you get too far away from it, it'll probably get lost and fall through the planet. It's a little needy.

## If things don't work

If things are broken, like compile errors, problems getting it to start, crashes, etc.
please consider opening an issue! If you can take the time to do it in a non-optimized
build with `RUST_BACKTRACE=1` and `RUST_LOG=debug` set, it would be much appreciated.
