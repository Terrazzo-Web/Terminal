class CodeMirrorJs {
    editorView;
    constructor(element, content, onchange) {
        const updateListener = JsDeps.EditorView.updateListener.of((update) => {
            if (update.docChanged) {
                const content = update.state.doc.toString();
                onchange(content);
            }
        });

        const state = JsDeps.EditorState.create({
            doc: content,
            extensions: [JsDeps.basicSetup, updateListener]
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
