# Brainpluck

Brainpluck is a compiler that compiles a language that looks like Rust (but is really more like C) into Brainfuck. It can also JIT-compile any Brainfuck into Wasm.

**Check out the demo [here](https://migi.github.io/brainpluck/).**

Features:

* Functions
* Pointers
* Local variables
* Branching (if/else)
* Looping (while)
* 8-bit and 32-bit integer arithmetic (+, -, *, /, %, >, >=, etc)
* I/O (same as brainfuck)

## How does it work?

The compilation to Brainfuck works in two stages:

* First, the code is compiled into a kind of assembly language (called "SAM") with an instruction set that is intentionally kept as simple as possible.
* The generated Brainfuck code first writes this assembly bytecode to the tape, and then it runs an interpreter for this bytecode.

## The high-level language

## The assembly code

## The Brainfuck to wasm JIT compiler

