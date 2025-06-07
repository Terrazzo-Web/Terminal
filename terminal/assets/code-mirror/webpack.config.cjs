const path = require('path');

module.exports = {
    mode: 'production',
    entry: './src/index.js',
    output: {
        filename: 'codemirror.js',
        path: path.resolve(__dirname, 'dist'),
        library: 'CodeMirror',
        libraryTarget: 'window',
    },
    resolve: {
        extensions: ['.js']
    }
};
