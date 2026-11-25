// A minimal module returning v128.const zero. Unsupported engines use the scalar build.
const simd = WebAssembly.validate(Uint8Array.of(
  0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0,
  10, 22, 1, 20, 0, 253, 12, ...Array(16).fill(0), 11
));
let api;
let loading;
export default async function init(options) {
  loading ??= simd ? import('./pkg-simd/sim_gui.js') : import('./pkg/sim_gui.js');
  api = await loading;
  const instance = await api.default(options);
  init.__wbindgen_wasm_module = api.default.__wbindgen_wasm_module;
  return instance;
}
// Keep module evaluation synchronous so new workers can accept their first message immediately.
export const start_web = (...args) => api.start_web(...args);
export const run_web_job = (...args) => api.run_web_job(...args);
export const run_web_bulk_shard = (...args) => api.run_web_bulk_shard(...args);
export const run_web_bulk_shard_quiet = (...args) => api.run_web_bulk_shard_quiet(...args);
export const merge_web_bulk_shards = (...args) => api.merge_web_bulk_shards(...args);
