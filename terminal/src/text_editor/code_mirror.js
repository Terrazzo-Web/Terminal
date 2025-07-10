class CodeMirrorJs {
    editorView;
    reloadFromDisk; // Set to true when the file is updated from disk
    basePath;
    fullPath;
    constructor(
        element,
        content,
        onchange,
        basePath,
        fullPath,
    ) {
        this.basePath = basePath;
        this.fullPath = fullPath;
        this.reloadFromDisk = true;
        const updateListener = JsDeps.EditorView.updateListener.of((update) => {
            if (!this.reloadFromDisk && update.docChanged) {
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
        this.reloadFromDisk = false;
    }

    set_content(content) {
        this.reloadFromDisk = true;
        this.editorView.dispatch({
            changes: {
                from: 0,
                to: this.editorView.state.doc.length,
                insert: content
            }
        });
        this.reloadFromDisk = false;
    }

    cargo_check(diagnostics) {
        const lints = [];
        let docLength = this.editorView.state.doc.length;
        for (const diagnostic of diagnostics) {
            if (diagnostic.file_path != this.fullPath)
                continue;
            for (const span of diagnostic.spans) {
                if (!this.fullPath.endsWith(span.file_name))
                    continue;
                if (span.byte_end >= docLength)
                    continue;
                const severity = diagnostic.level == "error" || diagnostic.level == "warning"
                    ? diagnostic.level
                    : "info";
                const lint = {
                    from: span.byte_start,
                    to: span.byte_end,
                    severity,
                    source: "cargo-check",
                    message: diagnostic.message,
                };
                lints.push(lint);
            }
        }
        const setLintsTransaction = JsDeps.setDiagnostics(this.editorView.state, lints);
        this.editorView.dispatch(setLintsTransaction);
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
