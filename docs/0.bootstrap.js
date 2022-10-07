(window["webpackJsonp"] = window["webpackJsonp"] || []).push([[0],{

/***/ "../pkg/brainfuc.js":
/*!**************************!*\
  !*** ../pkg/brainfuc.js ***!
  \**************************/
/*! exports provided: compile, CompilationResult, __wbindgen_throw */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./brainfuc_bg.wasm */ \"../pkg/brainfuc_bg.wasm\");\n/* harmony import */ var _brainfuc_bg_js__WEBPACK_IMPORTED_MODULE_1__ = __webpack_require__(/*! ./brainfuc_bg.js */ \"../pkg/brainfuc_bg.js\");\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"compile\", function() { return _brainfuc_bg_js__WEBPACK_IMPORTED_MODULE_1__[\"compile\"]; });\n\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"CompilationResult\", function() { return _brainfuc_bg_js__WEBPACK_IMPORTED_MODULE_1__[\"CompilationResult\"]; });\n\n/* harmony reexport (safe) */ __webpack_require__.d(__webpack_exports__, \"__wbindgen_throw\", function() { return _brainfuc_bg_js__WEBPACK_IMPORTED_MODULE_1__[\"__wbindgen_throw\"]; });\n\n\n\n\n//# sourceURL=webpack:///../pkg/brainfuc.js?");

/***/ }),

/***/ "../pkg/brainfuc_bg.js":
/*!*****************************!*\
  !*** ../pkg/brainfuc_bg.js ***!
  \*****************************/
/*! exports provided: compile, CompilationResult, __wbindgen_throw */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* WEBPACK VAR INJECTION */(function(module) {/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"compile\", function() { return compile; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"CompilationResult\", function() { return CompilationResult; });\n/* harmony export (binding) */ __webpack_require__.d(__webpack_exports__, \"__wbindgen_throw\", function() { return __wbindgen_throw; });\n/* harmony import */ var _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! ./brainfuc_bg.wasm */ \"../pkg/brainfuc_bg.wasm\");\n\n\nconst lTextDecoder = typeof TextDecoder === 'undefined' ? (0, module.require)('util').TextDecoder : TextDecoder;\n\nlet cachedTextDecoder = new lTextDecoder('utf-8', { ignoreBOM: true, fatal: true });\n\ncachedTextDecoder.decode();\n\nlet cachedUint8Memory0 = new Uint8Array();\n\nfunction getUint8Memory0() {\n    if (cachedUint8Memory0.byteLength === 0) {\n        cachedUint8Memory0 = new Uint8Array(_brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"memory\"].buffer);\n    }\n    return cachedUint8Memory0;\n}\n\nfunction getStringFromWasm0(ptr, len) {\n    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));\n}\n\nlet cachedInt32Memory0 = new Int32Array();\n\nfunction getInt32Memory0() {\n    if (cachedInt32Memory0.byteLength === 0) {\n        cachedInt32Memory0 = new Int32Array(_brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"memory\"].buffer);\n    }\n    return cachedInt32Memory0;\n}\n\nlet WASM_VECTOR_LEN = 0;\n\nconst lTextEncoder = typeof TextEncoder === 'undefined' ? (0, module.require)('util').TextEncoder : TextEncoder;\n\nlet cachedTextEncoder = new lTextEncoder('utf-8');\n\nconst encodeString = (typeof cachedTextEncoder.encodeInto === 'function'\n    ? function (arg, view) {\n    return cachedTextEncoder.encodeInto(arg, view);\n}\n    : function (arg, view) {\n    const buf = cachedTextEncoder.encode(arg);\n    view.set(buf);\n    return {\n        read: arg.length,\n        written: buf.length\n    };\n});\n\nfunction passStringToWasm0(arg, malloc, realloc) {\n\n    if (realloc === undefined) {\n        const buf = cachedTextEncoder.encode(arg);\n        const ptr = malloc(buf.length);\n        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);\n        WASM_VECTOR_LEN = buf.length;\n        return ptr;\n    }\n\n    let len = arg.length;\n    let ptr = malloc(len);\n\n    const mem = getUint8Memory0();\n\n    let offset = 0;\n\n    for (; offset < len; offset++) {\n        const code = arg.charCodeAt(offset);\n        if (code > 0x7F) break;\n        mem[ptr + offset] = code;\n    }\n\n    if (offset !== len) {\n        if (offset !== 0) {\n            arg = arg.slice(offset);\n        }\n        ptr = realloc(ptr, len, len = offset + arg.length * 3);\n        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);\n        const ret = encodeString(arg, view);\n\n        offset += ret.written;\n    }\n\n    WASM_VECTOR_LEN = offset;\n    return ptr;\n}\n/**\n* @param {string} hir\n* @returns {CompilationResult}\n*/\nfunction compile(hir) {\n    const ptr0 = passStringToWasm0(hir, _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbindgen_malloc\"], _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbindgen_realloc\"]);\n    const len0 = WASM_VECTOR_LEN;\n    const ret = _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"compile\"](ptr0, len0);\n    return CompilationResult.__wrap(ret);\n}\n\n/**\n*/\nclass CompilationResult {\n\n    static __wrap(ptr) {\n        const obj = Object.create(CompilationResult.prototype);\n        obj.ptr = ptr;\n\n        return obj;\n    }\n\n    __destroy_into_raw() {\n        const ptr = this.ptr;\n        this.ptr = 0;\n\n        return ptr;\n    }\n\n    free() {\n        const ptr = this.__destroy_into_raw();\n        _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbg_compilationresult_free\"](ptr);\n    }\n    /**\n    * @returns {string}\n    */\n    get sam() {\n        try {\n            const retptr = _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbindgen_add_to_stack_pointer\"](-16);\n            _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"compilationresult_sam\"](retptr, this.ptr);\n            var r0 = getInt32Memory0()[retptr / 4 + 0];\n            var r1 = getInt32Memory0()[retptr / 4 + 1];\n            return getStringFromWasm0(r0, r1);\n        } finally {\n            _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbindgen_add_to_stack_pointer\"](16);\n            _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbindgen_free\"](r0, r1);\n        }\n    }\n    /**\n    * @returns {string}\n    */\n    get bf() {\n        try {\n            const retptr = _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbindgen_add_to_stack_pointer\"](-16);\n            _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"compilationresult_bf\"](retptr, this.ptr);\n            var r0 = getInt32Memory0()[retptr / 4 + 0];\n            var r1 = getInt32Memory0()[retptr / 4 + 1];\n            return getStringFromWasm0(r0, r1);\n        } finally {\n            _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbindgen_add_to_stack_pointer\"](16);\n            _brainfuc_bg_wasm__WEBPACK_IMPORTED_MODULE_0__[\"__wbindgen_free\"](r0, r1);\n        }\n    }\n}\n\nfunction __wbindgen_throw(arg0, arg1) {\n    throw new Error(getStringFromWasm0(arg0, arg1));\n};\n\n\n/* WEBPACK VAR INJECTION */}.call(this, __webpack_require__(/*! ./../docs/node_modules/webpack/buildin/harmony-module.js */ \"./node_modules/webpack/buildin/harmony-module.js\")(module)))\n\n//# sourceURL=webpack:///../pkg/brainfuc_bg.js?");

/***/ }),

/***/ "../pkg/brainfuc_bg.wasm":
/*!*******************************!*\
  !*** ../pkg/brainfuc_bg.wasm ***!
  \*******************************/
/*! exports provided: memory, __wbg_compilationresult_free, compilationresult_sam, compilationresult_bf, compile, __wbindgen_add_to_stack_pointer, __wbindgen_free, __wbindgen_malloc, __wbindgen_realloc */
/***/ (function(module, exports, __webpack_require__) {

eval("\"use strict\";\n// Instantiate WebAssembly module\nvar wasmExports = __webpack_require__.w[module.i];\n__webpack_require__.r(exports);\n// export exports from WebAssembly module\nfor(var name in wasmExports) if(name != \"__webpack_init__\") exports[name] = wasmExports[name];\n// exec imports from WebAssembly module (for esm order)\n/* harmony import */ var m0 = __webpack_require__(/*! ./brainfuc_bg.js */ \"../pkg/brainfuc_bg.js\");\n\n\n// exec wasm module\nwasmExports[\"__webpack_init__\"]()\n\n//# sourceURL=webpack:///../pkg/brainfuc_bg.wasm?");

/***/ }),

/***/ "./index.js":
/*!******************!*\
  !*** ./index.js ***!
  \******************/
/*! no exports provided */
/***/ (function(module, __webpack_exports__, __webpack_require__) {

"use strict";
eval("__webpack_require__.r(__webpack_exports__);\n/* harmony import */ var wasm_brainfuc__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(/*! wasm-brainfuc */ \"../pkg/brainfuc.js\");\n\r\n\r\nlet examples = {};\r\nexamples[\"Fibonacci\"] = \r\n`fn main() {\r\n    let y : u8 = 5;\r\n    while y {\r\n        println(fib(y));\r\n        y = y - 1;\r\n    };\r\n    println(fib(y));\r\n}\r\n\r\nfn fib(x: u8) -> u8 {\r\n    if x {\r\n        let x_minus_1 : u8 = x - 1;\r\n        if x_minus_1 {\r\n            let x_minus_2 : u8 = x_minus_1 - 1;\r\n            let f1 : u8 = fib(x_minus_1);\r\n            let f2 : u8 = fib(x_minus_2);\r\n            f1 + f2\r\n        } else {\r\n            1\r\n        }\r\n    } else {\r\n        1\r\n    }\r\n}`;\r\n\r\nlet examples_order = [\"Fibonacci\"];\r\nlet default_example = \"Fibonacci\";\r\n\r\nfor (let example_name of examples_order) {\r\n    let option = document.createElement(\"option\");\r\n    option.value = example_name;\r\n    option.text = example_name;\r\n    option.selected = example_name == default_example;\r\n    document.getElementById(\"example_select\").appendChild(option);\r\n}\r\ndocument.getElementById(\"hir_code\").textContent = examples[default_example];\r\n\r\nfunction clickCompile() {\r\n    let hir = document.getElementById(\"hir_code\").textContent;\r\n\r\n    let compiled = wasm_brainfuc__WEBPACK_IMPORTED_MODULE_0__[\"compile\"](hir);\r\n\r\n    document.getElementById(\"compiled_sam\").textContent = compiled.sam;\r\n    document.getElementById(\"compiled_bf\").textContent = compiled.bf;\r\n}\r\n\r\ndocument.getElementById(\"compile_button\").onclick = clickCompile;\r\n\r\ndocument.getElementById(\"bf_copy_button\").onclick = function() {\r\n    let textarea = document.getElementById(\"compiled_bf\");\r\n    textarea.disabled = false;\r\n    textarea.select();\r\n    document.execCommand(\"copy\");\r\n    textarea.disabled = true;\r\n}\r\n\n\n//# sourceURL=webpack:///./index.js?");

/***/ }),

/***/ "./node_modules/webpack/buildin/harmony-module.js":
/*!*******************************************!*\
  !*** (webpack)/buildin/harmony-module.js ***!
  \*******************************************/
/*! no static exports found */
/***/ (function(module, exports) {

eval("module.exports = function(originalModule) {\n\tif (!originalModule.webpackPolyfill) {\n\t\tvar module = Object.create(originalModule);\n\t\t// module.parent = undefined by default\n\t\tif (!module.children) module.children = [];\n\t\tObject.defineProperty(module, \"loaded\", {\n\t\t\tenumerable: true,\n\t\t\tget: function() {\n\t\t\t\treturn module.l;\n\t\t\t}\n\t\t});\n\t\tObject.defineProperty(module, \"id\", {\n\t\t\tenumerable: true,\n\t\t\tget: function() {\n\t\t\t\treturn module.i;\n\t\t\t}\n\t\t});\n\t\tObject.defineProperty(module, \"exports\", {\n\t\t\tenumerable: true\n\t\t});\n\t\tmodule.webpackPolyfill = 1;\n\t}\n\treturn module;\n};\n\n\n//# sourceURL=webpack:///(webpack)/buildin/harmony-module.js?");

/***/ })

}]);