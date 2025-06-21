// JsDeps

import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';

import {
    EditorView,
    basicSetup
} from "codemirror";
import { EditorState } from '@codemirror/state';

import { languageServer } from '@marimo-team/codemirror-languageserver';
import { WebSocketTransport } from '@open-rpc/client-js';

// Export them for Webpack to expose as globals
export {
    Terminal,
    FitAddon,
    WebLinksAddon,

    basicSetup,
    EditorState,
    EditorView,

    languageServer,
    WebSocketTransport,
};
