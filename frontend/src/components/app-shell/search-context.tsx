import { useLocation } from '@tanstack/react-router';
import { createContext, type ReactNode, useContext, useState } from 'react';

/**
 * The shell's search box lives in the persistent chrome (Topbar) while the
 * list it filters lives in the routed page, so the query is shared through
 * context rather than passed down a tree that spans an <Outlet/>.
 */
interface SearchState {
  query: string;
  setQuery: (value: string) => void;
}

const SearchCtx = createContext<SearchState | null>(null);

export function SearchProvider({ children }: { children: ReactNode }) {
  const [query, setQuery] = useState('');
  // Nothing unmounts the box on navigation, so a query typed for one list
  // would arrive filtering the next. It belongs to the section that owns it:
  // narrowing browse by kind stays within `/browse` and is the same list, so
  // only the first path segment resets it. Reset during render rather than in
  // an effect, so the page that navigated in never renders — or searches —
  // against the query typed for the previous one.
  const { pathname } = useLocation();
  const section = pathname.split('/')[1] ?? '';
  const [owner, setOwner] = useState(section);
  if (owner !== section) {
    setOwner(section);
    setQuery('');
  }

  return (
    <SearchCtx.Provider value={{ query, setQuery }}>
      {children}
    </SearchCtx.Provider>
  );
}

export function useSearch(): SearchState {
  const ctx = useContext(SearchCtx);
  if (!ctx) throw new Error('useSearch must be used within SearchProvider');
  return ctx;
}
