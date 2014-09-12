## Introduction

playform aspires to be an open-world sandbox game written in Rust.
Right now, it renders a basic world and stops the player from colliding
with it, as well as allowing the player to place and destroy blocks.

Help is appreciated! You can hop over to the [issues page](https://github.com/bfops/playform/issues) to see what needs doing.

## What about [hematite](https://github.com/PistonDevelopers/hematite)?

We're aware of each other! While hematite intends to reproduce Minecraft behavior and interact with actual Minecraft, playform's design goal is just to be fun and Minecraft-inspired - we'll have no reservations about diverging.

That said, these projects should probably keep a closer eye on each other than they do! If you notice redundancies, feel free to point them out.

## Making it work

Have a `rustc` built no earlier than September 11th, 2014.
Install `libpng`, `SDL2` and `SDL2_ttf`.
Build with `cargo build`, which will grab all the Rust dependencies.
Run with `cargo run` and playform will start!

## How to play

  * Move: WASD
  * Jump: Space
  * Look around: Mouse
  * Remove block: Left-click
  * Place dirt block: Right-click
  * Toggle octree rendering: O
  * Toggle block outline rendering: L
  * Save line-of-sight: M

One mob spawns that will play a tag-like game with you: touch it and will chase you until it touches you back.

## Screenshots

![screenshot 1](/../screenshots/screenshots/screenshot1.png?raw=true)
![screenshot 2](/../screenshots/screenshots/screenshot2.png?raw=true)
![screenshot 3](/../screenshots/screenshots/screenshot3.png?raw=true)
![screenshot 4](/../screenshots/screenshots/screenshot4.png?raw=true)
