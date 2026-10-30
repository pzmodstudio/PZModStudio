import React, { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import i18next from 'i18next';
import { Code2, ExternalLink, FolderOpen, FolderPlus, Languages, Loader2, LucideIcon, Map as MapIcon, Package, Play, RefreshCw, RotateCcw, SearchCheck, Trash2, UploadCloud, Users, Wrench } from 'lucide-react';
import { CatalogModSummary, DraftReport, DraftStatus, FileProblem, MapTools, ModApiReport, ModInfo, ModValidation, MpReport, PublishOutcome, PublishProgress, UploadPreview, WorkshopItemMeta } from '../../types';
import { formatBytes } from '../browse/common';
import { MpCheckPanel } from '../common/MpCheckPanel';
import { TauriService } from '../../services/tauri';
import { problemText } from '../../i18n/backendText';
import { StudioPathsUI } from '../settings/SettingsModule';
import { tHtml } from '../../i18n/html';
import { languageName } from '../../utils/language';

/** The game's language folders and the language each one is (BCP 47), for the name beside the code. */
const GAME_LANGUAGE_CODES: Record<string, string> = {
  AR: 'ar', CA: 'ca', CH: 'zh-TW', CN: 'zh-CN', CS: 'cs', DA: 'da', DE: 'de', EN: 'en', ES: 'es', ES_CL: 'es-CL', ES_MX: 'es-MX', FI: 'fi', FR: 'fr', HU: 'hu',
  ID: 'id', IT: 'it', JP: 'ja', KO: 'ko', NL: 'nl', NO: 'no', PL: 'pl', PT: 'pt-PT', PTBR: 'pt-BR', RO: 'ro', RU: 'ru', TH: 'th', TR: 'tr', UA: 'uk',
};
/** The picker's groups, in the order shown. */
type PickGroup = 'upload' | 'local' | 'installed' | 'compat' | 'studio' | 'workshop';
const PICK_GROUPS: PickGroup[] = ['upload', 'local', 'installed', 'compat', 'studio', 'workshop'];
/** The Studio's own mods: shipped with the app, not the player's work. */
const STUDIO_MODS = new Set(['Z_StudioLink', 'Z_ClaudeAgent', 'Z_PZModStudio_Bridge', 'PZModStudio_Profiler']);
/** The screen's tabs, in the order shown. */
type DevTab = 'check' | 'reload' | 'mp' | 'api' | 'drafts' | 'publish' | 'maps' | 'new';
const DEV_TABS: DevTab[] = ['check', 'reload', 'mp', 'api', 'drafts', 'publish', 'maps', 'new'];
const DEV_TAB_ICON: Record<DevTab, LucideIcon> = {
  check: Wrench, reload: RefreshCw, mp: Users, api: SearchCheck, drafts: Languages, publish: UploadCloud, maps: MapIcon, new: FolderPlus,
};
const EMPTY_NEW_MOD = { id: '', name: '', author: '', description: '', also41: false };
const gameLanguageName = (folder: string): string => (GAME_LANGUAGE_CODES[folder] ? languageName(GAME_LANGUAGE_CODES[folder]) : '');

interface DeveloperModuleProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  catalogById?: Map<string, CatalogModSummary>;
  /** Starts the game in debug with only this mod (and what it requires) on; the list comes back when the game closes. */
  onLaunchOnly: (modId: string) => void;
  /** The list set aside by such a launch, when one is; and the way to put it back now. */
  devLaunch: { modId: string; since: string } | null;
  onRestoreList: () => void;
  /** After a new mod is made: the installed mods are read again, so it can be picked. */
  onRefreshMods: () => void;
}

/**
 * For the people who make the mods. Pick a mod you are working on (a local one, usually), check
 * every Lua and script file of it before the game does, open the wrong line in your editor, and
 * start the game with only that mod on.
 */
export const DeveloperModule: React.FC<DeveloperModuleProps> = ({ paths, mods, catalogById, onLaunchOnly, devLaunch, onRestoreList, onRefreshMods }) => {
  const { t } = useTranslation();
  const [modId, setModId] = useState<string>(() => localStorage.getItem('pz_dev_mod') ?? '');
  const [validation, setValidation] = useState<ModValidation | null>(null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);

  // By where each mod comes from, the ones being worked on first: the player's own upload
  // folders, plain folders in Zomboid/mods, installs from elsewhere, the Studio's compatible
  // copies, the Studio's own mods, and the Workshop subscriptions last, for a look.
  const choices = useMemo(() => {
    const groups: Record<PickGroup, ModInfo[]> = { upload: [], local: [], installed: [], compat: [], studio: [], workshop: [] };
    for (const m of mods) {
      if (m.mod_id.startsWith('Z_PZModStudio_')) continue;
      const c = catalogById?.get(m.mod_id);
      const group: PickGroup = STUDIO_MODS.has(m.mod_id)
        ? 'studio'
        : c?.source.kind === 'workshop'
        ? 'workshop'
        : c?.source.kind === 'workshop_upload'
        ? 'upload'
        : c?.origin?.kind === 'compat'
        ? 'compat'
        : c?.origin
        ? 'installed'
        : 'local';
      groups[group].push(m);
    }
    return groups;
  }, [mods, catalogById]);
  const mod = mods.find((m) => m.mod_id === modId) ?? null;
  const root = mod ? catalogById?.get(mod.mod_id)?.root_dir ?? null : null;

  // Publishing to the Workshop: the player's items, the one picked, its form, the files, the upload.
  const [uploads, setUploads] = useState<WorkshopItemMeta[]>([]);
  const [uploadFolder, setUploadFolder] = useState<string>(() => localStorage.getItem('pz_dev_upload') ?? '');
  const [form, setForm] = useState<{ title: string; description: string; tags: string; visibility: string; note: string }>({ title: '', description: '', tags: '', visibility: 'public', note: '' });
  const [formSaved, setFormSaved] = useState(true);
  const [uploadPreview, setUploadPreview] = useState<UploadPreview | null>(null);
  const [showUploadFiles, setShowUploadFiles] = useState(false);
  const [uploadBusy, setUploadBusy] = useState<'' | 'prepare' | 'save' | 'preview' | 'publish'>('');
  const [uploadProgress, setUploadProgress] = useState<PublishProgress | null>(null);
  const [uploadOutcome, setUploadOutcome] = useState<PublishOutcome | null>(null);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const upload = uploads.find((u) => u.folder === uploadFolder) ?? null;
  const loadUploads = async () => {
    try {
      const list = await TauriService.workshopUploads(paths);
      setUploads(list);
      return list;
    } catch {
      return [];
    }
  };
  useEffect(() => {
    void loadUploads();
    let off: (() => void) | undefined;
    void TauriService.listenWorkshopUploadProgress((p) => setUploadProgress(p)).then((u) => {
      off = u;
    });
    return () => off?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths]);
  useEffect(() => {
    setUploadPreview(null);
    setUploadOutcome(null);
    setUploadError(null);
    setUploadProgress(null);
    if (upload) {
      setForm({ title: upload.title, description: upload.description, tags: upload.tags.join('; '), visibility: upload.visibility, note: '' });
      setFormSaved(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [uploadFolder, uploads.length]);
  const pickUpload = (folder: string) => {
    setUploadFolder(folder);
    localStorage.setItem('pz_dev_upload', folder);
  };
  const editForm = (patch: Partial<typeof form>) => {
    setForm((f) => ({ ...f, ...patch }));
    if (!('note' in patch)) setFormSaved(false);
  };
  const tagsOf = (text: string) => text.split(/[;,]/).map((t) => t.trim()).filter(Boolean);
  const saveForm = async () => {
    if (!upload) return;
    setUploadBusy('save');
    setUploadError(null);
    try {
      await TauriService.workshopUploadSaveMeta(paths, upload.folder, form.title, form.description, tagsOf(form.tags), form.visibility);
      setFormSaved(true);
      await loadUploads();
    } catch (err) {
      setUploadError(String(err));
    } finally {
      setUploadBusy('');
    }
  };
  const prepareUpload = async () => {
    if (!root) return;
    setUploadBusy('prepare');
    setUploadError(null);
    try {
      const made = await TauriService.workshopUploadPrepare(paths, root);
      await loadUploads();
      pickUpload(made.folder);
    } catch (err) {
      setUploadError(String(err));
    } finally {
      setUploadBusy('');
    }
  };
  const removeUpload = async () => {
    if (!upload) return;
    if (!window.confirm(t('developer.publish.removeConfirm', { title: upload.title, path: upload.path }))) return;
    setUploadBusy('prepare');
    setUploadError(null);
    try {
      await TauriService.workshopUploadRemove(paths, upload.folder);
      pickUpload('');
      await loadUploads();
    } catch (err) {
      setUploadError(String(err));
    } finally {
      setUploadBusy('');
    }
  };
  const previewUpload = async () => {
    if (!upload) return;
    setUploadBusy('preview');
    setUploadError(null);
    try {
      setUploadPreview(await TauriService.workshopUploadPreview(paths, upload.folder));
    } catch (err) {
      setUploadError(String(err));
    } finally {
      setUploadBusy('');
    }
  };
  const publishUpload = async () => {
    if (!upload) return;
    if (!formSaved) await saveForm();
    if (!window.confirm(t('developer.publish.confirm', { title: form.title, what: upload.id ? t('developer.publish.confirmUpdate', { id: upload.id }) : t('developer.publish.confirmNew') }))) return;
    setUploadBusy('publish');
    setUploadError(null);
    setUploadOutcome(null);
    setUploadProgress({ folder: upload.folder, stage: 'copying', processed: 0, total: 0 });
    try {
      const out = await TauriService.workshopPublish(paths, upload.folder, form.note);
      setUploadOutcome(out);
      setForm((f) => ({ ...f, note: '' }));
      await loadUploads();
      setUploadPreview(await TauriService.workshopUploadPreview(paths, upload.folder).catch(() => null));
    } catch (err) {
      setUploadError(String(err));
    } finally {
      setUploadBusy('');
      setUploadProgress(null);
    }
  };
  const modHasUpload = !!mod && uploads.some((u) => u.mod_ids.includes(mod.mod_id));
  // Only a mod of the player's own (a local folder) is theirs to put on the Workshop.
  const modIsLocal = !!mod && catalogById?.get(mod.mod_id)?.source.kind === 'local' && !catalogById?.get(mod.mod_id)?.origin && !STUDIO_MODS.has(mod.mod_id);
  const notYours = upload?.published_elsewhere ?? null;
  const fileStateClass: Record<string, string> = { new: 'text-emerald-300', changed: 'text-amber-300', removed: 'text-red-300 line-through', same: 'text-slate-500' };

  // Against the installed build: the names the mod calls that it does not have.
  const [api, setApi] = useState<ModApiReport | null>(null);
  const [apiBusy, setApiBusy] = useState(false);
  const [apiError, setApiError] = useState<string | null>(null);
  const checkApi = async () => {
    if (!root || !mod) return;
    setApiBusy(true);
    setApiError(null);
    try {
      setApi(await TauriService.apiLintMod(paths, mod.mod_id, root));
    } catch (err) {
      setApi(null);
      setApiError(String(err));
    } finally {
      setApiBusy(false);
    }
  };
  useEffect(() => {
    setApi(null);
    setApiError(null);
  }, [root]);
  const apiByKind = useMemo(() => {
    const out: Record<'global' | 'method' | 'event', ModApiReport['findings']> = { global: [], method: [], event: [] };
    for (const f of api?.findings ?? []) out[f.kind].push(f);
    return out;
  }, [api]);

  // Translation drafts: the mod's languages, the ones picked, what the last run wrote.
  const [drafts, setDrafts] = useState<DraftStatus | null>(null);
  const [draftsError, setDraftsError] = useState<string | null>(null);
  const [draftLangs, setDraftLangs] = useState<Set<string>>(() => {
    try {
      return new Set(JSON.parse(localStorage.getItem('pz_dev_draft_langs') ?? '[]') as string[]);
    } catch {
      return new Set();
    }
  });
  const [draftBusy, setDraftBusy] = useState(false);
  const [draftReport, setDraftReport] = useState<DraftReport | null>(null);
  const loadDrafts = async (dir: string) => {
    setDraftsError(null);
    try {
      setDrafts(await TauriService.translationDraftsStatus(dir));
    } catch (err) {
      setDrafts(null);
      setDraftsError(String(err));
    }
  };
  useEffect(() => {
    setDrafts(null);
    setDraftReport(null);
    if (root) void loadDrafts(root);
  }, [root]);
  const toggleDraftLang = (lang: string) => {
    setDraftLangs((old) => {
      const next = new Set(old);
      if (next.has(lang)) next.delete(lang);
      else next.add(lang);
      localStorage.setItem('pz_dev_draft_langs', JSON.stringify([...next]));
      return next;
    });
  };
  const writeDrafts = async () => {
    if (!root || !drafts) return;
    const langs = drafts.languages.filter((l) => l.supported && draftLangs.has(l.lang) && l.missing > 0).map((l) => l.lang);
    if (langs.length === 0) return;
    setDraftBusy(true);
    setDraftReport(null);
    try {
      setDraftReport(await TauriService.writeTranslationDrafts(root, langs));
      await loadDrafts(root);
    } catch (err) {
      setDraftsError(String(err));
    } finally {
      setDraftBusy(false);
    }
  };
  const draftPick = drafts ? drafts.languages.filter((l) => l.supported && draftLangs.has(l.lang) && l.missing > 0) : [];

  const pick = (id: string) => {
    setModId(id);
    localStorage.setItem('pz_dev_mod', id);
    setValidation(null);
  };

  const validate = async () => {
    if (!root) return;
    setBusy(true);
    try {
      setValidation(await TauriService.validateMod(paths, root));
      window.setTimeout(() => document.getElementById("dev-validation")?.scrollIntoView({ block: "nearest", behavior: "smooth" }), 50);
    } catch {
      // Already shown by TauriService.
    } finally {
      setBusy(false);
    }
  };

  const openProblem = async (p: FileProblem) => {
    if (!root) return;
    try {
      const where = await TauriService.openInEditor(paths, `${root}/${p.path}`, p.line, p.col);
      setNote(i18next.t('developer.note.opened', { where }));
      setTimeout(() => setNote(null), 3000);
    } catch {
      // Already shown by TauriService.
    }
  };

  /**
   * Reload in the game while it runs. The Studio watches the mod's folder (Windows says the
   * moment a file is saved), checks each changed Lua file with the parser, and hands the good
   * ones to the bridge, which reloads them on the menu, in game or paused. Each stage comes back
   * as an event with its time.
   */
  const [watching, setWatching] = useState(false);
  const [reloadLog, setReloadLog] = useState<string[]>([]);
  const [reloading, setReloading] = useState(false);
  const log = (line: string) => setReloadLog((l) => [`${new Date().toLocaleTimeString()}  ${line}`, ...l].slice(0, 60));

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    TauriService.listenDevReload((e) => {
      if (e.stage === 'syntax') {
        e.problems.forEach((p) => log(i18next.t('developer.reload.syntax', { path: p.path, line: p.line ?? '?', col: p.col ?? '?', message: p.message })));
      } else if (e.stage === 'sent') {
        e.files.forEach((f) => log(i18next.t('developer.reload.changed', { path: f })));
      } else if (e.stage === 'applied') {
        e.results.forEach((r) => log(r));
        log(i18next.t('developer.reload.timing', {
          sent: Math.max(0, e.sent_ms - e.detected_ms),
          applied: Math.max(0, e.applied_ms - e.detected_ms),
          total: Math.max(0, e.answered_ms - e.detected_ms),
        }));
        setReloading(false);
      } else if (e.stage === 'not_running') {
        log(i18next.t('developer.reload.notRunning'));
        setReloading(false);
      } else if (e.stage === 'no_answer') {
        log(i18next.t('developer.reload.noAnswer'));
        setReloading(false);
      }
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    if (!watching || !root) return;
    void TauriService.devWatchStart(paths, root).then((ok) => {
      if (!ok) setWatching(false);
    });
    return () => {
      void TauriService.devWatchStop(paths);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [watching, root]);

  const reloadAllNow = async () => {
    if (!root) return;
    setReloading(true);
    await TauriService.devReloadAll(paths, root);
    // The stages come as events; this only keeps the button from being pressed twice.
    setTimeout(() => setReloading(false), 7000);
  };

  /** A new mod: the form, and what came of it. */
  const [newMod, setNewMod] = useState<typeof EMPTY_NEW_MOD>(EMPTY_NEW_MOD);
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<{ folder: string; files: string[]; id: string } | null>(null);
  const idFromName = (name: string) => name.replace(/[^A-Za-z0-9_-]+/g, '');
  const createMod = async () => {
    setCreating(true);
    try {
      const made = await TauriService.createMod(paths.user_zomboid_dir, {
        id: newMod.id.trim(),
        name: newMod.name.trim(),
        author: newMod.author.trim(),
        description: newMod.description,
        builds: newMod.also41 ? ['41', '42'] : ['42'],
      });
      setCreated({ ...made, id: newMod.id.trim() });
      setNewMod(EMPTY_NEW_MOD);
      onRefreshMods();
      pick(newMod.id.trim());
    } catch {
      // Already shown by TauriService.
    } finally {
      setCreating(false);
    }
  };

  /** TileZed and WorldEd: where each is, and a folder of the player's for each to look in. */
  const [tilezedDir, setTilezedDir] = useState<string>(() => localStorage.getItem('pz_tilezed_dir') ?? localStorage.getItem('pz_map_tools_dir') ?? '');
  const [worldedDir, setWorldedDir] = useState<string>(() => localStorage.getItem('pz_worlded_dir') ?? localStorage.getItem('pz_map_tools_dir') ?? '');
  const [mapTools, setMapTools] = useState<MapTools | null>(null);
  useEffect(() => {
    let live = true;
    void TauriService.mapTools([tilezedDir, worldedDir]).then((t) => live && setMapTools(t));
    return () => {
      live = false;
    };
  }, [tilezedDir, worldedDir]);
  const setTilezed = (dir: string) => {
    setTilezedDir(dir);
    localStorage.setItem('pz_tilezed_dir', dir);
  };
  const setWorlded = (dir: string) => {
    setWorldedDir(dir);
    localStorage.setItem('pz_worlded_dir', dir);
  };

  /** The multiplayer check of the chosen mod. */
  const [mp, setMp] = useState<MpReport | null>(null);
  const [mpBusy, setMpBusy] = useState(false);
  const checkMp = async () => {
    if (!root) return;
    setMpBusy(true);
    try {
      setMp(await TauriService.mpCheckMod(paths, root));
    } catch {
      // Already shown by TauriService.
    } finally {
      setMpBusy(false);
    }
  };

  const byFile = useMemo(() => {
    const map = new Map<string, FileProblem[]>();
    for (const p of validation?.problems ?? []) map.set(p.path, [...(map.get(p.path) ?? []), p]);
    return [...map.entries()];
  }, [validation]);

  /** The screen's tabs, one tool each; "new" makes a mod instead of working on the picked one. */
  const [tab, setTab] = useState<DevTab>(() => {
    const saved = localStorage.getItem('pz_dev_tab') as DevTab | null;
    return saved && DEV_TABS.includes(saved) ? saved : 'check';
  });
  const openTab = (id: DevTab) => {
    setTab(id);
    localStorage.setItem('pz_dev_tab', id);
  };
  // What each tab has to say at a glance: problems found, watching, the multiplayer verdict.
  const tabBadge: Partial<Record<DevTab, { text: string; cls: string }>> = {};
  if (validation) tabBadge.check = validation.problems.length > 0 ? { text: String(validation.problems.length), cls: 'bg-amber-500 text-slate-950' } : { text: '✓', cls: 'bg-emerald-900 text-emerald-300' };
  if (watching) tabBadge.reload = { text: '●', cls: 'text-emerald-400' };
  if (mp) tabBadge.mp = mp.verdict === 'no' ? { text: '✕', cls: 'bg-red-600 text-white' } : mp.verdict === 'partial' ? { text: '!', cls: 'bg-amber-500 text-slate-950' } : { text: '✓', cls: 'bg-emerald-900 text-emerald-300' };
  if (api) tabBadge.api = api.findings.length > 0 ? { text: String(api.findings.length), cls: 'bg-amber-500 text-slate-950' } : { text: '✓', cls: 'bg-emerald-900 text-emerald-300' };
  const draftsMissing = drafts?.languages.filter((l) => l.supported && l.missing > 0).length ?? 0;
  if (draftsMissing > 0) tabBadge.drafts = { text: String(draftsMissing), cls: 'bg-slate-700 text-slate-200' };
  const tabIntro: Record<DevTab, string> = {
    check: 'developer.check.intro',
    reload: 'developer.reload.intro',
    mp: 'developer.mp.intro',
    api: 'developer.api.intro',
    drafts: 'developer.drafts.intro',
    publish: 'developer.publish.intro',
    maps: 'developer.mapTools.intro',
    new: 'developer.newMod.intro',
  };
  const needsMod = tab !== 'publish' && tab !== 'maps' && tab !== 'new';

  return (
    <div className="flex-1 flex flex-col min-h-0 overflow-y-auto p-6 gap-4 font-sans">
      {devLaunch && (
        <div className="flex items-center gap-3 px-4 py-2.5 rounded-xl border border-amber-700/60 bg-amber-950/30 text-xs text-amber-200">
          <Play className="w-4 h-4 shrink-0" aria-hidden="true" />
          <span className="flex-1" dangerouslySetInnerHTML={{ __html: tHtml('developer.devLaunch.banner', { name: mods.find((m) => m.mod_id === devLaunch.modId)?.name ?? devLaunch.modId, since: devLaunch.since }) }} />
          <button type="button" onClick={onRestoreList} className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-[11px] font-bold bg-amber-700 hover:bg-amber-600 text-white cursor-pointer">
            <RotateCcw className="w-3 h-3" aria-hidden="true" /> {t('developer.devLaunch.restore')}
          </button>
        </div>
      )}

      {/* The screen and the mod everything below works on */}
      <div className="p-5 bg-gradient-to-r from-emerald-950/80 via-slate-900 to-teal-950/80 border border-emerald-800/60 rounded-2xl shadow-xl flex flex-col gap-4">
        <div className="flex flex-col md:flex-row md:items-start justify-between gap-4">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <Code2 className="w-6 h-6 text-emerald-400" aria-hidden="true" />
              <h1 className="text-xl font-bold text-slate-100 tracking-tight">{t('developer.header.title')}</h1>
            </div>
            <p className="text-xs text-slate-300 max-w-2xl leading-relaxed">{t('developer.header.subtitle')}</p>
          </div>
        </div>
        <div className="rounded-xl border border-slate-700/70 bg-slate-950/60 p-3 flex flex-col lg:flex-row lg:items-center gap-3">
          <div className="w-10 h-10 shrink-0 rounded-lg bg-emerald-900/40 border border-emerald-800 flex items-center justify-center text-emerald-300 max-lg:hidden">
            <Package className="w-5 h-5" aria-hidden="true" />
          </div>
          <div className="flex-1 min-w-0 flex flex-col gap-1">
            <div className="text-[10px] uppercase tracking-wider font-bold text-slate-500">{t('developer.pick.title')}</div>
            <select
              value={modId}
              onChange={(e) => pick(e.target.value)}
              aria-label={t('developer.pick.aria')}
              className="w-full max-w-md px-2.5 py-1.5 rounded-lg bg-slate-900 border border-slate-700 text-sm font-bold text-slate-100 cursor-pointer"
            >
              <option value="">{t('developer.pick.placeholder')}</option>
              {PICK_GROUPS.map((g) =>
                choices[g].length === 0 ? null : (
                  <optgroup key={g} label={t(`developer.pick.group.${g}`)}>
                    {choices[g].map((m) => <option key={m.mod_id} value={m.mod_id}>{m.name} — {m.mod_id}</option>)}
                  </optgroup>
                )
              )}
            </select>
            {mod && (
              <div className="flex flex-wrap items-center gap-1.5 text-[10px] font-mono text-slate-500 min-w-0">
                <span className="px-1.5 rounded border border-slate-700 text-slate-300">{mod.mod_id}</span>
                {mod.version && <span className="px-1.5 rounded border border-slate-700 text-slate-300">v{mod.version}</span>}
                {mod.workshop_id && <span className="px-1.5 rounded border border-sky-800 text-sky-300">#{mod.workshop_id}</span>}
                {root && <span className="break-all">{root}</span>}
              </div>
            )}
          </div>
          <div className="flex flex-wrap gap-2 shrink-0">
            <button type="button" disabled={!root} onClick={() => root && TauriService.openModFolder(paths, root)}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50" title={root ?? undefined}>
              <FolderOpen className="w-3.5 h-3.5" aria-hidden="true" /> {t('developer.pick.openFolder')}
            </button>
            <button
              type="button"
              disabled={!mod || !!devLaunch}
              onClick={() => mod && onLaunchOnly(mod.mod_id)}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer disabled:opacity-50"
              title={t('developer.pick.launchTitle')}
            >
              <Play className="w-3.5 h-3.5" aria-hidden="true" /> {t('developer.pick.launch')}
            </button>
          </div>
        </div>
      </div>

      {/* wraps to a second row instead of scrolling, so no scroll arrows */}
      <div className="sticky -top-6 z-10 -mx-6 px-6 pt-3 bg-slate-950/95 backdrop-blur">
        <div role="tablist" className="flex flex-wrap items-end gap-1 border-b border-slate-800">
          {DEV_TABS.map((id) => {
            const Icon = DEV_TAB_ICON[id];
            const badge = tabBadge[id];
            const active = tab === id;
            return (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={active}
                onClick={() => openTab(id)}
                className={`flex items-center gap-2 px-3 py-2 text-xs font-bold rounded-t-lg border border-b-0 -mb-px cursor-pointer ${
                  active ? 'bg-slate-900 border-slate-700 text-slate-100' : 'border-transparent text-slate-400 hover:text-slate-200'
                }`}
              >
                <Icon className="w-3.5 h-3.5" aria-hidden="true" />
                {t(`developer.tabs.${id}`)}
                {badge && <span className={`px-1.5 rounded-full text-[10px] font-mono ${badge.cls}`}>{badge.text}</span>}
              </button>
            );
          })}
        </div>
      </div>

      <p className="text-xs text-slate-400 max-w-3xl leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml(tabIntro[tab]) }} />

      {needsMod && !root ? (
        <div className="rounded-xl border border-dashed border-slate-700 bg-slate-900/40 px-4 py-8 text-center text-xs text-slate-400">{t('developer.noMod')}</div>
      ) : (
        <div className="bg-slate-900/80 border border-slate-800 rounded-xl p-5 flex flex-col gap-3">
          {tab === 'check' && (
            <>
              <div className="flex flex-wrap items-center justify-between gap-3">
                <button
                  type="button"
                  disabled={!root || busy}
                  onClick={() => void validate()}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer disabled:opacity-50"
                  title={t('developer.pick.checkTitle')}
                >
                  {busy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Wrench className="w-3.5 h-3.5" aria-hidden="true" />} {t('developer.pick.check')}
                </button>
                {validation && <div className="text-[11px] font-mono text-slate-500">{t('developer.results.counts', { lua: validation.lua_files, scripts: validation.script_files })}</div>}
              </div>
              {validation && (
                <div id="dev-validation" className="flex flex-col gap-3">
                  <div className="text-xs font-bold text-slate-200">
                    {validation.problems.length === 0 ? t('developer.results.allRead') : t('developer.results.problems', { problems: validation.problems.length, files: byFile.length })}
                  </div>
                  {validation.problems.length === 0 && <p className="text-[11px] text-slate-400">{t('developer.results.clean')}</p>}
                  {byFile.map(([file, problems]) => (
                    <div key={file} className="rounded-lg border border-slate-800 bg-slate-950/60">
                      <div className="px-3 py-1.5 text-[11px] font-mono text-slate-300 border-b border-slate-900 flex items-center justify-between gap-2">
                        <span className="truncate">{file}</span>
                        <span className={`shrink-0 px-1.5 rounded text-[10px] ${problems[0].kind === 'lua' ? 'bg-sky-950 text-sky-300' : 'bg-violet-950 text-violet-300'}`}>{problems[0].kind}</span>
                      </div>
                      {problems.map((p, i) => (
                        <button
                          key={i}
                          type="button"
                          onClick={() => void openProblem(p)}
                          className="w-full text-left px-3 py-1.5 text-[11px] text-slate-300 hover:bg-slate-900 cursor-pointer flex items-start gap-2"
                          title={t('developer.results.openTitle')}
                        >
                          <span className="font-mono text-amber-300 shrink-0 w-16">{p.line ? `${p.line}${p.col ? `:${p.col}` : ''}` : '—'}</span>
                          <span className="flex-1">{problemText(p)}</span>
                          <ExternalLink className="w-3 h-3 text-slate-500 shrink-0 mt-0.5" aria-hidden="true" />
                        </button>
                      ))}
                    </div>
                  ))}
                </div>
              )}
            </>
          )}

          {tab === 'reload' && (
            <>
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex flex-wrap items-center gap-3">
                  <button
                    type="button"
                    disabled={!root || reloading}
                    onClick={() => void reloadAllNow()}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50"
                    title={t('developer.reload.allTitle')}
                  >
                    {reloading ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <RefreshCw className="w-3.5 h-3.5" aria-hidden="true" />} {t('developer.reload.all')}
                  </button>
                  {watching && <span className="text-[11px] text-emerald-300">{t('developer.reload.watching', { name: mod?.name })}</span>}
                </div>
                <label className="flex items-center gap-2 text-[11px] text-slate-300 cursor-pointer">
                  {t('developer.reload.watchAria')}
                  <button
                    type="button"
                    role="switch"
                    aria-checked={watching}
                    aria-label={t('developer.reload.watchAria')}
                    disabled={!root}
                    onClick={() => setWatching((w) => !w)}
                    className={`w-10 h-5 shrink-0 rounded-full p-0.5 flex items-center transition cursor-pointer disabled:opacity-50 ${
                      watching ? 'bg-emerald-500 justify-end' : 'bg-slate-700 justify-start'
                    }`}
                  >
                    <span className="w-4 h-4 rounded-full bg-white shadow-md" />
                  </button>
                </label>
              </div>
              {reloadLog.length > 0 && (
                <div className="rounded-lg border border-slate-800 bg-slate-950/70 p-2 max-h-72 overflow-y-auto font-mono text-[10.5px] text-slate-300 flex flex-col gap-0.5">
                  {reloadLog.map((l, i) => (
                    <div key={i} className={l.includes('  failed ') ? 'text-red-300' : l.includes('  ok ') ? 'text-emerald-300' : ''}>{l}</div>
                  ))}
                </div>
              )}
            </>
          )}

          {tab === 'mp' && (
            <>
              <div>
                <button
                  type="button"
                  disabled={!root || mpBusy}
                  onClick={() => void checkMp()}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50"
                >
                  {mpBusy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Users className="w-3.5 h-3.5" aria-hidden="true" />} {t('developer.mp.check')}
                </button>
              </div>
              {mp && mod && (
                <>
                  <div className="text-[11px] text-slate-500">{t('developer.mp.summary', { count: mp.files_checked, server: mp.has_server_lua ? t('developer.mp.alsoServer') : '' })}</div>
                  <MpCheckPanel
                    reports={[{ mod_id: mod.mod_id, name: mod.name, report: mp }]}
                    listVerdict={false}
                    onOpen={(r, f) => void TauriService.openInEditor(paths, `${r}/${f.path}`, f.line, null).catch(() => undefined)}
                  />
                </>
              )}
            </>
          )}

          {tab === 'api' && (
            <>
              <div>
                <button
                  type="button"
                  disabled={!root || apiBusy}
                  onClick={() => void checkApi()}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50"
                  title={t('developer.api.checkTitle')}
                >
                  {apiBusy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <SearchCheck className="w-3.5 h-3.5" aria-hidden="true" />} {t('developer.api.check')}
                </button>
              </div>
              {apiError && <div className="text-[11px] text-amber-300">{apiError}</div>}
              {api && (
                <>
                  <div className="text-[11px] text-slate-500">
                    {t('developer.api.summary', { files: api.files_checked, calls: api.calls_checked, count: api.findings.length })}
                    {api.has_java && <span className="text-sky-300"> · {t('developer.api.hasJava')}</span>}
                  </div>
                  {api.findings.length === 0 && <p className="text-[11px] text-emerald-300">{t('developer.api.clean')}</p>}
                  {(['global', 'method', 'event'] as const).map((kind) =>
                    apiByKind[kind].length === 0 ? null : (
                      <div key={kind} className="rounded-lg border border-slate-800 bg-slate-950/60">
                        <div className="px-3 py-1.5 text-[11px] font-bold text-slate-300 border-b border-slate-900">{t(`developer.api.kind.${kind}`, { count: apiByKind[kind].length })}</div>
                        {apiByKind[kind].map((f, i) => (
                          <button
                            key={i}
                            type="button"
                            onClick={() => root && void TauriService.openInEditor(paths, `${root}/${f.file}`, f.line, null).catch(() => undefined)}
                            className="w-full text-left px-3 py-1.5 text-[11px] text-slate-300 hover:bg-slate-900 cursor-pointer flex items-start gap-2"
                            title={t('developer.results.openTitle')}
                          >
                            <span className="font-mono text-amber-300 shrink-0">{kind === 'method' ? ':' : ''}{f.name}{kind === 'global' ? '(' : ''}</span>
                            <span className="flex-1 min-w-0">
                              {f.suggestion && <span className="text-emerald-300">{t('developer.api.didYouMean', { name: f.suggestion })} </span>}
                              <span className="font-mono text-slate-500 break-all">{f.file}:{f.line}</span>
                            </span>
                            <ExternalLink className="w-3 h-3 text-slate-500 shrink-0 mt-0.5" aria-hidden="true" />
                          </button>
                        ))}
                      </div>
                    )
                  )}
                  <p className="text-[11px] text-slate-500 max-w-2xl">{t('developer.api.note')}</p>
                </>
              )}
            </>
          )}

          {tab === 'drafts' && (
            <>
              {draftsError && <div className="text-[11px] text-amber-300">{draftsError}</div>}
              {!drafts && !draftsError && <Loader2 className="w-4 h-4 animate-spin text-slate-500" aria-hidden="true" />}
              {drafts && (
                <>
                  <div className="text-[11px] text-slate-300">
                    {t('developer.drafts.source', { lang: drafts.source_lang, name: gameLanguageName(drafts.source_lang), keys: drafts.keys, count: drafts.categories.length })}
                    {drafts.format_b41 && <span className="text-slate-500"> · {t('developer.drafts.b41')}</span>}
                  </div>
                  {(['missing', 'complete', 'unknown'] as const).map((group) => {
                    const list = drafts.languages.filter((l) => (group === 'unknown' ? !l.supported : l.supported && (group === 'missing' ? l.missing > 0 : l.missing === 0)));
                    if (list.length === 0) return null;
                    return (
                      <div key={group} className="flex flex-col gap-1">
                        <div className="text-[11px] text-slate-400">{t(`developer.drafts.group.${group}`, { count: list.length })}</div>
                        <div className="flex flex-wrap gap-1.5">
                          {list.map((l) => {
                            const full = l.missing === 0;
                            const on = draftLangs.has(l.lang) && l.supported && !full;
                            const name = gameLanguageName(l.lang);
                            return (
                              <label
                                key={l.lang}
                                className={`flex items-center gap-1.5 px-2 py-1 rounded-md border text-[11px] ${
                                  on ? 'border-emerald-600/70 bg-emerald-950/30 text-emerald-200' : full ? 'border-slate-800 text-slate-500' : 'border-slate-700 text-slate-300 hover:bg-slate-800'
                                } ${!l.supported || full ? 'cursor-default' : 'cursor-pointer'}`}
                                title={
                                  !l.supported
                                    ? t('developer.drafts.unsupported', { lang: l.lang })
                                    : full
                                    ? t('developer.drafts.full', { name })
                                    : t('developer.drafts.langTitle', { name, present: l.present, keys: drafts.keys, machine: l.machine, review: l.review })
                                }
                              >
                                {group === 'missing' && <input type="checkbox" checked={on} onChange={() => toggleDraftLang(l.lang)} className="accent-emerald-500" />}
                                <span className="font-mono font-bold">{l.lang}</span>
                                {name && <span>{name}</span>}
                                <span className="text-slate-500">
                                  {full ? '✓' : t('developer.drafts.missingCount', { count: l.missing })}
                                  {l.machine > 0 && <span className="text-sky-400"> · {t('developer.drafts.machine', { count: l.machine })}</span>}
                                  {l.review > 0 && <span className="text-amber-400"> · {t('developer.drafts.review', { count: l.review })}</span>}
                                </span>
                              </label>
                            );
                          })}
                        </div>
                      </div>
                    );
                  })}
                  <div className="flex flex-wrap items-center gap-2">
                    <button
                      type="button"
                      disabled={draftBusy || draftPick.length === 0}
                      onClick={() => void writeDrafts()}
                      className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer disabled:opacity-50"
                      title={t('developer.drafts.writeTitle')}
                    >
                      {draftBusy ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <Languages className="w-3.5 h-3.5" aria-hidden="true" />}
                      {t('developer.drafts.write', { count: draftPick.length, keys: draftPick.reduce((n, l) => n + l.missing, 0) })}
                    </button>
                    <button type="button" onClick={() => void TauriService.openModFolder(paths, drafts.folder)} className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer" title={drafts.folder}>
                      <FolderOpen className="w-3.5 h-3.5" aria-hidden="true" /> {t('developer.drafts.openFolder')}
                    </button>
                  </div>
                  {draftReport && (
                    <div className="rounded-lg border border-slate-800 bg-slate-950/70 p-2 font-mono text-[10.5px] text-slate-300 flex flex-col gap-0.5">
                      <div className="text-emerald-300">{t('developer.drafts.done', { keys: draftReport.keys_written, files: draftReport.files.length, service: draftReport.service })}</div>
                      {draftReport.files.map((f) => (
                        <div key={f.path}>
                          {f.lang}/{f.category}: {t('developer.drafts.fileKeys', { count: f.keys })}
                          {f.review.length > 0 && <span className="text-amber-300"> · {t('developer.drafts.fileReview', { count: f.review.length, keys: f.review.slice(0, 5).join(', ') })}</span>}
                        </div>
                      ))}
                      <div className="text-slate-500">{t('developer.drafts.after')}</div>
                    </div>
                  )}
                </>
              )}
            </>
          )}

          {tab === 'publish' && (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <select
                  value={uploadFolder}
                  onChange={(e) => pickUpload(e.target.value)}
                  aria-label={t('developer.publish.pickAria')}
                  className="min-w-[280px] px-2.5 py-1.5 rounded-lg bg-slate-950 border border-slate-800 text-xs text-slate-200 cursor-pointer"
                >
                  <option value="">{uploads.length === 0 ? t('developer.publish.none') : t('developer.publish.pick')}</option>
                  {uploads.map((u) => (
                    <option key={u.folder} value={u.folder}>
                      {u.title} — {u.id ? `#${u.id}` : t('developer.publish.unpublished')} — {u.mod_ids.join(', ') || u.folder}
                    </option>
                  ))}
                </select>
                {root && mod && modIsLocal && !modHasUpload && (
                  <button
                    type="button"
                    disabled={uploadBusy !== ''}
                    onClick={() => void prepareUpload()}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50"
                    title={t('developer.publish.prepareTitle')}
                  >
                    {uploadBusy === 'prepare' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <FolderPlus className="w-3.5 h-3.5" aria-hidden="true" />}
                    {t('developer.publish.prepare', { name: mod.name })}
                  </button>
                )}
                {upload && (
                  <button type="button" onClick={() => TauriService.openModFolder(paths, upload.path)} className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer" title={upload.path}>
                    <FolderOpen className="w-3.5 h-3.5" aria-hidden="true" /> {t('developer.pick.openFolder')}
                  </button>
                )}
                {upload && (
                  <button type="button" disabled={uploadBusy !== ''} onClick={() => void removeUpload()} className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer disabled:opacity-50" title={t('developer.publish.removeTitle')}>
                    <Trash2 className="w-3.5 h-3.5" aria-hidden="true" /> {t('developer.publish.remove')}
                  </button>
                )}
                {upload?.id && (
                  <button type="button" onClick={() => void TauriService.openExternalUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${upload.id}`)} className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer">
                    <ExternalLink className="w-3.5 h-3.5" aria-hidden="true" /> {t('developer.publish.openPage')}
                  </button>
                )}
              </div>
              {notYours && (
                <div className="text-[11px] rounded-md border border-red-700/60 bg-red-950/40 text-red-200 px-3 py-2">
                  {t('developer.publish.notYours', { id: notYours })}{' '}
                  <button type="button" onClick={() => void TauriService.openExternalUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${notYours}`)} className="underline cursor-pointer">{t('developer.publish.openPage')}</button>
                </div>
              )}
              {upload && (
                <>
                  <div className="grid gap-2 md:grid-cols-[1fr_180px]">
                    <label className="flex flex-col gap-1 text-[11px] text-slate-400">
                      {t('developer.publish.titleField')}
                      <input type="text" value={form.title} onChange={(e) => editForm({ title: e.target.value })} className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500" />
                    </label>
                    <label className="flex flex-col gap-1 text-[11px] text-slate-400">
                      {t('developer.publish.visibility')}
                      <select value={form.visibility} onChange={(e) => editForm({ visibility: e.target.value })} className="bg-slate-950 border border-slate-800 rounded-lg px-2.5 py-1.5 text-xs text-slate-200 cursor-pointer">
                        {(['public', 'friends', 'private', 'unlisted'] as const).map((v) => <option key={v} value={v}>{t(`developer.publish.visibilities.${v}`)}</option>)}
                      </select>
                    </label>
                  </div>
                  <label className="flex flex-col gap-1 text-[11px] text-slate-400">
                    {t('developer.publish.tags')}
                    <input type="text" value={form.tags} onChange={(e) => editForm({ tags: e.target.value })} placeholder="Build 42; Items; QoL" className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500" />
                  </label>
                  <label className="flex flex-col gap-1 text-[11px] text-slate-400">
                    {t('developer.publish.description')}
                    <textarea value={form.description} onChange={(e) => editForm({ description: e.target.value })} rows={8} spellCheck={false} className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-emerald-500" />
                    <span className="text-[10px] text-slate-500">{t('developer.publish.descriptionNote', { count: form.description.length })}</span>
                  </label>
                  <div className="flex flex-wrap items-center gap-2">
                    <button type="button" disabled={formSaved || uploadBusy !== ''} onClick={() => void saveForm()} className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50">
                      {uploadBusy === 'save' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : null} {formSaved ? t('developer.publish.saved') : t('developer.publish.save')}
                    </button>
                    <button type="button" disabled={uploadBusy !== ''} onClick={() => void previewUpload()} className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer disabled:opacity-50" title={t('developer.publish.previewTitle')}>
                      {uploadBusy === 'preview' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <SearchCheck className="w-3.5 h-3.5" aria-hidden="true" />} {t('developer.publish.preview')}
                    </button>
                    {!upload.has_preview && <span className="text-[11px] text-amber-300">{t('developer.publish.noPreviewPng')}</span>}
                  </div>
                  {uploadPreview && (
                    <div className="rounded-lg border border-slate-800 bg-slate-950/60 p-3 flex flex-col gap-2 text-[11px]">
                      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-slate-300">
                        <span>{t('developer.publish.filesSummary', { count: uploadPreview.files.filter((f) => f.state !== 'removed').length, size: formatBytes(uploadPreview.total_bytes) })}</span>
                        {uploadPreview.last_upload_at !== null ? (
                          <span className="text-slate-500">
                            {t('developer.publish.sinceLast', { date: new Date(uploadPreview.last_upload_at * 1000).toLocaleString(i18next.language, { dateStyle: 'medium', timeStyle: 'short' }) })}
                            {' '}<span className="text-emerald-300">+{uploadPreview.new_count}</span> <span className="text-amber-300">~{uploadPreview.changed_count}</span> <span className="text-red-300">-{uploadPreview.removed_count}</span> <span>={uploadPreview.same_count}</span>
                          </span>
                        ) : (
                          <span className="text-slate-500">{t('developer.publish.noLast')}</span>
                        )}
                        <span className="text-slate-500">{t('developer.publish.ignored', { count: uploadPreview.ignored.length })}{uploadPreview.has_ignore_file ? '' : ` · ${t('developer.publish.noIgnoreFile')}`}</span>
                        <button type="button" onClick={() => setShowUploadFiles((v) => !v)} className="underline text-slate-400 hover:text-slate-200 cursor-pointer">{showUploadFiles ? t('developer.publish.hideFiles') : t('developer.publish.showFiles')}</button>
                      </div>
                      {uploadPreview.problems.map((p, i) => <div key={i} className="text-amber-300">{p}</div>)}
                      {showUploadFiles && (
                        <div className="max-h-64 overflow-y-auto font-mono text-[10.5px] flex flex-col gap-0.5">
                          {uploadPreview.files.slice(0, 400).map((f) => (
                            <div key={f.path} className={fileStateClass[f.state] ?? ''}>{f.state === 'new' ? '+ ' : f.state === 'changed' ? '~ ' : f.state === 'removed' ? '- ' : '  '}{f.path}{f.state !== 'removed' ? `  ${formatBytes(f.bytes)}` : ''}</div>
                          ))}
                          {uploadPreview.files.length > 400 && <div className="text-slate-500">…</div>}
                          {uploadPreview.ignored.map((p) => <div key={`i${p}`} className="text-slate-600">× {p}</div>)}
                        </div>
                      )}
                    </div>
                  )}
                  <div className="flex flex-wrap items-center gap-2">
                    <input type="text" value={form.note} onChange={(e) => editForm({ note: e.target.value })} placeholder={t('developer.publish.notePlaceholder')} className="flex-1 min-w-[240px] bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500" />
                    <button
                      type="button"
                      disabled={uploadBusy !== '' || !!notYours || !form.title.trim() || (uploadPreview?.problems.some((p) => p.startsWith('No Contents') || p.startsWith('No mod')) ?? false)}
                      onClick={() => void publishUpload()}
                      className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer disabled:opacity-50"
                      title={t('developer.publish.publishTitle')}
                    >
                      {uploadBusy === 'publish' ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <UploadCloud className="w-3.5 h-3.5" aria-hidden="true" />}
                      {upload.id ? t('developer.publish.update') : t('developer.publish.publish')}
                    </button>
                  </div>
                  {uploadProgress && (
                    <div className="flex flex-col gap-1 text-[11px] text-slate-300">
                      <span>{t(`developer.publish.stage.${uploadProgress.stage}`, { defaultValue: uploadProgress.stage })}{uploadProgress.total > 0 ? ` · ${formatBytes(uploadProgress.processed)} / ${formatBytes(uploadProgress.total)}` : ''}</span>
                      <div className="h-1.5 rounded bg-slate-800 overflow-hidden">
                        <div className={`h-full bg-sky-500 ${uploadProgress.total > 0 ? '' : 'animate-pulse w-1/3'}`} style={uploadProgress.total > 0 ? { width: `${Math.min(100, Math.round((uploadProgress.processed / uploadProgress.total) * 100))}%` } : undefined} />
                      </div>
                    </div>
                  )}
                  {uploadError && <div className="text-[11px] rounded-md border border-red-700/60 bg-red-950/40 text-red-200 px-3 py-2">{uploadError}</div>}
                  {uploadOutcome && (
                    <div className="text-[11px] rounded-md border border-emerald-700/60 bg-emerald-950/40 text-emerald-200 px-3 py-2 flex flex-col gap-1">
                      <span>{uploadOutcome.created ? t('developer.publish.doneNew', { id: uploadOutcome.id, files: uploadOutcome.files, size: formatBytes(uploadOutcome.bytes) }) : t('developer.publish.doneUpdate', { id: uploadOutcome.id, files: uploadOutcome.files, size: formatBytes(uploadOutcome.bytes) })}</span>
                      {uploadOutcome.needs_legal_agreement && (
                        <span className="text-amber-200">
                          {t('developer.publish.legal')}{' '}
                          <button type="button" onClick={() => void TauriService.openExternalUrl('https://steamcommunity.com/sharedfiles/workshoplegalagreement')} className="underline cursor-pointer">{t('developer.publish.legalLink')}</button>
                        </span>
                      )}
                      <button type="button" onClick={() => void TauriService.openExternalUrl(uploadOutcome.url)} className="self-start underline cursor-pointer">{t('developer.publish.openPage')}</button>
                    </div>
                  )}
                </>
              )}
              {!upload && uploadError && <div className="text-[11px] text-amber-300">{uploadError}</div>}
            </>
          )}

          {tab === 'maps' &&
            [
              { key: 'tilezed', name: 'TileZed', what: t('developer.mapTools.tilezedWhat'), dir: tilezedDir, setDir: setTilezed, exe: mapTools?.tilezed ?? null },
              { key: 'worlded', name: 'WorldEd', what: t('developer.mapTools.worldedWhat'), dir: worldedDir, setDir: setWorlded, exe: mapTools?.worlded ?? null },
            ].map((tool) => (
              <div key={tool.key} className="rounded-lg border border-slate-800 bg-slate-950/60 p-3 flex flex-col gap-2">
                <div className="flex items-start justify-between gap-3">
                  <div className="space-y-0.5">
                    <div className="text-xs font-bold text-slate-200">{tool.name}</div>
                    <p className="text-[11px] text-slate-400 max-w-2xl">{tool.what}</p>
                  </div>
                  <button
                    type="button"
                    disabled={!tool.exe}
                    onClick={() => tool.exe && void TauriService.launchMapTool(tool.exe).catch(() => undefined)}
                    title={tool.exe ?? t('developer.mapTools.notFoundTitle', { name: tool.name })}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-sky-700 hover:bg-sky-600 text-white cursor-pointer disabled:opacity-50 shrink-0"
                  >
                    <Play className="w-3.5 h-3.5" aria-hidden="true" /> {t('developer.mapTools.start', { name: tool.name })}
                  </button>
                </div>
                <input
                  type="text"
                  value={tool.dir}
                  onChange={(e) => tool.setDir(e.target.value)}
                  placeholder={t('developer.mapTools.folderPlaceholder', { name: tool.name })}
                  spellCheck={false}
                  className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-emerald-500"
                />
                <div className={`text-[10px] font-mono break-all ${tool.exe ? 'text-emerald-400/80' : 'text-slate-500'}`}>
                  {tool.exe ?? (mapTools ? t('developer.mapTools.notFound', { count: mapTools.searched.length }) : t('developer.mapTools.looking'))}
                </div>
              </div>
            ))}

          {tab === 'new' && (
            <>
              <form
                className="grid grid-cols-1 md:grid-cols-2 gap-2"
                onSubmit={(e) => {
                  e.preventDefault();
                  void createMod();
                }}
              >
                <label className="flex flex-col gap-1 text-[11px] text-slate-400">
                  {t('developer.newMod.name')}
                  <input type="text" value={newMod.name} autoFocus
                    onChange={(e) => setNewMod({ ...newMod, name: e.target.value, id: newMod.id === idFromName(newMod.name) ? idFromName(e.target.value) : newMod.id })}
                    className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500" />
                </label>
                <label className="flex flex-col gap-1 text-[11px] text-slate-400">
                  {t('developer.newMod.id')}
                  <input type="text" value={newMod.id} onChange={(e) => setNewMod({ ...newMod, id: e.target.value })} spellCheck={false}
                    className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 font-mono focus:outline-none focus:border-emerald-500" />
                </label>
                <label className="flex flex-col gap-1 text-[11px] text-slate-400">
                  {t('developer.newMod.author')}
                  <input type="text" value={newMod.author} onChange={(e) => setNewMod({ ...newMod, author: e.target.value })}
                    className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500" />
                </label>
                <label className="flex flex-col gap-1 text-[11px] text-slate-400">
                  {t('developer.newMod.description')}
                  <textarea rows={2} value={newMod.description} onChange={(e) => setNewMod({ ...newMod, description: e.target.value })}
                    className="bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500" />
                </label>
                <label className="flex items-center gap-2 text-[11px] text-slate-300 md:col-span-2">
                  <input type="checkbox" checked={newMod.also41} onChange={(e) => setNewMod({ ...newMod, also41: e.target.checked })} />
                  {t('developer.newMod.also41')}
                </label>
                <div className="flex gap-2 md:col-span-2">
                  <button type="submit" disabled={creating || !newMod.name.trim() || !newMod.id.trim()}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer disabled:opacity-50">
                    {creating ? <Loader2 className="w-3.5 h-3.5 animate-spin" aria-hidden="true" /> : <FolderPlus className="w-3.5 h-3.5" aria-hidden="true" />} {t('developer.newMod.make')}
                  </button>
                </div>
              </form>
              {created && (
                <div className="rounded-lg border border-emerald-800/60 bg-emerald-950/30 px-3 py-2 text-[11px] text-emerald-200 flex flex-col gap-1">
                  <div className="flex items-center justify-between gap-2">
                    <span dangerouslySetInnerHTML={{ __html: tHtml('developer.newMod.made', { id: created.id, folder: created.folder }) }} />
                    <button type="button" onClick={() => TauriService.openModFolder(paths, created.folder)} className="flex items-center gap-1 px-2 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 cursor-pointer shrink-0">
                      <FolderOpen className="w-3 h-3" aria-hidden="true" /> {t('developer.newMod.openFolder')}
                    </button>
                  </div>
                  <div className="font-mono text-[10px] text-emerald-300/80">{created.files.join(' · ')}</div>
                  <div className="text-emerald-300/80">{t('developer.newMod.next')}</div>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {note && <div className="text-[11px] text-emerald-300">{note}</div>}
    </div>
  );
};
