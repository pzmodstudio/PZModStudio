import React, { useEffect, useMemo, useState } from 'react';
import { Camera, GitCompare, Loader2, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { CatalogModSummary, HealthReport, ModInfo } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { ProfileSnapshot, SnapshotDiff, compareSnapshots, diffIsEmpty, takeSnapshot } from '../../utils/profileSnapshot';

type Entry = { file: string; taken_at: number; label: string; mods_on: number; findings: number; game_version: string | null };

const when = (t: number) => new Date(t * 1000).toLocaleString();

/**
 * Photos of the active profile: take one now (its mods, their versions, what Mod Health finds),
 * and compare any with now, to see what changed since: mods in or out, switched, moved, updated by
 * Steam, the game build, and the findings that appeared or went away.
 */
export const ProfilePhotos: React.FC<{
  paths: StudioPathsUI;
  profile: { id: string; name: string };
  mods: ModInfo[];
  catalog: CatalogModSummary[];
  report: HealthReport | null;
  nameOf: (id: string) => string;
}> = ({ paths, profile, mods, catalog, report, nameOf }) => {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<Entry[]>([]);
  const [picked, setPicked] = useState<string>('');
  const [busy, setBusy] = useState(false);
  const [diff, setDiff] = useState<{ against: Entry; d: SnapshotDiff } | null>(null);
  const [note, setNote] = useState<string | null>(null);

  const load = async () => {
    const list = await TauriService.profileSnapshotList(paths, profile.id);
    setEntries(list);
    setPicked((p) => (list.some((e) => e.file === p) ? p : list[0]?.file ?? ''));
  };
  useEffect(() => {
    setDiff(null);
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [profile.id]);

  const now = useMemo(
    () => (report ? takeSnapshot(profile, '', mods, catalog, report.issues, report.game_version ?? null) : null),
    [profile, mods, catalog, report]
  );

  const take = async () => {
    if (!now) return;
    const label = prompt(t('photos.labelAsk'), t('photos.labelDefault', { when: when(now.taken_at) }));
    if (label === null) return;
    setBusy(true);
    try {
      await TauriService.profileSnapshotSave(paths, profile.id, JSON.stringify({ ...now, label: label.trim() }));
      setNote(t('photos.taken', { mods: now.mods.filter((m) => m.on).length, findings: now.findings.length }));
      await load();
    } finally {
      setBusy(false);
    }
  };

  const compare = async () => {
    const entry = entries.find((e) => e.file === picked);
    if (!entry || !now) return;
    setBusy(true);
    try {
      const before = JSON.parse(await TauriService.profileSnapshotRead(paths, profile.id, entry.file)) as ProfileSnapshot;
      setDiff({ against: entry, d: compareSnapshots(before, now) });
      setNote(null);
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    const entry = entries.find((e) => e.file === picked);
    if (!entry || !confirm(t('photos.deleteConfirm', { label: entry.label || when(entry.taken_at) }))) return;
    await TauriService.profileSnapshotDelete(paths, profile.id, entry.file);
    setDiff(null);
    await load();
  };

  const names = (list: { id: string }[]) => list.slice(0, 15).map((m) => nameOf(m.id)).join(', ') + (list.length > 15 ? t('photos.andMore', { count: list.length - 15 }) : '');
  const section = (key: string, count: number, body: React.ReactNode, tone = 'text-slate-300') =>
    count > 0 && (
      <div className="text-[11px]">
        <span className={`font-bold ${tone}`}>{t(`photos.diff.${key}`, { count })}</span> <span className="text-slate-400">{body}</span>
      </div>
    );

  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/50 px-3 py-2 flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2 text-[11px]">
        <span className="font-bold text-slate-200">{t('photos.title')}</span>
        <button
          type="button"
          onClick={() => void take()}
          disabled={busy || !now}
          title={t('photos.takeTitle')}
          className="flex items-center gap-1 px-2 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-100 border border-slate-700 cursor-pointer disabled:opacity-50"
        >
          <Camera className="w-3 h-3" aria-hidden="true" /> {t('photos.take')}
        </button>
        {entries.length > 0 && (
          <>
            <select
              value={picked}
              onChange={(e) => {
                setPicked(e.target.value);
                setDiff(null);
              }}
              aria-label={t('photos.pick')}
              className="px-2 py-1 rounded bg-slate-950 border border-slate-800 text-slate-200 max-w-[320px]"
            >
              {entries.map((e) => (
                <option key={e.file} value={e.file}>
                  {(e.label || when(e.taken_at)) + ` · ${t('photos.entryCounts', { mods: e.mods_on, findings: e.findings })}`}
                </option>
              ))}
            </select>
            <button
              type="button"
              onClick={() => void compare()}
              disabled={busy || !now}
              className="flex items-center gap-1 px-2 py-1 rounded bg-sky-700 hover:bg-sky-600 text-white font-bold cursor-pointer disabled:opacity-50"
            >
              {busy ? <Loader2 className="w-3 h-3 animate-spin" aria-hidden="true" /> : <GitCompare className="w-3 h-3" aria-hidden="true" />} {t('photos.compare')}
            </button>
            <button type="button" onClick={() => void remove()} title={t('photos.delete')} aria-label={t('photos.delete')} className="p-1 text-slate-500 hover:text-red-300 cursor-pointer">
              <Trash2 className="w-3.5 h-3.5" aria-hidden="true" />
            </button>
          </>
        )}
        {entries.length === 0 && <span className="text-slate-500">{t('photos.none')}</span>}
      </div>
      {note && <div className="text-[11px] text-emerald-300">{note}</div>}
      {diff && (
        <div className="rounded border border-slate-800 bg-slate-950/50 px-3 py-2 flex flex-col gap-1">
          <div className="text-[11px] text-slate-300">{t('photos.since', { label: diff.against.label || when(diff.against.taken_at), when: when(diff.against.taken_at) })}</div>
          {diffIsEmpty(diff.d) ? (
            <div className="text-[11px] text-emerald-300">{t('photos.nothingChanged')}</div>
          ) : (
            <>
              {diff.d.game && section('game', 1, `${diff.d.game.before ?? '?'} → ${diff.d.game.after ?? '?'}`, 'text-amber-300')}
              {section('added', diff.d.added.length, names(diff.d.added))}
              {section('removed', diff.d.removed.length, names(diff.d.removed))}
              {section('switchedOn', diff.d.switchedOn.length, names(diff.d.switchedOn))}
              {section('switchedOff', diff.d.switchedOff.length, names(diff.d.switchedOff))}
              {section('moved', diff.d.moved.length, diff.d.moved.slice(0, 10).map((m) => `${nameOf(m.mod.id)} (${m.from}→${m.to})`).join(', '))}
              {section(
                'updated',
                diff.d.updated.length,
                diff.d.updated.slice(0, 15).map((u) => (u.fromVersion !== u.toVersion && (u.fromVersion || u.toVersion) ? `${nameOf(u.mod.id)} (${u.fromVersion ?? '?'} → ${u.toVersion ?? '?'})` : nameOf(u.mod.id))).join(', '),
                'text-sky-300'
              )}
              {section('newFindings', diff.d.newFindings.length, '', 'text-amber-300')}
              {diff.d.newFindings.slice(0, 12).map((f) => (
                <div key={f.key} className="ml-3 text-[10.5px] text-slate-400 truncate">
                  + <span className="font-mono text-slate-500">{f.code}</span> {f.mod_id ? <b className="text-slate-300">{nameOf(f.mod_id)}</b> : null} {f.text}
                </div>
              ))}
              {section('goneFindings', diff.d.goneFindings.length, '', 'text-emerald-300')}
              {diff.d.goneFindings.slice(0, 12).map((f) => (
                <div key={f.key} className="ml-3 text-[10.5px] text-slate-500 truncate">
                  − <span className="font-mono">{f.code}</span> {f.mod_id ? nameOf(f.mod_id) : null} {f.text}
                </div>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
};
