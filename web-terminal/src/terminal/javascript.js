class TerminalJs {
    terminal;
    fitAddon;
    webLinksAddon;
    constructor() {
        this.terminal = new Terminal({
            convertEol: true,
        });
        this.fitAddon = new FitAddon.FitAddon();
        this.webLinksAddon = new WebLinksAddon.WebLinksAddon();
    }
    open(node) {
        this.terminal.loadAddon(this.fitAddon);
        this.terminal.loadAddon(this.webLinksAddon);
        this.terminal.open(node);
        this.fitAddon.fit();
    }
    fit() {
        this.fitAddon.fit();
    }
    focus() {
        this.terminal.focus();
    }
    rows() {
        return this.terminal.rows;
    }
    cols() {
        return this.terminal.cols;
    }
    onData(callback) {
        this.terminal.onData(callback);
    }
    onResize(callback) {
        this.terminal.onResize(callback);
    }
    onTitleChange(callback) {
        this.terminal.onTitleChange(callback);
    }
    async send(data) {
        let terminalJs = this;
        return new Promise(function (resolve, reject) {
            terminalJs.terminal.write(data, function () {
                resolve(true);
            });
        })
    }
    dispose() {
        this.terminal.dispose();
        this.webLinksAddon.dispose();
        this.fitAddon.dispose();
    }
}
export {
    TerminalJs
};
