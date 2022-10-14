import * as wasm from "wasm-brainfuc";

function getElapsed(start) {
    const diff = performance.now() - start;
    if (diff >= 1000) {
        return (diff * 0.001).toFixed(2).toString() + "s";
    } else {
        return diff.toString() + "ms";
    }
}

let examples = {};
examples["Prime test"] = 
`fn is_prime(x: u32) -> bool {
    if x == 1 {
        return false;
    }
    if x % 2 == 0 {
        return x == 2;
    }
    let d : u32 = 3;
    while d * d <= x {
        if x % d == 0 {
            return false;
        }
        d = d + 2;
    }
    true
}

fn main() {
    let x : u32 = 100000;
    while x <= 100020 {
        print(x);
        if is_prime(x) {
            println(" is prime");
        } else {
            println(" is not prime");
        }
        x = x + 1;
    }
}`;

examples["Naive Fibonacci"] = 
`fn fib(x: u8) -> u8 {
    if x <= 1 {
        1
    } else {
        fib(x - 1) + fib(x - 2)
    }
}

fn main() {
    let y : u8 = 0;
    while y <= 12 {
        print("fib(");
        print(y);
        print(") = ");
        println(fib(y));
        y = y + 1;
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
    }
}`;

examples["Pointers"] = 
`fn read_string(s_ptr: &u8) {
    while 1 {
        *s_ptr = read_char();
        if *s_ptr == 10 {
            *s_ptr = 0;
            return;
        }
        s_ptr = s_ptr + 1;
    }
}

fn print_string(s_ptr: &u8) {
    while *s_ptr != 0 {
        print_char(*s_ptr);
        s_ptr = s_ptr + 1;
    }
}

fn main() {
    let top_of_stack : u8 = 0;
    let s_ptr : &u8 = &top_of_stack + 1000;
    println("Please enter your name: ");
    read_string(s_ptr);
    print("Hello ");
    print_string(s_ptr);
    println(".");
}`;

examples["Conversion"] = 
`fn u32_to_u8(x: u32) -> u8 {
    *(&x + 3)
}

fn u8_to_u32(x: u8) -> u32 {
    let result : u32 = 0;
    let p : &u8 = &result + 3;
    *p = x;
    result
}

fn main() {
    println(u32_to_u8(3));
    println(u8_to_u32(7));
}`;

let examples_order = ["Prime test", "Naive Fibonacci", "Fast Fibonacci", "Pointers"];
let default_example = "Prime test";

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

    try {
        let compiled = wasm.compile(hir);
        document.getElementById("compiled_sam").value = compiled.sam;
        document.getElementById("compiled_bf").value = compiled.bf;
    } catch (err) {
        alert("Error compiling code. See the console for more info. It likely won't be very helpful info though, because generating nice compiler errors would've been a lot more work.");
    }
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

// JIT:

let myWorker = null;
document.getElementById("jit_run_button").onclick = function() {
    if (myWorker != null) {
        myWorker.terminate();
    }
    let runBfStart = performance.now();
    myWorker = new Worker('worker.js');
    myWorker.onmessage = (e) => {
        let msg_type = e.data[0];
        if (msg_type == "output") {
            document.getElementById("bf_output").value += String.fromCharCode(e.data[1]);
        } else if (msg_type == "finished") {
            document.getElementById("bf_status").textContent = "finished in "+getElapsed(runBfStart);
            document.getElementById("bf_status").className = "status_finished";
            document.getElementById("stop_bf_button").disabled = true;
        } else if (msg_type == "need_more_input") {
            document.getElementById("bf_status").textContent = "awaiting input";
            document.getElementById("bf_status").className = "status_awaiting_input";
        } else if (msg_type == "error") {
            document.getElementById("bf_status").textContent = e.data[1];
            document.getElementById("bf_status").className = "status_crashed";
        } else {
            console.error("Unknown message type "+msg_type);
        }
    }
    let bf = document.getElementById("compiled_bf").value;

    try {
        let result = wasm.compile_bf_to_wasm(bf);
        myWorker.postMessage(["start", result]);
    } catch (err) {
        alert("Error compiling brainfuck code. See the console for more info.");
    }

    document.getElementById("bf_output").value = "";
    document.getElementById("bf_status").textContent = "running";
    document.getElementById("bf_status").className = "status_running";
    document.getElementById("stop_bf_button").disabled = false;
    document.getElementById("stop_bf_button").onclick = () => {
        myWorker.terminate();
        document.getElementById("bf_status").textContent = "stopped";
        document.getElementById("bf_status").className = "status_stopped";
    };
}

// Copy BF button:

document.getElementById("bf_copy_button").onclick = function() {
    let textarea = document.getElementById("compiled_bf");
    textarea.select();
    document.execCommand("copy");
}
console.log("Brainpluck version: "+wasm.init_brainpluck());

// Send input button:

function sendInput() {
    if (myWorker != null) {
        let input = document.getElementById("bf_input").value;
        myWorker.postMessage(["add_input", input]);
        document.getElementById("bf_input").value = "";
    }
}
document.getElementById("send_input_button").onclick = sendInput;

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

// Perf:

document.getElementById("perf_button").onclick = function() {
    let bf = document.getElementById("compiled_bf").value;
    let input = document.getElementById("bf_input").value;

    document.getElementById("bf_output").value = "Running...\n\nThis page will freeze until the entire program finishes.";

    setTimeout(() => {
        let result = wasm.perf_bf(bf, input);

        document.getElementById("bf_output").value = result;
        document.getElementById("bf_output").disabled = false;
    });
}