# The Monsters

Every Halloween, my dad and I set up animatronic monsters to scare people when they come toward our house.  Scary, right?

This contains three parts: The Program, the Main Board, and the common shared files and libraries.

## The Program (found in the 'program' directory)

This uses Tauri's software technology to build a small, fast, and good-looking app both for computers and phones.

Other ports of the program will be made afterwards.

Linux note: run `WEBKIT_DISABLE_DMABUF_RENDERER 1` on bash or `set -gx WEBKIT_DISABLE_DMABUF_RENDERER 1` on fish to run the Tauri apps.

## The Main Board (found in the 'controller' directory)

This uses [Embassy](https://embassy.dev/) with the NUCLEO-F767ZI board. Embassy is used because of the ease of use with Rust and its amazingly small size. This board controls the monsters, using its OTG Micro USB connector as a serial port for computer-to-board communications.

## Shared Libraries and Files (found in the 'common' directory)

This holds any multi-used library, file, consts, etc.
