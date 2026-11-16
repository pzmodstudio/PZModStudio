import React, { useMemo, useState } from 'react';
import { ArrowRight, ChevronDown, ChevronRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { HealthIssue, ModInfo } from '../../types';
import { AuditVerdict, auditList } from '../../utils/audit';
import { issueText } from '../../i18n/backendText';

const ORDER: AuditVerdict[] = ['broken', 'choose', 'unneeded', 'check', 'keep'];
const TONE: Record<AuditVerdict, string> = {
  broken: 'border-red-700/60 bg-red-950/20 text-red-200',
  choose: 'border-amber-700/60 bg-amber-950/20 text-amber-200',
  unneeded: 'border-sky-700/60 bg-sky-950/20 text-sky-200',
  check: 'border-violet-700/60 bg-violet-950/20 text-violet-200',
  keep: 'border-slate-800 bg-slate-900/40 text-slate-300',
};

/**
 * The list mod by mod (utils/audit): each switched-on mod under one verdict — broken, choose,
 * unneeded, check, keep — with the findings behind it and Go to each. "Keep" starts folded.
 */
export const AuditPanel: React.FC<{
  mods: ModInfo[];
  issues: HealthIssue[];
  nameOf: (id: string) => string;
  onGo: (issue: HealthIssue) => void;
}> = ({ mods, issues, nameOf, onGo }) => {
  const { t } = useTranslation();
  const rows = useMemo(() => auditList(mods.filter((m) => m.enabled).map((m) => m.mod_id), issues), [mods, issues]);
  const [open, setOpen] = useState<Record<AuditVerdict, boolean>>({ broken: true, choose: true, unneeded: true, check: false, keep: false });
  const [expanded, setExpanded] = useState<string | null>(null);

  return (
    <div className="flex flex-col gap-2">
      <p className="text-xs text-slate-400 max-w-3xl">{t('health.audit.intro')}</p>
      {ORDER.map((verdict) => {
        const list = rows.filter((r) => r.verdict === verdict);
        if (list.length === 0) return null;
        return (
          <div key={verdict} className={`rounded-lg border ${TONE[verdict]}`}>
            <button
              type="button"
              onClick={() => setOpen((o) => ({ ...o, [verdict]: !o[verdict] }))}
              className="w-full flex items-center gap-2 px-3 py-2 text-left cursor-pointer"
            >
              {open[verdict] ? <ChevronDown className="w-3.5 h-3.5" aria-hidden="true" /> : <ChevronRight className="w-3.5 h-3.5" aria-hidden="true" />}
              <span className="text-xs font-bold">{t(`health.audit.${verdict}.title`, { count: list.length })}</span>
              <span className="text-[11px] opacity-75">{t(`health.audit.${verdict}.what`)}</span>
            </button>
            {open[verdict] && (
              <ul className="px-3 pb-2 flex flex-col gap-1">
                {list.map((row) => (
                  <li key={row.mod_id} className="rounded border border-slate-800/80 bg-slate-950/40 px-2 py-1">
                    <button
                      type="button"
                      disabled={row.reasons.length === 0}
                      onClick={() => setExpanded((e) => (e === row.mod_id ? null : row.mod_id))}
                      className="w-full flex items-center gap-2 text-left text-[11.5px] text-slate-100 cursor-pointer disabled:cursor-default"
                    >
                      <span className="font-bold truncate">{nameOf(row.mod_id)}</span>
                      {row.reasons.length > 0 && <span className="text-[10px] text-slate-400 shrink-0">{t('health.audit.reasons', { count: row.reasons.length })}</span>}
                    </button>
                    {expanded === row.mod_id && (
                      <ul className="mt-1 flex flex-col gap-1">
                        {row.reasons.map((issue, i) => (
                          <li key={i} className="flex items-start gap-2 text-[11px] text-slate-300">
                            <ArrowRight className="w-3 h-3 mt-0.5 shrink-0 text-slate-500" aria-hidden="true" />
                            <span className="flex-1 min-w-0 line-clamp-2">{issueText(issue)}</span>
                            <button type="button" onClick={() => onGo(issue)} className="shrink-0 px-2 py-0.5 rounded text-[10px] font-bold bg-sky-800 hover:bg-sky-700 text-white cursor-pointer">
                              {t('health.verdict.go')}
                            </button>
                          </li>
                        ))}
                      </ul>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </div>
        );
      })}
    </div>
  );
};
