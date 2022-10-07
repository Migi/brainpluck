import * as wasm from "wasm-brainfuc";

let examples = {};
examples["Fibonacci"] = 
`fn main() {
    let y : u8 = 5;
    while y {
        println(fib(y));
        y = y - 1;
    };
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

let examples_order = ["Fibonacci"];
let default_example = "Fibonacci";

for (let example_name of examples_order) {
    let option = document.createElement("option");
    option.value = example_name;
    option.text = example_name;
    option.selected = example_name == default_example;
    document.getElementById("example_select").appendChild(option);
}
document.getElementById("hir_code").textContent = examples[default_example];

// Compiling:

function clickCompile() {
    let hir = document.getElementById("hir_code").value;

    let compiled = wasm.compile(hir);

    document.getElementById("compiled_sam").textContent = compiled.sam;
    document.getElementById("compiled_bf").textContent = compiled.bf;
}
document.getElementById("compile_button").onclick = clickCompile;

// Running:

function clickRun() {
    let bf = document.getElementById("compiled_bf").value;
    let input = document.getElementById("bf_input").value;

    let result = wasm.parse_and_run_bf(bf, input);

    document.getElementById("bf_output").textContent = result;
}
document.getElementById("run_button").onclick = clickRun;

document.getElementById("bf_copy_button").onclick = function() {
    let textarea = document.getElementById("compiled_bf");
    textarea.select();
    document.execCommand("copy");
}
