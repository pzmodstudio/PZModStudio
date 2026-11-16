import React, { useEffect, useState } from 'react';
import { Check, Pencil, Plus, StickyNote, Tags, Trash2, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ModInfo, ModMeta } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError } from '../../services/tauri';

interface ModMetaDialogProps {
  paths: StudioPathsUI;
  /** The mods (one, or several picked) and the tab to open on; null = closed. Several: groups only. */
  open: { mods: ModInfo[]; tab: 'note' | 'groups' } | null;
  meta: ModMeta;
  onChange: (meta: ModMeta) => void;
  onClose: () => void;
}

/**
 * The player's own note on a mod and the groups they put it in. Both are the app's: the game's
 * Link shows them (and turns a group on or off) but they are made here. Every group change is saved
 * at once; a group name typed and never added, or a rename never confirmed, is saved on closing.
 */
export const ModMetaDialog: React.FC<ModMetaDialogProps> = ({ paths, open, meta, onChange, onClose }) => {
  const { t } = useTranslation();
  const [tab, setTab] = useState<'note' | 'groups'>('note');
  const [note, setNote] = useState('');
  const [newGroup, setNewGroup] = useState('');
  const [renaming, setRenaming] = useState<{ from: string; to: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** A group change was just saved: said for a moment beside the list. */
  const [saved, setSaved] = useState(false);
  const key = open?.mods.map((m) => m.mod_id).join('|');

  useEffect(() => {
    if (!open) return;
    setTab(open.mods.length > 1 ? 'groups' : open.tab);
    setNote(meta.notes[open.mods[0]?.mod_id ?? ''] ?? '');
    setNewGroup('');
    setRenaming(null);
    setError(null);
    setSaved(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, open?.tab]);
  useEffect(() => {
    if (!saved) return;
    const id = window.setTimeout(() => setSaved(false), 1800);
    return () => window.clearTimeout(id);
  }, [saved]);

  if (!open || open.mods.length === 0) return null;
  const mod = open.mods[0];
  const ids = open.mods.map((m) => m.mod_id);
  const several = ids.length > 1;
  const run = async (f: () => Promise<ModMeta>): Promise<boolean> => {
    setError(null);
    try {
      onChange(await f());
      setSaved(true);
      return true;
    } catch (err) {
      setError(explainError(err));
      return false;
    }
  };
  /** How many of the mods are in the group: all of them shows ticked, some of them half. */
  const inGroupCount = (name: string) => {
    const g = meta.groups.find((x) => x.name === name);
    return g ? ids.filter((id) => g.mods.includes(id)).length : 0;
  };
  const addGroup = async (): Promise<boolean> => {
    if (!newGroup.trim()) return true;
    const ok = await run(() => TauriService.setInModGroup(paths, newGroup.trim(), ids, true));
    if (ok) setNewGroup('');
    return ok;
  };
  const confirmRename = async (): Promise<boolean> => {
    if (!renaming) return true;
    const to = renaming.to.trim();
    if (!to || to === renaming.from) {
      setRenaming(null);
      return true;
    }
    const ok = await run(() => TauriService.renameModGroup(paths, renaming.from, to));
    if (ok) setRenaming(null);
    return ok;
  };
  /** Closing keeps what was typed and not yet added (a new group, a rename): nothing is lost. */
  const close = async () => {
    const renamed = await confirmRename();
    const added = await addGroup();
    if (renamed && added) onClose();
  };

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-6" onClick={() => void close()}>
      <div className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-lg p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold text-slate-100">{several ? t('dialogs.meta.severalTitle', { count: ids.length }) : mod.name}</h3>
            <p className="text-[11px] text-slate-400 mt-0.5">{t('dialogs.meta.blurb')}</p>
          </div>
          <button onClick={() => void close()} aria-label={t('dialogs.meta.close')} className="text-slate-500 hover:text-slate-300 p-1 cursor-pointer">
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>

        <div className="flex gap-1 p-1 rounded-lg bg-slate-950 border border-slate-800 w-fit">
          {(several ? (['groups'] as const) : (['note', 'groups'] as const)).map((k) => (
            <button key={k} onClick={() => setTab(k)} className={`flex items-center gap-1.5 px-3 py-1 rounded-md text-xs cursor-pointer ${tab === k ? 'bg-emerald-700/40 text-emerald-100' : 'text-slate-400 hover:text-slate-200'}`}>
              {k === 'note' ? <StickyNote className="w-3.5 h-3.5" /> : <Tags className="w-3.5 h-3.5" />}
              {t(`dialogs.meta.tab_${k}`)}
            </button>
          ))}
        </div>

        {tab === 'note' ? (
          <div className="space-y-2">
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              rows={4}
              maxLength={500}
              placeholder={t('dialogs.meta.notePlaceholder')}
              className="w-full px-2.5 py-2 rounded-lg bg-slate-950 border border-slate-700 text-xs text-slate-100 resize-none"
            />
            <div className="flex justify-end gap-2">
              {meta.notes[mod.mod_id] && (
                <button onClick={() => void run(() => TauriService.setModNote(paths, mod.mod_id, '')).then(() => setNote(''))} className="flex items-center gap-1 px-3 py-1.5 rounded-md border border-slate-700 text-xs text-slate-300 hover:bg-slate-800 cursor-pointer">
                  <Trash2 className="w-3.5 h-3.5" />{t('dialogs.meta.deleteNote')}
                </button>
              )}
              <button onClick={() => void run(() => TauriService.setModNote(paths, mod.mod_id, note)).then(onClose)} className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-emerald-700 text-xs text-white hover:bg-emerald-600 cursor-pointer">
                <Check className="w-3.5 h-3.5" />{t('dialogs.meta.saveNote')}
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-2">
            {meta.groups.length === 0 && <p className="text-[11px] text-slate-500">{t('dialogs.meta.noGroups')}</p>}
            <div className="max-h-64 overflow-y-auto space-y-1">
              {meta.groups.map((g) => (
                <div key={g.name} className="flex items-center gap-2 rounded-md border border-slate-800 bg-slate-950/60 px-2.5 py-1.5">
                  {(() => {
                    const n = inGroupCount(g.name);
                    const all = n === ids.length;
                    return (
                      <input
                        type="checkbox"
                        checked={all}
                        ref={(el) => {
                          if (el) el.indeterminate = n > 0 && !all;
                        }}
                        // Ticking puts every one of them in; unticking a full group takes them all out.
                        onChange={() => void run(() => TauriService.setInModGroup(paths, g.name, ids, !all))}
                        className="accent-emerald-500 cursor-pointer"
                        aria-label={g.name}
                      />
                    );
                  })()}
                  {renaming?.from === g.name ? (
                    <form className="flex-1 flex gap-1" onSubmit={(e) => { e.preventDefault(); void confirmRename(); }}>
                      <input autoFocus value={renaming.to} onChange={(e) => setRenaming({ from: g.name, to: e.target.value })} className="flex-1 px-2 py-0.5 rounded bg-slate-900 border border-slate-700 text-xs text-slate-100" />
                      <button type="submit" className="p-1 text-emerald-300 hover:bg-slate-800 rounded cursor-pointer" aria-label={t('dialogs.meta.rename')}><Check className="w-3.5 h-3.5" /></button>
                    </form>
                  ) : (
                    <span className="flex-1 text-xs text-slate-200">{g.name} <span className="text-slate-500">({g.mods.length})</span></span>
                  )}
                  <button onClick={() => setRenaming({ from: g.name, to: g.name })} title={t('dialogs.meta.rename')} className="p-1 text-slate-400 hover:text-slate-100 hover:bg-slate-800 rounded cursor-pointer"><Pencil className="w-3.5 h-3.5" /></button>
                  <button onClick={() => { if (confirm(t('dialogs.meta.deleteGroupConfirm', { name: g.name }))) void run(() => TauriService.deleteModGroup(paths, g.name)); }} title={t('dialogs.meta.deleteGroup')} className="p-1 text-slate-400 hover:text-red-300 hover:bg-slate-800 rounded cursor-pointer"><Trash2 className="w-3.5 h-3.5" /></button>
                </div>
              ))}
            </div>
            <form className="flex gap-2" onSubmit={(e) => { e.preventDefault(); void addGroup(); }}>
              <input value={newGroup} onChange={(e) => setNewGroup(e.target.value)} placeholder={t('dialogs.meta.newGroupPlaceholder')} maxLength={40} className="flex-1 px-2.5 py-1.5 rounded-lg bg-slate-950 border border-slate-700 text-xs text-slate-100" />
              <button type="submit" className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-emerald-700 text-xs text-white hover:bg-emerald-600 cursor-pointer"><Plus className="w-3.5 h-3.5" />{t('dialogs.meta.addGroup')}</button>
            </form>
            <p className="text-[10px] text-slate-500">{t('dialogs.meta.groupsBlurb')}</p>
            <div className="flex items-center justify-end gap-3">
              {saved && (
                <span className="flex items-center gap-1 text-[11px] text-emerald-300" role="status">
                  <Check className="w-3.5 h-3.5" aria-hidden="true" />
                  {t('dialogs.meta.saved')}
                </span>
              )}
              <button onClick={() => void close()} className="flex items-center gap-1 px-3 py-1.5 rounded-md bg-emerald-700 text-xs text-white hover:bg-emerald-600 cursor-pointer">
                <Check className="w-3.5 h-3.5" />{t('dialogs.meta.done')}
              </button>
            </div>
          </div>
        )}
        {error && <div className="text-[11px] text-red-300">{error}</div>}
      </div>
    </div>
  );
};
