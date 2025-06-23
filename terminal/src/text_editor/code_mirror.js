class CodeMirrorJs {
    editorView;
    constructor(element, content, onchange) {
        const updateListener = JsDeps.EditorView.updateListener.of((update) => {
            if (update.docChanged) {
                const content = update.state.doc.toString();
                onchange(content);
            }
        });

        // // Configure the language server plugin
        // const languageServer = JsDeps.languageServer({
        //     serverUri: 'ws://127.0.0.1:3002',
        //     rootUri: 'file:///home/richard/Documents/Terminal/terminal',
        //     documentUri: 'file:///home/richard/Documents/Terminal/terminal/src/backend.rs',
        //     languageId: 'rust',

        //     // Optional: Customize keyboard shortcuts
        //     keyboardShortcuts: {
        //         rename: 'F2',                // Default: F2
        //         goToDefinition: 'ctrlcmd',   // Ctrl/Cmd + Click
        //     },

        //     // Optional: Allow HTML content in tooltips
        //     allowHTMLContent: true,
        // });

        const state = JsDeps.EditorState.create({
            doc: content,
            extensions: [
                JsDeps.basicSetup,
                JsDeps.oneDark,
                JsDeps.rust(),
                JsDeps.LSPClient({
                    serverUri: "ws://localhost:5000", // Your language server WebSocket URL
                    capabilities: {
                        textDocument: {
                            inlayHints: true
                        }
                    }
                }),
                updateListener,
            ]
        });

        this.editorView = new JsDeps.EditorView({
            state,
            parent: element,
        });
    }
}
export {
    CodeMirrorJs
};
