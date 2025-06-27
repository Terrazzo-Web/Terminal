const languageClient = new JsDeps.LanguageServerClient({
    transport: new JsDeps.WebSocketTransport('ws://127.0.0.1:3002'),
    autoClose: true,
});

class CodeMirrorJs {
    editorView;
    constructor(
        element,
        content,
        onchange,
        basePath,
        fullPath,
    ) {
        const updateListener = JsDeps.EditorView.updateListener.of((update) => {
            if (update.docChanged) {
                const content = update.state.doc.toString();
                onchange(content);
            }
        });

        let languageServer = JsDeps.languageServerWithClient({
            client: languageClient,
            rootUri: `file://${basePath}`,
            documentUri: `file://${fullPath}`,
            languageId: 'rust',
            keyboardShortcuts: {
                rename: 'F2',
                goToDefinition: 'ctrlcmd',
            },
            allowHTMLContent: true,
        });

        const state = JsDeps.EditorState.create({
            doc: content,
            tooltips: JsDeps.tooltips({
                position: "absolute",
            }),
            extensions: [
                JsDeps.basicSetup,
                JsDeps.lintGutter(),
                JsDeps.oneDark,
                JsDeps.rust(),
                languageServer,
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
