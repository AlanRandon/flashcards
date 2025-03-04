const CARGO_MANIFEST_DIR = require("node:process").env.CARGO_MANIFEST_DIR;

/** @type {import('postcss-load-config').Config} */
const config = {
  plugins: {
    "postcss-import": {},
    "postcss-url": [
      {
        filter: "node_modules/**/*",
        url: "copy",
        assetsPath: `${CARGO_MANIFEST_DIR}/dist/static`,
        useHash: true,
      },
      {
        filter: "src/**/*",
        url: "copy",
        assetsPath: `${CARGO_MANIFEST_DIR}/dist/static`,
        useHash: true,
      },
    ],
    "@tailwindcss/postcss": {},
    autoprefixer: {},
    cssnano: {},
  },
};

module.exports = config;
