// JsDeps

import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';

import { basicSetup } from "codemirror";
import { EditorState } from '@codemirror/state';
import { EditorView, tooltips } from "@codemirror/view";
import { oneDark } from '@codemirror/theme-one-dark';
import { rust } from "@codemirror/lang-rust"

import { LanguageServerClient, languageServerWithClient } from '@marimo-team/codemirror-languageserver';
import { WebSocketTransport } from '@open-rpc/client-js';
import { lintGutter } from '@codemirror/lint';

// Export them for Webpack to expose as globals
export {
    Terminal,
    FitAddon,
    WebLinksAddon,

    basicSetup,
    EditorState,
    EditorView,
    tooltips,

    oneDark,
    rust,

    LanguageServerClient,
    languageServerWithClient,
    WebSocketTransport,
    lintGutter
};
