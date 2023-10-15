import type { Config } from "tailwindcss"
import plugin from "tailwindcss/plugin"
import typography from "@tailwindcss/typography"

export default {
	content: ["src/**/*.{rs,css,ts}", "templates/**/*.html"],
	theme: {
		extend: {},
	},
	plugins: [plugin(({ matchUtilities, theme }) => {
		matchUtilities(
			{
				"auto-grid": (value) => ({
					display: "grid",
					"grid-template-columns": `repeat(auto-fill, minmax(min(${value}, 100%), 1fr))`,
				}),
			},
			{ values: theme("width") }
		)
	}), typography],
} satisfies Config

