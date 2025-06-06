class CodeMirrorJs {
    editorView;
    constructor(element, content) {
        this.editorView = new EditorView({
            doc: content,
            extensions: [basicSetup],
            parent: element,
        });
    }
}
export {
    CodeMirrorJs
};
