import * as wasm from "wasm-brainfuc";

let examples = {};
examples["Fibonacci"] = 
`fn main() {
    let y : u8 = 5;
    while y {
        print("fib(");
        print(y);
        print(") = ");
        println(fib(y));
        y = y - 1;
    };
    print("fib(0) = ");
    println(fib(y));
}

fn fib(x: u8) -> u8 {
    if x {
        if x - 1 {
            let f1 : u8 = fib(x - 1);
            let f2 : u8 = fib(x - 2);
            f1 + f2
        } else {
            1
        }
    } else {
        1
    }
}`;

examples["Fast Fibonacci"] = 
`fn main() {
    println(fib(27));
}

fn fib(x: u8) -> u32 {
    let y : u8 = 0;
    let fib_y : u32 = 1;
    let fib_y_minus_1 : u32 = 0;
    while y - x {
        let prev_fib_y : u32 = fib_y;
        fib_y = fib_y + fib_y_minus_1;
        fib_y_minus_1 = prev_fib_y;
        y = y + 1;
    };
    fib_y
}`;

let examples_order = ["Fibonacci", "Fast Fibonacci"];
let default_example = "Fibonacci";

for (let example_name of examples_order) {
    let option = document.createElement("option");
    option.value = example_name;
    option.text = example_name;
    option.selected = example_name == default_example;
    document.getElementById("example_select").appendChild(option);
}
document.getElementById("hir_code").textContent = examples[default_example];

// Changing example:

function changeExample() {
    let example_name = document.getElementById("example_select").value;
    document.getElementById("hir_code").value = examples[example_name];
}
document.getElementById("example_select").onchange = changeExample;

// Compiling:

function clickCompile() {
    let hir = document.getElementById("hir_code").value;

    let compiled = wasm.compile(hir);

    document.getElementById("compiled_sam").value = compiled.sam;
    document.getElementById("compiled_bf").value = compiled.bf;
}
document.getElementById("compile_button").onclick = clickCompile;

// Debugging:

function clickDebug() {
    let hir = document.getElementById("hir_code").value;
    let input = document.getElementById("bf_input").value;

    let result = wasm.debug_program(hir, input);

    document.getElementById("compiled_sam").value = result.sam;
    document.getElementById("compiled_bf").value = "Debugging runs the assembly code directly";
    document.getElementById("bf_output").value = result.output;
}
document.getElementById("debug_button").onclick = clickDebug;

// Running:

function clickRun() {
    let bf = document.getElementById("compiled_bf").value;
    let input = document.getElementById("bf_input").value;

    let result = wasm.parse_and_run_bf(bf, input);

    document.getElementById("bf_output").value = result;
}
document.getElementById("run_button").onclick = clickRun;

document.getElementById("bf_copy_button").onclick = function() {
    let textarea = document.getElementById("compiled_bf");
    textarea.select();
    document.execCommand("copy");
}

console.log("Brainpluck version: "+wasm.init_brainpluck());
