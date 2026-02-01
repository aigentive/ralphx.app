/**
 * TaskFormFields - Shared styles for task form components
 *
 * macOS Tahoe styling with blue-gray palette (hsl 220 10% xx%).
 * No gradients, no glow shadows - just flat colors and simple transitions.
 */

// ============================================================================
// Shared Input Styles (Tahoe: flat, blue-gray palette)
// ============================================================================

export const inputBaseStyles = `
  w-full h-10 px-3 rounded-lg text-[13px]
  bg-[hsl(220_10%_12%)] border border-[hsla(220_10%_100%/0.08)]
  text-[hsl(220_10%_90%)] placeholder:text-[hsl(220_10%_40%)]
  transition-colors duration-150
  focus:outline-none focus:border-[hsl(14_100%_60%)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

export const selectBaseStyles = `
  w-full h-10 px-3 rounded-lg text-[13px]
  bg-[hsl(220_10%_12%)] border border-[hsla(220_10%_100%/0.08)]
  text-[hsl(220_10%_90%)] cursor-pointer
  transition-colors duration-150
  focus:outline-none focus:border-[hsl(14_100%_60%)]
  disabled:opacity-50 disabled:cursor-not-allowed
  appearance-none
  bg-[url('data:image/svg+xml;charset=utf-8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20width%3D%2216%22%20height%3D%2216%22%20viewBox%3D%220%200%2024%2024%22%20fill%3D%22none%22%20stroke%3D%22hsl(220%2010%25%2050%25)%22%20stroke-width%3D%222%22%3E%3Cpath%20d%3D%22M6%209l6%206%206-6%22%2F%3E%3C%2Fsvg%3E')]
  bg-[length:16px_16px] bg-[right_12px_center] bg-no-repeat
  pr-10
`.replace(/\n/g, ' ').trim();

export const textareaBaseStyles = `
  w-full px-3 py-2.5 rounded-lg text-[13px] leading-relaxed
  bg-[hsl(220_10%_12%)] border border-[hsla(220_10%_100%/0.08)]
  text-[hsl(220_10%_90%)] placeholder:text-[hsl(220_10%_40%)]
  transition-colors duration-150 resize-none
  focus:outline-none focus:border-[hsl(14_100%_60%)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

export const labelStyles = "block text-[11px] font-medium text-[hsl(220_10%_45%)] uppercase tracking-wide mb-2";

// ============================================================================
// Button Styles (Tahoe: flat, no glow shadows)
// ============================================================================

export const buttonPrimaryStyles = `
  h-10 px-4 rounded-lg text-[13px] font-medium
  bg-[hsl(14_100%_60%)] text-white
  transition-colors duration-150
  hover:bg-[hsl(14_100%_55%)]
  focus:outline-none
  disabled:opacity-50 disabled:cursor-not-allowed
  flex items-center justify-center gap-2
`.replace(/\n/g, ' ').trim();

export const buttonSecondaryStyles = `
  h-10 px-4 rounded-lg text-[13px] font-medium
  bg-transparent border border-[hsla(220_10%_100%/0.1)] text-[hsl(220_10%_70%)]
  transition-colors duration-150
  hover:bg-[hsla(220_10%_100%/0.05)] hover:text-[hsl(220_10%_90%)]
  focus:outline-none
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();
