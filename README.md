[![Build Status](https://travis-ci.org/bfops/playform.svg?branch=master)](https://travis-ci.org/bfops/playform)

## Current Status

Two main things are in the works:

  * Atmospheric scattering: to give the sky a more realistic color progression across the day (and especially during sunrise & sunset).
  * Performance refactoring: load speed and voxel brushes are slower than they really should be. I'm working on it.

## Overview

An interactive, modifiable voxel sandbox project in Rust, inspired in part by [Voxel Farm](http://procworld.blogspot.com/) and Minecraft. I try to keep a dev blog [here](http://playformdev.blogspot.com/).

It's very much a WIP. Hopefully as the Rust ecosystem grows (and, in a perfect world, when Rust gets a story for linking with C++),
the hackiest parts of Playform can be outsourced to other libraries (physics and graphics APIs, threading, networking).

Some picture things (perpetually outdated):

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 5](/../screenshots/screenshots/screenshot5.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)
![screenshot 3](/../screenshots/screenshots/screenshot3.png?raw=true)
![screenshot 4](/../screenshots/screenshots/screenshot4.png?raw=true)

## Making it work

Make sure you have:

  * A **nightly** build of the Rust compiler and cargo (if it doesn't build on latest, file an issue)
  * OpenGL 3.3+
  * SDL2
  * SDL2\_ttf
  * libnanomsg
  * portaudio
  * m4

Playform has a separate server and client, which can be built and run in `server/bin` and `client/bin`,
but there's also a server+client (singleplayer) bundled binary that builds in the root directory.

`cargo build --release` and `cargo run --release` are pretty much required to run Playform with reasonable performance.

## Controls

  * Move: WASD
  * Jump: Space
  * Look around: Mouse
  * Tree tool: Left mouse button (this is slow)
  * Dig tool: Right mouse button
  * Toggle HUD: H

One mob (red rectangular block) spawns that will play "tag" with you: tag it and it will chase you until it tags you back. If you get too far away from it, it'll probably get lost and fall through the planet. It's a little needy that way.

## License & Credit

I'm not intimately familiar with how licensing works: if I've done something wrong, please let me know. To state my intent in a non-legally-binding way: I want Playform itself (i.e. the code I've written in this repository) to be MIT licensed (see the LICENSE file).
It includes some snippets that can be easily found published online. I believe those snippets come with links to the online source.

Some of the assets are not mine, and I don't own the rights to them. In particular, thanks to:

  * [http://vector.me/browse/104477/free\_vector\_grass](http://vector.me/browse/104477/free_vector_grass) for the textures used for the grass billboards
  * [http://soundbible.com/1818-Rainforest-Ambience.html](http://soundbible.com/1818-Rainforest-Ambience.html) for the awesome ambient sound
  * [http://soundbible.com/1432-Walking-On-Gravel.html](http://soundbible.com/1432-Walking-On-Gravel.html) for the footstep sounds
