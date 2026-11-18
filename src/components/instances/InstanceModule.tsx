import { freshProfile, profileModCounts } from '../../utils/profiles';
import { ChangePreview } from '../common/ChangePreview';
import React, { useState, useEffect } from 'react';
import { Layers, ListOrdered, Plus, Check, Trash2, Zap, FolderArchive, RefreshCw, Edit3, Share2, X, Save, Loader2, Unplug } from 'lucide-react';
import { ModInfo, StudioPathsUI, AppInstance, STUDIO_LINK_MOD_ID } from '../../types';
import { TauriService, explainError } from '../../services/tauri';
import { useTranslation } from 'react-i18next';
import { tHtml } from '../../i18n/html';
import { useChangeRevision } from '../../services/changes';
import { planProfileCleanup, ProfileCleanupPlan } from '../../utils/profileCleanup';
import { useUiMode } from '../../hooks/useUiMode';

interface InstanceModuleProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  onApplyInstanceLoadOrder: (loadOrder: string[], activeIds: string[]) => void;
  onProfileActivated?: (inst: AppInstance) => void;
  /** The active profile was deleted, so nothing is active any more. */
  onProfileDeleted?: () => void;
  /** Takes the user to Browse Mods, for a profile that starts with nothing in it. */
  onGoToBrowse?: () => void;
  /** Opens the Mod List (the active profile's mods). */
  onGoToModList?: () => void;
  /** A profile was renamed or edited: the header and the other screens show it as it is now. */
  onProfilesChanged?: () => void;
  /** Mods were unsubscribed with a deleted profile: the list is scanned again. */
  onRefreshMods?: () => void;
}

/** The delete dialog: which Workshop items of the profile could go with it, and which are ticked. */
interface PendingDelete {
  inst: AppInstance;
  plan: ProfileCleanupPlan;
  ticked: Set<string>;
  steamReady: boolean;
  busy: boolean;
}

/** What a new profile starts with. */
type NewProfileMods = 'current' | 'all' | 'none' | 'pick';

export const InstanceModule: React.FC<InstanceModuleProps> = ({
  paths,
  mods,
  onApplyInstanceLoadOrder,
  onProfileActivated,
  onProfileDeleted,
  onGoToBrowse,
  onGoToModList,
  onProfilesChanged,
  onRefreshMods,
}) => {
  const { t } = useTranslation();
  /** Simple: the profile's own saves (isolating) stay in Complete; the launch file said in plain words. */
  const simple = useUiMode()[0] === 'simple';
  const changeRev = useChangeRevision();
  const [instances, setInstances] = useState<AppInstance[]>([]);
  /** The mods on disk now, by id: a profile's mods that are not among them are counted on its card. */
  const installedIds = React.useMemo(() => new Set(mods.filter((m) => !m.not_installed).map((m) => m.mod_id)), [mods]);
  /** The same mods with the Workshop item each comes from, for the card's counts. */
  const installedWorkshop = React.useMemo(
    () => new Map(mods.filter((m) => !m.not_installed).map((m) => [m.mod_id, m.workshop_id || null] as [string, string | null])),
    [mods]
  );
  const [newInstanceName, setNewInstanceName] = useState('');
  /** The "new profile" form, folded away until asked for (open when there is no profile yet). */
  const [showCreate, setShowCreate] = useState(false);
  /** Where the new-profile window is: 1 name, 2 which mods, 3 check and create. */
  const [createStep, setCreateStep] = useState(1);
  const [profileCount, setProfileCount] = useState<number | null>(null);
  /**
   * A profile is a mod list, so making one asks which mods. The old form took whatever happened
   * to be switched on, which is right often enough to stay the default and wrong whenever the
   * point of the new profile is to be different from the one you are in.
   */
  const [newProfileMods, setNewProfileMods] = useState<NewProfileMods>('current');
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [pickSearch, setPickSearch] = useState('');
  const [newInstanceDesc, setNewInstanceDesc] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  /** The list came back once: until then "no profiles yet" would be a guess. */
  const [loadedOnce, setLoadedOnce] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  // Edit Modal State
  const [editingInstance, setEditingInstance] = useState<AppInstance | null>(null);
  /** The build the profile is for, as edited: '' = as last played. */
  const [editBuild, setEditBuild] = useState<string>('');
  const [editName, setEditName] = useState('');
  const [editDesc, setEditDesc] = useState('');
  const [pendingDelete, setPendingDelete] = useState<PendingDelete | null>(null);

  const activeMods = mods.filter((m) => m.enabled);
  /** What the player has on, as the Mod List counts it: the Studio Link is never a row there. */
  const shownOn = activeMods.filter((m) => m.mod_id !== STUDIO_LINK_MOD_ID).length;
  /** Names and sizes of what isolated profiles share because it is too big to copy. */
  const [leftShared, setLeftShared] = useState<string[]>([]);

  const loadInstances = async () => {
    setIsLoading(true);
    try {
      const list: AppInstance[] = await TauriService.listInstances(paths.user_zomboid_dir);
      setInstances(list);
      // What isolating left shared because it was too big to copy: said beside each isolated one.
      if (list.some((i) => i.own_saves)) {
        const left = await TauriService.profileLeftShared(paths).catch(() => []);
        setLeftShared(left.map((l) => `${l.rel.split(/[\\/]/).pop() || l.rel} (${Math.max(1, Math.round(l.bytes / (1024 * 1024)))} MB)`));
      } else {
        setLeftShared([]);
      }
    } catch (err) {
      console.error('Error loading profiles:', err);
    } finally {
      setIsLoading(false);
      setLoadedOnce(true);
    }
  };

  useEffect(() => {
    loadInstances();
  }, [paths.user_zomboid_dir, changeRev]);

  const handleCreateInstance = async () => {
    if (!newInstanceName.trim()) {
      alert(t('profiles.needName'));
      return;
    }

    // Creating a profile writes the mod list the game reads. With nothing scanned — the game
    // folder is not set yet, or the scan failed — that would switch every mod off and say it
    // worked. Better to send the user to Settings.
    if (mods.length === 0) {
      alert(t('profiles.noModsYet'));
      return;
    }

    const chosen: string[] =
      newProfileMods === 'all'
        ? mods.map((m) => m.mod_id)
        : newProfileMods === 'none'
        ? []
        : newProfileMods === 'pick'
        ? mods.filter((m) => picked.has(m.mod_id)).map((m) => m.mod_id)
        : activeMods.map((m) => m.mod_id);

    if (newProfileMods === 'pick' && chosen.length === 0) {
      alert(t('profiles.nothingTicked'));
      return;
    }
    if (newProfileMods === 'none' && !confirm(t('profiles.confirmEmpty'))) {
      return;
    }

    try {
      const activeModIds = chosen;
      // The order the list already has, so a profile does not reshuffle what you sorted.
      const loadOrder = mods.map((m) => m.mod_id);

      const created: AppInstance = await TauriService.createInstance(
        paths.user_zomboid_dir || '',
        newInstanceName.trim(),
        newInstanceDesc.trim() || undefined,
        activeModIds,
        loadOrder
      );

      setNewInstanceName('');
      setNewInstanceDesc('');
      setPicked(new Set());
      setPickSearch('');
      await loadInstances();

      // Activate it so the rest of the app unlocks, and only then say so: when the launch file
      // changes too, its window asks first, and the message waits for the answer.
      await handleActivateInstance(created, true);
    } catch (err: any) {
      alert(t('profiles.createError', { error: explainError(err) }));
    }
  };

  const handleOpenEdit = (inst: AppInstance) => {
    setEditingInstance(inst);
    setEditName(inst.name);
    setEditDesc(inst.description || '');
    setEditBuild(inst.target_build ?? '');
  };

  const handleSaveEdit = async () => {
    if (!editingInstance || !editName.trim()) return;

    try {
      const current = await freshProfile(paths.user_zomboid_dir, editingInstance);
      const updated: AppInstance = {
        ...current,
        name: editName.trim(),
        description: editDesc.trim() || undefined,
        target_build: editBuild || null,
      };

      await TauriService.updateInstance(paths.user_zomboid_dir, updated);
      setStatusMessage(t('profiles.updated', { name: updated.name }));
      setEditingInstance(null);
      await loadInstances();
      onProfilesChanged?.();
    } catch (err: any) {
      alert(t('profiles.updateError', { error: explainError(err) }));
    }
  };

  /** A profile whose switch also changes the game's launch file, waiting to be confirmed. */
  const [pendingActivation, setPendingActivation] = useState<{ inst: AppInstance; plan: NonNullable<Awaited<ReturnType<typeof TauriService.javaAgentsPlan>>>; justCreated?: boolean } | null>(null);

  // Isolate this profile: the shared worlds to bring along, chosen before they move (its options
  // and mod settings start as a copy).
  const [ownSaves, setOwnSaves] = useState<{ inst: AppInstance; worlds: string[]; bring: string[] } | null>(null);
  const [ownBusy, setOwnBusy] = useState(false);
  const openOwnSaves = async (inst: AppInstance) => {
    const worlds = await TauriService.profileSaves(paths, 'shared').catch(() => [] as string[]);
    setOwnSaves({ inst, worlds, bring: [] });
  };
  const applyOwnSaves = async (inst: AppInstance, own: boolean, bring: string[]) => {
    setOwnBusy(true);
    try {
      await TauriService.setOwnSaves(paths.user_zomboid_dir, inst.id, own, bring);
      setStatusMessage(t(own ? 'profiles.ownSaves.done' : 'profiles.ownSaves.backDone', { name: inst.name }));
      setOwnSaves(null);
      await loadInstances();
    } catch (err) {
      setStatusMessage(explainError(err));
    } finally {
      setOwnBusy(false);
    }
  };

  /** `justCreated`: the profile was made a moment ago, and the message says so once it is on. */
  const handleActivateInstance = async (inst: AppInstance, justCreated = false) => {
    // The launch file is the game's, outside the mods folder: its change is shown first.
    const plan = await TauriService.javaAgentsPlan(paths, inst.active_mod_ids);
    if (plan && (plan.removed.length > 0 || plan.restored.length > 0)) {
      setPendingActivation({ inst, plan, justCreated });
      return;
    }
    await activateNow(inst, justCreated);
  };

  const activateNow = async (inst: AppInstance, justCreated = false) => {
    setPendingActivation(null);
    try {
      await TauriService.activateInstance(paths.user_zomboid_dir, inst.id);
      onApplyInstanceLoadOrder(inst.load_order, inst.active_mod_ids);
      if (onProfileActivated) {
        onProfileActivated(inst);
      }
      setStatusMessage(t(justCreated ? 'profiles.created' : 'profiles.activated', { name: inst.name }));
      await loadInstances();
    } catch (err: any) {
      alert(t('profiles.activateError', { error: explainError(err) }));
    }
  };

  /**
   * A profile is a named mod list, which is exactly what a .pzpack holds, so sharing one reuses
   * that format: the other player gets the mod ids with their Workshop ids and can subscribe.
   */
  const handleShareInstance = async (inst: AppInstance) => {
    const byId = new Map(mods.map((m) => [m.mod_id, m]));
    const active = new Set(inst.active_mod_ids);
    const preset = {
      id: `profile_${inst.id}`,
      name: inst.name,
      description: inst.description || t('profiles.sharedDescription'),
      author: undefined,
      created_at: new Date().toISOString(),
      mods: inst.load_order
        .filter((id) => active.has(id))
        .map((id) => ({
          mod_id: id,
          name: byId.get(id)?.name ?? id,
          workshop_id: byId.get(id)?.workshop_id,
          enabled: true,
        })),
      load_order: inst.load_order.filter((id) => active.has(id)),
    };
    if (preset.mods.length === 0) {
      alert(t('profiles.nothingToShare'));
      return;
    }
    try {
      const fileName = `${inst.name.toLowerCase().replace(/[^a-z0-9]/g, '_') || 'profile'}.pzpack`;
      const filePath = await TauriService.pickSaveFile(fileName, t('profiles.pzpackFilter'), 'pzpack');
      if (!filePath) return;
      await TauriService.exportPresetFile(paths, preset as never, filePath);
      setStatusMessage(t('profiles.savedTo', { name: inst.name, path: filePath }));
    } catch (err) {
      alert(t('profiles.saveError', { error: explainError(err) }));
    }
  };

  /**
   * Deleting a profile: when it is the only profile using some Workshop items, the dialog offers
   * to unsubscribe from those too, and never from an item another profile shares (Steam
   * subscribes to items, so one shared mod keeps its whole item). A local folder is never touched.
   */
  const handleDeleteInstance = async (inst: AppInstance) => {
    const plan = planProfileCleanup(inst, instances, mods);
    if (plan.exclusive.length === 0) {
      if (!confirm(t('profiles.confirmDelete', { name: inst.name }))) return;
      await deleteNow(inst, []);
      return;
    }
    setPendingDelete({ inst, plan, ticked: new Set(plan.exclusive.map((i) => i.workshop_id)), steamReady: false, busy: false });
    const steamReady = await TauriService.steamClientReady(paths);
    setPendingDelete((cur) => (cur && cur.inst.id === inst.id ? { ...cur, steamReady } : cur));
  };

  /** Deletes the profile, then unsubscribes from the ticked Workshop items (none to only delete). */
  const deleteNow = async (inst: AppInstance, unsubscribeItems: string[]) => {
    // A patch package is made for the mods of the profile it was built in: its merges are about
    // those versions of those files. Left behind, it goes on loading last for every other
    // profile, which is how a merge for mods you no longer use ends up in a list that never
    // asked for it.
    const itsPackages = inst.active_mod_ids.filter(
      (id) => id.startsWith('Z_PZModStudio_') && id !== 'Z_PZModStudio_Bridge'
    );
    let alsoPackages = false;
    if (itsPackages.length > 0) {
      alsoPackages = confirm(t('profiles.confirmDeletePackages', { count: itsPackages.length, list: itsPackages.join(', ') }));
    }
    setPendingDelete((cur) => (cur ? { ...cur, busy: true } : cur));
    try {
      await TauriService.deleteInstance(paths.user_zomboid_dir, inst.id);
      if (alsoPackages) {
        for (const folder of itsPackages) {
          try {
            await TauriService.deleteMergedPackage(paths.user_zomboid_dir, paths.mod_list_ini_path, folder);
          } catch (err) {
            alert(t('profiles.packageDeleteError', { folder, error: explainError(err) }));
          }
        }
      }
      let unsubscribed = 0;
      let unsubscribeFailed: string | null = null;
      if (unsubscribeItems.length > 0) {
        try {
          const outcome = await TauriService.workshopSubscribe(paths, unsubscribeItems, false);
          unsubscribed = outcome.done.length;
          if (outcome.failed.length > 0) unsubscribeFailed = outcome.failed.map(([id, e]) => `${id}: ${e}`).join('; ');
          // The game's list still named them; left there, Mod Health would report every one
          // as switched on but not installed.
          if (inst.is_active && outcome.done.length > 0) {
            const gone = new Set(outcome.done);
            const stillOn = mods.filter((m) => m.enabled && !(m.workshop_id && gone.has(m.workshop_id))).map((m) => m.mod_id);
            await TauriService.writeModListIni(paths.mod_list_ini_path, stillOn);
          }
        } catch (err) {
          unsubscribeFailed = explainError(err);
        }
      }
      setStatusMessage(
        unsubscribeFailed
          ? t('profiles.deletedUnsubscribeFailed', { name: inst.name, done: unsubscribed, error: unsubscribeFailed })
          : unsubscribed > 0
          ? t('profiles.deletedUnsubscribed', { name: inst.name, count: unsubscribed })
          : alsoPackages
          ? t('profiles.deletedWithPackages', { name: inst.name, count: itsPackages.length })
          : t('profiles.deleted', { name: inst.name })
      );
      setPendingDelete(null);
      await loadInstances();
      if (unsubscribeItems.length > 0) onRefreshMods?.();
      // The header and the sidebar were still showing it as active until a restart.
      if (inst.is_active) onProfileDeleted?.();
    } catch (err: any) {
      setPendingDelete((cur) => (cur ? { ...cur, busy: false } : cur));
      alert(t('profiles.deleteError', { error: explainError(err) }));
    }
  };

  const toggleTicked = (workshopId: string) => {
    setPendingDelete((cur) => {
      if (!cur) return cur;
      const ticked = new Set(cur.ticked);
      if (ticked.has(workshopId)) ticked.delete(workshopId);
      else ticked.add(workshopId);
      return { ...cur, ticked };
    });
  };

  const formOpen = showCreate;
  const openCreate = () => {
    setCreateStep(1);
    setShowCreate(true);
  };
  const closeCreate = () => setShowCreate(false);
  /** How many mods the new profile starts with switched on, for the last step. */
  const chosenCount =
    newProfileMods === 'all' ? mods.length : newProfileMods === 'none' ? 0 : newProfileMods === 'pick' ? picked.size : activeMods.length;
  useEffect(() => {
    if (!formOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeCreate();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [formOpen]);
  // A profile was just made: the form folds away (the cards show it, with the status line).
  useEffect(() => {
    if (profileCount !== null && instances.length > profileCount) setShowCreate(false);
    setProfileCount(instances.length);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [instances.length]);

  return (
    <div className="space-y-6">
      {pendingActivation && (
        <ChangePreview
          title={t('profiles.launchFile.title', { name: pendingActivation.inst.name })}
          blurb={pendingActivation.plan.gameRunning
            ? t(simple ? 'simple.profiles.launchFileLater' : 'profiles.launchFile.blurbLater', { file: pendingActivation.plan.file })
            : t(simple ? 'simple.profiles.launchFile' : 'profiles.launchFile.blurb', { file: pendingActivation.plan.file })}
          sections={[
            // Simple names the mod, not the line of the launch file (-agentlib:...).
            { label: t(simple ? 'simple.profiles.launchOut' : 'profiles.launchFile.out'), tone: 'off', rows: pendingActivation.plan.removed.map((m, k) => ({ name: simple ? m : pendingActivation.plan.removed_args[k] ?? m, note: t(simple ? 'simple.profiles.outNote' : 'profiles.launchFile.outNote', { mod: m }) })) },
            { label: t(simple ? 'simple.profiles.launchBack' : 'profiles.launchFile.back'), tone: 'on', rows: pendingActivation.plan.restored.map((m, k) => ({ name: simple ? m : pendingActivation.plan.restored_args[k] ?? m, note: t(simple ? 'simple.profiles.backNote' : 'profiles.launchFile.backNote', { mod: m }) })) },
          ]}
          primary={{ label: t('profiles.launchFile.activate', { name: pendingActivation.inst.name }), onClick: () => void activateNow(pendingActivation.inst, pendingActivation.justCreated) }}
          onClose={() => {
            // A new profile whose switch was not confirmed is made but not on: say that, not "activated".
            if (pendingActivation.justCreated) setStatusMessage(t('profiles.createdNotActive', { name: pendingActivation.inst.name }));
            setPendingActivation(null);
          }}
        />
      )}
      {/* Header: the screen's name, how many profiles, and the way to make a new one */}
      <div className="rounded-xl border border-slate-800 bg-slate-900/60 px-4 py-2.5 flex flex-wrap items-center gap-x-3 gap-y-2">
        <Layers className="w-5 h-5 text-emerald-400 shrink-0" aria-hidden="true" />
        <h1 className="text-base font-bold text-slate-100">{t('profiles.title')}</h1>
        <span className="text-[11px] font-mono px-2 py-0.5 rounded border border-slate-700 text-slate-300">{t('profiles.saved', { count: instances.length })}</span>
        {isLoading && <RefreshCw className="w-4 h-4 text-emerald-400 animate-spin" aria-hidden="true" />}
        <span className="flex-1 min-w-[240px] text-[11px] text-slate-400" dangerouslySetInnerHTML={{ __html: tHtml('profiles.blurb') }} />
        <button
          type="button"
          onClick={openCreate}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold cursor-pointer transition bg-emerald-600 hover:bg-emerald-500 text-white"
        >
          <Plus className="w-3.5 h-3.5" aria-hidden="true" />
          {t('profiles.newProfile')}
        </button>
      </div>

      {statusMessage && (
        <div className="p-3 bg-emerald-950/80 border border-emerald-700/80 rounded-xl text-xs text-emerald-300 font-bold flex items-center justify-between">
          <span>{statusMessage}</span>
          <button onClick={() => setStatusMessage(null)} className="text-emerald-400 hover:underline">
            {t('profiles.close')}
          </button>
        </div>
      )}

      {/* The new-profile form, when open; the profiles below, full width */}
      <div className="flex flex-col gap-6">
        {formOpen && (
          <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-6" onClick={closeCreate}>
            <div
              role="dialog"
              aria-modal="true"
              aria-label={t('profiles.createTitle')}
              onClick={(e) => e.stopPropagation()}
              className="w-full max-w-xl max-h-full flex flex-col rounded-2xl border border-slate-700 bg-slate-900 shadow-2xl overflow-hidden"
            >
              <div className="flex items-start justify-between gap-3 px-5 pt-4 pb-3 border-b border-slate-800">
                <div className="space-y-1">
                  <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
                    <Plus className="w-4 h-4 text-emerald-400" aria-hidden="true" />
                    {t('profiles.createTitle')}
                  </h3>
                  <p className="text-[11.5px] text-slate-400">{t('profiles.createBlurb')}</p>
                </div>
                <button type="button" onClick={closeCreate} aria-label={t('profiles.close')} className="p-1 rounded text-slate-500 hover:text-slate-200 hover:bg-slate-800 cursor-pointer">
                  <X className="w-4 h-4" aria-hidden="true" />
                </button>
              </div>

              {/* The three steps, the one in front lit */}
              <ol className="flex items-center gap-2 px-5 py-3 border-b border-slate-800 text-[11px]">
                {[1, 2, 3].map((n) => (
                  <li key={n} className="flex items-center gap-2">
                    <span
                      className={`w-5 h-5 rounded-full flex items-center justify-center font-bold ${
                        createStep === n ? 'bg-emerald-600 text-white' : createStep > n ? 'bg-emerald-900 text-emerald-300' : 'bg-slate-800 text-slate-500'
                      }`}
                    >
                      {createStep > n ? <Check className="w-3 h-3" aria-hidden="true" /> : n}
                    </span>
                    <span className={createStep === n ? 'text-slate-100 font-bold' : 'text-slate-500'}>{t(`profiles.wizard.step${n}`)}</span>
                    {n < 3 && <span className="w-6 h-px bg-slate-700" aria-hidden="true" />}
                  </li>
                ))}
              </ol>

              <div className="flex-1 min-h-0 overflow-y-auto px-5 py-4 space-y-3">
                <p className="text-[11.5px] text-sky-200/90 bg-sky-950/30 border border-sky-900/50 rounded-lg px-3 py-2 leading-relaxed">{t(`profiles.wizard.hint${createStep}`)}</p>
                {createStep === 1 && (
                  <>
            <div>
              <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">
                {t('profiles.name')}
              </label>
              <input
                type="text"
                placeholder={t('profiles.namePlaceholder')}
                value={newInstanceName}
                onChange={(e) => setNewInstanceName(e.target.value)}
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && newInstanceName.trim()) setCreateStep(2);
                }}
                className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2 text-xs text-slate-200 focus:outline-none focus:border-emerald-500 font-medium"
              />
            </div>

            <div>
              <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">
                {t('profiles.descriptionOptional')}
              </label>
              <textarea
                rows={3}
                placeholder={t('profiles.descriptionPlaceholder')}
                value={newInstanceDesc}
                onChange={(e) => setNewInstanceDesc(e.target.value)}
                className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2 text-xs text-slate-200 focus:outline-none focus:border-emerald-500 font-sans"
              />
            </div>

                  </>
                )}
                {createStep === 2 && (
            <div>
              <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">
                {t('profiles.whichMods')}
              </label>
              <div className="grid grid-cols-2 gap-1.5">
                {([
                  { id: 'current', label: t('profiles.whatIsOn'), count: shownOn },
                  // Simple: switching on everything installed is a choice of its own, below.
                  ...(simple ? [] : [{ id: 'all', label: t('profiles.everything'), count: mods.length }]),
                  { id: 'pick', label: t('profiles.pickThem'), count: picked.size },
                  { id: 'none', label: t('profiles.nothingYet'), count: 0 },
                ] as { id: NewProfileMods; label: string; count: number }[]).map((option) => (
                  <button
                    key={option.id}
                    type="button"
                    onClick={() => setNewProfileMods(option.id)}
                    aria-pressed={newProfileMods === option.id}
                    className={`px-2 py-1.5 rounded-lg text-[11px] font-bold border transition cursor-pointer text-left ${
                      newProfileMods === option.id
                        ? 'bg-emerald-950/70 text-emerald-200 border-emerald-700'
                        : 'bg-slate-950 text-slate-300 border-slate-800 hover:border-slate-700'
                    }`}
                  >
                    {option.label}
                    <span className="block text-[10px] font-mono opacity-70">
                      {option.id === 'none' ? t('profiles.startEmpty') : t('profiles.modsCount', { count: option.count })}
                    </span>
                  </button>
                ))}
              </div>

              {/* Simple: everything installed is offered apart, with what it means (hundreds of mods on at once). */}
              {simple && (
                <div className={`mt-2 rounded-lg border px-2.5 py-2 text-[11px] leading-relaxed ${newProfileMods === 'all' ? 'border-amber-800/70 bg-amber-950/30 text-amber-200' : 'border-slate-800 bg-slate-950/60 text-slate-400'}`}>
                  {newProfileMods === 'all' ? (
                    <>
                      {t('simple.profiles.everythingChosen', { count: mods.length })}{' '}
                      <button type="button" onClick={() => setNewProfileMods('current')} className="underline text-slate-200 hover:text-white cursor-pointer">
                        {t('simple.profiles.everythingUndo')}
                      </button>
                    </>
                  ) : (
                    <>
                      {t('simple.profiles.everythingHint', { count: mods.length })}{' '}
                      <button type="button" onClick={() => setNewProfileMods('all')} className="underline text-slate-300 hover:text-white cursor-pointer">
                        {t('simple.profiles.everythingButton', { count: mods.length })}
                      </button>
                    </>
                  )}
                </div>
              )}

              {newProfileMods === 'pick' && (
                <div className="mt-2 rounded-xl border border-slate-800 bg-slate-950 overflow-hidden">
                  <div className="flex items-center gap-2 p-2 border-b border-slate-800">
                    <input
                      type="text"
                      value={pickSearch}
                      onChange={(e) => setPickSearch(e.target.value)}
                      placeholder={t('profiles.searchMods')}
                      className="flex-1 bg-slate-900 border border-slate-800 rounded-lg px-2 py-1 text-[11px] text-slate-200 focus:outline-none focus:border-emerald-500"
                    />
                    <button
                      type="button"
                      onClick={() => setPicked(new Set(mods.map((m) => m.mod_id)))}
                      className="text-[10px] px-1.5 py-1 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer"
                    >
                      {t('profiles.all')}
                    </button>
                    <button
                      type="button"
                      onClick={() => setPicked(new Set())}
                      className="text-[10px] px-1.5 py-1 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer"
                    >
                      {t('profiles.none')}
                    </button>
                  </div>
                  <div className="max-h-56 overflow-y-auto divide-y divide-slate-900">
                    {mods
                      .filter((m) => {
                        const q = pickSearch.trim().toLowerCase();
                        return !q || m.name.toLowerCase().includes(q) || m.mod_id.toLowerCase().includes(q);
                      })
                      .map((m) => (
                        <label
                          key={m.mod_id}
                          className="flex items-center gap-2 px-2 py-1.5 text-[11px] text-slate-300 hover:bg-slate-900/80 cursor-pointer"
                        >
                          <input
                            type="checkbox"
                            checked={picked.has(m.mod_id)}
                            onChange={(e) => {
                              const next = new Set(picked);
                              if (e.target.checked) next.add(m.mod_id);
                              else next.delete(m.mod_id);
                              setPicked(next);
                            }}
                            className="accent-emerald-500"
                          />
                          <span className="truncate">{m.name}</span>
                        </label>
                      ))}
                  </div>
                </div>
              )}

              {newProfileMods === 'none' && onGoToBrowse && (
                <button
                  type="button"
                  onClick={onGoToBrowse}
                  className="mt-2 w-full py-1.5 rounded-lg text-[11px] font-bold bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 cursor-pointer"
                  title={t('profiles.findInBrowseTitle')}
                >
                  {t('profiles.findInBrowse')}
                </button>
              )}
            </div>

                )}
                {createStep === 3 && (
                  <dl className="grid grid-cols-[140px_1fr] gap-x-3 gap-y-2 text-xs">
                    <dt className="text-slate-500">{t('profiles.name')}</dt>
                    <dd className="text-slate-100 font-bold break-words">{newInstanceName.trim()}</dd>
                    {newInstanceDesc.trim() && (
                      <>
                        <dt className="text-slate-500">{t('profiles.descriptionOptional')}</dt>
                        <dd className="text-slate-300 break-words">{newInstanceDesc.trim()}</dd>
                      </>
                    )}
                    <dt className="text-slate-500">{t('profiles.whichMods')}</dt>
                    <dd className="text-slate-200">{t('profiles.wizard.modsOn', { count: chosenCount, total: mods.length })}</dd>
                  </dl>
                )}
              </div>

              <div className="flex items-center gap-2 px-5 py-3 border-t border-slate-800 bg-slate-950/40">
                {createStep > 1 && (
                  <button type="button" onClick={() => setCreateStep(createStep - 1)} className="px-3 py-2 rounded-lg text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 cursor-pointer">
                    {t('profiles.wizard.back')}
                  </button>
                )}
                <div className="flex-1" />
                {createStep < 3 ? (
                  <button
                    type="button"
                    onClick={() => setCreateStep(createStep + 1)}
                    disabled={(createStep === 1 && !newInstanceName.trim()) || (createStep === 2 && newProfileMods === 'pick' && picked.size === 0)}
                    className="px-4 py-2 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white cursor-pointer"
                  >
                    {t('profiles.wizard.next')}
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={handleCreateInstance}
                    disabled={!newInstanceName.trim()}
                    className="flex items-center gap-1.5 px-4 py-2 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white cursor-pointer"
                  >
                    <Plus className="w-3.5 h-3.5" aria-hidden="true" />
                    {t('profiles.wizard.create')}
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

        <div className="space-y-4">
          {instances.length === 0 && !loadedOnce ? (
            <div className="h-64 bg-slate-900/80 border border-slate-800 rounded-2xl flex items-center justify-center gap-2 text-xs text-slate-400" role="status">
              <Loader2 className="w-4 h-4 animate-spin text-emerald-400" aria-hidden="true" /> {t('profiles.loading')}
            </div>
          ) : instances.length > 0 ? (
            <div className="grid grid-cols-1 md:grid-cols-2 2xl:grid-cols-3 gap-4">
              {/* The active profile first: it is the one the game reads now. */}
              {[...instances].sort((x, y) => Number(y.is_active) - Number(x.is_active)).map((inst) => (
                <div
                  key={inst.id}
                  className={`p-4 rounded-2xl border transition flex flex-col justify-between space-y-4 shadow-lg ${
                    inst.is_active
                      ? 'bg-gradient-to-br from-emerald-950/80 via-slate-900 to-slate-900 border-emerald-500/90 ring-1 ring-emerald-500/50'
                      : 'bg-slate-900/80 border-slate-800 hover:border-slate-700'
                  }`}
                >
                  <div className="space-y-2">
                    <div className="flex items-start justify-between">
                      <div className="flex items-center gap-2">
                        <FolderArchive className={`w-5 h-5 ${inst.is_active ? 'text-emerald-400' : 'text-slate-400'}`} />
                        <h4 className="text-sm font-bold text-slate-100">{inst.name}</h4>
                      </div>

                      {inst.is_active ? (
                        <span className="text-[10px] font-mono font-bold bg-emerald-950 text-emerald-300 border border-emerald-700 px-2 py-0.5 rounded-full flex items-center gap-1">
                          <Check className="w-3 h-3" /> {t('profiles.active')}
                        </span>
                      ) : (
                        <span className="text-[10px] font-mono text-slate-500">{t('profiles.inactive')}</span>
                      )}
                    </div>

                    <p className="text-xs text-slate-300 line-clamp-2 leading-relaxed">
                      {inst.description || t('profiles.defaultDescription')}
                    </p>

                    <div className="flex items-center gap-2 text-[11px] font-mono text-slate-400 pt-1">
                      {(() => {
                        const { active, total, workshopItems } = profileModCounts(inst, installedWorkshop);
                        return (
                          <span className="text-emerald-400 font-bold" title={t('profiles.activeOfTitle', { total, items: workshopItems })}>
                            {t('profiles.activeOf', { count: active, total })}
                          </span>
                        );
                      })()}
                      {(() => {
                        const missing = inst.active_mod_ids.map((id) => id.trim().replace(/^\\/, '')).filter((id) => id && !installedIds.has(id));
                        return missing.length > 0 && mods.length > 0 ? (
                          <>
                            <span>•</span>
                            <span className="text-amber-300" title={t('profiles.notInstalledTitle', { ids: missing.join(', ') })}>
                              {t('profiles.notInstalled', { count: missing.length })}
                            </span>
                          </>
                        ) : null;
                      })()}
                      {inst.target_build ? (
                        <>
                          <span>•</span>
                          <span className="text-sky-300" title={t('profiles.buildSetTitle')}>{t('profiles.buildSet', { build: inst.target_build })}</span>
                        </>
                      ) : inst.last_played_build ? (
                        <>
                          <span>•</span>
                          <span title={t('profiles.playedOnTitle')}>{t('profiles.playedOn', { build: inst.last_played_build })}</span>
                        </>
                      ) : null}
                    </div>
                  </div>

                  <div className="flex items-center justify-between gap-2 text-[11px]">
                    {inst.own_saves ? (
                      <>
                        <span className="text-sky-300" title={t('profiles.ownSaves.onTitle')}>{t('profiles.ownSaves.on')}</span>
                        <button
                          type="button"
                          disabled={ownBusy}
                          onClick={() => {
                            if (window.confirm(t('profiles.ownSaves.backConfirm', { name: inst.name }))) void applyOwnSaves(inst, false, []);
                          }}
                          className="text-slate-400 hover:text-slate-200 underline cursor-pointer disabled:opacity-50"
                        >
                          {t('profiles.ownSaves.back')}
                        </button>
                      </>
                    ) : simple ? null : (
                      <button
                        type="button"
                        onClick={() => void openOwnSaves(inst)}
                        className="text-slate-400 hover:text-slate-200 underline cursor-pointer"
                        title={t('profiles.ownSaves.giveTitle')}
                      >
                        {t('profiles.ownSaves.give')}
                      </button>
                    )}
                  </div>
                  {inst.own_saves && leftShared.length > 0 && (
                    <p className="text-[11px] text-slate-400 leading-relaxed" title={t('profiles.ownSaves.leftSharedTitle')}>
                      {t('profiles.ownSaves.leftShared', { count: leftShared.length, list: leftShared.join(', ') })}
                    </p>
                  )}

                  <div className="flex items-center gap-2 pt-2 border-t border-slate-800/80">
                    {inst.is_active && onGoToModList && (
                      <button
                        onClick={onGoToModList}
                        className="flex-1 py-2 bg-slate-800 hover:bg-slate-700 text-emerald-200 border border-emerald-800/70 rounded-xl text-xs font-bold transition flex items-center justify-center gap-1.5 cursor-pointer"
                        title={t('profiles.seeModsTitle')}
                      >
                        <ListOrdered className="w-3.5 h-3.5" aria-hidden="true" />
                        <span>{t('profiles.seeMods')}</span>
                      </button>
                    )}
                    {!inst.is_active && (
                      <button
                        onClick={() => handleActivateInstance(inst)}
                        className="flex-1 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-xl text-xs font-bold transition flex items-center justify-center gap-1.5 cursor-pointer shadow"
                      >
                        <Zap className="w-3.5 h-3.5 fill-white" />
                        <span>{t('profiles.activate')}</span>
                      </button>
                    )}

                    <button
                      onClick={() => handleShareInstance(inst)}
                      className="p-2 bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 rounded-xl transition cursor-pointer"
                      title={t('profiles.shareTitle')}
                      aria-label={t('profiles.shareLabel', { name: inst.name })}
                    >
                      <Share2 className="w-3.5 h-3.5" aria-hidden="true" />
                    </button>

                    <button
                      onClick={() => handleOpenEdit(inst)}
                      className="p-2 bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 rounded-xl transition cursor-pointer"
                      title={t('profiles.renameTitle')}
                      aria-label={t('profiles.renameLabel', { name: inst.name })}
                    >
                      <Edit3 className="w-3.5 h-3.5" />
                    </button>

                    <button
                      onClick={() => handleDeleteInstance(inst)}
                      className="p-2 bg-slate-800 hover:bg-rose-950 text-slate-400 hover:text-rose-400 border border-slate-700 hover:border-rose-700 rounded-xl transition cursor-pointer"
                      title={t('profiles.deleteTitle')}
                      aria-label={t('profiles.deleteLabel', { name: inst.name })}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="h-64 bg-slate-900/80 border border-slate-800 rounded-2xl flex flex-col items-center justify-center p-8 text-center space-y-3">
              <div className="w-14 h-14 rounded-2xl bg-slate-800/80 border border-slate-700/60 flex items-center justify-center shadow">
                <Layers className="w-7 h-7 text-emerald-400" />
              </div>
              <div className="space-y-1 max-w-md">
                <h3 className="text-sm font-bold text-slate-200">{t('profiles.noneTitle')}</h3>
                <p className="text-xs text-slate-400 leading-relaxed">
                  {t('profiles.noneBlurb')}
                </p>
              </div>
              <button type="button" onClick={openCreate} className="flex items-center gap-1.5 px-4 py-2 rounded-lg text-xs font-bold bg-emerald-600 hover:bg-emerald-500 text-white cursor-pointer">
                <Plus className="w-3.5 h-3.5" aria-hidden="true" />
                {t('profiles.newProfile')}
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Delete Profile Dialog: with the Workshop items only this profile uses */}
      {pendingDelete && (
        <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4" onClick={() => !pendingDelete.busy && setPendingDelete(null)}>
          <div className="bg-slate-900 border border-rose-900/60 rounded-2xl max-w-lg w-full p-6 space-y-4 shadow-2xl animate-in fade-in zoom-in-95" onClick={(e) => e.stopPropagation()}>
            <div className="flex items-start justify-between gap-3 border-b border-slate-800 pb-3">
              <div>
                <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
                  <Trash2 className="w-4 h-4 text-rose-400" aria-hidden="true" />
                  <span>{t('profiles.deleteDialog.title', { name: pendingDelete.inst.name })}</span>
                </h3>
                <p className="text-[11px] text-slate-400 mt-1 leading-relaxed">{t('profiles.deleteDialog.blurb', { count: pendingDelete.plan.exclusive.length })}</p>
              </div>
              <button onClick={() => setPendingDelete(null)} disabled={pendingDelete.busy} className="text-slate-400 hover:text-slate-200 p-1 cursor-pointer" aria-label={t('profiles.cancel')}>
                <X className="w-4 h-4" aria-hidden="true" />
              </button>
            </div>

            <div className="space-y-3 max-h-80 overflow-y-auto pr-1">
              <div>
                <div className="flex items-center justify-between mb-1">
                  <span className="text-[11px] font-bold text-amber-300 uppercase tracking-wider">{t('profiles.deleteDialog.onlyHere', { count: pendingDelete.plan.exclusive.length })}</span>
                  <button
                    type="button"
                    onClick={() => setPendingDelete((cur) => cur && ({ ...cur, ticked: cur.ticked.size === cur.plan.exclusive.length ? new Set() : new Set(cur.plan.exclusive.map((i) => i.workshop_id)) }))}
                    className="text-[11px] text-slate-400 hover:text-slate-200 cursor-pointer"
                  >
                    {pendingDelete.ticked.size === pendingDelete.plan.exclusive.length ? t('profiles.deleteDialog.tickNone') : t('profiles.deleteDialog.tickAll')}
                  </button>
                </div>
                <div className="rounded-xl border border-slate-800 divide-y divide-slate-800/80">
                  {pendingDelete.plan.exclusive.map((item) => (
                    <label key={item.workshop_id} className="flex items-start gap-2.5 px-3 py-2 text-xs text-slate-200 hover:bg-slate-950/60 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={pendingDelete.ticked.has(item.workshop_id)}
                        onChange={() => toggleTicked(item.workshop_id)}
                        disabled={pendingDelete.busy}
                        className="mt-0.5 accent-amber-500"
                      />
                      <span className="min-w-0">
                        <span className="block truncate font-medium">{item.mod_names.join(', ')}</span>
                        <span className="block text-[10px] font-mono text-slate-500">{item.workshop_id}</span>
                      </span>
                    </label>
                  ))}
                </div>
              </div>

              {pendingDelete.plan.shared.length > 0 && (
                <div>
                  <span className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">{t('profiles.deleteDialog.shared', { count: pendingDelete.plan.shared.length })}</span>
                  <div className="rounded-xl border border-slate-800/70 divide-y divide-slate-800/60 opacity-70">
                    {pendingDelete.plan.shared.map((item) => (
                      <div key={item.workshop_id} className="px-3 py-1.5 text-xs text-slate-300">
                        <span className="block truncate">{item.mod_names.join(', ')}</span>
                        <span className="block text-[10px] text-slate-500">{t('profiles.deleteDialog.usedBy', { profiles: item.used_by.join(', ') })}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {pendingDelete.plan.local_mod_names.length > 0 && (
                <p className="text-[11px] text-slate-500 leading-relaxed">{t('profiles.deleteDialog.local', { count: pendingDelete.plan.local_mod_names.length, names: pendingDelete.plan.local_mod_names.join(', ') })}</p>
              )}
              {!pendingDelete.steamReady && (
                <p className="text-[11px] text-amber-300/90 leading-relaxed">{t('profiles.deleteDialog.steamNotRunning')}</p>
              )}
            </div>

            <div className="flex flex-col sm:flex-row items-stretch gap-2 pt-2">
              <button
                type="button"
                onClick={() => void deleteNow(pendingDelete.inst, [...pendingDelete.ticked])}
                disabled={pendingDelete.busy || !pendingDelete.steamReady || pendingDelete.ticked.size === 0}
                className="flex-1 py-2.5 bg-amber-600 hover:bg-amber-500 text-white rounded-xl text-xs font-bold transition flex items-center justify-center gap-2 cursor-pointer shadow disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {pendingDelete.busy ? <Loader2 className="w-4 h-4 animate-spin" aria-hidden="true" /> : <Unplug className="w-4 h-4" aria-hidden="true" />}
                <span>{t('profiles.deleteDialog.deleteAndUnsubscribe', { count: pendingDelete.ticked.size })}</span>
              </button>
              <button
                type="button"
                onClick={() => void deleteNow(pendingDelete.inst, [])}
                disabled={pendingDelete.busy}
                className="flex-1 py-2.5 bg-rose-700 hover:bg-rose-600 text-white rounded-xl text-xs font-bold transition flex items-center justify-center gap-2 cursor-pointer shadow disabled:opacity-50"
              >
                <Trash2 className="w-4 h-4" aria-hidden="true" />
                <span>{t('profiles.deleteDialog.deleteOnly')}</span>
              </button>
              <button
                type="button"
                onClick={() => setPendingDelete(null)}
                disabled={pendingDelete.busy}
                className="px-4 py-2.5 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-xl text-xs font-bold transition cursor-pointer disabled:opacity-50"
              >
                {t('profiles.cancel')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Edit Profile Modal */}
      {editingInstance && (
        <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className="bg-slate-900 border border-slate-800 rounded-2xl max-w-md w-full p-6 space-y-4 shadow-2xl animate-in fade-in zoom-in-95">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3">
              <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
                <Edit3 className="w-4 h-4 text-emerald-400" />
                <span>{t('profiles.editTitle')}</span>
              </h3>
              <button
                onClick={() => setEditingInstance(null)}
                className="text-slate-400 hover:text-slate-200 p-1"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-3">
              <div>
                <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">
                  {t('profiles.name')}
                </label>
                <input
                  type="text"
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2 text-xs text-slate-200 focus:outline-none focus:border-emerald-500 font-medium"
                />
              </div>

              <div>
                <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">
                  {t('profiles.description')}
                </label>
                <textarea
                  rows={3}
                  value={editDesc}
                  onChange={(e) => setEditDesc(e.target.value)}
                  className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2 text-xs text-slate-200 focus:outline-none focus:border-emerald-500 font-sans"
                />
              </div>

              <div>
                <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">
                  {t('profiles.gameBuild')}
                </label>
                <select
                  value={editBuild}
                  onChange={(e) => setEditBuild(e.target.value)}
                  aria-label={t('profiles.gameBuildLabel')}
                  className="w-full bg-slate-950 border border-slate-800 rounded-xl px-3.5 py-2 text-xs text-slate-200 focus:outline-none focus:border-emerald-500"
                >
                  <option value="">
                    {editingInstance.last_played_build ? t('profiles.asPlayed', { build: editingInstance.last_played_build }) : t('profiles.asPlayedNever')}
                  </option>
                  <option value="41">{t('profiles.build41')}</option>
                  <option value="42">{t('profiles.build42')}</option>
                </select>
                <p className="text-[10px] text-slate-500 mt-1">
                  {t('profiles.buildHelp')}
                </p>
              </div>
            </div>

            <div className="flex items-center gap-2 pt-2">
              <button
                onClick={handleSaveEdit}
                className="flex-1 py-2.5 bg-emerald-600 hover:bg-emerald-500 text-white rounded-xl text-xs font-bold transition flex items-center justify-center gap-2 cursor-pointer shadow"
              >
                <Save className="w-4 h-4" />
                <span>{t('profiles.save')}</span>
              </button>
              <button
                onClick={() => setEditingInstance(null)}
                className="px-4 py-2.5 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-xl text-xs font-bold transition cursor-pointer"
              >
                {t('profiles.cancel')}
              </button>
            </div>
          </div>
        </div>
      )}
      {ownSaves && (
        <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm flex items-center justify-center p-4 z-50">
          <div className="bg-slate-900 border border-sky-700/60 rounded-2xl p-5 max-w-lg w-full space-y-3 shadow-2xl">
            <h3 className="text-sm font-bold text-slate-100">{t('profiles.ownSaves.title', { name: ownSaves.inst.name })}</h3>
            <p className="text-xs text-slate-300 leading-relaxed">{t('profiles.ownSaves.explain')}</p>
            <ul className="list-disc pl-5 space-y-1 text-xs text-slate-200 leading-relaxed">
              <li>{t('profiles.ownSaves.itemSaves')}</li>
              <li>{t('profiles.ownSaves.itemOptions')}</li>
              <li>{t('profiles.ownSaves.itemModSettings')}</li>
            </ul>
            <p className="text-[11px] text-slate-400 leading-relaxed">{t('profiles.ownSaves.sharedNote')}</p>
            {ownSaves.worlds.length > 0 ? (
              <>
                <div className="text-[11px] font-bold text-slate-300">{t('profiles.ownSaves.bringTitle')}</div>
                <div className="max-h-56 overflow-y-auto space-y-1 bg-slate-950/60 border border-slate-800 rounded-lg p-2">
                  {ownSaves.worlds.map((w) => (
                    <label key={w} className="flex items-center gap-2 text-xs text-slate-200 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={ownSaves.bring.includes(w)}
                        onChange={(e) =>
                          setOwnSaves({ ...ownSaves, bring: e.target.checked ? [...ownSaves.bring, w] : ownSaves.bring.filter((x) => x !== w) })
                        }
                        className="cursor-pointer"
                      />
                      <span className="truncate">{w.split('/').slice(1).join('/')}</span>
                      <span className="text-[10px] text-slate-500">{w.split('/')[0]}</span>
                    </label>
                  ))}
                </div>
              </>
            ) : (
              <p className="text-[11px] text-slate-400">{t('profiles.ownSaves.noneShared')}</p>
            )}
            <div className="flex justify-end gap-2 pt-1">
              <button type="button" onClick={() => setOwnSaves(null)} className="px-3 py-2 rounded-lg text-xs text-slate-300 hover:bg-slate-800 cursor-pointer">
                {t('profiles.ownSaves.cancel')}
              </button>
              <button
                type="button"
                disabled={ownBusy}
                onClick={() => void applyOwnSaves(ownSaves.inst, true, ownSaves.bring)}
                className="px-4 py-2 bg-sky-700 hover:bg-sky-600 disabled:opacity-50 text-white rounded-lg text-xs font-bold cursor-pointer"
              >
                {t('profiles.ownSaves.confirm', { count: ownSaves.bring.length })}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

