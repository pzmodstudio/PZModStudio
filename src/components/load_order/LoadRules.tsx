import React, { useEffect, useMemo, useState } from 'react';
import { ModInfo, UserRule } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { ListOrdered, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';

interface LoadRulesProps {
  paths: StudioPathsUI;
  /** The mod the rules are about; null = closed. */
  mod: ModInfo | null;
  mods: ModInfo[];
  onClose: () => void;
  /** A rule was added or removed: the sort and the health check read them again. */
  onRulesChanged: () => void;
  toast: (message: string, ms?: number) => void;
  /** The game build installed now, so a proposed rule says what it was tested on. */
  installedBuild?: string | null;
}

/**
 * The player's own rules for one mod: which mods it must load after, and which mods must load
 * after it. The sort honours them like a mod's own loadModAfter; the health check says when the
 * list breaks one.
 */
export const LoadRules: React.FC<LoadRulesProps> = ({ paths, mod, mods, onClose, onRulesChanged, toast, installedBuild = null }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [rules, setRules] = useState<UserRule[] | null>(null);
  const [search, setSearch] = useState('');
  const [note, setNote] = useState('');
  const [busy, setBusy] = useState(false);

  const reload = async () => setRules(await TauriService.listLoadRules(paths));
  useEffect(() => {
    if (!mod) return;
    setRules(null);
    setSearch('');
    setNote('');
    void reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mod?.mod_id, changeRev]);

  const nameOf = (id: string) => mods.find((m) => m.mod_id === id)?.name ?? id;

  const candidates = useMemo(() => {
    if (!mod) return [];
    const q = search.trim().toLowerCase();
    if (!q) return [];
    return mods
      .filter((m) => m.mod_id !== mod.mod_id && !m.mod_id.startsWith('Z_PZModStudio_'))
      .filter((m) => m.name.toLowerCase().includes(q) || m.mod_id.toLowerCase().includes(q))
      .slice(0, 12);
  }, [mods, mod, search]);

  if (!mod) return null;
  const mine = (rules ?? []).filter((r) => r.mod_id === mod.mod_id);
  const theirs = (rules ?? []).filter((r) => r.after === mod.mod_id);

  const add = async (after: ModInfo) => {
    setBusy(true);
    try {
      await TauriService.addLoadRule(paths, mod.mod_id, after.mod_id, note.trim() || null);
      toast(t('dialogs.rules.added', { name: mod.name, after: after.name }), 5000);
      setSearch('');
      setNote('');
      await reload();
      onRulesChanged();
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(false);
    }
  };

  const remove = async (rule: UserRule) => {
    setBusy(true);
    try {
      await TauriService.removeLoadRule(paths, rule.mod_id, rule.after);
      await reload();
      onRulesChanged();
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(false);
    }
  };

  /**
   * A rule of the player's, offered to everyone: the JSON line for community/rules.json goes to
   * the clipboard, named with the build it was tested on, and GitHub opens on that file. A pull
   * request is the rest; nothing is sent from here.
   */
  const propose = async (rule: UserRule) => {
    const entry = {
      mod_id: rule.mod_id,
      after: rule.after,
      note: rule.note ?? t('dialogs.rules.defaultNote', { name: nameOf(rule.mod_id), after: nameOf(rule.after) }),
      source: t('dialogs.rules.testedOn', { build: installedBuild ?? t('dialogs.rules.buildUnknown') }),
    };
    if (await TauriService.githubTokenStatus()) {
      // With a token: the Studio adds the rule to the file on a branch of the fork and opens the pull request.
      toast(t('dialogs.rules.openingPr'), 20000);
      try {
        const r = await TauriService.proposeRule(entry);
        toast(t('dialogs.rules.prOpened', { url: r.pr_url }), 10000);
        TauriService.openExternalUrl(r.pr_url);
      } catch {
        // Already shown by TauriService.
      }
      return;
    }
    await navigator.clipboard.writeText(JSON.stringify(entry, null, 2));
    toast(t('dialogs.rules.clipboard'), 8000);
    TauriService.openExternalUrl('https://github.com/pzmodstudio/PZModStudio/edit/main/community/rules.json');
  };

  const Row = ({ rule, text }: { rule: UserRule; text: React.ReactNode }) => (
    <div className="p-2.5 flex items-start justify-between gap-3 hover:bg-slate-950/60 transition">
      <div className="min-w-0">
        <div className="text-xs text-slate-200">{text}</div>
        {rule.note && <div className="text-[10px] text-slate-500 mt-0.5">{rule.note}</div>}
      </div>
      <div className="flex items-center gap-1.5 shrink-0">
        <button
          onClick={() => void propose(rule)}
          className="px-2 py-1 rounded-lg text-[11px] text-cyan-300 hover:text-cyan-200 border border-slate-800 hover:border-cyan-900 cursor-pointer transition"
          title={t('dialogs.rules.shareTitle')}
        >
          {t('dialogs.rules.share')}
        </button>
        <button
          onClick={() => void remove(rule)}
          disabled={busy}
          className="px-2 py-1 rounded-lg text-[11px] text-slate-400 hover:text-red-300 border border-slate-800 hover:border-red-900 cursor-pointer transition disabled:opacity-50"
          title={t('dialogs.rules.removeTitle')}
        >
          {t('dialogs.rules.remove')}
        </button>
      </div>
    </div>
  );

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-6" onClick={onClose}>
      <div className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-xl p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
              <ListOrdered className="w-4 h-4 text-cyan-400" />
              <span>{t('dialogs.rules.title', { name: mod.name })}</span>
            </h3>
            <p className="text-[11px] text-slate-400 mt-0.5">
              {t('dialogs.rules.blurb')}
            </p>
          </div>
          <button onClick={onClose} className="text-slate-500 hover:text-slate-300 p-1 cursor-pointer">
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>

        <div className="space-y-2">
          <div className="text-[10px] font-bold uppercase tracking-wider text-slate-500">{t('dialogs.rules.mustLoadAfter', { name: mod.name })}</div>
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t('dialogs.rules.whichMod')}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500"
          />
          {candidates.length > 0 && (
            <div className="rounded-xl border border-slate-800 divide-y divide-slate-900 max-h-48 overflow-y-auto">
              {candidates.map((m) => (
                <button
                  key={m.mod_id}
                  type="button"
                  disabled={busy}
                  onClick={() => void add(m)}
                  className="w-full text-left px-3 py-2 text-xs text-slate-300 hover:bg-slate-950/70 cursor-pointer flex items-center justify-between gap-2 disabled:opacity-50"
                >
                  <span className="truncate">{m.name}</span>
                  <span className="font-mono text-[10px] text-slate-500 shrink-0">{m.mod_id}</span>
                </button>
              ))}
            </div>
          )}
          <input
            type="text"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder={t('dialogs.rules.notePlaceholder')}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-[11px] text-slate-300 focus:outline-none focus:border-emerald-500"
          />
        </div>

        {rules === null ? (
          <p className="text-xs text-slate-400 py-2 text-center">{t('dialogs.rules.reading')}</p>
        ) : mine.length === 0 && theirs.length === 0 ? (
          <p className="text-xs text-slate-400 py-2 text-center">{t('dialogs.rules.none')}</p>
        ) : (
          <div className="rounded-xl border border-slate-800 divide-y divide-slate-800 max-h-64 overflow-y-auto">
            {mine.map((r) => (
              <Row key={`${r.mod_id}>${r.after}`} rule={r} text={<>{t('dialogs.rules.loadsAfter')} <b>{nameOf(r.after)}</b></>} />
            ))}
            {theirs.map((r) => (
              <Row key={`${r.mod_id}>${r.after}`} rule={r} text={<><b>{nameOf(r.mod_id)}</b> {t('dialogs.rules.loadsAfterThis')}</>} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
