// JsDeps

import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';

import {
    EditorView,
    basicSetup
} from "codemirror";
import { EditorState } from '@codemirror/state';

// Export them for Webpack to expose as globals
export {
    Terminal,
    FitAddon,
    WebLinksAddon,

    basicSetup,
    EditorState,
    EditorView,
};
