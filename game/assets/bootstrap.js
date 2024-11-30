import init, { start } from './wasm/game.js';

function run() {
    start();
}

init().then(run)
