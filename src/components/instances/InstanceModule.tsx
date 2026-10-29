import { freshProfile } from '../../utils/profiles';
import { ChangePreview } from '../common/ChangePreview';
import React, { useState, useEffect } from 'react';
import { Layers, Plus, Check, Trash2, Zap, FolderArchive, RefreshCw, Edit3, Share2, X, Save, Loader2, Unplug } from 'lucide-react';
import { ModInfo, StudioPathsUI, AppInstance } from '../../types';
import { TauriService, explainError } from '../../services/tauri';
import { useTranslation } from 'react-i18next';
import { tHtml } from '../../i18n/html';
import { useChangeRevision } from '../../services/changes';
import { planProfileCleanup, ProfileCleanupPlan } from '../../utils/profileCleanup';

interface InstanceModuleProps {
  paths: StudioPathsUI;
  mods: ModInfo[];
  onApplyInstanceLoadOrder: (loadOrder: string[], activeIds: string[]) => void;
  onProfileActivated?: (inst: AppInstance) => void;
  /** The active profile was deleted, so nothing is active any more. */
  onProfileDeleted?: () => void;
  /** Takes the user to Browse Mods, for a profile that starts with nothing in it. */
  onGoToBrowse?: () => void;
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
  onProfilesChanged,
  onRefreshMods,
}) => {
  const { t } = useTranslation();
  const changeRev = useChangeRevision();
  const [instances, setInstances] = useState<AppInstance[]>([]);
  const [newInstanceName, setNewInstanceName] = useState('');
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
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  // Edit Modal State
  const [editingInstance, setEditingInstance] = useState<AppInstance | null>(null);
  /** The build the profile is for, as edited: '' = as last played. */
  const [editBuild, setEditBuild] = useState<string>('');
  const [editName, setEditName] = useState('');
  const [editDesc, setEditDesc] = useState('');
  const [pendingDelete, setPendingDelete] = useState<PendingDelete | null>(null);

  const activeMods = mods.filter((m) => m.enabled);

  const loadInstances = async () => {
    setIsLoading(true);
    try {
      const list: AppInstance[] = await TauriService.listInstances(paths.user_zomboid_dir);
      setInstances(list);
    } catch (err) {
      console.error('Error loading profiles:', err);
    } finally {
      setIsLoading(false);
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

      // Activate it so the rest of the app unlocks, and only then say so.
      await handleActivateInstance(created);
      setStatusMessage(t('profiles.created', { name: created.name }));
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
  const [pendingActivation, setPendingActivation] = useState<{ inst: AppInstance; plan: NonNullable<Awaited<ReturnType<typeof TauriService.javaAgentsPlan>>> } | null>(null);

  const handleActivateInstance = async (inst: AppInstance) => {
    // The launch file is the game's, outside the mods folder: its change is shown first.
    const plan = await TauriService.javaAgentsPlan(paths, inst.active_mod_ids);
    if (plan && (plan.removed.length > 0 || plan.restored.length > 0)) {
      setPendingActivation({ inst, plan });
      return;
    }
    await activateNow(inst);
  };

  const activateNow = async (inst: AppInstance) => {
    setPendingActivation(null);
    try {
      await TauriService.activateInstance(paths.user_zomboid_dir, inst.id);
      onApplyInstanceLoadOrder(inst.load_order, inst.active_mod_ids);
      if (onProfileActivated) {
        onProfileActivated(inst);
      }
      setStatusMessage(t('profiles.activated', { name: inst.name }));
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

  return (
    <div className="space-y-6">
      {pendingActivation && (
        <ChangePreview
          title={t('profiles.launchFile.title', { name: pendingActivation.inst.name })}
          blurb={pendingActivation.plan.gameRunning
            ? t('profiles.launchFile.blurbLater', { file: pendingActivation.plan.file })
            : t('profiles.launchFile.blurb', { file: pendingActivation.plan.file })}
          sections={[
            { label: t('profiles.launchFile.out'), tone: 'off', rows: pendingActivation.plan.removed.map((m, k) => ({ name: pendingActivation.plan.removed_args[k] ?? m, note: t('profiles.launchFile.outNote', { mod: m }) })) },
            { label: t('profiles.launchFile.back'), tone: 'on', rows: pendingActivation.plan.restored.map((m, k) => ({ name: pendingActivation.plan.restored_args[k] ?? m, note: t('profiles.launchFile.backNote', { mod: m }) })) },
          ]}
          primary={{ label: t('profiles.launchFile.activate', { name: pendingActivation.inst.name }), onClick: () => void activateNow(pendingActivation.inst) }}
          onClose={() => setPendingActivation(null)}
        />
      )}
      {/* Top Banner */}
      <div className="p-5 bg-gradient-to-r from-emerald-950/80 via-slate-900 to-teal-950/80 border border-emerald-800/60 rounded-2xl shadow-xl flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Layers className="w-6 h-6 text-emerald-400" />
            <h2 className="text-xl font-bold text-slate-100 tracking-tight">
              {t('profiles.title')}
            </h2>
            {isLoading && <RefreshCw className="w-4 h-4 text-emerald-400 animate-spin ml-2" />}
          </div>
          <p className="text-xs text-slate-300 max-w-2xl leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('profiles.blurb') }} />
        </div>

        <span className="text-xs font-mono bg-emerald-950 text-emerald-300 border border-emerald-800 px-3 py-1.5 rounded-xl font-bold">
          {t('profiles.saved', { count: instances.length })}
        </span>
      </div>

      {statusMessage && (
        <div className="p-3 bg-emerald-950/80 border border-emerald-700/80 rounded-xl text-xs text-emerald-300 font-bold flex items-center justify-between">
          <span>{statusMessage}</span>
          <button onClick={() => setStatusMessage(null)} className="text-emerald-400 hover:underline">
            {t('profiles.close')}
          </button>
        </div>
      )}

      {/* Main Grid: Create Profile & Instance Cards */}
      <div className="grid grid-cols-12 gap-6">
        {/* Left Column: Create New Profile */}
        <div className="col-span-12 lg:col-span-4 bg-slate-900/80 border border-slate-800 rounded-2xl p-5 space-y-4 shadow-lg">
          <div className="border-b border-slate-800 pb-3 space-y-1">
            <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
              <Plus className="w-4 h-4 text-emerald-400" />
              <span>{t('profiles.createTitle')}</span>
            </h3>
            <p className="text-xs text-slate-400">
              {t('profiles.createBlurb')}
            </p>
          </div>

          <div className="space-y-3">
            <div>
              <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">
                {t('profiles.name')}
              </label>
              <input
                type="text"
                placeholder={t('profiles.namePlaceholder')}
                value={newInstanceName}
                onChange={(e) => setNewInstanceName(e.target.value)}
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

            <div>
              <label className="text-[11px] font-bold text-slate-400 uppercase tracking-wider block mb-1">
                {t('profiles.whichMods')}
              </label>
              <div className="grid grid-cols-2 gap-1.5">
                {([
                  { id: 'current', label: t('profiles.whatIsOn'), count: activeMods.length },
                  { id: 'all', label: t('profiles.everything'), count: mods.length },
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

            <button
              onClick={handleCreateInstance}
              disabled={!newInstanceName.trim()}
              className="w-full py-3 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 disabled:opacity-40 text-white rounded-xl text-xs font-bold transition cursor-pointer shadow-lg flex items-center justify-center gap-2 mt-2"
            >
              <Plus className="w-4 h-4" />
              <span>{t('profiles.createTitle')}</span>
            </button>
          </div>
        </div>

        {/* Right Column: Display Cards of All Saved Profiles */}
        <div className="col-span-12 lg:col-span-8 space-y-4">
          {instances.length > 0 ? (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {instances.map((inst) => (
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
                      <span className="text-emerald-400 font-bold">{t('profiles.activeMods', { count: inst.active_mod_ids.length })}</span>
                      <span>•</span>
                      <span>{t('profiles.total', { count: inst.load_order.length })}</span>
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

                  <div className="flex items-center gap-2 pt-2 border-t border-slate-800/80">
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
    </div>
  );
};

