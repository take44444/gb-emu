/* tslint:disable */
/* eslint-disable */
/**
*/
export class AudioHandle {
  free(): void;
/**
* @returns {AudioHandle}
*/
  static new(): AudioHandle;
/**
* @param {Float32Array} buffer
*/
  append(buffer: Float32Array): void;
/**
* @returns {number}
*/
  length(): number;
}
/**
*/
export class GameBoyHandle {
  free(): void;
/**
* @param {Uint8Array} cart_rom
* @param {Uint8Array} save
* @returns {GameBoyHandle}
*/
  static new(cart_rom: Uint8Array, save: Uint8Array): GameBoyHandle;
/**
* @param {Function} apu_callback
* @param {Function} send_callback
*/
  set_callback(apu_callback: Function, send_callback: Function): void;
/**
* @returns {string}
*/
  title(): string;
/**
* @returns {Uint8Array}
*/
  save(): Uint8Array;
/**
* @returns {boolean}
*/
  emulate_cycle(): boolean;
/**
* @returns {Uint8ClampedArray}
*/
  frame_buffer(): Uint8ClampedArray;
/**
* @param {string} k
*/
  key_down(k: string): void;
/**
* @param {string} k
*/
  key_up(k: string): void;
/**
* @returns {any}
*/
  serial_is_master(): any;
/**
* @param {number} val
*/
  serial_receive(val: number): void;
/**
* @returns {number}
*/
  serial_data(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_gameboyhandle_free: (a: number) => void;
  readonly gameboyhandle_new: (a: number, b: number, c: number, d: number) => number;
  readonly gameboyhandle_set_callback: (a: number, b: number, c: number) => void;
  readonly gameboyhandle_title: (a: number) => number;
  readonly gameboyhandle_save: (a: number) => number;
  readonly gameboyhandle_emulate_cycle: (a: number) => number;
  readonly gameboyhandle_frame_buffer: (a: number) => number;
  readonly gameboyhandle_key_down: (a: number, b: number, c: number) => void;
  readonly gameboyhandle_key_up: (a: number, b: number, c: number) => void;
  readonly gameboyhandle_serial_is_master: (a: number) => number;
  readonly gameboyhandle_serial_receive: (a: number, b: number) => void;
  readonly gameboyhandle_serial_data: (a: number) => number;
  readonly __wbg_audiohandle_free: (a: number) => void;
  readonly audiohandle_new: () => number;
  readonly audiohandle_append: (a: number, b: number, c: number) => void;
  readonly audiohandle_length: (a: number) => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h9f8d421cd8441cd2: (a: number, b: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
