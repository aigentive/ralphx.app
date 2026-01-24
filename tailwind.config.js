import tailwindcssAnimate from "tailwindcss-animate";

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    // Override default colors with our design system
    colors: {
      transparent: 'transparent',
      current: 'currentColor',

      // Background colors
      bg: {
        base: 'var(--bg-base)',
        surface: 'var(--bg-surface)',
        elevated: 'var(--bg-elevated)',
        hover: 'var(--bg-hover)',
      },

      // Text colors
      text: {
        primary: 'var(--text-primary)',
        secondary: 'var(--text-secondary)',
        muted: 'var(--text-muted)',
      },

      // Accent colors
      accent: {
        primary: 'var(--accent-primary)',
        secondary: 'var(--accent-secondary)',
        hover: 'var(--accent-hover)',
      },

      // Status colors
      status: {
        success: 'var(--status-success)',
        warning: 'var(--status-warning)',
        error: 'var(--status-error)',
        info: 'var(--status-info)',
      },

      // Border colors
      border: {
        subtle: 'var(--border-subtle)',
        DEFAULT: 'var(--border-default)',
        focus: 'var(--border-focus)',
      },
    },

    // Override default spacing with 8pt grid
    spacing: {
      0: 'var(--space-0)',
      1: 'var(--space-1)',
      2: 'var(--space-2)',
      3: 'var(--space-3)',
      4: 'var(--space-4)',
      5: 'var(--space-5)',
      6: 'var(--space-6)',
      8: 'var(--space-8)',
      10: 'var(--space-10)',
      12: 'var(--space-12)',
      16: 'var(--space-16)',
      // Keep some utility values
      px: '1px',
      full: '100%',
      screen: '100vh',
    },

    // Override default font families
    fontFamily: {
      display: 'var(--font-display)',
      body: 'var(--font-body)',
      mono: 'var(--font-mono)',
    },

    // Override default font sizes
    fontSize: {
      xs: 'var(--text-xs)',
      sm: 'var(--text-sm)',
      base: 'var(--text-base)',
      lg: 'var(--text-lg)',
      xl: 'var(--text-xl)',
      '2xl': 'var(--text-2xl)',
      '3xl': 'var(--text-3xl)',
    },

    // Override default border radius
    borderRadius: {
      none: '0',
      sm: 'var(--radius-sm)',
      DEFAULT: 'var(--radius-md)',
      md: 'var(--radius-md)',
      lg: 'var(--radius-lg)',
      xl: 'var(--radius-xl)',
      full: 'var(--radius-full)',
    },

    // Override default box shadows
    boxShadow: {
      sm: 'var(--shadow-sm)',
      DEFAULT: 'var(--shadow-md)',
      md: 'var(--shadow-md)',
      lg: 'var(--shadow-lg)',
      none: 'none',
    },

    // Extend with additional utilities
    extend: {
      // Transition utilities
      transitionDuration: {
        fast: '150ms',
        normal: '200ms',
        slow: '300ms',
      },
    },
  },
  plugins: [tailwindcssAnimate],
}
