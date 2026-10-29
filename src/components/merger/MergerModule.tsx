import { freshProfile } from '../../utils/profiles';
import React, { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { VfsConflict, ModInfo, AppInstance, MissingModsReport, profileMembers } from '../../types';
import { ImportCheck, importCheckHasNews } from './ImportCheck';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, MasterPatchStatusInfoUI, MergedPackageInfoUI, explainError } from '../../services/tauri';
import { RequireFix, applyRequireFix, packageForPatch } from '../../utils/requireFix';
import { ConflictSummaryCard } from './CodeView';
import { PatchFilePicker } from './PatchFilePicker';
import { PackageList } from './PackageList';
import { CodePanels } from './CodePanels';
import { Choice, EditorPanel } from './EditorPanel';
import { CreatePackageModal, RenamePackageModal, UnpackageModal } from './PackageModals';
import i18next from 'i18next';
import { useTranslation } from 'react-i18next';
import { useChangeRevision } from '../../services/changes';
import {
  GitCompare,
  CheckCircle2,
  AlertTriangle,
  FileCode,
  EyeOff,
  Eye,
  Layers,
  ShieldCheck,
  FolderX,
  AlertCircle,
  Wand2,
  GripHorizontal,
  Sparkle,
  PackageCheck,
  Package,
  Unlock,
  RefreshCw,
  Download,
  FolderOpen,
  ArrowLeft,
  Undo2,
  Wrench,
} from 'lucide-react';
import { tHtml } from '../../i18n/html';

interface MergerModuleProps {
  conflicts: VfsConflict[];
  /** Installed mods, to pick one whose file you want to patch. */
  mods: ModInfo[];
  paths: StudioPathsUI;
  /** Saves the decision for one file into the package; the path is for files the scan never listed. */
  onResolveConflict: (conflictId: string, resolvedCode: string, packageFolderName?: string, relativePath?: string) => void;
  onAutoMergeAll?: (packageFolderName?: string) => Promise<boolean>;
  onOptimizeAndResolve: (packageFolderName?: string) => Promise<boolean>;
  onGoToSettings: () => void;
  /** Rescans; the folder names the package whose saved merges should be shown. */
  onRescan: (packageFolder?: string) => Promise<void> | void;
  onClearConflicts?: () => void;
  onLoadMockups?: (mockups: VfsConflict[]) => void;
  onToggleMod?: (modId: string) => void;
  onRefreshMods?: () => Promise<void> | void;
  onPackageOpened?: (folderName: string) => void;
  onPackageClosed?: () => void;
  /** Open a package and select this conflicting file (from Mod Health). */
  focusRelativePath?: string | null;
  onFocusHandled?: () => void;
  /**
   * Open a package and this mod's files to patch (from the Monitor: a card that names the mod, or
   * the hunt that ended on it). With a file name from the log, that file opens straight away.
   */
  patchFocus?: { modId: string; fileHint?: string | null; fix?: RequireFix | null } | null;
  onPatchFocusHandled?: () => void;
  /** Profiles, to show the active one's packages and say whose the others are. */
  profiles?: AppInstance[];
  activeProfileId?: string | null;
  onProfilesChanged?: () => void;
}

/** A mod's own file opened to patch, as opposed to a conflict the scan found. */
const isPatchTarget = (c: VfsConflict | null | undefined) => !!c && c.id.startsWith('modfile_');

export const MergerModule: React.FC<MergerModuleProps> = ({
  conflicts: scannedConflicts,
  mods,
  paths,
  onResolveConflict,
  onAutoMergeAll,
  onOptimizeAndResolve,
  onGoToSettings,
  onRescan,
  onClearConflicts,
  onLoadMockups: _onLoadMockups,
  onToggleMod,
  onRefreshMods,
  onPackageOpened,
  onPackageClosed,
  focusRelativePath,
  onFocusHandled,
  patchFocus,
  onPatchFocusHandled,
  profiles = [],
  activeProfileId = null,
  onProfilesChanged,
}) => {
  /**
   * Files opened to patch a mod's own bug. They are not conflicts -- only one mod ships them --
   * so the scan does not list them until the package holds a decision for one; until then they
   * live here, beside the scanned ones, and behave the same.
   */
  const [patchTargets, setPatchTargets] = useState<VfsConflict[]>([]);
  const conflicts = useMemo(
    () => [
      ...scannedConflicts,
      ...patchTargets.filter((p) => !scannedConflicts.some((c) => c.relative_path === p.relative_path)),
    ],
    [scannedConflicts, patchTargets]
  );

  /** The picker for a mod's own file to patch, and the mod it opens on when the Monitor named one. */
  const [patchPicker, setPatchPicker] = useState<{ open: boolean; modId: string | null }>({ open: false, modId: null });

  const [selectedConflictId, setSelectedConflictId] = useState<string>(conflicts[0]?.id || '');
  const [filterNoise, setFilterNoise] = useState<boolean>(false);
  const [isRescanning, setIsRescanning] = useState<boolean>(false);
  const [isCleaning, setIsCleaning] = useState<boolean>(false);
  const [isGenerating, setIsGenerating] = useState<boolean>(false);
  useTranslation();
  const changeRev = useChangeRevision();
  const [toastMessage, setToastMessage] = useState<string | null>(null);
  const [patchStatus, setPatchStatus] = useState<MasterPatchStatusInfoUI | null>(null);

  // Multi-Package State
  const packageSwitchIdRef = useRef(0);
  const [packages, setPackages] = useState<MergedPackageInfoUI[]>([]);
  const [packagesLoaded, setPackagesLoaded] = useState(false);
  const [selectedPackageFolder, setSelectedPackageFolder] = useState<string>('Z_PZModStudio_MergedPatch');
  const [isPackageOpened, setIsPackageOpened] = useState<boolean>(false);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState<boolean>(false);
  const [renamingPackage, setRenamingPackage] = useState<MergedPackageInfoUI | null>(null);
  const [isUnpackageModalOpen, setIsUnpackageModalOpen] = useState<boolean>(false);

  // Toast Helper
  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => setToastMessage(null), 4000);
  };

  const fetchPackages = useCallback(async () => {
    const list = await TauriService.listMergedPackages(paths.user_zomboid_dir || '', paths.mod_list_ini_path || '');
    setPackages(list);
    setPackagesLoaded(true);
    if (list.length > 0 && !list.some((p) => p.folder_name === selectedPackageFolder)) {
      setSelectedPackageFolder(list[0].folder_name);
    }
  }, [paths.user_zomboid_dir, paths.mod_list_ini_path, selectedPackageFolder]);

  /**
   * A package belongs to the profiles that have it among their mods. With a profile active the
   * list shows that profile's packages (and the bridge, which is everyone's); the others are a
   * click away, each labelled with the profiles it belongs to.
   */
  const activeProfile = profiles.find((p) => p.id === activeProfileId) ?? null;
  const [showOtherProfilesPackages, setShowOtherProfilesPackages] = useState(false);
  const ownersByPackage = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const p of profiles) {
      for (const id of profileMembers(p)) {
        if (id.startsWith('Z_PZModStudio_')) map.set(id, [...(map.get(id) ?? []), p.name]);
      }
    }
    return map;
  }, [profiles]);
  const inActiveProfile = useCallback(
    (pkg: MergedPackageInfoUI) =>
      !activeProfile || pkg.is_companion_bridge || profileMembers(activeProfile).includes(pkg.folder_name),
    [activeProfile]
  );
  const shownPackages = useMemo(
    () => (showOtherProfilesPackages ? packages : packages.filter(inActiveProfile)),
    [packages, showOtherProfilesPackages, inActiveProfile]
  );
  const otherProfilesCount = packages.length - packages.filter(inActiveProfile).length;

  /** Profiles hold packages by folder name: a renamed package stays in the profiles it was in. */
  const renameInProfiles = async (from: string, to: string) => {
    if (from === to) return;
    const swap = (list: string[]) => list.map((id) => (id === from ? to : id));
    for (const p of profiles) {
      if (!profileMembers(p).includes(from) && !p.load_order.includes(from)) continue;
      await TauriService.updateInstance(paths.user_zomboid_dir || '', {
        ...p,
        active_mod_ids: swap(p.active_mod_ids),
        load_order: swap(p.load_order),
        members: p.members ? swap(p.members) : p.members,
      });
    }
    onProfilesChanged?.();
  };

  const fetchPatchStatus = useCallback(async () => {
    if (selectedPackageFolder) {
      const status = await TauriService.getMasterPatchStatus(paths.user_zomboid_dir || '', paths.mod_list_ini_path || '', selectedPackageFolder);
      setPatchStatus(status);
    }
  }, [paths.user_zomboid_dir, paths.mod_list_ini_path, selectedPackageFolder]);

  useEffect(() => {
    fetchPackages();
  }, [fetchPackages, conflicts, changeRev]);

  const handleOpenPackage = async (folderName: string) => {
    const switchId = ++packageSwitchIdRef.current;
    setSelectedPackageFolder(folderName);
    setIsPackageOpened(true);
    onPackageOpened?.(folderName);
    // Fetch with explicit folderName to avoid stale closure
    try {
      const status = await TauriService.getMasterPatchStatus(
        paths.user_zomboid_dir || '', paths.mod_list_ini_path || '', folderName
      );
      // Guard: only apply if this is still the latest switch
      if (packageSwitchIdRef.current !== switchId) return;
      setPatchStatus(status);
    } catch (err) {
      console.error('Error fetching patch status:', err);
    }
    if (packageSwitchIdRef.current !== switchId) return;
    if (onRescan) await onRescan(folderName);
  };

  const handleClosePackage = () => {
    setIsPackageOpened(false);
    onPackageClosed?.();
    if (onClearConflicts) onClearConflicts();
  };

  const handleTogglePackageInModlist = async (pkg: MergedPackageInfoUI) => {
    if (!paths.user_zomboid_dir) return;
    const targetState = !pkg.is_active_in_modlist;
    const success = await TauriService.togglePackageInModlist(
      paths.user_zomboid_dir,
      paths.mod_list_ini_path,
      pkg.folder_name,
      targetState
    );
    if (success) {
      if (onRefreshMods) {
        await onRefreshMods();
      } else if (onToggleMod) {
        onToggleMod(pkg.folder_name);
      }
      await fetchPackages();
      if (onRescan) await onRescan();
      showToast(
        targetState
          ? i18next.t('merger.toasts.visible', { name: pkg.display_name })
          : i18next.t('merger.toasts.hidden', { name: pkg.display_name })
      );
    }
  };

  /**
   * Opens one of a mod's own files for patching. It joins the list like a conflict and is edited
   * and saved the same way; the package, loading last, is then what the game reads for it.
   */
  /**
   * Saves a patched mod file into the package. While the package is packaged it is published in
   * place as well -- the package stays packaged and the game reads the new file on its next
   * launch -- because unpacking would take every published file away until the next publish.
   * Returns whether it was published.
   */
  const savePatchTarget = async (id: string, relativePath: string, content: string): Promise<boolean> => {
    if (isCurrentlyPackaged) {
      await TauriService.publishFileIntoPackage(paths, selectedPackageFolder, relativePath, content);
      await fetchPatchStatus();
      await fetchPackages();
      if (onRefreshMods) await onRefreshMods();
      return true;
    }
    onResolveConflict(id, content, selectedPackageFolder, relativePath);
    return false;
  };

  const openModFileToPatch = async (modId: string, relativePath: string, fix?: RequireFix | null) => {
    try {
      const opened = await TauriService.modFileToPatch(paths, modId, relativePath);
      // A fix the Studio already knows is applied and saved on the way in: the file lands in the
      // package resolved, and what is left to do is publish.
      const fixed = fix ? applyRequireFix(opened.merged_output || opened.base_content, fix) : null;
      const entry = fixed ? { ...opened, merged_output: fixed.content, status: 'RESOLVED' as const } : opened;
      setPatchTargets((current) => [...current.filter((c) => c.relative_path !== entry.relative_path), entry]);
      setSelectedConflictId(entry.id);
      setEditorContent(entry.merged_output || entry.base_content);
      setIsManuallyEdited(false);
      setPatchPicker({ open: false, modId: null });
      if (fixed) {
        const published = await savePatchTarget(entry.id, entry.relative_path, fixed.content);
        showToast(
          published
            ? i18next.t('merger.toasts.requirePublished', { line: fixed.line, from: fix!.from, to: fix!.to })
            : i18next.t('merger.toasts.requireSaved', { line: fixed.line, from: fix!.from, to: fix!.to })
        );
      } else if (fix) {
        showToast(i18next.t('merger.toasts.noRequire', { path: relativePath, from: fix.from }));
      } else {
        showToast(i18next.t('merger.toasts.opened', { path: relativePath }));
      }
    } catch (err) {
      alert(explainError(err));
    }
  };

  const handleUnmergeAll = async () => {
    if (
      confirm(i18next.t('merger.toasts.unmergeAllConfirm'))
    ) {
      if (paths.user_zomboid_dir && selectedPackageFolder) {
        await TauriService.clearDraftResolutions(paths.user_zomboid_dir, selectedPackageFolder);
      }
      if (currentConflict) {
        setEditorContent(unmergedContent(currentConflict));
        setIsManuallyEdited(false);
      }
      if (onRescan) await onRescan();
      showToast(i18next.t('merger.toasts.allReverted'));
    }
  };

  const handleUnmergeSingleConflict = async () => {
    if (!currentConflict || !paths.user_zomboid_dir || !selectedPackageFolder) return;
    setEditorContent(unmergedContent(currentConflict));
    setIsManuallyEdited(false);
    await TauriService.saveDraftResolution(
      paths.user_zomboid_dir,
      selectedPackageFolder,
      currentConflict.relative_path,
      '',
      'CLEARED'
    );
    if (onRescan) await onRescan();
    showToast(i18next.t('merger.toasts.reverted', { path: currentConflict.relative_path }));
  };

  // Noise Filtering logic
  const filteredConflicts = useMemo(() => {
    return conflicts.filter((c) => {
      if (filterNoise && c.is_identical_noise) {
        return false;
      }
      return true;
    });
  }, [conflicts, filterNoise]);

  const currentConflict = useMemo(() => {
    const found = conflicts.find((c) => c.id === selectedConflictId);
    if (found) return found;
    return filteredConflicts[0] || conflicts[0];
  }, [conflicts, filteredConflicts, selectedConflictId]);

  // Vertical resizable split panel height percentage (Top carousel vs Bottom Monaco editor)
  const [topHeightPercent, setTopHeightPercent] = useState<number>(50);
  const [isResizing, setIsResizing] = useState<boolean>(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Editor content state
  /**
   * What the editor starts with for a file: the decision the package holds, failing that what
   * the Studio proposes, failing that the copy of the mod that loads last.
   *
   * Never the base when the base game has no such file. The scan puts a note there --
   * "-- vanilla file not present for this path" -- and falling back to it put that note in the
   * editor as if it were the file, marked "Your own edit", one Save away from publishing a
   * one-line comment in place of two mods' code.
   */
  const startingContent = (c: VfsConflict | undefined): string => {
    if (!c) return '';
    const vanillaMissing =
      c.base_content.startsWith('-- vanilla file not present') ||
      c.base_content.startsWith('-- vanilla file unreadable');
    return (
      c.merged_output ||
      c.auto_ast_output ||
      (vanillaMissing ? c.competing_mods[c.competing_mods.length - 1]?.content ?? '' : c.base_content) ||
      ''
    );
  };

  /**
   * What "unmerged" means for a file: the game's own copy, or -- where the base game has no such
   * file -- the copy of the mod that loads last, which is what the game reads without a package.
   * Never the scan's "not present" note.
   */
  const unmergedContent = (c: VfsConflict | undefined): string => {
    if (!c) return '';
    const vanillaMissing =
      c.base_content.startsWith('-- vanilla file not present') ||
      c.base_content.startsWith('-- vanilla file unreadable');
    return vanillaMissing
      ? c.competing_mods[c.competing_mods.length - 1]?.content ?? ''
      : c.base_content;
  };

  const [editorContent, setEditorContent] = useState<string>(startingContent(currentConflict));
  const [isManuallyEdited, setIsManuallyEdited] = useState<boolean>(false);

  useEffect(() => {
    if (currentConflict) {
      setEditorContent(startingContent(currentConflict));
      setIsManuallyEdited(false);
    }
  }, [currentConflict?.id, currentConflict?.merged_output]);

  const hasAnyMergedConflicts = useMemo(() => {
    return conflicts.some((c) => c.status === 'RESOLVED' || c.status === 'AUTO_MERGED');
  }, [conflicts]);

  /**
   * A file is settled when the package holds a decision for it. Anything else -- including the
   * `MANUAL_CONFLICT` the scan's empty status becomes on the way in -- still needs one, which is
   * what each row says with "Needs Review".
   *
   * Asking for `PENDING` instead meant "Merge All" switched itself off while twelve rows said
   * Needs Review, telling the user everything was already resolved: disabled exactly when it was
   * the thing to press.
   */
  const isSettled = (status?: string) => status === 'RESOLVED' || status === 'AUTO_MERGED';

  const hasPendingConflicts = useMemo(() => {
    return conflicts.some((c) => !isSettled(c.status));
  }, [conflicts]);

  const isCurrentConflictUnmerged = useMemo(() => {
    if (!currentConflict) return true;
    const isPending = !isSettled(currentConflict.status);
    const isBaseCode = !currentConflict.merged_output || editorContent === currentConflict.base_content;
    return isPending && isBaseCode && !isManuallyEdited;
  }, [currentConflict, editorContent, isManuallyEdited]);

  const isCurrentConflictSaved = useMemo(() => {
    if (!currentConflict) return true;
    const isResolved = currentConflict.status === 'RESOLVED';
    const matchesOutput = editorContent === (currentConflict.merged_output || currentConflict.base_content);
    return isResolved && matchesOutput && !isManuallyEdited;
  }, [currentConflict, editorContent, isManuallyEdited]);

  const isCurrentConflictAstApplied = useMemo(() => {
    if (!currentConflict) return true;
    const isMergedStatus = currentConflict.status === 'RESOLVED' || currentConflict.status === 'AUTO_MERGED';
    const targetAst = currentConflict.auto_ast_output || currentConflict.merged_output || currentConflict.base_content;
    const matchesAst = editorContent === targetAst;
    return isMergedStatus && matchesAst && !isManuallyEdited;
  }, [currentConflict, editorContent, isManuallyEdited]);

  const handleCreatePackage = async (name: string, description?: string) => {
    const pkg = await TauriService.createMergedPackage(paths.user_zomboid_dir, paths.mod_list_ini_path, name, description);
    if (pkg) {
      setIsCreateModalOpen(false);
      // A package made while a profile is active is that profile's.
      if (activeProfile) {
        try {
          const current = await freshProfile(paths.user_zomboid_dir || '', activeProfile);
          await TauriService.setProfileMembers(paths.user_zomboid_dir || '', activeProfile.id, [...profileMembers(current), pkg.folder_name]);
          onProfilesChanged?.();
        } catch {
          // Already shown by TauriService; the package is made either way.
        }
      }
      await fetchPackages();
      setSelectedPackageFolder(pkg.folder_name);
      setIsPackageOpened(true);
      // The scan shows the merges of the package now open, not of the one open before.
      onPackageOpened?.(pkg.folder_name);
      if (onRefreshMods) await onRefreshMods();
      if (onRescan) await onRescan(pkg.folder_name);
      showToast(i18next.t('merger.toasts.created', { name: pkg.display_name }));
    }
  };

  const handleRenamePackage = async (folderName: string, name: string, description?: string) => {
    if (folderName === 'Z_PZModStudio_Bridge') {
      alert(i18next.t('merger.toasts.bridgeRename'));
      setRenamingPackage(null);
      return;
    }
    const pkg = await TauriService.renameMergedPackage(paths.user_zomboid_dir, paths.mod_list_ini_path, folderName, name, description);
    if (pkg) {
      setRenamingPackage(null);
      await renameInProfiles(folderName, pkg.folder_name);
      await fetchPackages();
      setSelectedPackageFolder(pkg.folder_name);
      if (isPackageOpened && selectedPackageFolder === folderName) onPackageOpened?.(pkg.folder_name);
      if (onRefreshMods) await onRefreshMods();
      if (onRescan) await onRescan(isPackageOpened && selectedPackageFolder === folderName ? pkg.folder_name : undefined);
      showToast(i18next.t('merger.toasts.updated', { name: pkg.display_name }));
    }
  };

  const handleDeletePackage = async (folderName: string) => {
    if (folderName === 'Z_PZModStudio_Bridge') {
      alert(i18next.t('merger.toasts.bridgeDelete'));
      return;
    }
    if (confirm(i18next.t('merger.toasts.deleteConfirm', { name: folderName }))) {
      const deleted = await TauriService.deleteMergedPackage(paths.user_zomboid_dir, paths.mod_list_ini_path, folderName);
      if (!deleted) return; // failure already shown by TauriService
      if (paths.user_zomboid_dir) {
        const remainingPackages = await TauriService.listMergedPackages(paths.user_zomboid_dir, paths.mod_list_ini_path);
        setPackages(remainingPackages);
        if (remainingPackages.length > 0) {
          setSelectedPackageFolder(remainingPackages[0].folder_name);
        } else {
          setSelectedPackageFolder('');
          setIsPackageOpened(false);
          if (onClearConflicts) onClearConflicts();
        }
      }
      if (onRefreshMods) await onRefreshMods();
      showToast(i18next.t('merger.toasts.deleted', { name: folderName }));
    }
  };

  const handleExportPackage = async (folder: string = selectedPackageFolder) => {
    try {
      const cleanName = folder.replace('Z_PZModStudio_', '').toLowerCase();
      const filePath = await TauriService.pickSaveFile(
        `${cleanName}_package.pzmerge`,
        i18next.t('merger.toasts.pzmergeFilter'),
        'pzmerge'
      );
      if (filePath) {
        await TauriService.exportMergedPackage(paths.user_zomboid_dir, folder, filePath);
        showToast(i18next.t('merger.toasts.exported', { path: filePath }));
      }
    } catch (err: any) {
      alert(i18next.t('merger.toasts.exportError', { error: explainError(err) }));
    }
  };

  /** What an imported package found missing or different here, shown before it is published. */
  const [importCheck, setImportCheck] = useState<{ name: string; report: MissingModsReport } | null>(null);

  const handleImportPackage = async () => {
    try {
      const filePath = await TauriService.pickOpenFile(
        i18next.t('merger.toasts.pzmergeFilter'),
        'pzmerge'
      );
      if (filePath) {
        const imported = await TauriService.importMergedPackage(paths.user_zomboid_dir, paths.mod_list_ini_path, filePath);
        if (imported) {
          await fetchPackages();
          setSelectedPackageFolder(imported.folder_name);
          setIsPackageOpened(true);
          onPackageOpened?.(imported.folder_name);
          if (onRefreshMods) await onRefreshMods();
          if (onRescan) await onRescan(imported.folder_name);
          showToast(i18next.t('merger.toasts.imported', { name: imported.display_name }));
          if (importCheckHasNews(imported.version_check)) {
            setImportCheck({ name: imported.display_name, report: imported.version_check });
          }
        }
      }
    } catch (err: any) {
      alert(i18next.t('merger.toasts.importError', { error: explainError(err) }));
    }
  };

  const handleOpenPhysicalFolder = async (folder: string = selectedPackageFolder) => {
    if (paths.user_zomboid_dir && folder) {
      await TauriService.openPackageFolder(paths.user_zomboid_dir, folder);
    }
  };

  const handleCleanMasterPatch = async () => {
    setIsCleaning(true);
    try {
      const cleaned = await TauriService.cleanMasterPatch({
        workshop_dir: paths.workshop_dir,
        pz_install_dir: paths.pz_install_dir,
        user_zomboid_dir: paths.user_zomboid_dir,
        mod_list_ini_path: paths.mod_list_ini_path,
        package_folder_name: selectedPackageFolder,
      });
      if (!cleaned) return; // failure already shown by TauriService
      showToast(i18next.t('merger.toasts.unpacked', { name: selectedPackageFolder }));
      await fetchPatchStatus();
      await fetchPackages();
      if (onRefreshMods) await onRefreshMods();
      await onRescan();
    } catch (err) {
      console.error('Clean failed:', err);
    } finally {
      setIsCleaning(false);
    }
  };

  // Every conflict through the automatic engine, and the package published (App: generateMasterPatch).
  const handleAutoMergeAllClick = async () => {
    // A failure was already shown by TauriService; don't claim success.
    if (onAutoMergeAll && !(await onAutoMergeAll(selectedPackageFolder))) return;
    if (currentConflict) {
      const targetAst = currentConflict.auto_ast_output || currentConflict.merged_output || currentConflict.base_content || '';
      setEditorContent(targetAst);
      setIsManuallyEdited(false);
    }
    showToast(i18next.t('merger.toasts.allMerged'));
  };

  // Step 2: Save Individual Conflict File Edit
  const handleSaveSingleConflict = async () => {
    if (!currentConflict) return;
    if (isPatchTarget(currentConflict)) {
      // An unchanged copy fixes nothing and freezes the file at today's version of the mod; the
      // game would also blame the package for every line the mod's own copy already produced.
      const modsOwn = currentConflict.competing_mods[0]?.content;
      if (modsOwn !== undefined && editorContent === modsOwn) {
        showToast(i18next.t('merger.toasts.nothingChanged', { path: currentConflict.relative_path }));
        return;
      }
      try {
        const published = await savePatchTarget(currentConflict.id, currentConflict.relative_path, editorContent);
        setIsManuallyEdited(false);
        showToast(
          published
            ? i18next.t('merger.toasts.filePublished', { path: currentConflict.relative_path })
            : i18next.t('merger.toasts.fileSaved', { path: currentConflict.relative_path })
        );
      } catch {
        // Already shown by TauriService.
      }
      return;
    }
    onResolveConflict(currentConflict.id, editorContent, selectedPackageFolder, currentConflict.relative_path);
    setIsManuallyEdited(false);
    showToast(i18next.t('merger.toasts.mergeSaved', { path: currentConflict.relative_path }));
  };

  const escapeRegExp = (str: string) => str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Step 3b: Toggle Append vs Remove Mod block in editor content
  const handleToggleAddMod = (modContent: string, modUid: string) => {
    const blockStart = `-- === MOD: [${modUid}] ===`;
    const blockEnd = `-- === END MOD: [${modUid}] ===`;

    if (editorContent.includes(blockStart)) {
      // REMOVE mod block
      const regex = new RegExp(`\\n*${escapeRegExp(blockStart)}[\\s\\S]*?${escapeRegExp(blockEnd)}\\n*`, 'g');
      const newContent = editorContent.replace(regex, '\n\n').trim();
      setEditorContent(newContent || currentConflict?.base_content || '');
      setIsManuallyEdited(true);
      showToast(i18next.t('merger.toasts.codeRemoved', { mod: modUid }));
    } else {
      // ADD mod block
      const formatted = `\n\n${blockStart}\n${modContent}\n${blockEnd}`;
      setEditorContent((prev) => {
        const trimmed = prev.trim();
        if (!trimmed || (currentConflict && trimmed === currentConflict.base_content.trim())) {
          return `${currentConflict?.base_content || ''}${formatted}`;
        }
        return `${prev}${formatted}`;
      });
      setIsManuallyEdited(true);
      showToast(i18next.t('merger.toasts.codeAdded', { mod: modUid }));
    }
  };

  // Step 4: Reset to Automatic AST Merge
  /**
   * What this file will contain. The editor is still there for anything else, but the four
   * answers people actually want are one click each, and the active one is obvious.
   */
  /** False when the scan put a note in place of a file the base game does not have. */
  const hasVanillaBase =
    !!currentConflict &&
    !currentConflict.base_content.startsWith('-- vanilla file not present') &&
    !currentConflict.base_content.startsWith('-- vanilla file unreadable');

  const fileChoices = useMemo<Choice[]>(() => {
    if (!currentConflict) return [];
    const vanillaMissing = !hasVanillaBase;
    const merged = currentConflict.auto_ast_output || currentConflict.merged_output || '';
    const isDefinition = currentConflict.file_type === 'SCRIPT_DEFINITION' || currentConflict.file_type === 'TRANSLATION';
    const choices: Choice[] = [];
    if (merged) {
      choices.push({
        key: 'merged',
        label: isDefinition ? i18next.t('merger.choices.mergeThem') : i18next.t('merger.choices.keepBoth'),
        help: isDefinition ? i18next.t('merger.choices.mergeThemHelp') : i18next.t('merger.choices.keepBothHelp'),
        content: merged,
      });
    }
    for (const mod of currentConflict.competing_mods) {
      choices.push({
        key: `mod:${mod.mod_id}`,
        label: i18next.t('merger.choices.only', { mod: mod.mod_name }),
        help: isDefinition ? i18next.t('merger.choices.onlyDefinitionHelp', { mod: mod.mod_name }) : i18next.t('merger.choices.onlyFileHelp', { mod: mod.mod_name }),
        content: mod.content,
      });
    }
    if (!vanillaMissing) {
      choices.push({
        key: 'vanilla',
        label: i18next.t('merger.choices.vanilla'),
        help: i18next.t('merger.choices.vanillaHelp'),
        content: currentConflict.base_content,
      });
    }
    return choices;
  }, [currentConflict]);

  /** Which of them the editor is showing right now, if any. */
  const activeChoice = useMemo(() => {
    const same = (a: string, b: string) => a.replace(/\r\n/g, '\n').trim() === b.replace(/\r\n/g, '\n').trim();
    return fileChoices.find((c) => same(c.content, editorContent))?.key ?? 'custom';
  }, [fileChoices, editorContent]);

  const chooseForFile = (choice: Choice) => {
    setEditorContent(choice.content);
    setIsManuallyEdited(true);
    showToast(i18next.t('merger.toasts.chosen', { choice: choice.label, path: currentConflict?.relative_path ?? '' }));
  };

  const handleResetToAutoMerge = () => {
    if (!currentConflict) return;
    const targetAst = currentConflict.auto_ast_output || currentConflict.merged_output || currentConflict.base_content || '';
    setEditorContent(targetAst);
    setIsManuallyEdited(true);
    showToast(i18next.t('merger.toasts.astRestored', { path: currentConflict.relative_path }));
  };

  // Step 5: Final Packaging on Disk
  const handleGenerateMasterPatch = async () => {
    setIsGenerating(true);
    try {
      if (!(await onOptimizeAndResolve(selectedPackageFolder))) return;
      showToast(i18next.t('merger.toasts.publishedIn', { folder: selectedPackageFolder }));
      await fetchPatchStatus();
      await fetchPackages();
      if (onRefreshMods) await onRefreshMods();
    } catch (err) {
      console.error('Generate failed:', err);
    } finally {
      setIsGenerating(false);
    }
  };

  const [scanProgress, setScanProgress] = useState<{ percent: number; text: string } | null>(null);

  const handleRescanClick = async () => {
    setIsRescanning(true);

    setScanProgress({ percent: 15, text: i18next.t('merger.toasts.scan1') });
    await new Promise((r) => setTimeout(r, 150));

    setScanProgress({ percent: 35, text: i18next.t('merger.toasts.scan2') });
    await onRescan();

    setScanProgress({ percent: 65, text: i18next.t('merger.toasts.scan3') });
    await new Promise((r) => setTimeout(r, 150));

    setScanProgress({ percent: 90, text: i18next.t('merger.toasts.scan4') });
    await new Promise((r) => setTimeout(r, 150));

    setScanProgress({ percent: 100, text: i18next.t('merger.toasts.scan5') });

    setTimeout(() => {
      setIsRescanning(false);
      setScanProgress(null);
      showToast(i18next.t('merger.toasts.scanDone'));
    }, 400);
  };

  // Handle vertical panel dragging
  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  };

  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isResizing || !containerRef.current) return;
      const containerRect = containerRef.current.getBoundingClientRect();
      const relativeY = e.clientY - containerRect.top;
      const newPercent = (relativeY / containerRect.height) * 100;
      setTopHeightPercent(Math.max(20, Math.min(80, newPercent)));
    },
    [isResizing]
  );

  const handleMouseUp = useCallback(() => {
    setIsResizing(false);
  }, []);

  useEffect(() => {
    if (isResizing) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
    } else {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    }
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing, handleMouseMove, handleMouseUp]);

  const conflictLine = currentConflict?.conflict_line ?? 1;
  // Vanilla line every pane is scrolled to: the first real conflict, else the first change.
  const [focusBaseLine, setFocusBaseLine] = useState<number | null>(null);
  useEffect(() => setFocusBaseLine(null), [currentConflict?.id]);
  const defaultFocus =
    currentConflict?.collisions?.[0]?.base_start ?? currentConflict?.mod_changes?.[0]?.regions?.[0]?.base_start ?? conflictLine;
  const focusLine = focusBaseLine ?? defaultFocus;
  const collisionBaseRanges = useMemo<Array<[number, number]>>(
    () => (currentConflict?.collisions ?? []).map((c) => [c.base_start, c.base_end]),
    [currentConflict]
  );
  const changedBaseRanges = useMemo<Array<[number, number]>>(
    () => (currentConflict?.mod_changes ?? []).flatMap((m) => m.regions.filter((r) => r.kind !== 'added').map((r) => [r.base_start, r.base_end] as [number, number])),
    [currentConflict]
  );
  const totalFileLines = currentConflict?.total_file_lines ?? currentConflict?.base_content.split('\n').length ?? 1;

  const handleSelectConflict = (c: VfsConflict) => {
    if (isManuallyEdited && !confirm(i18next.t('merger.toasts.unsaved'))) return;
    setSelectedConflictId(c.id);
    setEditorContent(c.merged_output || c.base_content);
    setIsManuallyEdited(false);
  };

  // Opened from Mod Health on one file: open the package the game uses (or the default one),
  // scan, then select that conflict.
  const focusStartedRef = useRef<string | null>(null);
  const [focusScanDone, setFocusScanDone] = useState(false);
  /** The conflict Mod Health sent, lit in the list for a moment. */
  const [flashConflictId, setFlashConflictId] = useState<string | null>(null);
  useEffect(() => {
    if (!focusRelativePath) {
      focusStartedRef.current = null;
      return;
    }
    const hit = conflicts.find((c) => c.relative_path === focusRelativePath);
    if (hit && isPackageOpened) {
      setPeekConflictId(hit.id);
      handleSelectConflict(hit);
      // Where it is in the list: scrolled to and lit for a moment, and never hidden by a filter.
      setFilterNoise(false);
      setFlashConflictId(hit.id);
      window.setTimeout(() => setFlashConflictId((id) => (id === hit.id ? null : id)), 2500);
      setFocusScanDone(false);
      onFocusHandled?.();
      return;
    }
    if (focusStartedRef.current === focusRelativePath) {
      if (focusScanDone) {
        showToast(i18next.t('merger.toasts.noLongerConflict', { path: focusRelativePath }));
        setFocusScanDone(false);
        onFocusHandled?.();
      }
      return;
    }
    if (!packagesLoaded) return; // pick the right package once the list is in
    focusStartedRef.current = focusRelativePath;
    const target =
      packages.find((p) => !p.is_companion_bridge && p.is_active_in_modlist && p.is_packaged) ??
      packages.find((p) => !p.is_companion_bridge);
    (async () => {
      if (!isPackageOpened) await handleOpenPackage(target?.folder_name ?? 'Z_PZModStudio_MergedPatch');
      else await onRescan();
      setFocusScanDone(true);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusRelativePath, conflicts, isPackageOpened, focusScanDone, packagesLoaded]);

  /**
   * The Monitor sends a mod to patch: the package it goes into is opened (the same choice as a
   * conflict sent from Mod Health), then either the file the log named opens in the editor, or
   * the mod's files are listed to pick from.
   */
  const patchFocusStartedRef = useRef<string | null>(null);
  useEffect(() => {
    if (!patchFocus) {
      patchFocusStartedRef.current = null;
      return;
    }
    const key = `${patchFocus.modId}|${patchFocus.fileHint ?? ''}`;
    if (patchFocusStartedRef.current === key) return;
    if (!packagesLoaded) return;
    patchFocusStartedRef.current = key;
    (async () => {
      try {
        // The package is opened at once and the file right after; the scan that an opened
        // package starts runs behind them, so the editor is in front of you before it ends.
        const folder = packageForPatch(packages);
        if (!isPackageOpened) {
          setSelectedPackageFolder(folder);
          setIsPackageOpened(true);
          onPackageOpened?.(folder);
          void handleOpenPackage(folder);
        }
        const files = await TauriService.modFiles(paths, patchFocus.modId);
        const hint = (patchFocus.fileHint ?? '').toLowerCase();
        const named = hint
          ? files.find((f) => {
              const p = f.relative_path.toLowerCase();
              return p === hint || p.endsWith('/' + hint);
            })
          : undefined;
        if (named) {
          await openModFileToPatch(patchFocus.modId, named.relative_path, patchFocus.fix);
        } else {
          setPatchPicker({ open: true, modId: patchFocus.modId });
          if (hint) showToast(i18next.t('merger.toasts.notReadFile', { file: patchFocus.fileHint }));
        }
      } catch (err) {
        alert(explainError(err));
      } finally {
        onPatchFocusHandled?.();
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [patchFocus, packagesLoaded, isPackageOpened]);

  if (!paths.is_valid) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-6 bg-slate-950 text-slate-200">
        <div className="max-w-md w-full bg-slate-900/80 border border-amber-500/40 rounded-2xl p-6 text-center space-y-4 shadow-xl">
          <div className="w-12 h-12 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/30 flex items-center justify-center mx-auto">
            <FolderX className="w-6 h-6" />
          </div>
          <div>
            <h3 className="text-base font-bold text-slate-100">{i18next.t('merger.setup.title')}</h3>
            <p className="text-xs text-slate-400 mt-1 leading-relaxed">
              {i18next.t('merger.setup.blurb')}
            </p>
          </div>
          <button
            onClick={onGoToSettings}
            className="w-full flex items-center justify-center gap-2 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white font-medium text-xs py-2.5 rounded-lg shadow transition cursor-pointer"
          >
            {i18next.t('merger.setup.button')}
          </button>
        </div>
      </div>
    );
  }


  const currentPackage = packages.find((p) => p.folder_name === selectedPackageFolder) || packages[0];
  const allResolved = conflicts.length > 0 && conflicts.every((c) => isSettled(c.status));
  const isCurrentlyPackaged = patchStatus?.is_packaged ?? currentPackage?.is_packaged ?? false;
  // A packaged package is read-only, except for a mod's own file opened to patch: that one is
  // edited and published in place, since unpacking for it would empty the package in the game.
  // A packaged package hides its files until unpacked -- except the one Mod Health or the Monitor
  // sent the player to, which is shown read-only, with Save publishing in place like a patch.
  const [peekConflictId, setPeekConflictId] = useState<string | null>(null);
  const editorLocked = isCurrentlyPackaged && !isPatchTarget(currentConflict) && currentConflict?.id !== peekConflictId;

  return (
    <div className="flex-1 flex flex-col overflow-hidden bg-slate-950 text-slate-200 select-none">
      <PatchFilePicker
        open={patchPicker.open}
        initialModId={patchPicker.modId}
        paths={paths}
        mods={mods}
        onClose={() => setPatchPicker({ open: false, modId: null })}
        onPick={(modId, relativePath) => void openModFileToPatch(modId, relativePath)}
      />

      {toastMessage && (
        <div className="bg-gradient-to-r from-cyan-900/90 to-emerald-900/90 border-b border-cyan-500/50 text-cyan-100 text-xs px-4 py-2 flex items-center justify-between animate-fade-in shadow-md shrink-0">
          <div className="flex items-center gap-2">
            <Sparkle className="w-4 h-4 text-cyan-300 animate-spin shrink-0" />
            <span>{toastMessage}</span>
          </div>
          <button onClick={() => setToastMessage(null)} className="text-slate-400 hover:text-white text-xs">
            ✕
          </button>
        </div>
      )}

      {/* LEVEL 1: Overview & Selection (When no package is opened) */}
      {!isPackageOpened ? (
        <PackageList
          packages={shownPackages}
          activeProfileName={activeProfile?.name ?? null}
          otherProfilesCount={otherProfilesCount}
          showOtherProfiles={showOtherProfilesPackages}
          onToggleOtherProfiles={() => setShowOtherProfilesPackages((v) => !v)}
          ownersOf={(folder) => ownersByPackage.get(folder) ?? []}
          isInActiveProfile={inActiveProfile}
          selectedFolder={selectedPackageFolder}
          onSelect={(folder) => {
            setSelectedPackageFolder(folder);
            fetchPatchStatus();
          }}
          onOpen={handleOpenPackage}
          onRename={setRenamingPackage}
          onDelete={handleDeletePackage}
          onToggleInModlist={handleTogglePackageInModlist}
          onExport={handleExportPackage}
          onOpenFolder={handleOpenPhysicalFolder}
          onImport={handleImportPackage}
          onCreate={() => setIsCreateModalOpen(true)}
          onCreateForProfile={
            activeProfile
              ? () => {
                  // A folder name cannot hold these; the package's name is the profile's otherwise.
                  const name = activeProfile.name.replace(/[<>:"/\\|?*]/g, '').trim().replace(/\.+$/, '');
                  if (name) void handleCreatePackage(name, i18next.t('merger.list.forProfileDescription', { name: activeProfile.name }));
                }
              : undefined
          }
        />
      ) : (
        /* LEVEL 2: Workspace of the Opened Package */
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Top Bar for Level 2 Workspace */}
          <div className="bg-slate-900 border-b border-slate-800 p-2.5 flex items-center justify-between gap-4 shrink-0 shadow-md">
            <div className="flex items-center gap-3">
              <button
                onClick={handleClosePackage}
                className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-bold rounded-lg transition shadow cursor-pointer"
                title={i18next.t('merger.workspace.backTitle')}
              >
                <ArrowLeft className="w-3.5 h-3.5 text-cyan-400" />
                <span>{i18next.t('merger.workspace.back')}</span>
              </button>

              {/* Interactive Package Switcher Dropdown */}
              <div className="flex items-center gap-1.5 bg-slate-800/90 border border-slate-700/80 rounded-lg px-2.5 py-1 shadow-sm">
                <Package className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                <span className="text-[10px] text-slate-400 font-semibold uppercase tracking-wider">{i18next.t('merger.workspace.package')}</span>
                <select
                  value={selectedPackageFolder}
                  onChange={(e) => handleOpenPackage(e.target.value)}
                  className="bg-transparent text-xs font-bold text-emerald-300 focus:outline-none cursor-pointer pr-1"
                >
                  {packages.filter((p) => !p.is_companion_bridge).map((p) => (
                    <option key={p.folder_name} value={p.folder_name} className="bg-slate-900 text-slate-200">
                      {p.display_name} {p.is_packaged ? i18next.t('merger.workspace.optPackaged') : i18next.t('merger.workspace.optDraft')} {p.is_active_in_modlist ? i18next.t('merger.workspace.optVisible') : i18next.t('merger.workspace.optHidden')}
                    </option>
                  ))}
                </select>
              </div>

              {currentPackage?.is_companion_bridge && (
                <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-cyan-500/20 text-cyan-300 border border-cyan-500/40">
                  {currentPackage.is_studio_link ? i18next.t('merger.workspace.link') : i18next.t('merger.workspace.companion')}
                </span>
              )}

              {isCurrentlyPackaged ? (
                <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-emerald-950/90 text-emerald-300 border border-emerald-600 shadow-sm" title={i18next.t('merger.list.packagedTitle')}>
                  {i18next.t('merger.workspace.packaged')}
                </span>
              ) : (
                <span className="text-[10px] font-bold px-2 py-0.5 rounded bg-amber-950/90 text-amber-300 border border-amber-600 shadow-sm animate-pulse" title={i18next.t('merger.workspace.openDraftTitle')}>
                  {i18next.t('merger.list.openDraft')}
                </span>
              )}

              {/* ModList Visibility Toggle Button in Level 2 Top Bar */}
              {currentPackage && (
                <button
                  onClick={() => handleTogglePackageInModlist(currentPackage)}
                  className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-[11px] font-bold border transition shadow cursor-pointer ${
                    currentPackage.is_active_in_modlist
                      ? 'bg-emerald-950/80 text-emerald-300 border-emerald-600/70 hover:bg-emerald-900'
                      : 'bg-slate-800 text-slate-400 border-slate-700 hover:bg-slate-700 hover:text-slate-200'
                  }`}
                  title={currentPackage.is_active_in_modlist ? i18next.t('merger.workspace.visibleTitle') : i18next.t('merger.workspace.hiddenTitle')}
                >
                  {currentPackage.is_active_in_modlist ? <Eye className="w-3 h-3 text-emerald-400" /> : <EyeOff className="w-3 h-3 text-slate-400" />}
                  <span>{currentPackage.is_active_in_modlist ? i18next.t('merger.list.visible') : i18next.t('merger.list.hidden')}</span>
                </button>
              )}
            </div>

            <div className="flex items-center gap-2 shrink-0">
              {!isCurrentlyPackaged && (
                <>
                  <button
                    onClick={() => {
                      setPatchPicker({ open: true, modId: null });
                    }}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-bold rounded-lg transition shadow bg-slate-900 hover:bg-slate-800 text-slate-200 border border-slate-700 cursor-pointer"
                    title={i18next.t('merger.workspace.patchTitle')}
                  >
                    <Wrench className="w-3.5 h-3.5 text-amber-400" />
                    {i18next.t('merger.workspace.patch')}
                  </button>

                  <button
                    onClick={handleAutoMergeAllClick}
                    disabled={!hasPendingConflicts}
                    className={`flex items-center gap-1.5 px-3 py-1.5 text-xs font-bold rounded-lg transition shadow ${
                      !hasPendingConflicts
                        ? 'bg-slate-900 border border-slate-800 text-slate-600 cursor-not-allowed opacity-50'
                        : 'bg-emerald-950/90 hover:bg-emerald-900 text-emerald-300 border border-emerald-700/60 cursor-pointer'
                    }`}
                    title={!hasPendingConflicts ? i18next.t('merger.workspace.allMergedTitle') : i18next.t('merger.workspace.mergeAllTitle')}
                  >
                    <Wand2 className={`w-3.5 h-3.5 ${!hasPendingConflicts ? 'text-slate-600' : 'text-emerald-400'}`} />
                    <span>{i18next.t('merger.workspace.mergeAll')}</span>
                  </button>

                  <button
                    onClick={handleUnmergeAll}
                    disabled={!hasAnyMergedConflicts}
                    className={`flex items-center gap-1.5 px-3 py-1.5 border text-xs font-semibold rounded-lg transition shadow ${
                      !hasAnyMergedConflicts
                        ? 'bg-slate-900 border-slate-800 text-slate-600 cursor-not-allowed opacity-50'
                        : 'bg-slate-800 hover:bg-slate-700 text-slate-300 border-slate-700 cursor-pointer'
                    }`}
                    title={!hasAnyMergedConflicts ? i18next.t('merger.workspace.noMergesTitle') : i18next.t('merger.workspace.unmergeAllTitle')}
                  >
                    <Undo2 className={`w-3.5 h-3.5 ${!hasAnyMergedConflicts ? 'text-slate-600' : 'text-amber-400'}`} />
                    <span>{i18next.t('merger.workspace.unmergeAll')}</span>
                  </button>
                </>
              )}

              <button
                onClick={() => setFilterNoise(!filterNoise)}
                className={`px-2.5 py-1.5 text-xs font-semibold rounded-lg flex items-center gap-1.5 border transition cursor-pointer shadow ${
                  filterNoise
                    ? 'bg-emerald-950/80 text-emerald-300 border-emerald-700/80 font-bold'
                    : 'bg-slate-800 hover:bg-slate-700 text-slate-300 border-slate-700'
                }`}
                title={filterNoise ? i18next.t('merger.workspace.noiseOnTitle') : i18next.t('merger.workspace.noiseOffTitle')}
              >
                {filterNoise ? <EyeOff className="w-3.5 h-3.5 text-emerald-400" /> : <Eye className="w-3.5 h-3.5 text-slate-400" />}
                <span>{filterNoise ? i18next.t('merger.workspace.noDuplicates') : i18next.t('merger.workspace.all')}</span>
              </button>

              <button
                onClick={() => handleOpenPhysicalFolder()}
                className="p-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-bold rounded-lg transition shadow cursor-pointer"
                title={i18next.t('merger.workspace.folderTitle')}
              >
                <FolderOpen className="w-4 h-4 text-amber-400" />
              </button>

              <button
                onClick={() => handleExportPackage()}
                className="p-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-bold rounded-lg transition shadow cursor-pointer"
                title={i18next.t('merger.workspace.exportTitle')}
              >
                <Download className="w-4 h-4 text-emerald-400" />
              </button>

              <button
                onClick={handleRescanClick}
                disabled={isCurrentlyPackaged || isRescanning}
                className={`p-1.5 border text-xs font-medium rounded-lg transition shadow ${
                  isCurrentlyPackaged || isRescanning
                    ? 'bg-slate-900 border-slate-800 text-slate-600 cursor-not-allowed opacity-50'
                    : 'bg-slate-800 hover:bg-slate-700 text-slate-200 border-slate-700 cursor-pointer'
                }`}
                title={isCurrentlyPackaged ? i18next.t('merger.workspace.rescanPackagedTitle') : i18next.t('merger.workspace.rescanTitle')}
              >
                <RefreshCw className={`w-4 h-4 ${isCurrentlyPackaged ? 'text-slate-600' : 'text-cyan-400'} ${isRescanning ? 'animate-spin' : ''}`} />
              </button>

              {isCurrentlyPackaged ? (
                <button
                  onClick={() => setIsUnpackageModalOpen(true)}
                  disabled={isCleaning}
                  className="flex items-center gap-1.5 px-4 py-1.5 bg-amber-950/80 hover:bg-amber-900 text-amber-300 border border-amber-700/60 text-xs font-bold rounded-lg transition shadow cursor-pointer"
                  title={i18next.t('merger.workspace.unpackOpenTitle')}
                >
                  <Unlock className="w-4 h-4 text-amber-400" />
                  <span>{isCleaning ? i18next.t('merger.editor.opening') : i18next.t('merger.editor.unmergeOpen')}</span>
                </button>
              ) : (
                <button
                  onClick={handleGenerateMasterPatch}
                  disabled={isGenerating}
                  className={`flex items-center gap-1.5 px-4 py-1.5 text-xs font-bold rounded-lg transition shadow-lg shrink-0 cursor-pointer ${
                    allResolved
                      ? 'bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white animate-pulse'
                      : 'bg-emerald-600 hover:bg-emerald-500 text-white'
                  }`}
                  title={i18next.t('merger.workspace.publishTitle')}
                >
                  <PackageCheck className={`w-4 h-4 ${isGenerating ? 'animate-spin' : ''}`} />
                  <span>{isGenerating ? i18next.t('merger.workspace.packaging') : i18next.t('merger.list.publish')}</span>
                </button>
              )}
            </div>
          </div>

          {/* Read-Only Status Banner when isCurrentlyPackaged */}
          {isCurrentlyPackaged && (
            <div className="bg-emerald-950/40 border-b border-emerald-800/50 px-4 py-2 flex items-center justify-between gap-3 shrink-0 text-xs animate-fade-in">
              <div className="flex items-center gap-2 text-emerald-300">
                <PackageCheck className="w-4 h-4 text-emerald-400 shrink-0" />
                <span dangerouslySetInnerHTML={{ __html: tHtml('merger.workspace.packagedBanner') }} />
              </div>
              <button
                onClick={() => setIsUnpackageModalOpen(true)}
                disabled={isCleaning}
                className="px-3 py-1 bg-amber-600 hover:bg-amber-500 text-white rounded-lg font-bold text-[11px] transition shadow-md cursor-pointer shrink-0 flex items-center gap-1.5"
                title={i18next.t('merger.workspace.unpackEditTitle')}
              >
                <Unlock className="w-3.5 h-3.5" />
                <span>{isCleaning ? i18next.t('merger.editor.opening') : i18next.t('merger.workspace.unpackEdit')}</span>
              </button>
            </div>
          )}

          {/* Main Body: If package is closed/packaged, hide editor content until unlocked */}
          {editorLocked ? (
            <div className="flex-1 flex flex-col items-center justify-center p-8 bg-slate-950/80 overflow-y-auto text-slate-200 animate-fade-in">
              <div className="max-w-xl w-full bg-slate-900/90 border border-emerald-500/30 rounded-2xl p-8 shadow-2xl space-y-6 text-center">
                <div className="w-20 h-20 mx-auto rounded-3xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shadow-inner">
                  <PackageCheck className="w-10 h-10" />
                </div>

                <div className="space-y-2">
                  <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full text-xs font-bold bg-emerald-950/90 text-emerald-300 border border-emerald-700/60">
                    <ShieldCheck className="w-4 h-4 text-emerald-400" />
                    <span>{i18next.t('merger.workspace.closedBadge')}</span>
                  </div>
                  <h2 className="text-2xl font-bold text-slate-100 font-mono">
                    {currentPackage?.display_name || selectedPackageFolder}
                  </h2>
                  <p className="text-xs text-slate-400 max-w-md mx-auto leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('merger.workspace.closedBlurb') }} />
                </div>

                {/* Summary Info Cards */}
                <div className="grid grid-cols-2 gap-3 text-left">
                  <div className="bg-slate-950/90 border border-slate-800 rounded-xl p-3.5 space-y-1">
                    <span className="text-[11px] font-medium text-slate-400 flex items-center gap-1.5">
                      <FileCode className="w-3.5 h-3.5 text-cyan-400" />
                      {i18next.t('merger.workspace.mergedFiles')}
                    </span>
                    <p className="text-lg font-bold text-emerald-400 font-mono">
                      {i18next.t('merger.workspace.files', { count: conflicts.length })}
                    </p>
                  </div>
                  <div className="bg-slate-950/90 border border-slate-800 rounded-xl p-3.5 space-y-1">
                    <span className="text-[11px] font-medium text-slate-400 flex items-center gap-1.5">
                      <Layers className="w-3.5 h-3.5 text-emerald-400" />
                      {i18next.t('merger.workspace.stateInModList')}
                    </span>
                    <p className="text-lg font-bold text-cyan-400">
                      {currentPackage?.is_active_in_modlist ? i18next.t('merger.workspace.activeVisible') : i18next.t('merger.workspace.hidden')}
                    </p>
                  </div>
                </div>

                {/* Open / Unpackage Action CTA */}
                <div className="pt-2 space-y-3">
                  <button
                    onClick={() => setIsUnpackageModalOpen(true)}
                    disabled={isCleaning}
                    className="w-full py-3.5 px-6 bg-gradient-to-r from-amber-600 via-amber-500 to-orange-600 hover:from-amber-500 hover:to-orange-500 text-white font-bold text-sm rounded-xl shadow-lg hover:shadow-amber-500/25 transition-all flex items-center justify-center gap-2.5 cursor-pointer"
                  >
                    <Unlock className="w-5 h-5 text-amber-100" />
                    <span>{isCleaning ? i18next.t('merger.workspace.openingPackage') : i18next.t('merger.editor.unmergeOpen')}</span>
                  </button>
                  <p className="text-[11px] text-slate-500">
                    {i18next.t('merger.workspace.openBlurb')}
                  </p>
                </div>
              </div>
            </div>
          ) : (
            /* Workspace Body */
            <div className="flex-1 flex overflow-hidden">
              <div className="w-80 border-r border-slate-800 flex flex-col bg-slate-900/50 shrink-0">
                <div className="p-3 border-b border-slate-800 flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-semibold text-slate-300">
                    <GitCompare className="w-4 h-4 text-emerald-400" />
                    <span>{i18next.t('merger.workspace.vfsConflicts')}</span>
                  </div>
                  <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-slate-800 text-cyan-400 border border-slate-700">
                    {i18next.t('merger.workspace.files', { count: filteredConflicts.length })}
                  </span>
                </div>

                {isRescanning && scanProgress && (
                  <div className="p-3 bg-cyan-950/80 border-b border-cyan-700/60 space-y-2 animate-fade-in shadow-lg">
                    <div className="flex items-center justify-between text-[11px] font-mono font-bold text-cyan-300">
                      <span className="flex items-center gap-1.5 truncate">
                        <RefreshCw className="w-3.5 h-3.5 text-cyan-400 animate-spin shrink-0" />
                        <span className="truncate">{scanProgress.text}</span>
                      </span>
                      <span className="shrink-0">{scanProgress.percent}%</span>
                    </div>
                    <div className="w-full bg-slate-900 rounded-full h-1.5 overflow-hidden border border-cyan-800">
                      <div
                        className="bg-gradient-to-r from-cyan-500 to-emerald-400 h-full transition-all duration-300 rounded-full"
                        style={{ width: `${scanProgress.percent}%` }}
                      />
                    </div>
                  </div>
                )}

                <div className="flex-1 overflow-y-auto p-2 space-y-1">
                  {filteredConflicts.length === 0 ? (
                    <div className="p-6 text-center text-xs text-slate-500 space-y-2">
                      <ShieldCheck className="w-8 h-8 text-emerald-500/50 mx-auto" />
                      <p>{i18next.t('merger.workspace.noPending')}</p>
                    </div>
                  ) : (
                    filteredConflicts.map((c) => {
                      const isSelected = c.id === selectedConflictId;
                      const isResolved = c.status === 'RESOLVED';
                      const isAutoMerged = c.status === 'AUTO_MERGED';

                      return (
                        <div
                          key={c.id}
                          ref={c.id === flashConflictId ? (el) => el?.scrollIntoView({ block: 'center', behavior: 'smooth' }) : undefined}
                          onClick={() => handleSelectConflict(c)}
                          className={`p-2.5 rounded-lg border text-xs cursor-pointer transition-all ${
                            c.id === flashConflictId
                              ? 'bg-amber-950/40 border-amber-400 shadow-sm ring-2 ring-amber-400/60'
                              : isSelected
                              ? 'bg-slate-800 border-emerald-500/70 shadow-sm ring-1 ring-emerald-500/30'
                              : 'bg-slate-900/80 border-slate-800 hover:border-slate-700'
                          }`}
                        >
                          <div className="flex items-center justify-between mb-1">
                            <span className="font-mono text-[10px] px-1.5 py-0.5 rounded bg-slate-950 text-slate-400 border border-slate-800">
                              {c.file_type === 'SCRIPT_DEFINITION' ? i18next.t('merger.workspace.kindDefinition') : c.file_type === 'TRANSLATION' ? i18next.t('merger.workspace.kindWords') : c.file_type}
                            </span>
                            {isResolved ? (
                              <span className="flex items-center gap-1 text-[10px] text-emerald-400 font-bold bg-emerald-950/80 px-1.5 py-0.5 rounded border border-emerald-800/80">
                                <CheckCircle2 className="w-3 h-3 text-emerald-400" /> {i18next.t('merger.workspace.resolved')}
                              </span>
                            ) : isAutoMerged ? (
                              <span className="flex items-center gap-1 text-[10px] text-cyan-300 font-bold bg-cyan-950/80 px-1.5 py-0.5 rounded border border-cyan-800/80">
                                <Sparkle className="w-3 h-3 text-cyan-400" /> {i18next.t('merger.workspace.autoMerged')}
                              </span>
                            ) : (
                              <span className="flex items-center gap-1 text-[10px] text-amber-400 font-medium bg-amber-950/60 px-1.5 py-0.5 rounded border border-amber-800/60">
                                <AlertTriangle className="w-3 h-3 text-amber-400" /> {i18next.t('merger.workspace.needsReview')}
                              </span>
                            )}
                          </div>
                          <div className="font-mono text-slate-200 font-semibold truncate" title={c.relative_path}>
                            {c.relative_path.split('/').pop()}
                          </div>
                        </div>
                      );
                    })
                  )}
                </div>
              </div>

              {currentConflict ? (
                <div ref={containerRef} className="flex-1 flex flex-col overflow-hidden min-w-0">
                  <div
                    style={{ height: `${topHeightPercent}%` }}
                    className="flex flex-col bg-slate-900/80 p-3 min-h-0 overflow-hidden"
                  >
                    {/* Header Toolbar */}
                    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-800 pb-2 mb-2 shrink-0">
                      <div className="flex items-center gap-2 overflow-hidden max-w-xl">
                        <FileCode className="w-4 h-4 text-cyan-400 shrink-0" />
                        <span className="text-xs font-bold text-slate-200 truncate">
                          {i18next.t('merger.workspace.collidingFile')} <code className="text-emerald-400 font-mono">{currentConflict.relative_path}</code>
                        </span>
                      </div>

                      <div className="flex items-center gap-2 shrink-0">
                        <span className="flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-full bg-amber-500/20 text-amber-300 border border-amber-500/40 font-bold">
                          <AlertCircle className="w-3 h-3 text-amber-400" />
                          {i18next.t('merger.workspace.line', { line: conflictLine })}
                        </span>

                        <span className="flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-full bg-slate-950 text-slate-400 border border-slate-800">
                          <Layers className="w-3 h-3 text-cyan-400" />
                          {i18next.t('merger.workspace.sources', { count: currentConflict.competing_mods.length + 1 })}
                        </span>
                      </div>
                    </div>

                    {/* What each mod does to this file */}
                    <ConflictSummaryCard conflict={currentConflict} onFocus={setFocusBaseLine} />

                    <CodePanels
                      currentConflict={currentConflict}
                      totalFileLines={totalFileLines}
                      hasVanillaBase={hasVanillaBase}
                      editorContent={editorContent}
                      isManuallyEdited={isManuallyEdited}
                      setEditorContent={setEditorContent}
                      setIsManuallyEdited={setIsManuallyEdited}
                      showToast={showToast}
                      focusLine={focusLine}
                      changedBaseRanges={changedBaseRanges}
                      collisionBaseRanges={collisionBaseRanges}
                      isCurrentlyPackaged={isCurrentlyPackaged}
                      handleToggleAddMod={handleToggleAddMod}
                    />
                </div>

                {/* Draggable Vertical Splitter Bar */}
                <div
                  onMouseDown={handleMouseDown}
                  className={`h-2.5 bg-slate-900 hover:bg-emerald-600/60 border-y border-slate-800 cursor-ns-resize flex items-center justify-center transition shrink-0 select-none ${
                    isResizing ? 'bg-emerald-500 text-white' : 'text-slate-600 hover:text-white'
                  }`}
                  title={i18next.t('merger.workspace.dragTitle')}
                >
                  <GripHorizontal className="w-5 h-3" />
                </div>

                <EditorPanel
                  topHeightPercent={topHeightPercent}
                  currentConflict={currentConflict}
                  isCurrentlyPackaged={isCurrentlyPackaged}
                  fileChoices={fileChoices}
                  activeChoice={activeChoice}
                  chooseForFile={chooseForFile}
                  editorLocked={editorLocked}
                  isManuallyEdited={isManuallyEdited}
                  setIsUnpackageModalOpen={setIsUnpackageModalOpen}
                  isCleaning={isCleaning}
                  handleResetToAutoMerge={handleResetToAutoMerge}
                  isCurrentConflictAstApplied={isCurrentConflictAstApplied}
                  handleUnmergeSingleConflict={handleUnmergeSingleConflict}
                  isCurrentConflictUnmerged={isCurrentConflictUnmerged}
                  handleSaveSingleConflict={handleSaveSingleConflict}
                  isCurrentConflictSaved={isCurrentConflictSaved}
                  editorContent={editorContent}
                  setEditorContent={setEditorContent}
                  setIsManuallyEdited={setIsManuallyEdited}
                />
              </div>
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center text-slate-500 bg-slate-950/50">
                <GitCompare className="w-12 h-12 mb-2 text-slate-700" />
              </div>
            )}
          </div>
        )}
      </div>
    )}

      {importCheck && (
        <ImportCheck packageName={importCheck.name} report={importCheck.report} paths={paths} onClose={() => setImportCheck(null)} />
      )}
      <CreatePackageModal open={isCreateModalOpen} onClose={() => setIsCreateModalOpen(false)} onCreate={handleCreatePackage} />
      <RenamePackageModal pkg={renamingPackage} onClose={() => setRenamingPackage(null)} onRename={handleRenamePackage} />
      <UnpackageModal
        open={isUnpackageModalOpen}
        packageName={currentPackage?.display_name}
        onClose={() => setIsUnpackageModalOpen(false)}
        onConfirm={handleCleanMasterPatch}
      />
    </div>
  );
};
