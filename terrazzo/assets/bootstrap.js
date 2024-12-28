import init, { start } from './wasm/terrazzo.js';

function run() {
    start();
}

init().then(run)
