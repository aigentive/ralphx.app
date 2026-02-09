const isDev = import.meta.env.DEV;

export const logger = {
  debug: (...args: unknown[]) => {
    if (isDev) console.debug("[debug]", ...args);
  },
  log: (...args: unknown[]) => {
    if (isDev) console.log(...args);
  },
  warn: console.warn.bind(console),
  error: console.error.bind(console),
};
