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

        let extensions = [
            JsDeps.basicSetup,
            JsDeps.lintGutter(),
            JsDeps.oneDark,
            updateListener,
        ];
        const language = getLanguage(fullPath);
        if (language) {
            extensions.push(language());
        }

        const state = JsDeps.EditorState.create({
            doc: content,
            tooltips: JsDeps.tooltips({
                position: "absolute",
            }),
            extensions,
        });

        this.editorView = new JsDeps.EditorView({
            state,
            parent: element,
        });
    }
}

function getLanguage(fileName) {
    const lastDotIndex = fileName.lastIndexOf('.');
    if (lastDotIndex === -1 || lastDotIndex === fileName.length - 1) {
        return null;
    }

    const ext = fileName.slice(lastDotIndex + 1).toLowerCase();
    return JsDeps.languages[ext] || null;
}

export {
    CodeMirrorJs
};
