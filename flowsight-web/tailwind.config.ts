import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        background: 'var(--color-background)',
        foreground: 'var(--color-foreground)',
        surface: 'var(--color-surface)',
        border: 'var(--color-border)',
        buy: 'var(--color-buy)',
        sell: 'var(--color-sell)',
        neutral: 'var(--color-neutral)',
        grid: 'var(--color-grid)',
        axis: 'var(--color-axis)',
      },
      screens: {
        mobile: 'var(--breakpoint-mobile)',
        tablet: 'var(--breakpoint-tablet)',
        desktop: 'var(--breakpoint-desktop)',
      },
      spacing: {
        'panel-left': 'var(--panel-left-width)',
        'panel-upper': 'var(--panel-right-upper-height)',
        'panel-lower': 'var(--panel-right-lower-height)',
      },
      animation: {
        'flash-buy': 'flash-buy 0.5s ease-out',
        'flash-sell': 'flash-sell 0.5s ease-out',
      },
      keyframes: {
        'flash-buy': {
          '0%': { backgroundColor: 'var(--color-buy)', opacity: '0.3' },
          '100%': { backgroundColor: 'transparent', opacity: '1' },
        },
        'flash-sell': {
          '0%': { backgroundColor: 'var(--color-sell)', opacity: '0.3' },
          '100%': { backgroundColor: 'transparent', opacity: '1' },
        },
      },
    },
  },
  plugins: [],
};

export default config;