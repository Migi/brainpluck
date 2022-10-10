import * as wasm from "wasm-brainfuc";

let examples = {};
examples["Slow Fibonacci"] = 
`fn main() {
    let y : u8 = 0;
    while y <= 5 {
        print("fib(");
        print(y);
        print(") = ");
        println(fib(y));
        y = y + 1;
    };
}

fn fib(x: u8) -> u8 {
    if x > 0 {
        if x > 1 {
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
    let x : u8 = 0;
    let fib_x : u32 = 1;
    let fib_x_minus_1 : u32 = 0;
    while x <= 46 {
        print("fib(");
        print(x);
        print(") = ");
        println(fib_x);
        let prev_fib_x : u32 = fib_x;
        fib_x = fib_x + fib_x_minus_1;
        fib_x_minus_1 = prev_fib_x;
        x = x + 1;
    };
}`;

examples["Prime test"] = 
`fn main() {
    let x : u32 = 10000;
    while x <= 10020 {
        print(x);
        let result : u8 = is_prime(x);
        if result {
            println(" is prime");
        } else {
            println(" is not prime");
        };
        x = x + 1;
    };
}

fn is_prime(x: u32) -> u8 {
    if x == 1 {
        return 0;
    };
    if x % 2 == 0 {
        return x == 2;
    };
    let d : u32 = 3;
    while d * d <= x {
        if x % d == 0 {
            return 0;
        };
        d = d + 2;
    };
    1
}`;

let examples_order = ["Slow Fibonacci", "Fast Fibonacci", "Prime test"];
let default_example = "Slow Fibonacci";

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

document.getElementById("compile_button").onclick = function() {
    let hir = document.getElementById("hir_code").value;

    let compiled = wasm.compile(hir);

    document.getElementById("compiled_sam").value = compiled.sam;
    document.getElementById("compiled_bf").value = compiled.bf;
};

// Debugging:

document.getElementById("debug_button").onclick = function() {
    let hir = document.getElementById("hir_code").value;
    let input = document.getElementById("bf_input").value;

    let result = wasm.debug_program(hir, input);

    document.getElementById("compiled_sam").value = result.sam;
    document.getElementById("compiled_bf").value = "Debugging runs the assembly code directly";
    document.getElementById("bf_output").value = result.output;
};

// Running:

document.getElementById("run_button").onclick = function() {
    let bf = document.getElementById("compiled_bf").value;
    let input = document.getElementById("bf_input").value;

    document.getElementById("bf_output").value = "Running...\n\nThis page will freeze until the entire program finishes.";

    setTimeout(() => {
        let result = wasm.parse_and_run_bf(bf, input);

        document.getElementById("bf_output").value = result;
    });
}

// Copy BF button:

document.getElementById("bf_copy_button").onclick = function() {
    let textarea = document.getElementById("compiled_bf");
    textarea.select();
    document.execCommand("copy");
}

console.log("Brainpluck version: "+wasm.init_brainpluck());
