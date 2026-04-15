# The Monster Controller

Every Halloween, my dad and I set up animatronic monsters to scare people when they come toward our house.  Scary, right?

This contains three parts: The Program, the Main Board, and the IoT Board.

# The Program (found in the 'program' directory)

This uses Tauri's software technology to build a small, fast, and good-looking app both for computers and phones.

# The Main Board (found in the 'nucleo' directory)

This uses (Embassy)[https://embassy.dev/] with the NUCLEO-F767ZI board. Embassy is used because of the ease of use with Rust and its amazingly small size. This board will control the monsters, using its Micro USB connector as a serial port for computer-to-board communications.

# The IoT Board (found in the 'nano-iot' directory)

This will most likely use Zephyr (mainly because I'm on Linux, and I'd like to learn more about it). This board will act has a wireless bridge between the main board and the IoT.
