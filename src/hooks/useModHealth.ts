import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useChangeRevision } from '../services/changes';
import { CatalogModSummary, HealthInputs, HealthReport, ModInfo } from '../types';
import { HealthChanges, healthChanges } from '../utils/healthChanges';
import { StudioPathsUI } from '../components/settings/SettingsModule';
import { TauriService, reportBackendError } from '../services/tauri';

/**
 * Runs the Rust health check (game-exact rules) for the enabled mods, in order, shortly after
 * the list changes. The single source of truth for dependency, order and compatibility problems.
 */
export function useModHealth(paths: StudioPathsUI, mods: ModInfo[]) {
  const [report, setReport] = useState<HealthReport | null>(null);
  const [catalog, setCatalog] = useState<CatalogModSummary[]>([]);
  const [checking, setChecking] = useState(false);
  /** Where the check under way is: the quick one (order, dependencies, rules), the full one (every file of every mod), or the scripts two mods declare. */
  const [stage, setStage] = useState<'quick' | 'full' | 'scripts' | null>(null);
  // Only what touches the list: a note or a setting saved elsewhere is no reason to read every
  // script of every mod again.
  const changeRev = useChangeRevision(['mods', 'profiles']);
  const requestId = useRef(0);
  /** A full check has answered once: from then on the quick one goes first (it borrows its findings). */
  const hasReport = useRef(false);
  /** The scripts two active mods declare, from the last reading: kept on each new report until the next reading is back. */
  const declaredTwiceLast = useRef<HealthReport['issues']>([]);
  const withDeclaredTwice = (r: HealthReport): HealthReport => ({ ...r, issues: [...r.issues, ...declaredTwiceLast.current] });
  /** When the last full check ended, and what it read (to tell later whether a new one would find anything new). */
  const [checkedAt, setCheckedAt] = useState<number | null>(null);
  const checkedInputs = useRef<{ inputs: HealthInputs; order: string } | null>(null);
  const [changes, setChanges] = useState<HealthChanges | null>(null);

  const activeIds = useMemo(() => mods.filter((m) => m.enabled).map((m) => m.mod_id), [mods]);
  // Re-check only when the active list really changes (not on unrelated re-renders).
  const activeKey = activeIds.join('\n');

  /** Checks again while a part of the check is read behind, a few times at most. */
  const behindTries = useRef(0);
  const behindTimer = useRef<number | undefined>(undefined);

  const recheck = useCallback(async () => {
    if (!paths.is_valid || mods.length === 0) return;
    const id = ++requestId.current;
    setChecking(true);
    setStage(hasReport.current ? 'quick' : 'full');
    try {
      const active = activeKey ? activeKey.split('\n') : [];
      // In two steps: what the order, the dependencies and the rules say comes back in well under a
      // second (the slow findings from the last full check), so a switch or a move answers at once;
      // the full check, reading every file of every mod (seconds on a long list), then replaces it.
      if (hasReport.current) {
        const quick = await TauriService.getModHealth(paths, active, true);
        if (id !== requestId.current) return;
        setReport({ ...withDeclaredTwice(quick.report), partial: true });
        setCatalog(quick.catalog.mods);
        // The full check reads every file of every mod: several changes in a row (moving a mod
        // step by step, switching a few) run it once, after the last, instead of one each that
        // would only be thrown away while they kept the PC busy.
        setStage('full');
        await new Promise((resolve) => window.setTimeout(resolve, 1200));
        if (id !== requestId.current) return;
      }
      // What this check reads, taken before it reads: a change made while it runs shows as a change.
      const inputs = await TauriService.healthInputs(paths, active).catch(() => null);
      const health = await TauriService.getModHealth(paths, active);
      hasReport.current = true;
      if (id === requestId.current) {
        checkedInputs.current = inputs ? { inputs, order: activeKey } : null;
        setCheckedAt(Date.now());
        setChanges(null);
      }
      // A part read behind (a cold disk, a long list): look again shortly, a few times.
      window.clearTimeout(behindTimer.current);
      if ((health.report.incomplete ?? []).length > 0 && behindTries.current < 4) {
        behindTries.current += 1;
        behindTimer.current = window.setTimeout(() => void recheckRef.current(), 15000);
      } else if ((health.report.incomplete ?? []).length === 0) {
        behindTries.current = 0;
      }
      if (id === requestId.current) {
        setReport(withDeclaredTwice(health.report));
        setCatalog(health.catalog.mods);
        setStage('scripts');
      }
      // Reading every script of every active mod takes a moment, so it follows the check the Mod
      // List is waiting on instead of holding it up.
      const declaredTwice = await TauriService.scriptConflicts(paths, active);
      if (id === requestId.current) {
        declaredTwiceLast.current = declaredTwice;
        setReport({ ...health.report, issues: [...health.report.issues, ...declaredTwice] });
      }
    } catch (err) {
      if (id === requestId.current) reportBackendError('Checking mod health', err);
    } finally {
      if (id === requestId.current) {
        setChecking(false);
        setStage(null);
      }
    }
  }, [paths, activeKey, mods.length, changeRev]);

  const recheckRef = useRef(recheck);
  recheckRef.current = recheck;
  useEffect(() => () => window.clearTimeout(behindTimer.current), []);

  useEffect(() => {
    const t = setTimeout(recheck, 300); // toggles come in bursts
    return () => clearTimeout(t);
  }, [recheck]);

  // Mod id -> the copy the game uses (source, origin), for source tags in the UI.
  const catalogById = useMemo(() => {
    const map = new Map<string, CatalogModSummary>();
    for (const m of catalog) if (!m.shadowed_by) map.set(m.id, m);
    return map;
  }, [catalog]);

  /**
   * Compares what the last full check read with what is there now, without reading mod files (a
   * fraction of a second): Mod Health says it when opened instead of checking again by itself.
   */
  const lookForChanges = useCallback(async () => {
    const then = checkedInputs.current;
    if (!then || !paths.is_valid) return;
    try {
      const now = await TauriService.healthInputs(paths, activeKey ? activeKey.split('\n') : []);
      const c = healthChanges(then.inputs, now);
      setChanges({ ...c, list: c.list || then.order !== activeKey });
    } catch {
      /* nothing to say: the button is there */
    }
  }, [paths, activeKey]);

  return { report, checking, stage, recheck, catalog, catalogById, checkedAt, changes, lookForChanges };
}
