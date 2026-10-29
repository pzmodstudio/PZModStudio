import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ExternalLink, ChevronDown, ChevronRight } from 'lucide-react';
import { backendText } from '../../i18n/backendText';
import { MpFinding, MpModReport, MpVerdict } from '../../types';

/** The order the verdicts are shown in: the worst first. */
const RANK: Record<MpVerdict, number> = { no: 0, partial: 1, likely_ok: 2, ok: 3 };

const TONE: Record<MpVerdict, { badge: string; border: string }> = {
  no: { badge: 'bg-red-950 text-red-300 border-red-800', border: 'border-red-900/60' },
  partial: { badge: 'bg-amber-950 text-amber-300 border-amber-800', border: 'border-amber-900/60' },
  likely_ok: { badge: 'bg-slate-800 text-slate-300 border-slate-700', border: 'border-slate-800' },
  ok: { badge: 'bg-emerald-950 text-emerald-300 border-emerald-800', border: 'border-emerald-900/60' },
};

/** The rows that make a mod "partial": not handled, in a file that knows nothing of multiplayer, and not the game's own line. */
export const countsAgainst = (f: MpFinding): boolean => !f.guarded && !f.aware && !f.same_as_game;

/**
 * Whether the mods would work in multiplayer, as far as the check can see: one verdict for the
 * list, one per mod, and under each mod the actions that would not reach the other players.
 * In single player none of this applies.
 */
export const MpCheckPanel: React.FC<{ reports: MpModReport[]; onOpen?: (root: string, f: MpFinding) => void; listVerdict?: boolean }> = ({ reports, onOpen, listVerdict = true }) => {
  const { t } = useTranslation();
  const [openNotes, setOpenNotes] = useState<Set<string>>(new Set());
  const [showFine, setShowFine] = useState(false);
  if (reports.length === 0) return null;

  const sorted = [...reports].sort((a, b) => RANK[a.report.verdict] - RANK[b.report.verdict] || a.name.localeCompare(b.name));
  const bad = sorted.filter((r) => r.report.verdict === 'no');
  const partial = sorted.filter((r) => r.report.verdict === 'partial');
  const fine = sorted.filter((r) => r.report.verdict === 'likely_ok' || r.report.verdict === 'ok');

  const toggleNotes = (id: string) => {
    setOpenNotes((cur) => {
      const next = new Set(cur);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const rowText = (f: MpFinding) => backendText('mp', f.method, undefined, f.what);

  const card = ({ mod_id, name, report }: MpModReport) => {
    const against = report.findings.filter(countsAgainst);
    const notes = report.findings.filter((f) => !countsAgainst(f));
    const tone = TONE[report.verdict];
    return (
      <div key={mod_id} className={`rounded-lg border ${tone.border} bg-slate-950/60`}>
        <div className="px-3 py-1.5 text-[11px] border-b border-slate-900 flex items-center justify-between gap-2">
          <span className="text-slate-200 font-bold truncate">{name} <span className="font-mono font-normal text-slate-500">{mod_id}</span></span>
          <span className={`shrink-0 px-2 py-0.5 rounded-md border text-[10px] font-bold ${tone.badge}`} title={t(`mpCheck.verdictWhy.${report.verdict}`)}>
            {t(`mpCheck.verdict.${report.verdict}`)}
          </span>
        </div>
        {report.declared && (
          <div className="px-3 py-1.5 text-[11px] text-slate-300 border-b border-slate-900">
            {t(`mpCheck.declared.${report.declared}`)}
            {report.declared_quote && <span className="text-slate-500"> — “{report.declared_quote}”</span>}
          </div>
        )}
        {report.verdict === 'partial' && report.has_server_lua && (
          <div className="px-3 py-1 text-[10px] text-slate-500 border-b border-slate-900">{t('mpCheck.serverLuaNote')}</div>
        )}
        {against.map((f, i) => (
          <button
            key={i}
            type="button"
            onClick={() => onOpen?.(report.root, f)}
            title={`${t('mpCheck.rowTitle', { what: rowText(f), handler: f.handler, evidence: f.evidence })}${onOpen ? t('mpCheck.rowClick') : ''}`}
            className={`w-full text-left px-3 py-1.5 text-[11px] flex items-start gap-2 ${onOpen ? 'hover:bg-slate-900 cursor-pointer' : 'cursor-default'}`}
          >
            <span className="font-mono shrink-0 w-6 text-center text-amber-300">!</span>
            <span className="flex-1 min-w-0">
              <span className="text-slate-200">{t('mpCheck.rowMeans', { what: rowText(f) })}</span>
              <div className="font-mono text-[10px] text-slate-500 truncate">{f.path}:{f.line}  {f.snippet}</div>
            </span>
            {onOpen && <ExternalLink className="w-3 h-3 text-slate-600 shrink-0 mt-0.5" aria-hidden="true" />}
          </button>
        ))}
        {notes.length > 0 && (
          <>
            <button
              type="button"
              onClick={() => toggleNotes(mod_id)}
              className="w-full text-left px-3 py-1 text-[10px] text-slate-500 hover:text-slate-300 flex items-center gap-1 cursor-pointer"
            >
              {openNotes.has(mod_id) ? <ChevronDown className="w-3 h-3" aria-hidden="true" /> : <ChevronRight className="w-3 h-3" aria-hidden="true" />}
              {t('mpCheck.notes', { count: notes.length })}
            </button>
            {openNotes.has(mod_id) && notes.map((f, i) => (
              <button
                key={`n${i}`}
                type="button"
                onClick={() => onOpen?.(report.root, f)}
                title={`${t('mpCheck.rowTitle', { what: rowText(f), handler: f.handler, evidence: f.evidence })}${onOpen ? t('mpCheck.rowClick') : ''}`}
                className={`w-full text-left px-3 py-1 text-[11px] flex items-start gap-2 opacity-80 ${onOpen ? 'hover:bg-slate-900 cursor-pointer' : 'cursor-default'}`}
              >
                <span className="font-mono shrink-0 w-6 text-center text-slate-500">i</span>
                <span className="flex-1 min-w-0">
                  <span className="text-slate-300">{rowText(f)}</span>
                  <span className="text-slate-500"> — {f.same_as_game ? t('mpCheck.noteSameAsGame') : f.guarded ? t('mpCheck.noteGuarded') : t('mpCheck.noteAware')}</span>
                  <div className="font-mono text-[10px] text-slate-500 truncate">{f.path}:{f.line}  {f.snippet}</div>
                </span>
              </button>
            ))}
          </>
        )}
      </div>
    );
  };

  return (
    <div className="flex flex-col gap-2">
      {listVerdict && (
        <div className={`rounded-lg border px-3 py-2 text-[11px] ${bad.length > 0 ? 'border-red-800 bg-red-950/40 text-red-200' : partial.length > 0 ? 'border-amber-800 bg-amber-950/40 text-amber-200' : 'border-emerald-800 bg-emerald-950/40 text-emerald-200'}`}>
          <div className="font-bold">
            {bad.length > 0
              ? t('mpCheck.list.no', { count: bad.length })
              : partial.length > 0
              ? t('mpCheck.list.partial', { count: partial.length })
              : t('mpCheck.list.ok', { count: reports.length })}
          </div>
          <div className="text-[10px] opacity-80 mt-0.5">{t('mpCheck.list.scope')}</div>
        </div>
      )}
      {bad.map(card)}
      {partial.map(card)}
      {fine.length > 0 && (
        <>
          <button type="button" onClick={() => setShowFine((v) => !v)} className="text-left text-[11px] text-slate-400 hover:text-slate-200 flex items-center gap-1 cursor-pointer">
            {showFine ? <ChevronDown className="w-3 h-3" aria-hidden="true" /> : <ChevronRight className="w-3 h-3" aria-hidden="true" />}
            {t('mpCheck.fine', { count: fine.length })}
          </button>
          {showFine && fine.map(card)}
        </>
      )}
    </div>
  );
};
