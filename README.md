# Brainpluck

Brainpluck is a compiler that compiles a language that looks like Rust (but is really more like C) into Brainfuck. It can also JIT-compile any Brainfuck into Wasm.

**Check out the demo [here](https://migi.github.io/brainpluck/).**

## How does it work?

The compilation to Brainfuck works in two stages:

* First, the code is compiled into a kind of assembly language with an instruction set that is intentionally kept as simple as possible (explained below).
* The generated Brainfuck code first writes this assembly bytecode to the tape, and then it runs an interpreter for this bytecode.

The JIT compiler that compiles Brainfuck code into Wasm works in three stages:
* The Brainfuck code is parsed, then optimized (loops like `[-]` and `[->>>+<<<]` become single instructions).
* A pass similar to Binaryen's [Asyncify](https://kripken.github.io/blog/wasm/2019/07/16/asyncify.html) enables asynchronous I/O (i.e., waiting if a `,` instruction is encountered but no input is available)
* The result is compiled into Wat code, which the [wat crate](https://crates.io/crates/wat) turns into Wasm.

## The "high-level" language

The high-level language has a syntax which is similar to Rust, but it only implements the basics:
* Functions / recursion
* Raw pointers
* Local variables
* Branching (if/else)
* Looping (while)
* 8-bit and 32-bit integer arithmetic (+, -, *, /, %, >, >=, etc)
* I/O (simple wrappers around Brainfuck's I/O)

It has no lifetime analysis, generics, pattern matching, or any of that other good stuff. It's really more like C than like Rust. I'm not going to add any of that high-level stuff either, as that's not really the point of this project.

The compiler also makes no attempt to try to generate nice error messages if your code has an error. If you make a mistake, the error message is about as helpful as

![Computer says no](https://media.tenor.com/8Aba1SNxQcQAAAAC/computer-says-no.gif)

## The assembly code

The high-level language gets compiled into bytecode for a hypothetical machine with an infinite tape of memory (8-bit cells) and the following registers:
* `A` and `C`: 4-byte data registers
* `X`: 1-byte data register
* `B`: 4-byte stack pointer (similar to `esp` in x86)
* `I`: 4-byte instruction pointer (similar to `eip` in x86)

The instruction set is intentionally kept very simple. Data in memory can only be accessed "at `B`" with instructions such as `ReadAAtB`/`WriteAAtB`, for reading/writing 4-byte words where `B` points to. Accessing arbitrary data on the heap (e.g. through pointers) is still possible by first backing up `B` to `C` (with `SwapBAndC`), then setting `B` to the pointer (using e.g. `CopyAToB`), then calling `ReadAAtB`/`WriteAAtB` and finally restoring `B` again using `SwapBAndC`.

The other instructions are about what you'd expect if you know x86. There are:
* some instructions to manipulate registers, like `SetA(const)`, `CopyAToB`, etc.
* some arithmetic instructions (implementing `+`, `-`, `*`, `/`, `%`),
* some comparison operators (setting `X` to 1, 0, or -1 depending on whether `A` is greater/equal/lower than the value "at `B`"),
* a `Jump(offset)` and `JumpIfX(offset)` instruction for branching/looping,
* I/O instructions like `PrintCharX`, `StdinX`, `PrintA`, etc.
* `Call(address)` and `Ret`, which work similarly to `call` and `ret` in x86

## The Brainfuck interpreter of the assembly code

Writing an interpreter for this assembly code in Rust is quite easy. Writing an interpreter for it in Brainfuck is a lot trickier.

The biggest issue is that Brainfuck doesn't have arbitrary memory access. Brainfuck has only one pointer (the cell pointer) and you can only manipulate the cell that it points to. But if you want to implement an interpreter which can execute instructions like `ReadAAtB`, you have to manipulate 3 different locations in memory:
* The registers `A`, `B` and `I`
* The data that `B` points to
* The data that instruction pointer `I` points to (so you can decode that the instruction is `ReadAAtB` in the first place)

The Brainfuck interpreter solves this by splitting the tape into two tracks: even cells are program memory, odd cells are for keeping the registers and temporary data. However, this data in the odd cells is not kept at a fixed position. It can move around. More specifically, this data is at a variable offset from zero, and the interpreter uses a 6th register `P` which stores this offset. One of the things the interpreter can then do is to move all the registers and all the temporary data (in the odd cells) two cells to the right and increase `P` by 1.

One iteration of the interpreter then works as follows:
* Go read the next instruction, as follows:
    * While `P < I`, move all registers and temporary data 2 cells right and increment `P`
    * While `P > I`, move all registers and temporary data 2 cells left and decrement `P`
* Decode the next instruction, and store it in the temporary data
* If the instruction involves the data at `B` somehow (e.g. `ReadAAtB`):
    * While `P < B`, move all registers and temporary data 2 cells right and increment `P`
    * While `P > B`, move all registers and temporary data 2 cells left and decrement `P`
* Execute the instruction
* Increment `I` by the size of the instruction (unless it was `Jump`, `Call`, etc.)

The vast majority of the execution time is in the loops that move the registers/temporary data and change `P`. I optimized this by doing larger jumps if possible: the binary representation of `P` is compared bit-by-bit to the pointer we're moving to (`I` or `B`), from most significant bit to least significant bit, and if they differ in for example the 7th least-significant bit then `P` is incremented/decremented by 2^7 and the registers/temporary data are moved 2\*2^7 cells at a time. This loop is unrolled for jumps up to size 256. For an optimizing Brainfuck interpreter which can coalesce N `>` instructions in a row into a single `cell_ptr += N`, this optimization dramatically speeds up the execution time (a factor 10x or more).

The generated Brainfuck code assumes cells are 8-bit integers with wrapping addition/subtraction. It never moves the cell pointer left of the starting position.

## The Brainfuck to Wasm JIT compiler

Arbitrary Brainfuck code can be turned into a Wasm module which runs the code with 8-bit wrapping cells.

The Wasm module has two exports:
* `run_bf()`: runs (or resumes) the Brainfuck code. Returns 0 if the code ran successfully. Returns 1 if the code encountered a `,` instruction but no input was available yet (see below). Throws a `RuntimeError` if the cell pointer goes out of bounds (the Mandelbrot program does this so stick some `>`'s at the front).
* `cell_ptr`: a global variable which contains the cell pointer.

The Wasm module requires 3 imports:
* `imports.tape`: the Brainfuck tape / Wasm memory.
* `imports.write_output_byte(b)`: a function that is called whenever a `.` is encountered. Its argument is the output byte generated.
* `imports.read_input_byte()`: a function that is called whenever a `,` is encountered. It should return the input byte, or if no input is available it should return `0`. If you return `0`, the `run_bf()` function will immediately return `1`. Rerun `run_bf()` at any point when input is available to resume execution.
