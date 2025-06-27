// JsDeps

import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';

import { basicSetup } from "codemirror";
import { EditorState } from '@codemirror/state';
import { EditorView, tooltips } from "@codemirror/view";

import { oneDark } from '@codemirror/theme-one-dark';
import { lintGutter } from '@codemirror/lint';

import { cpp } from "@codemirror/lang-cpp"
import { css } from "@codemirror/lang-css"
import { go } from "@codemirror/lang-go"
import { html } from "@codemirror/lang-html"
import { java } from "@codemirror/lang-java"
import { json } from "@codemirror/lang-json"
import { markdown } from "@codemirror/lang-markdown"
import { python } from "@codemirror/lang-python"
import { sass } from "@codemirror/lang-sass"
import { xml } from "@codemirror/lang-xml"
import { yaml } from "@codemirror/lang-yaml"
import { rust } from "@codemirror/lang-rust"

const languages = {
    cpp, "c++": cpp, "h": cpp, "hpp": cpp,
    css,
    go,
    html,
    java,
    json,
    md: markdown,
    py: python,
    sass,
    xml,
    yml: yaml, yaml,
    rs: rust,
};

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
    lintGutter,
    languages,
};
