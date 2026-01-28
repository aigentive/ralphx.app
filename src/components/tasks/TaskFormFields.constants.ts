/**
 * TaskFormFields - Shared styles for task form components
 *
 * Extracted to fix react-refresh/only-export-components lint rule.
 * These constants provide consistent Refined Studio styling across all task forms.
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

// ============================================================================
// Shared Input Styles
// ============================================================================

export const inputBaseStyles = `
  w-full h-10 px-3 rounded-lg text-[13px]
  bg-white/[0.03] border border-white/[0.08]
  text-white/90 placeholder:text-white/30
  transition-all duration-150
  focus:outline-none focus:border-[#ff6b35]/50 focus:bg-white/[0.05]
  focus:shadow-[0_0_0_3px_rgba(255,107,53,0.1)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

export const selectBaseStyles = `
  w-full h-10 px-3 rounded-lg text-[13px]
  bg-white/[0.03] border border-white/[0.08]
  text-white/90 cursor-pointer
  transition-all duration-150
  focus:outline-none focus:border-[#ff6b35]/50 focus:bg-white/[0.05]
  focus:shadow-[0_0_0_3px_rgba(255,107,53,0.1)]
  disabled:opacity-50 disabled:cursor-not-allowed
  appearance-none
  bg-[url('data:image/svg+xml;charset=utf-8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20width%3D%2216%22%20height%3D%2216%22%20viewBox%3D%220%200%2024%2024%22%20fill%3D%22none%22%20stroke%3D%22rgba(255%2C255%2C255%2C0.4)%22%20stroke-width%3D%222%22%3E%3Cpath%20d%3D%22M6%209l6%206%206-6%22%2F%3E%3C%2Fsvg%3E')]
  bg-[length:16px_16px] bg-[right_12px_center] bg-no-repeat
  pr-10
`.replace(/\n/g, ' ').trim();

export const textareaBaseStyles = `
  w-full px-3 py-2.5 rounded-lg text-[13px] leading-relaxed
  bg-white/[0.03] border border-white/[0.08]
  text-white/90 placeholder:text-white/30
  transition-all duration-150 resize-none
  focus:outline-none focus:border-[#ff6b35]/50 focus:bg-white/[0.05]
  focus:shadow-[0_0_0_3px_rgba(255,107,53,0.1)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();

export const labelStyles = "block text-[12px] font-medium text-white/50 uppercase tracking-wide mb-2";

// ============================================================================
// Button Styles
// ============================================================================

export const buttonPrimaryStyles = `
  h-10 px-4 rounded-lg text-[13px] font-medium
  bg-[#ff6b35] text-white
  transition-all duration-150
  hover:bg-[#ff8050] hover:shadow-[0_4px_12px_rgba(255,107,53,0.3)]
  focus:outline-none focus:shadow-[0_0_0_3px_rgba(255,107,53,0.3)]
  disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:shadow-none
  flex items-center justify-center gap-2
`.replace(/\n/g, ' ').trim();

export const buttonSecondaryStyles = `
  h-10 px-4 rounded-lg text-[13px] font-medium
  bg-transparent border border-white/[0.1] text-white/70
  transition-all duration-150
  hover:bg-white/[0.05] hover:border-white/[0.15] hover:text-white/90
  focus:outline-none focus:shadow-[0_0_0_3px_rgba(255,255,255,0.05)]
  disabled:opacity-50 disabled:cursor-not-allowed
`.replace(/\n/g, ' ').trim();
