import type { Config } from 'tailwindcss'

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: '#0b1020',
        panel: '#111936',
        glass: 'rgba(148, 163, 184, 0.08)',
        accent: '#0ea5e9',
        accent2: '#14b8a6',
      },
      boxShadow: {
        glow: '0 0 0 1px rgba(148,163,184,.2), 0 12px 36px rgba(2,6,23,.55)',
      },
      backgroundImage: {
        'material-gradient': 'linear-gradient(135deg, rgba(14,165,233,.28), rgba(20,184,166,.2))',
      },
    },
  },
  plugins: [],
} satisfies Config
