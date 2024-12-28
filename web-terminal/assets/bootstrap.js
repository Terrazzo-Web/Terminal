import init, { start } from './wasm/web_terminal.js';

function run() {
    start();
}

init().then(run)
