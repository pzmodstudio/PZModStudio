import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RotateCcw, Save, Search } from 'lucide-react';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError } from '../../services/tauri';
import { ServerOptionDef, ServerSettingsSchema, ServerSettingsValues } from '../../types';

interface Props {
  paths: StudioPathsUI;
  /** The server settings name (the .ini without its extension). */
  server: string;
}

type Kind = 'server' | 'sandbox';

/**
 * Every option of a server's settings, as the game lists them: the server's .ini and the sandbox,
 * the mods' sandbox options included, page by page, with each one's help and default. Only what is
 * changed here is written, both files backed up first.
 */
export const ServerOptionsEditor: React.FC<Props> = ({ paths, server }) => {
  const { t } = useTranslation();
  const [schema, setSchema] = useState<ServerSettingsSchema | null | undefined>(undefined);
  const [values, setValues] = useState<ServerSettingsValues | null>(null);
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [kind, setKind] = useState<Kind>('sandbox');
  const [page, setPage] = useState<string>('');
  const [query, setQuery] = useState('');
  const [onlyChanged, setOnlyChanged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [said, setSaid] = useState<{ ok: boolean; text: string } | null>(null);

  const load = useCallback(async () => {
    const [s, v] = await Promise.all([
      TauriService.serverSettingsSchema(paths).catch(() => null),
      TauriService.serverSettingsValues(paths, server).catch(() => null),
    ]);
    setSchema(s);
    setValues(v);
    setEdits({});
  }, [paths, server]);

  useEffect(() => {
    void load();
  }, [load]);

  const pages = useMemo(() => (schema?.pages ?? []).filter(([k]) => k === kind).map(([, name]) => name), [schema, kind]);
  useEffect(() => {
    if (pages.length && !pages.includes(page)) setPage(pages[0]);
  }, [pages, page]);

  const key = (o: ServerOptionDef) => `${o.kind}|${o.name}`;
  const current = (o: ServerOptionDef): string => {
    const k = key(o);
    if (k in edits) return edits[k];
    const v = o.kind === 'server' ? values?.server[o.name] : values?.sandbox[o.name];
    return v ?? o.default;
  };
  const differsFromDefault = (o: ServerOptionDef) => current(o).trim().toLowerCase() !== o.default.trim().toLowerCase();

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (schema?.options ?? []).filter((o) => {
      if (o.kind !== kind) return false;
      if (q) return o.name.toLowerCase().includes(q) || o.label.toLowerCase().includes(q) || o.help.toLowerCase().includes(q);
      if (onlyChanged) return differsFromDefault(o) || key(o) in edits;
      return o.page === page;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [schema, kind, page, query, onlyChanged, edits, values]);

  const set = (o: ServerOptionDef, v: string) => setEdits((e) => ({ ...e, [key(o)]: v }));
  const pending = Object.keys(edits).length;

  const save = async () => {
    setBusy(true);
    setSaid(null);
    const serverEdits: Record<string, string> = {};
    const sandboxEdits: Record<string, string> = {};
    for (const [k, v] of Object.entries(edits)) {
      const [kd, name] = k.split('|');
      (kd === 'server' ? serverEdits : sandboxEdits)[name] = v;
    }
    try {
      const written = await TauriService.serverSettingsWrite(paths, server, serverEdits, sandboxEdits);
      setSaid({ ok: true, text: t('servers.options.saved', { count: written.length }) });
      await load();
    } catch (err) {
      setSaid({ ok: false, text: explainError(err) });
    } finally {
      setBusy(false);
    }
  };

  if (schema === undefined) return <div className="text-xs text-slate-400">{t('servers.options.loading')}</div>;
  if (schema === null)
    return <div className="text-xs text-slate-300 leading-relaxed bg-slate-950 border border-slate-800 rounded-xl p-4">{t('servers.options.noSchema')}</div>;

  const control = (o: ServerOptionDef) => {
    const v = current(o);
    const base = 'bg-slate-900 border border-slate-700 rounded px-2 py-1 text-xs text-slate-100';
    if (o.type === 'boolean') {
      return <input type="checkbox" checked={v === 'true'} onChange={(e) => set(o, e.target.checked ? 'true' : 'false')} className="w-4 h-4 cursor-pointer" />;
    }
    if (o.type === 'enum' && o.values.length > 0) {
      return (
        <select value={v} onChange={(e) => set(o, e.target.value)} className={`${base} max-w-[16rem] cursor-pointer`}>
          {o.values.map((label, i) => (
            <option key={i} value={String(i + 1)}>
              {label}
            </option>
          ))}
        </select>
      );
    }
    if (o.type === 'integer' || o.type === 'double') {
      return (
        <input
          type="number"
          value={v}
          step={o.type === 'integer' ? 1 : 'any'}
          min={o.min ?? undefined}
          max={o.max ?? undefined}
          onChange={(e) => set(o, e.target.value)}
          className={`${base} w-28`}
        />
      );
    }
    if (o.type === 'text') {
      return <textarea value={v} onChange={(e) => set(o, e.target.value)} rows={2} className={`${base} w-72`} />;
    }
    return <input type="text" value={v} onChange={(e) => set(o, e.target.value)} className={`${base} w-72`} />;
  };

  return (
    <div className="flex flex-col gap-3 flex-1 min-h-0">
      <div className="flex flex-wrap items-center gap-2">
        {(['sandbox', 'server'] as Kind[]).map((k) => (
          <button
            key={k}
            type="button"
            onClick={() => setKind(k)}
            className={`px-3 py-1.5 rounded-lg text-xs font-bold cursor-pointer ${kind === k ? 'bg-purple-600 text-white' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'}`}
          >
            {t(`servers.options.kind.${k}`)}
          </button>
        ))}
        <div className="relative ml-auto">
          <Search className="w-3.5 h-3.5 absolute left-2 top-1/2 -translate-y-1/2 text-slate-500" aria-hidden="true" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t('servers.options.search')}
            className="pl-7 pr-2 py-1 rounded bg-slate-950 border border-slate-700 text-xs text-slate-200 w-56"
          />
        </div>
        <label className="flex items-center gap-1.5 text-[11px] text-slate-300 cursor-pointer select-none">
          <input type="checkbox" checked={onlyChanged} onChange={(e) => setOnlyChanged(e.target.checked)} className="cursor-pointer" />
          {t('servers.options.onlyChanged')}
        </label>
      </div>

      <div className="flex gap-3 flex-1 min-h-0">
        {!query && !onlyChanged && (
          <div className="w-44 shrink-0 overflow-y-auto space-y-0.5 pr-1">
            {pages.map((p) => (
              <button
                key={p}
                type="button"
                onClick={() => setPage(p)}
                className={`w-full text-left px-2 py-1 rounded text-[11px] cursor-pointer truncate ${p === page ? 'bg-slate-800 text-slate-100 font-bold' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'}`}
                title={p}
              >
                {p}
              </button>
            ))}
          </div>
        )}
        <div className="flex-1 overflow-y-auto space-y-1.5 pr-1">
          {shown.length === 0 && <div className="text-xs text-slate-500">{t('servers.options.none')}</div>}
          {shown.map((o) => (
            <div key={key(o)} className={`flex items-start gap-3 p-2 rounded-lg border ${key(o) in edits ? 'border-purple-700/70 bg-purple-950/20' : 'border-slate-800 bg-slate-950/60'}`}>
              <div className="flex-1 min-w-0">
                <div className="text-xs font-bold text-slate-200 flex items-center gap-2">
                  <span className="truncate">{o.label}</span>
                  {differsFromDefault(o) && (
                    <span className="text-[9px] px-1 py-0.5 rounded border border-amber-700/60 text-amber-300 font-normal shrink-0">{t('servers.options.notDefault')}</span>
                  )}
                </div>
                <div className="text-[10px] text-slate-500 font-mono">{o.name}</div>
                {o.help && <div className="text-[11px] text-slate-400 leading-snug whitespace-pre-line mt-0.5">{o.help}</div>}
              </div>
              <div className="flex items-center gap-1.5 shrink-0">
                {control(o)}
                {differsFromDefault(o) && (
                  <button
                    type="button"
                    onClick={() => set(o, o.default)}
                    title={t('servers.options.resetTitle', { value: o.type === 'enum' && o.values.length ? o.values[Number(o.default) - 1] ?? o.default : o.default })}
                    className="p-1 rounded text-slate-500 hover:text-slate-200 hover:bg-slate-800 cursor-pointer"
                  >
                    <RotateCcw className="w-3.5 h-3.5" aria-hidden="true" />
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="flex items-center gap-3">
        {said && <div className={`text-[11px] flex-1 ${said.ok ? 'text-emerald-300' : 'text-red-300'}`}>{said.text}</div>}
        {!said && <div className="text-[11px] text-slate-500 flex-1">{t('servers.options.how')}</div>}
        {pending > 0 && (
          <button type="button" onClick={() => setEdits({})} className="px-3 py-2 rounded-lg text-xs text-slate-300 hover:bg-slate-800 cursor-pointer">
            {t('servers.options.discard')}
          </button>
        )}
        <button
          type="button"
          onClick={() => void save()}
          disabled={busy || pending === 0}
          className="px-4 py-2 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white rounded-lg text-xs font-bold cursor-pointer flex items-center gap-2"
        >
          <Save className="w-4 h-4" aria-hidden="true" />
          {t('servers.options.save', { count: pending })}
        </button>
      </div>
    </div>
  );
};
