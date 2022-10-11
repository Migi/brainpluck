class Queue {
    constructor() {
        this.elements = {};
        this.head = 0;
        this.tail = 0;
    }
    enqueue(element) {
        this.elements[this.tail] = element;
        this.tail++;
    }
    dequeue() {
        const item = this.elements[this.head];
        delete this.elements[this.head];
        this.head++;
        return item;
    }
    peek() {
        return this.elements[this.head];
    }
    length() {
        return this.tail - this.head;
    }
    isEmpty() {
        return this.length() === 0;
    }
}

let inputQueue = new Queue();
let wasmState = {
    "module": null,
    "cell_ptr": 0,
    "async_start_block": 0
};

onmessage = function(e) {
    let messageType = e.data[0];

    if (messageType == "start") {
        const memory = new WebAssembly.Memory({ initial: 100 });

        const importObject = {
            imports: {
                read_input_byte() {
                    if (inputQueue.isEmpty()) {
                        return 0;
                    } else {
                        return inputQueue.dequeue();
                    }
                },
                async_request_more_input(cell_ptr, async_start_block) {
                    wasmState["cell_ptr"] = cell_ptr;
                    wasmState["async_start_block"] = async_start_block;
                    console.log(wasmState);
                    postMessage(["async_request_more_input"]);
                },
                write_output_byte(byte) {
                    postMessage(["output", byte]);
                },
            },
            js: { mem: memory },
        };

        let wasmModulePromise = WebAssembly.instantiate(e.data[1], importObject);
        
        wasmModulePromise.then((module) => {
            wasmState.module = module;
            try {
                let return_val = module.instance.exports.run_bf(0, 0);
                if (return_val == 0) {
                    postMessage(["finished"]);
                    wasmState.module = null;
                }
            } catch (err) {
                postMessage(["error", err]);
            }
        });
    } else if (messageType == "add_input") {
        for (let c of e.data[1]) {
            let code = c.charCodeAt(0);
            if (code > 255) {
                inputQueue.enqueue("?".charCodeAt(0));
            } else {
                inputQueue.enqueue(code);
            }
        }
        inputQueue.enqueue("\n".charCodeAt(0));
        if (wasmState.module != null) {
            try {
                let return_val = wasmState.module.instance.exports.run_bf(wasmState["cell_ptr"], wasmState["async_start_block"]);
                if (return_val == 0) {
                    postMessage(["finished"]);
                    wasmState.module = null;
                }
            } catch (err) {
                postMessage(["error", err]);
            }
        }
    }
}