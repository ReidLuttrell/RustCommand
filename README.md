# Reid Luttrell

# RustCommand

A Missile Command style arcade game implemented in the Rust Programming Language.
The goal of the game is to intercept the incoming missiles without letting them touch the ground. Use the arrow keys to move your crosshair and space to fire an interceptor.
If a rocket passes through an interceptor's explosion radius, it will be destroyed.
Press escape to quit the game. If 5 missiles hit the ground, you lose!
Utilizes the ggez crate as a base for the 2D game engine.

Building the code:

Simply run "cargo build" in the main directory.

Running the code:

If this is being run in windows, easiest way is to type "cargo run" and the game should open a window and start. Otherwise, you will likely have to run the RustCommand.exe generated in the target directory.

Testing:

The game was tested by playing the game and observing that it functioned in the way i expected.
For example, all drawing code was visually inspected in the game, game logic such as shooting rockets is tested by shooting a rocket and see if that indeed happens, interceptions etc can all be visually inspected in the same way. Cursor border collisions were tested by trying to get off the screen with the cursor, and so on.

Retrospect:

I intended this to be a pretty simplified version of missile command, so to that end I'm pretty satisfied with how it turned out. Missile command, depending on the version has some really good animations, and I wish I would have been able to be more true to them, but they would seriously increase the size and complexity of the code, and the game was getting very large in terms of code size very quickly, so for time constraints and ease of grading, I decided to settle on this approximation. I think the animations I made have their own charm anyways. That being said, in the future I think the next improvement would be making the explosion animations more dynamic and interesting.