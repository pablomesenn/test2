/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,jsx}'],
  theme: {
    extend: {
      fontFamily: {
        sans: ['"IBM Plex Sans"', 'system-ui', 'sans-serif'],
        serif: ['"Instrument Serif"', 'Georgia', 'serif'],
        mono: ['"JetBrains Mono"', '"IBM Plex Mono"', 'ui-monospace', 'monospace'],
      },
      colors: {
        ink: {
          50: '#f7f7f8',
          100: '#ececef',
          200: '#cdcdd2',
          300: '#a4a4ad',
          400: '#75757f',
          500: '#56565f',
          600: '#42424a',
          700: '#34343a',
          800: '#222226',
          900: '#161619',
          950: '#0a0a0c',
        },
        accent: {
          400: '#fbbf24',
          500: '#f59e0b',
          600: '#d97706',
        },
        success: {
          400: '#34d399',
          500: '#10b981',
        },
        danger: {
          400: '#fb7185',
          500: '#f43f5e',
        },
        primary: {
          400: '#60a5fa',
          500: '#3b82f6',
          600: '#2563eb',
        },
      },
      boxShadow: {
        soft: '0 1px 2px rgba(0,0,0,0.4), 0 8px 24px rgba(0,0,0,0.18)',
        glow: '0 0 0 1px rgba(245,158,11,0.35), 0 0 24px rgba(245,158,11,0.18)',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: 0, transform: 'translateY(4px)' },
          '100%': { opacity: 1, transform: 'translateY(0)' },
        },
        slideIn: {
          '0%': { opacity: 0, transform: 'translateX(8px)' },
          '100%': { opacity: 1, transform: 'translateX(0)' },
        },
      },
      animation: {
        fadeIn: 'fadeIn 220ms ease-out',
        slideIn: 'slideIn 220ms ease-out',
      },
    },
  },
  plugins: [],
};
