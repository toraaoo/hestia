import { useLocation } from '@tanstack/react-router';
import {
  createContext,
  type ReactNode,
  useContext,
  useMemo,
  useState,
} from 'react';

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
  // Nothing unmounts the box on navigation, so the query outlives the page
  // that owns it; reset during render, before the new page reads it.
  const { pathname } = useLocation();
  const section = pathname.split('/')[1] ?? '';
  const [owner, setOwner] = useState(section);
  if (owner !== section) {
    setOwner(section);
    setQuery('');
  }

  const value = useMemo(() => ({ query, setQuery }), [query]);

  return <SearchCtx.Provider value={value}>{children}</SearchCtx.Provider>;
}

export function useSearch(): SearchState {
  const ctx = useContext(SearchCtx);
  if (!ctx) throw new Error('useSearch must be used within SearchProvider');
  return ctx;
}
