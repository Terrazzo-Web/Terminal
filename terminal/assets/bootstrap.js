import init, { start } from './wasm/terrazzo_terminal.js';

function run() {
    start();
}

init().then(run)
