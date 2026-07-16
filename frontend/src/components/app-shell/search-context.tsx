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
