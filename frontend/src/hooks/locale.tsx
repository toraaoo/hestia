import { createContext, type ReactNode, useContext, useState } from 'react';

import { getLocale, type locales, setLocale } from '@/paraglide/runtime.js';

export type Locale = (typeof locales)[number];

interface LocaleContextValue {
  locale: Locale;
  changeLocale: (locale: Locale) => void;
}

const LocaleContext = createContext<LocaleContextValue | null>(null);

/**
 * Locale as React state: a change persists through Paraglide (localStorage)
 * and re-renders in place — no page reload, the way a desktop app switches
 * language. The subtree is keyed by locale so a change remounts it and every
 * rendered message re-evaluates; the router and query client live outside
 * React, so the route and caches survive the remount.
 */
export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(getLocale());

  const changeLocale = (next: Locale) => {
    if (next === locale) return;
    setLocale(next, { reload: false });
    document.documentElement.lang = next;
    setLocaleState(next);
  };

  return (
    <LocaleContext.Provider value={{ locale, changeLocale }}>
      <LocaleBoundary key={locale}>{children}</LocaleBoundary>
    </LocaleContext.Provider>
  );
}

function LocaleBoundary({ children }: { children: ReactNode }) {
  return children;
}

export function useLocale(): LocaleContextValue {
  const ctx = useContext(LocaleContext);
  if (!ctx) throw new Error('useLocale must be used within LocaleProvider');
  return ctx;
}
