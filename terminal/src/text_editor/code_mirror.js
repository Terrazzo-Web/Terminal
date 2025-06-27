let languageClient;

class CodeMirrorJs {
    editorView;
    constructor(
        element,
        content,
        onchange,
        basePath,
        fullPath,
    ) {
        const rootUri = `file://${basePath}`;
        const documentUri = `file://${fullPath}`;
        if (languageClient?.rootUri != rootUri) {
            languageClient?.client?.close();
            languageClient = {
                rootUri,
                client: new JsDeps.LanguageServerClient({
                    rootUri,
                    transport: new JsDeps.WebSocketTransport('ws://127.0.0.1:3002'),
                    autoClose: true,
                })
            };
        }

        let languageServer = JsDeps.languageServerWithClient({
            client: languageClient.client,
            rootUri,
            documentUri,
            languageId: 'rust',
            keyboardShortcuts: {
                rename: 'F2',
                goToDefinition: 'ctrlcmd',
            },
            allowHTMLContent: true,
        });


        const updateListener = JsDeps.EditorView.updateListener.of((update) => {
            if (update.docChanged) {
                const content = update.state.doc.toString();
                onchange(content);
            }
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
