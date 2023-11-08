const path = require("path");

module.exports = {
  entry: {
    style: [
      "normalize.css/normalize.css",
    ]
  },
  output: {
    asyncChunks: false,
    filename: "[name].js",
    module: true,
  },
  experiments: {
    futureDefaults: true,
    outputModule: true,
  },
  module: {
    rules: [
      {
        test: /\.wasm$/,
        type: "webassembly/async",
      },
    ],
  },
  devServer: {
    static: {
      directory: path.join(__dirname, "dist"),
    },
    compress: true,
    port: 8080,
  },
};
