import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import { Bot, Check, ExternalLink, Loader2, ShieldAlert } from 'lucide-react';
import { GameBot, StudioPathsUI } from '../../types';
import { TauriService } from '../../services/tauri';

/**
 * The game's own Discord bot, set in the server's .ini: it lives inside the server process, so
 * the chat goes both ways whenever the server runs, with or without the Studio.
 */
export const GameBotPanel: React.FC<{ paths: StudioPathsUI; filePath: string }> = ({ paths, filePath }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [bot, setBot] = useState<GameBot | null>(null);
  const [token, setToken] = useState('');
  const [chat, setChat] = useState('');
  const [log, setLog] = useState('');
  const [enabled, setEnabled] = useState(false);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);

  const take = (b: GameBot) => {
    setBot(b);
    setChat(b.chat_channel);
    setLog(b.log_channel);
    setEnabled(b.enabled);
  };

  useEffect(() => {
    setNote(null);
    setToken('');
    void TauriService.gameBot(paths, filePath).then((b) => b && take(b));
  }, [paths, filePath, changeRev]);

  if (!bot) return null;

  // These fields are written to the .ini only with the button, unlike the switches below.
  const dirty = token.trim() !== '' || chat.trim() !== bot.chat_channel || log.trim() !== bot.log_channel || enabled !== bot.enabled;

  const save = async (clearCommand = false) => {
    setBusy(true);
    try {
      const b = await TauriService.saveGameBot(paths, filePath, {
        enabled,
        token: token.trim() ? token.trim() : null,
        chatChannel: chat,
        logChannel: log,
        clearCommandChannel: clearCommand,
      });
      take(b);
      setToken('');
      setNote({ ok: true, text: t('servers.gameBot.saved') });
    } catch (err) {
      const text = String(err);
      setNote({ ok: false, text: text.startsWith('not_token:') ? t('servers.gameBot.notToken') : text });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="bg-slate-950/60 border border-slate-800 rounded-xl p-4 space-y-3">
      <div className="flex items-center gap-2 text-xs font-bold text-slate-200">
        <Bot className="w-4 h-4 text-purple-300" aria-hidden="true" /> {t('servers.gameBot.title')}
      </div>
      <p className="text-[11px] text-slate-400">{t('servers.gameBot.intro')}</p>
      <ol className="list-decimal pl-5 text-[11px] text-slate-400 space-y-0.5">
        <li>{t('servers.gameBot.step1')}</li>
        <li>{t('servers.gameBot.step2')}</li>
        <li>{t('servers.gameBot.step3')}</li>
      </ol>

      {bot.command_channel && (
        <div className="flex items-start gap-2 p-2 rounded-lg border border-red-800/70 bg-red-950/30 text-[11px] text-red-200">
          <ShieldAlert className="w-4 h-4 shrink-0 mt-0.5" aria-hidden="true" />
          <span className="flex-1">{t('servers.gameBot.commandChannel', { channel: bot.command_channel })}</span>
          <button type="button" disabled={busy} onClick={() => void save(true)} className="px-2 py-0.5 rounded border border-red-700 hover:bg-red-900/50 cursor-pointer">
            {t('servers.gameBot.clearCommand')}
          </button>
        </div>
      )}

      <div className="grid sm:grid-cols-2 gap-2">
        <label className="space-y-1">
          <span className="block text-[11px] text-slate-300">{t('servers.gameBot.token')}</span>
          <input
            type="password"
            autoComplete="off"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder={bot.has_token ? t('servers.gameBot.tokenSaved') : t('servers.gameBot.tokenPlaceholder')}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-purple-500"
          />
        </label>
        <label className="space-y-1">
          <span className="block text-[11px] text-slate-300">{t('servers.gameBot.chatChannel')}</span>
          <input value={chat} onChange={(e) => setChat(e.target.value)} placeholder="general" spellCheck={false}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-purple-500" />
        </label>
        <label className="space-y-1">
          <span className="block text-[11px] text-slate-300">{t('servers.gameBot.logChannel')}</span>
          <input value={log} onChange={(e) => setLog(e.target.value)} placeholder={t('servers.gameBot.optional')} spellCheck={false}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-purple-500" />
          <span className="block text-[10.5px] text-slate-500">{t('servers.gameBot.logHint')}</span>
        </label>
        <label className="flex items-center gap-2 text-xs text-slate-200 self-end pb-1.5 cursor-pointer">
          <input type="checkbox" checked={enabled} onChange={() => setEnabled(!enabled)} />
          {t('servers.gameBot.enabled')}
        </label>
      </div>
      <p className="text-[10.5px] text-amber-300/80">{t('servers.gameBot.tokenInIni')}</p>
      <div className="flex flex-wrap gap-2 items-center">
        <button type="button" disabled={busy} onClick={() => void save()}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-purple-600 hover:bg-purple-500 text-white cursor-pointer disabled:opacity-50 ${dirty ? 'ring-2 ring-amber-400' : ''}`}>
          {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Check className="w-3.5 h-3.5" aria-hidden="true" />}
          {t('servers.gameBot.save')}
        </button>
        {bot.invite_url && (
          <button type="button" onClick={() => void TauriService.openExternalUrl(bot.invite_url!)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
            <ExternalLink className="w-3.5 h-3.5" aria-hidden="true" /> {t('servers.gameBot.invite')}
          </button>
        )}
        {dirty ? (
          <span className="text-[11px] font-bold text-amber-300">{t('servers.gameBot.unsaved')}</span>
        ) : (
          <span className="text-[11px] text-slate-500">{t('servers.gameBot.restartNote')}</span>
        )}
      </div>
      {note && <div className={`text-[11px] ${note.ok ? 'text-emerald-300' : 'text-red-300'}`}>{note.text}</div>}
    </div>
  );
};
