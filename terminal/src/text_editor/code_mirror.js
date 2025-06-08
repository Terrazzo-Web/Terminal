class CodeMirrorJs {
    editorView;
    constructor(element, content, onchange) {
        const updateListener = CodeMirror.EditorView.updateListener.of((update) => {
            if (update.docChanged) {
                const content = update.state.doc.toString();
                onchange(content);
            }
        });

        const state = CodeMirror.EditorState.create({
            doc: content,
            extensions: [CodeMirror.basicSetup, updateListener]
        });

        this.editorView = new CodeMirror.EditorView({
            state,
            parent: element,
        });
    }
}
export {
    CodeMirrorJs
};
