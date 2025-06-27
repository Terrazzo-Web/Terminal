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
                    workspaceFolders: [{
                        uri: rootUri,
                        name: "Rust"
                    }],
                    transport: new JsDeps.WebSocketTransport('ws://127.0.0.1:3002'),
                    autoClose: true,
                    capabilities: {
                        textDocument: {
                            hover: {
                                dynamicRegistration: true,
                                contentFormat: ["markdown", "plaintext"],
                            },
                            publishDiagnostics: {
                                relatedInformation: true,
                                versionSupport: true,
                                tagSupport: {
                                    valueSet: [
                                        1,
                                        2
                                    ]
                                },
                                codeDescriptionSupport: true,
                                dataSupport: true
                            },

                            moniker: {},
                            synchronization: {
                                dynamicRegistration: true,
                                willSave: true,
                                didSave: true,
                                willSaveWaitUntil: false,
                                change: "full",  // send full document on every edit
                            },
                            codeAction: {
                                dynamicRegistration: true,
                                codeActionLiteralSupport: {
                                    codeActionKind: {
                                        valueSet: [
                                            "",
                                            "quickfix",
                                            "refactor",
                                            "refactor.extract",
                                            "refactor.inline",
                                            "refactor.rewrite",
                                            "source",
                                            "source.organizeImports",
                                        ],
                                    },
                                },
                                resolveSupport: {
                                    properties: ["edit"],
                                },
                            },
                            completion: {
                                dynamicRegistration: true,
                                completionItem: {
                                    snippetSupport: true,
                                    commitCharactersSupport: true,
                                    documentationFormat: ["markdown", "plaintext"],
                                    deprecatedSupport: false,
                                    preselectSupport: false,
                                },
                                contextSupport: false,
                            },
                            signatureHelp: {
                                dynamicRegistration: true,
                                signatureInformation: {
                                    documentationFormat: ["markdown", "plaintext"],
                                },
                            },
                            declaration: {
                                dynamicRegistration: true,
                                linkSupport: true,
                            },
                            definition: {
                                dynamicRegistration: true,
                                linkSupport: true,
                            },
                            typeDefinition: {
                                dynamicRegistration: true,
                                linkSupport: true,
                            },
                            implementation: {
                                dynamicRegistration: true,
                                linkSupport: true,
                            },
                            rename: {
                                dynamicRegistration: true,
                                prepareSupport: true,
                            },
                        },
                        workspace: {
                            didChangeConfiguration: {
                                dynamicRegistration: true,
                            },
                        },
                    },
                })
            };
        }

        let languageServer = JsDeps.languageServerWithClient({
            client: languageClient.client,
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
