import type { Config } from "tailwindcss";
// import plugin from "tailwindcss/plugin";

export default {
  theme: {
    extend: {
      typography: {
        DEFAULT: {
          css: {
            maxWidth: "none",
          },
        },
      },
    },
  },
  plugins: [
    // plugin(({ matchUtilities, theme }) => {
    //   matchUtilities(
    //     {
    //       "auto-grid": (value) => ({
    //         display: "grid",
    //         "grid-template-columns": `repeat(auto-fill, minmax(min(${value}, 100%), 1fr))`,
    //       }),
    //     },
    //     { values: theme("width") },
    //   );
    // }),
  ],
} satisfies Config;
