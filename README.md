# Desktop Pet
This is an app written in Java to allow a cute furry friend to accompany you while you work, relax, game, or anything else on your computer.

Your pet can be walked with the arrow keys, and has a 50% of walking themself every 10 minutes.

You can click on your pet to pet them and you can drag them to a new location on your screen

## Download
To download, check the [releases tab](https://github.com/Incandescent-Turtle/desktop-pet/releases) and download the .zip or .jar file. 

If you are on windows, download the .zip file and extract the contents. You can then run the .exe provided to make your pet appear. If you get a warning about an unsigned .exe or something, that is okay! You can hit "More Info" and "Run Anyway". If you feel uncomfy running unfamiliar apps, all of the code is open source on this page and you can build the project yourself and run it!

## Linux
The Java version can't move its window on Wayland, so Linux has its own version written in Rust, in the [`linux/`](linux/) folder. It uses the same sprites and behaves the same way.

It needs a Wayland desktop that supports layer-shell: COSMIC (Pop!_OS), KDE Plasma, Sway, Hyprland and most others. GNOME doesn't support it.

The Rust version only runs on Linux for now; it won't build or run on Windows. On Windows, use the Java version from the [releases tab](https://github.com/Incandescent-Turtle/desktop-pet/releases). Windows support is planned soon.

To build and run it, install Rust from [rustup.rs](https://rustup.rs), then:
```sh
cd linux
cargo run --release
```
Click your pet once to give it keyboard focus. Then the arrow keys walk it, W sends it wandering and Esc quits.

If you move the mouse too fast while dragging, your pet gets dropped, move her more carefull! Click and drag again to pick it back up.

## Adding Animations and Changing Pet
Currently these are unsupported features, and the codebase is not very friendly if you want to add your own.
A re-work of the programs design should make it way easier for delevopers to add in new animations and features, as well as different pets.
I am not sure if I will add the ability for end users to add their own pets or animations, however in the future multiple pet options may be provided.

## Images
<img width="442" alt="image" src="https://github.com/Incandescent-Turtle/desktop-pet/assets/59327500/be9c01f9-6dfb-42fa-8c39-0159e6f3e694">
<img width="427" alt="image" src="https://github.com/Incandescent-Turtle/desktop-pet/assets/59327500/956193f6-ac75-4856-8ac3-d52c958c0396">
<img width="481" alt="image" src="https://github.com/Incandescent-Turtle/desktop-pet/assets/59327500/d77c078f-cef8-43f2-b2a0-13c06d497bdc">

