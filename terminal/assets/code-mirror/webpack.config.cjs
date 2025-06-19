const path = require('path');

module.exports = {
    mode: 'production',
    entry: './src/index.js',
    output: {
        filename: 'jsdeps.js',
        path: path.resolve(__dirname, 'dist'),
        library: 'JsDeps',
        libraryTarget: 'window',
    },
    resolve: {
        extensions: ['.js']
    }
};
