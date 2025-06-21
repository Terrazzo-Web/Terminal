class CodeMirrorJs {
    editorView;
    constructor(element, content, onchange) {
        const updateListener = JsDeps.EditorView.updateListener.of((update) => {
            if (update.docChanged) {
                const content = update.state.doc.toString();
                onchange(content);
            }
        });

        // Create a WebSocket transport
        const transport = new JsDeps.WebSocketTransport('ws://localhost:3002');

        // Configure the language server plugin
        const ls = JsDeps.languageServer({
            transport,
            rootUri: 'file:///home/richard/Documents/Github/Terminal',
            documentUri: 'file:///home/richard/Documents/Github/Terminal/terminal/src/text_editor/remotes.rs',
            languageId: 'rust',

            // Optional: Customize keyboard shortcuts
            keyboardShortcuts: {
                rename: 'F2',                // Default: F2
                goToDefinition: 'ctrlcmd',   // Ctrl/Cmd + Click
            },

            // Optional: Allow HTML content in tooltips
            allowHTMLContent: true,
        });

        const state = JsDeps.EditorState.create({
            doc: content,
            extensions: [JsDeps.basicSetup, updateListener, ls]
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
