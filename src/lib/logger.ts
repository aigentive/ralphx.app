const isDev = import.meta.env.DEV;
const isUiDebug = isDev && __UI_DEBUG__;

export const logger = {
  debug: (...args: unknown[]) => {
    if (isUiDebug) console.debug("[debug]", ...args);
  },
  log: (...args: unknown[]) => {
    if (isDev) console.log(...args);
  },
  warn: console.warn.bind(console),
  error: console.error.bind(console),
};
