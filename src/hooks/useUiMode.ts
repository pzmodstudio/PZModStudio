import { useEffect, useState } from 'react';
import { UI_MODE_EVENT, UiMode, readUiMode, setUiMode } from '../utils/uiMode';

/** The Simple / Complete mode, kept in step across every screen that reads it. */
export function useUiMode(): [UiMode, (mode: UiMode) => void] {
  const [mode, setMode] = useState<UiMode>(() => readUiMode());
  useEffect(() => {
    const onChange = (e: Event) => setMode((e as CustomEvent<UiMode>).detail);
    window.addEventListener(UI_MODE_EVENT, onChange);
    return () => window.removeEventListener(UI_MODE_EVENT, onChange);
  }, []);
  return [mode, setUiMode];
}
