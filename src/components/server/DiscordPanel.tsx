import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import i18next from 'i18next';
import { Check, Loader2, Send, Terminal, Trash2 } from 'lucide-react';
import { DiscordCommandsStatus, DiscordSettings, StudioPathsUI } from '../../types';
import { TauriService } from '../../services/tauri';
import { GameBotPanel } from './GameBotPanel';

/**
 * A server's Discord channel, one way: the webhook the player made in Discord (kept in the
 * Windows Credential Manager, never shown again), and which news are posted to it.
 */
export const DiscordPanel: React.FC<{ paths: StudioPathsUI; server: string; filePath: string }> = ({ paths, server, filePath }) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [settings, setSettings] = useState<DiscordSettings | null>(null);
  const [hasWebhook, setHasWebhook] = useState(false);
  const [webhook, setWebhook] = useState('');
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null);
  const [cmdChannel, setCmdChannel] = useState('');
  const [cmdRole, setCmdRole] = useState('');
  const [cmdStatus, setCmdStatus] = useState<DiscordCommandsStatus | null>(null);

  useEffect(() => {
    setNote(null);
    setWebhook('');
    void TauriService.discordView(paths, server).then((v) => {
      if (!v) return;
      setSettings(v.settings);
      setHasWebhook(v.has_webhook);
      setCmdChannel(v.settings.command_channel ?? '');
      setCmdRole(v.settings.command_role ?? '');
    });
  }, [paths, server, changeRev]);

  const commandsOn = settings?.commands ?? false;
  useEffect(() => {
    if (!commandsOn) {
      setCmdStatus(null);
      return;
    }
    const look = () => void TauriService.discordCommandsStatus().then(setCmdStatus);
    look();
    const id = window.setInterval(look, 3000);
    return () => window.clearInterval(id);
  }, [commandsOn]);

  if (!settings) return null;

  /** A backend error ("code: English text") in the player's language. */
  const explain = (err: unknown) => {
    const text = String(err);
    const m = text.match(/^(not_webhook|unreachable|unknown_webhook|status|rate_limited|no_webhook): (.*)$/s);
    return m ? t(`servers.discord.errors.${m[1]}`, { detail: m[2] }) : text;
  };

  const save = async (next: DiscordSettings, newWebhook?: string) => {
    setBusy(true);
    try {
      const v = await TauriService.discordSave(paths, server, { ...next, language: i18next.language.startsWith('es') ? 'es' : 'en' }, newWebhook);
      setSettings(v.settings);
      setHasWebhook(v.has_webhook);
      if (newWebhook !== undefined) setWebhook('');
      setNote({ ok: true, text: newWebhook === '' ? t('servers.discord.forgotten') : t('servers.discord.saved') });
    } catch (err) {
      setNote({ ok: false, text: explain(err) });
    } finally {
      setBusy(false);
    }
  };

  const test = async () => {
    setBusy(true);
    try {
      await TauriService.discordTest(paths, server);
      setNote({ ok: true, text: t('servers.discord.testSent') });
    } catch (err) {
      setNote({ ok: false, text: explain(err) });
    } finally {
      setBusy(false);
    }
  };

  type EventKey = 'status_message' | 'status' | 'players' | 'restarts' | 'updates';
  const toggle = (key: EventKey) => void save({ ...settings, [key]: !settings[key] });
  const events: EventKey[] = ['status_message', 'status', 'players', 'restarts', 'updates'];

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <div className="text-sm font-bold text-slate-100">{t('servers.discord.title')}</div>
        <p className="text-[11px] text-slate-400">{t('servers.discord.intro')}</p>
        <p className="text-[11px] text-amber-300/90">{t('servers.discord.whileOpen')}</p>
      </div>

      <div className="bg-slate-950/60 border border-slate-800 rounded-xl p-4 space-y-3">
        <div className="text-xs font-bold text-slate-200">{t('servers.discord.webhook')}</div>
        <p className="text-[11px] text-slate-400">{t('servers.discord.webhookHow')}</p>
        <div className="flex flex-wrap gap-2 items-center">
          <input
            type="password"
            autoComplete="off"
            value={webhook}
            onChange={(e) => setWebhook(e.target.value)}
            placeholder={hasWebhook ? t('servers.discord.webhookSaved') : 'https://discord.com/api/webhooks/...'}
            aria-label={t('servers.discord.webhook')}
            className="flex-1 min-w-[260px] bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-purple-500"
          />
          <button
            type="button"
            disabled={busy || webhook.trim() === ''}
            onClick={() => void save({ ...settings, enabled: true }, webhook)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-purple-600 hover:bg-purple-500 text-white cursor-pointer disabled:opacity-50"
          >
            {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Check className="w-3.5 h-3.5" aria-hidden="true" />}
            {t('servers.discord.saveWebhook')}
          </button>
          {hasWebhook && (
            <>
              <button
                type="button"
                disabled={busy}
                onClick={() => void test()}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50"
              >
                <Send className="w-3.5 h-3.5" aria-hidden="true" /> {t('servers.discord.test')}
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => void save({ ...settings, enabled: false }, '')}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-red-900/60 text-slate-300 cursor-pointer disabled:opacity-50"
                title={t('servers.discord.forgetTitle')}
              >
                <Trash2 className="w-3.5 h-3.5" aria-hidden="true" /> {t('servers.discord.forget')}
              </button>
            </>
          )}
        </div>
        {note && <div className={`text-[11px] ${note.ok ? 'text-emerald-300' : 'text-red-300'}`}>{note.text}</div>}
      </div>

      {hasWebhook && (
        <div className="bg-slate-950/60 border border-slate-800 rounded-xl p-4 space-y-3">
          <label className="flex items-center gap-2 text-xs font-bold text-slate-200 cursor-pointer">
            <input type="checkbox" checked={settings.enabled} disabled={busy} onChange={() => void save({ ...settings, enabled: !settings.enabled })} />
            {t('servers.discord.enabled')}
          </label>
          <div className="grid sm:grid-cols-2 gap-2">
            {events.map((key) => (
              <label key={key} className={`flex items-start gap-2 p-2 rounded-lg border border-slate-800 cursor-pointer ${settings.enabled ? '' : 'opacity-50'}`}>
                <input type="checkbox" className="mt-0.5" checked={settings[key]} disabled={busy || !settings.enabled} onChange={() => toggle(key)} />
                <span>
                  <span className="block text-xs text-slate-200">{t(`servers.discord.events.${key}`)}</span>
                  <span className="block text-[10.5px] text-slate-500">{t(`servers.discord.events.${key}Example`)}</span>
                </span>
              </label>
            ))}
          </div>
        </div>
      )}

      <GameBotPanel paths={paths} filePath={filePath} />

      <div className="bg-slate-950/60 border border-slate-800 rounded-xl p-4 space-y-3">
        <div className="flex items-center gap-2 text-xs font-bold text-slate-200">
          <Terminal className="w-4 h-4 text-purple-300" aria-hidden="true" /> {t('servers.discord.commands.title')}
        </div>
        <p className="text-[11px] text-slate-400">{t('servers.discord.commands.intro')}</p>
        <p className="text-[11px] text-slate-400 font-mono">{t('servers.discord.commands.list')}</p>
        <div className="grid sm:grid-cols-2 gap-2">
          <label className="space-y-1">
            <span className="block text-[11px] text-slate-300">{t('servers.discord.commands.channel')}</span>
            <input value={cmdChannel} onChange={(e) => setCmdChannel(e.target.value)} placeholder="server-admin" spellCheck={false}
              className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-purple-500" />
          </label>
          <label className="space-y-1">
            <span className="block text-[11px] text-slate-300">{t('servers.discord.commands.role')}</span>
            <input value={cmdRole} onChange={(e) => setCmdRole(e.target.value)} placeholder="Admin" spellCheck={false}
              className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-purple-500" />
          </label>
        </div>
        <div className="flex flex-wrap gap-2 items-center">
          <label className="flex items-center gap-2 text-xs text-slate-200 cursor-pointer">
            <input
              type="checkbox"
              checked={settings.commands}
              disabled={busy || (!settings.commands && (cmdChannel.trim() === '' || cmdRole.trim() === ''))}
              onChange={() => void save({ ...settings, commands: !settings.commands, command_channel: cmdChannel.trim(), command_role: cmdRole.trim() })}
            />
            {t('servers.discord.commands.enabled')}
          </label>
          {settings.commands && (cmdChannel.trim() !== settings.command_channel || cmdRole.trim() !== settings.command_role) && (
            <button type="button" disabled={busy || cmdChannel.trim() === '' || cmdRole.trim() === ''}
              onClick={() => void save({ ...settings, command_channel: cmdChannel.trim(), command_role: cmdRole.trim() })}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-purple-600 hover:bg-purple-500 text-white cursor-pointer disabled:opacity-50">
              <Check className="w-3.5 h-3.5" aria-hidden="true" /> {t('servers.discord.commands.save')}
            </button>
          )}
        </div>
        {!settings.commands && (cmdChannel.trim() === '' || cmdRole.trim() === '') && (
          <p className="text-[11px] text-slate-500">{t('servers.discord.commands.fillFirst')}</p>
        )}
        {settings.commands && cmdStatus && cmdStatus.server === server && (
          <div className={`text-[11px] ${cmdStatus.listening ? 'text-emerald-300' : 'text-red-300'}`}>
            {cmdStatus.listening
              ? t('servers.discord.commands.listening', { channel: cmdStatus.channel })
              : (() => {
                  const m = cmdStatus.error.match(/^(no_channel|no_role|no_token|bad_token|forbidden|unreachable): (.*)$/s);
                  return m ? t(`servers.discord.commands.errors.${m[1]}`, { channel: cmdStatus.channel, role: settings.command_role, detail: m[2] }) : cmdStatus.error;
                })()}
          </div>
        )}
        {settings.commands && cmdStatus && cmdStatus.server !== server && (
          <div className="text-[11px] text-amber-300">{t('servers.discord.commands.otherServer', { server: cmdStatus.server })}</div>
        )}
        <p className="text-[10.5px] text-amber-300/80">{t('servers.discord.commands.needs')}</p>
      </div>
    </div>
  );
};
