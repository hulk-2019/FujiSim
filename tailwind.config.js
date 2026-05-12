/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        border: "hsl(240 4% 16%)",
        input: "hsl(240 4% 16%)",
        ring: "hsl(240 5% 65%)",
        background: "hsl(240 10% 4%)",
        foreground: "hsl(0 0% 98%)",
        primary: {
          DEFAULT: "hsl(0 0% 98%)",
          foreground: "hsl(240 6% 10%)",
        },
        secondary: {
          DEFAULT: "hsl(240 4% 16%)",
          foreground: "hsl(0 0% 98%)",
        },
        muted: {
          DEFAULT: "hsl(240 4% 16%)",
          foreground: "hsl(240 5% 65%)",
        },
        accent: {
          DEFAULT: "hsl(240 4% 16%)",
          foreground: "hsl(0 0% 98%)",
        },
        destructive: {
          DEFAULT: "hsl(0 63% 31%)",
          foreground: "hsl(0 0% 98%)",
        },
        card: {
          DEFAULT: "hsl(240 10% 4%)",
          foreground: "hsl(0 0% 98%)",
        },
        fuji: {
          provia: "#5a8fbe",
          velvia: "#d14a3a",
          astia: "#e6a979",
          classic: "#9b7e5e",
          neg: "#7a8a6e",
          acros: "#2a2a2a",
        },
      },
      borderRadius: {
        lg: "0.625rem",
        md: "0.5rem",
        sm: "0.375rem",
      },
    },
  },
  plugins: [],
};
