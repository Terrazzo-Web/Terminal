class CodeMirrorJs {
    editorView;
    constructor(element, content) {
        this.editorView = new CodeMirror.EditorView({
            doc: content,
            extensions: [CodeMirror.basicSetup],
            parent: element,
        });
    }
}
export {
    CodeMirrorJs
};
