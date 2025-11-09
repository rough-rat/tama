# firmware

## layout

| path | description |
| ---- | ---- | 
| `crates/tama-core` | platform independent logic |
| `crates/tama-desktop` | simulator code for desktop | 

## building

To run the simulator it should be enough to do `cargo run`. You might need to install `sdl2` libraries first ([embedded_graphics_simulator guide](https://github.com/embedded-graphics/simulator#setup)).

## simulator

- wasd -> arrows
- j -> A
- k -> B
- escape -> quit 

## roadmap

- [ ] engine
    - [x] scene management
    - [x] input management 
    - [ ] sprites
    - [ ] animation
    - [ ] custom fonts
    - [ ] audio

- [ ] system
  - [ ] games
    - [ ] *totally not flappy bird* ™
      - [x] game logic (actually not really because there's not even score tracking)
      - [ ] assets
  - [ ] ???
