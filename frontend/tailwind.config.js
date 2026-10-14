/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      // Design system "Aurora": every page styles itself with the slate /
      // blue / cyan utilities, so remapping those scales restyles the whole
      // app consistently. slate → cool "ink" neutrals, blue → nebula violet,
      // cyan → aurora teal (blue→cyan gradients become violet→teal).
      colors: {
        slate: {
          50: '#f6f7fb', 100: '#eef0f6', 200: '#dfe2ec', 300: '#c5cada', 400: '#9097ad',
          500: '#6b7289', 600: '#4f5569', 700: '#353a4c', 800: '#222636', 900: '#151824', 950: '#0b0d16',
        },
        blue: {
          50: '#f0efff', 100: '#e3e1ff', 200: '#cbc6ff', 300: '#aba2fd', 400: '#8f7ffa',
          500: '#7a62f3', 600: '#6847e6', 700: '#5837c9', 800: '#472fa2', 900: '#3b2a80', 950: '#231849',
        },
        cyan: {
          50: '#ebfdfa', 100: '#cdfaf2', 200: '#9ff3e6', 300: '#62e6d6', 400: '#2fcfc1',
          500: '#16b2a6', 600: '#0f8f88', 700: '#11726e', 800: '#135b59', 900: '#144b4a', 950: '#052c2c',
        },
      },
      fontFamily: {
        sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        display: ['Sora', 'Inter', 'sans-serif'],
        mono: ['JetBrains Mono', 'ui-monospace', 'monospace'],
      },
      animation: {
        'fade-in': 'fadeIn 0.3s ease-out',
        'slide-up': 'slideUp 0.3s ease-out',
        'glass-shimmer': 'glassShimmer 4s ease-in-out infinite',
        'float': 'float 6s ease-in-out infinite',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(8px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        glassShimmer: {
          '0%, 100%': { backgroundPosition: '0% 50%' },
          '50%': { backgroundPosition: '100% 50%' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-12px)' },
        },
      },
      backgroundSize: {
        '200%': '200% 200%',
      },
    },
  },
  plugins: [],
}
