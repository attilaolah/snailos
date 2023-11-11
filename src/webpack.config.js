const path = require("path");

module.exports = {
  entry: {
    style: [
      "normalize.css/normalize.css",
      "xterm/css/xterm.css",
      "./src/css/style.css",
    ],
  },
  output: {
    module: true,
    filename: "[name].js",
    asyncChunks: false,
  },
  experiments: {
    futureDefaults: true,
    outputModule: true,
  },
  module: {
    rules: [{
      test: /\.wasm$/,
      type: "webassembly/async",
    }],
  },
  devServer: {
    static: [{
      directory: __dirname,
    }],
    compress: true,
    port: 8080,
  },
};
