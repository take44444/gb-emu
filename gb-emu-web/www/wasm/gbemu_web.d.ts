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
* @param {Function} callback
*/
  set_apu_callback(callback: Function): void;
/**
* @returns {string}
*/
  title(): string;
/**
* @returns {Uint8Array}
*/
  save(): Uint8Array;
/**
* @returns {string}
*/
  to_json(): string;
/**
* @param {string} json
*/
  connect(json: string): void;
/**
*/
  disconnect(): void;
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
* @param {string} k
*/
  key_down2(k: string): void;
/**
* @param {string} k
*/
  key_up2(k: string): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_gameboyhandle_free: (a: number) => void;
  readonly gameboyhandle_new: (a: number, b: number, c: number, d: number) => number;
  readonly gameboyhandle_set_apu_callback: (a: number, b: number) => void;
  readonly gameboyhandle_title: (a: number, b: number) => void;
  readonly gameboyhandle_save: (a: number) => number;
  readonly gameboyhandle_to_json: (a: number, b: number) => void;
  readonly gameboyhandle_connect: (a: number, b: number, c: number) => void;
  readonly gameboyhandle_disconnect: (a: number) => void;
  readonly gameboyhandle_emulate_cycle: (a: number) => number;
  readonly gameboyhandle_frame_buffer: (a: number) => number;
  readonly gameboyhandle_key_down: (a: number, b: number, c: number) => void;
  readonly gameboyhandle_key_up: (a: number, b: number, c: number) => void;
  readonly gameboyhandle_key_down2: (a: number, b: number, c: number) => void;
  readonly gameboyhandle_key_up2: (a: number, b: number, c: number) => void;
  readonly __wbg_audiohandle_free: (a: number) => void;
  readonly audiohandle_new: () => number;
  readonly audiohandle_append: (a: number, b: number, c: number) => void;
  readonly audiohandle_length: (a: number) => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h4adb2439f653c6ac: (a: number, b: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
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
