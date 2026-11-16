import { useCallback, useEffect, useRef, useState } from 'react';
import { HealthReport } from '../types';
import { StudioPathsUI } from '../components/settings/SettingsModule';
import { TauriService, reportBackendError } from '../services/tauri';

/**
 * Mod Health for a list that is not the active profile's (a server's `Mods=`): the same check,
 * run on demand, kept apart from the app's own (useModHealth), which the sidebar and the Mod List
 * read and which must stay the profile's. Null ids: nothing is checked.
 */
export function useListHealth(paths: StudioPathsUI, ids: string[] | null) {
  const [report, setReport] = useState<HealthReport | null>(null);
  const [checking, setChecking] = useState(false);
  const requestId = useRef(0);
  const key = ids ? ids.join('\n') : null;

  const recheck = useCallback(async () => {
    if (key === null || !paths.is_valid) return;
    const id = ++requestId.current;
    setChecking(true);
    try {
      const active = key ? key.split('\n') : [];
      const health = await TauriService.getModHealth(paths, active);
      if (id !== requestId.current) return;
      setReport(health.report);
      // What two mods of the list both declare, as the profile's check adds it.
      const declaredTwice = await TauriService.scriptConflicts(paths, active);
      if (id === requestId.current) setReport({ ...health.report, issues: [...health.report.issues, ...declaredTwice] });
    } catch (err) {
      if (id === requestId.current) reportBackendError("Checking the server's mods", err);
    } finally {
      if (id === requestId.current) setChecking(false);
    }
  }, [paths, key]);

  useEffect(() => {
    setReport(null);
    if (key !== null) void recheck();
  }, [key, recheck]);

  return { report, checking, recheck };
}
