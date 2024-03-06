import type { Config } from "tailwindcss";
import plugin from "tailwindcss/plugin";
import typography from "@tailwindcss/typography";
import forms from "@tailwindcss/forms";

export default {
  content: ["src/**/*.{rs,css,ts}", "templates/**/*.html"],
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
    plugin(({ matchUtilities, theme }) => {
      matchUtilities(
        {
          "auto-grid": (value) => ({
            display: "grid",
            "grid-template-columns": `repeat(auto-fill, minmax(min(${value}, 100%), 1fr))`,
          }),
        },
        { values: theme("width") },
      );
    }),
    typography,
    forms,
  ],
} satisfies Config;
