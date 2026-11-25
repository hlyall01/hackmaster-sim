import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { Worker } from 'node:worker_threads';

// Run the real controller in an isolated JS worker. Only WASM calculations and
// nested-worker transport are stubbed; allocation, startup, progress, and failures
// use the shipped controller unchanged.
const wasmStub = `
export default async function init() { init.__wbindgen_wasm_module = new WebAssembly.Module(new Uint8Array([0,97,115,109,1,0,0,0])); }
export function run_web_job(request) { return JSON.stringify({Ok:{runs:JSON.parse(request).kind.Bulk.runs}}); }
export function run_web_bulk_shard_quiet(request,firstRun,runs) { return '{"Ok":{"count":18446744073709551615,"firstRun":'+firstRun+',"runs":'+runs+'}}'; }
export function merge_web_bulk_shards(request,shards) {
 const input=JSON.parse(request);if(input.testMergeError)throw Error('forced merge failure');
 if(input.checkU64 && shards.split('"count":18446744073709551615').length-1!==JSON.parse(shards).length)throw Error('counter lost precision');
 const ranges=JSON.parse(shards).sort((a,b)=>a.firstRun-b.firstRun);let next=0;
 for(const range of ranges){if(range.firstRun!==next)throw Error('duplicated or missing fight');next+=range.runs;}
 if(next!==input.kind.Bulk.runs)throw Error('incomplete result');return JSON.stringify({Ok:{runs:next}});
}`;
const source = (await readFile(new URL('../web/worker.js', import.meta.url), 'utf8'))
  .replace("'./sim-module.js'", JSON.stringify('data:text/javascript,' + encodeURIComponent(wasmStub)))
  .replace("'./worker-pool.js'", JSON.stringify(new URL('../web/worker-pool.js', import.meta.url).href));
const harness = `
const {parentPort,workerData}=require('node:worker_threads');
globalThis.self={navigator:{hardwareConcurrency:32},postMessage:data=>parentPort.postMessage(data)};
globalThis.Worker=class {
 constructor(){this.dead=false;this.timers=[];}
 later(fn,delay){this.timers.push(setTimeout(()=>{if(!this.dead)fn();},delay));}
 postMessage(data){
  if(!data.request){
   if(workerData.mode==='unavailable')this.later(()=>{let prevented=false;this.onerror?.({message:'forced startup failure',preventDefault(){prevented=true;}});if(!prevented)self.postMessage(JSON.stringify({Err:'startup error bubbled'}));},0);
   else this.later(()=>{parentPort.postMessage({testEvent:'childReady'});this.onmessage?.({data:{ready:true}});},workerData.mode==='delayed'?60000:0);
  }else this.later(()=>{
   let value;
   if(workerData.mode==='failure')value={Err:'forced calculation failure'};
   else if(workerData.mode==='malformed')value={shard:'{broken'};
   else if(data.serialized && workerData.mode==='framed')value={shard:'{"count":18446744073709551615,"firstRun":'+data.firstRun+',"runs":'+data.runs+'}'};
   else value=JSON.stringify({Ok:{firstRun:data.firstRun,runs:data.runs}});
   this.onmessage?.({data:value});
  },1);
 }
 terminate(){this.dead=true;for(const timer of this.timers)clearTimeout(timer);}
};
(async()=>{await import('data:text/javascript,'+encodeURIComponent(workerData.source));parentPort.on('message',data=>self.onmessage({data}));parentPort.postMessage({testEvent:'loaded'});})().catch(error=>{throw error;});
`;

async function calculate(mode, runs = 10000, extra = {}) {
  const worker = new Worker(harness, { eval: true, workerData: { source, mode } });
  const messages = [];
  try {
    return await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('Controller timed out')), 10000);
      worker.on('error', error => { clearTimeout(timeout); reject(error); });
      worker.on('message', data => {
        if (data.testEvent === 'loaded') {
          worker.postMessage(JSON.stringify({ kind: { Bulk: { runs, seed: 1, workers: 24 } }, ...extra }));
        } else if (data.testEvent) {
          messages.push(data);
        } else {
          const value = JSON.parse(data);
          if (value.progress) messages.push(value);
          else { clearTimeout(timeout); resolve({ result: value, messages }); }
        }
      });
    });
  } finally { await worker.terminate(); }
}

test('controller completes without waiting for children that are still starting', async () => {
  const { result, messages } = await calculate('delayed');
  assert.deepEqual(result, { Ok: { runs: 10000 } });
  assert.ok(messages.some(value => value.progress?.[0] > 0));
  assert.ok(!messages.some(value => value.testEvent === 'childReady'));
});

test('controller allocates every fight once when workers join and finish out of order', async () => {
  const { result, messages } = await calculate('normal', 1000000);
  assert.deepEqual(result, { Ok: { runs: 1000000 } });
  const progress = messages.filter(value => value.progress);
  assert.deepEqual(progress[0].progress, [0, 1000000]);
  assert.ok(progress[1].progress[0] > 0);
  let previous = -1;
  for (const value of progress) {
    assert.ok(value.progress[0] >= previous);
    assert.equal(value.progress[1], 1000000);
    previous = value.progress[0];
  }
  assert.equal(previous, 1000000);
});

test('unavailable nested workers cannot abort a successful coordinator calculation', async () => {
  const { result } = await calculate('unavailable', 1000000);
  assert.deepEqual(result, { Ok: { runs: 1000000 } });
});

test('calculation and merge failures publish errors instead of partial results or hanging', async () => {
  assert.match((await calculate('failure', 1000000)).result.Err, /forced calculation failure/);
  assert.match((await calculate('normal', 10000, { testMergeError: true })).result.Err, /forced merge failure/);
});

test('framed child and local shard replies preserve u64 counters, and invalid JSON fails the job', async () => {
  assert.deepEqual((await calculate('framed', 1000000, {checkU64:true})).result, {Ok:{runs:1000000}});
  assert.ok((await calculate('malformed', 1000000)).result.Err);
});
