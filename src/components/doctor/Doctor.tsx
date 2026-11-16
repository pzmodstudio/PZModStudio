import React, { useEffect, useMemo, useState } from 'react';
import { Stethoscope, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { GameMemory, HealthIssue, TranslatedErrorCard } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService } from '../../services/tauri';
import { DoctorStep, doctorStep } from '../../utils/doctor';
import { issueText } from '../../i18n/backendText';

type Symptom = 'crash' | 'mod' | 'slow' | 'other';

/**
 * A guided way through a problem, for a player who does not know where to look: what is wrong
 * (buttons, not free text), then one step at a time from what the Studio knows (utils/doctor),
 * each with what to do and a way to do it; after a step, play and come back, and it looks again.
 * When nothing explains it, it says so and hands over what it knows.
 */
export const Doctor: React.FC<{
  paths: StudioPathsUI;
  issues: HealthIssue[];
  cards: TranslatedErrorCard[];
  nameOf: (id: string) => string;
  onClose: () => void;
  onGoIssue: (issue: HealthIssue) => void;
  onSwitchOffMod: (modName: string) => void;
  onGoMonitor: () => void;
  onGoPerformance: () => void;
  onGoAudit: () => void;
  onRecheck: () => void;
}> = ({ paths, issues, cards, nameOf, onClose, onGoIssue, onSwitchOffMod, onGoMonitor, onGoPerformance, onGoAudit, onRecheck }) => {
  const { t } = useTranslation();
  const [symptom, setSymptom] = useState<Symptom | null>(null);
  const [tried, setTried] = useState<string[]>([]);
  const [waiting, setWaiting] = useState(false);
  const [memory, setMemory] = useState<GameMemory | null>(null);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    void TauriService.gameMemory(paths).then(setMemory);
  }, [paths]);

  const step: DoctorStep = useMemo(
    () => doctorStep({ issues, cards, outOfMemory: memory?.out_of_memory.length ?? 0, windowsLow: memory?.windows_low_memory.length ?? 0, tried }),
    [issues, cards, memory, tried]
  );

  /** Did what this step says, now play and come back. */
  const done = (s: DoctorStep) => {
    setTried((prev) => [...prev, s.id]);
    setWaiting(true);
    setNote(null);
  };

  const copyWhatIKnow = async () => {
    const lines = [
      t('doctor.report.head'),
      ...issues.filter((i) => i.state === 'act').slice(0, 10).map((i) => `- ${issueText(i)}`),
      ...cards.filter((c) => c.severity === 'EXCEPTION').slice(0, 10).map((c) => `- ${c.mod_name ?? '?'}: ${c.raw_error}`),
      memory ? t('doctor.report.memory', { oom: memory.out_of_memory.length, low: memory.windows_low_memory.length }) : '',
    ].filter(Boolean);
    await navigator.clipboard.writeText(lines.join('\n'));
    setNote(t('doctor.copied'));
  };

  const button = (label: string, onClick: () => void, primary = false) => (
    <button
      type="button"
      onClick={onClick}
      className={`px-3 py-1.5 rounded-lg text-xs font-bold cursor-pointer ${primary ? 'bg-sky-700 hover:bg-sky-600 text-white' : 'bg-slate-800 hover:bg-slate-700 text-slate-100 border border-slate-700'}`}
    >
      {label}
    </button>
  );

  let body: React.ReactNode;
  if (!symptom) {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.ask')}</p>
        <div className="grid sm:grid-cols-2 gap-2">
          {(['crash', 'mod', 'slow', 'other'] as Symptom[]).map((s) => (
            <button key={s} type="button" onClick={() => setSymptom(s)} className="text-left rounded-lg border border-slate-700 bg-slate-900 hover:bg-slate-800 px-3 py-2 cursor-pointer">
              <div className="text-xs font-bold text-slate-100">{t(`doctor.symptom.${s}.title`)}</div>
              <div className="text-[11px] text-slate-400">{t(`doctor.symptom.${s}.text`)}</div>
            </button>
          ))}
        </div>
      </>
    );
  } else if (symptom === 'slow') {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.slow')}</p>
        <div className="flex flex-wrap gap-2">{button(t('doctor.goPerformance'), () => { onGoPerformance(); onClose(); }, true)}</div>
      </>
    );
  } else if (symptom === 'mod') {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.mod')}</p>
        <div className="flex flex-wrap gap-2">
          {button(t('doctor.goAudit'), () => { onGoAudit(); onClose(); }, true)}
          {button(t('doctor.goMonitor'), () => { onGoMonitor(); onClose(); })}
        </div>
      </>
    );
  } else if (symptom === 'other') {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.other')}</p>
        <div className="flex flex-wrap gap-2">{button(t('doctor.copy'), () => void copyWhatIKnow(), true)}</div>
      </>
    );
  } else if (waiting) {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.playAndComeBack')}</p>
        <div className="flex flex-wrap gap-2">
          {button(t('doctor.itWorks'), onClose, true)}
          {button(t('doctor.stillBroken'), () => { onRecheck(); setWaiting(false); })}
        </div>
      </>
    );
  } else if (step.kind === 'health') {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.health', { count: step.count })}</p>
        <div className="rounded border border-red-800/60 bg-red-950/20 px-3 py-2 text-[11.5px] text-slate-200">
          {step.issue.mod_id && <b>{nameOf(step.issue.mod_id)}: </b>}
          {issueText(step.issue)}
        </div>
        <div className="flex flex-wrap gap-2">
          {button(t('doctor.goFix'), () => { onGoIssue(step.issue); done(step); onClose(); }, true)}
          {button(t('doctor.alreadyDone'), () => done(step))}
        </div>
      </>
    );
  } else if (step.kind === 'blamed') {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.blamed', { mod: step.mod })}</p>
        <div className="rounded border border-amber-800/60 bg-amber-950/20 px-3 py-2 text-[11px] font-mono text-slate-300 break-all line-clamp-3">{step.card.raw_error}</div>
        <div className="flex flex-wrap gap-2">
          {button(t('doctor.switchOffTry', { mod: step.mod }), () => { onSwitchOffMod(step.mod); done(step); }, true)}
          {button(t('doctor.goMonitor'), () => { onGoMonitor(); onClose(); })}
          {button(t('doctor.skip'), () => setTried((p) => [...p, step.id]))}
        </div>
      </>
    );
  } else if (step.kind === 'memory') {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.memory', { oom: step.outOfMemory, low: step.windowsLow })}</p>
        <div className="flex flex-wrap gap-2">
          {button(t('doctor.goPerformance'), () => { onGoPerformance(); done(step); onClose(); }, true)}
          {button(t('doctor.skip'), () => setTried((p) => [...p, step.id]))}
        </div>
      </>
    );
  } else {
    body = (
      <>
        <p className="text-sm text-slate-200">{t('doctor.unknown')}</p>
        <div className="flex flex-wrap gap-2">
          {button(t('doctor.hunt'), () => { onGoMonitor(); onClose(); }, true)}
          {button(t('doctor.copy'), () => void copyWhatIKnow())}
        </div>
      </>
    );
  }

  return (
    <div className="fixed inset-0 z-[70] bg-black/60 backdrop-blur-sm flex items-center justify-center p-6" onClick={onClose}>
      <div role="dialog" aria-modal="true" className="w-full max-w-xl rounded-2xl border border-slate-700 bg-slate-950 p-5 flex flex-col gap-3" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm font-bold text-slate-100">
            <Stethoscope className="w-4 h-4 text-sky-300" aria-hidden="true" /> {t('doctor.title')}
          </div>
          <button type="button" onClick={onClose} aria-label={t('doctor.close')} className="text-slate-500 hover:text-slate-200 cursor-pointer">
            <X className="w-4 h-4" />
          </button>
        </div>
        {body}
        {note && <div className="text-[11px] text-emerald-300">{note}</div>}
        {symptom && (
          <button type="button" onClick={() => { setSymptom(null); setTried([]); setWaiting(false); }} className="self-start text-[11px] text-slate-500 hover:text-slate-300 cursor-pointer">
            {t('doctor.startOver')}
          </button>
        )}
      </div>
    </div>
  );
};
