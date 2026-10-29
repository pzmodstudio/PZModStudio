import React from 'react';
import { MergedPackageInfoUI } from '../../services/tauri';
import { Box, Download, Edit2, Eye, EyeOff, FolderOpen, Lock, Package, PackageCheck, Plus, Trash2, Upload } from 'lucide-react';
import i18next from 'i18next';

interface PackageListProps {
  packages: MergedPackageInfoUI[];
  selectedFolder: string;
  onSelect: (folderName: string) => void;
  onOpen: (folderName: string) => void;
  onRename: (pkg: MergedPackageInfoUI) => void;
  onDelete: (folderName: string) => void;
  onToggleInModlist: (pkg: MergedPackageInfoUI) => void;
  onExport: (folderName: string) => void;
  onOpenFolder: (folderName: string) => void;
  onImport: () => void;
  onCreate: () => void;
  /** Makes a package named after the active profile, when the profile has none of its own. */
  onCreateForProfile?: () => void;
  /** The active profile, whose packages the list shows; null = no profile, every package. */
  activeProfileName?: string | null;
  /** Packages that belong to other profiles only, hidden until asked for. */
  otherProfilesCount?: number;
  showOtherProfiles?: boolean;
  onToggleOtherProfiles?: () => void;
  /** The profiles a package belongs to. */
  ownersOf?: (folderName: string) => string[];
  isInActiveProfile?: (pkg: MergedPackageInfoUI) => boolean;
}

/** Level 1 of the Merger: every package as a card, before one is opened. */
export const PackageList: React.FC<PackageListProps> = ({
  packages,
  selectedFolder,
  onSelect,
  onOpen,
  onRename,
  onDelete,
  onToggleInModlist,
  onExport,
  onOpenFolder,
  onImport,
  onCreate,
  onCreateForProfile,
  activeProfileName = null,
  otherProfilesCount = 0,
  showOtherProfiles = false,
  onToggleOtherProfiles,
  ownersOf = () => [],
  isInActiveProfile = () => true,
}) => (
  <div className="flex-1 flex flex-col overflow-hidden">
    {/* Top Bar for Level 1 */}
    <div className="bg-slate-900 border-b border-slate-800 p-3.5 flex items-center justify-between gap-4 shrink-0 shadow-md">
      <div className="flex items-center gap-2.5">
        <div className="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
          <Box className="w-4 h-4" />
        </div>
        <div>
          <h2 className="text-sm font-bold text-slate-100 flex items-center gap-2">
            <span>{i18next.t('merger.list.title')}</span>
            <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-slate-800 text-emerald-400 border border-slate-700">
              {i18next.t('merger.list.count', { count: packages.length })}
            </span>
          </h2>
          <p className="text-[11px] text-slate-400">
            {i18next.t('merger.list.blurb')}
          </p>
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        {activeProfileName && (otherProfilesCount > 0 || showOtherProfiles) && (
          <button
            onClick={onToggleOtherProfiles}
            aria-pressed={showOtherProfiles}
            className={`flex items-center gap-1.5 px-3 py-1.5 border text-xs font-bold rounded-lg transition shadow shrink-0 cursor-pointer ${
              showOtherProfiles
                ? 'bg-purple-950/60 text-purple-200 border-purple-600/60 hover:bg-purple-900/60'
                : 'bg-slate-800 hover:bg-slate-700 text-slate-200 border-slate-700'
            }`}
            title={i18next.t('merger.list.otherProfilesTitle', { name: activeProfileName })}
          >
            {showOtherProfiles ? <Eye className="w-3.5 h-3.5" /> : <EyeOff className="w-3.5 h-3.5" />}
            <span>
              {showOtherProfiles
                ? i18next.t('merger.list.onlyProfile', { name: activeProfileName })
                : i18next.t('merger.list.otherProfiles', { count: otherProfilesCount })}
            </span>
          </button>
        )}
        <button
          onClick={onImport}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 text-xs font-bold rounded-lg transition shadow shrink-0 cursor-pointer"
          title={i18next.t('merger.list.importTitle')}
        >
          <Upload className="w-3.5 h-3.5 text-cyan-400" />
          <span>{i18next.t('merger.list.import')}</span>
        </button>

        {activeProfileName && onCreateForProfile && !packages.some((p) => !p.is_companion_bridge && isInActiveProfile(p)) && (
          <button
            onClick={onCreateForProfile}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-emerald-200 border border-emerald-700/60 text-xs font-bold rounded-lg transition shadow shrink-0 cursor-pointer"
            title={i18next.t('merger.list.forProfileTitle', { name: activeProfileName })}
          >
            <Plus className="w-3.5 h-3.5" />
            <span>{i18next.t('merger.list.forProfile', { name: activeProfileName })}</span>
          </button>
        )}
        <button
          onClick={onCreate}
          className="flex items-center gap-1.5 px-3.5 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-bold rounded-lg transition shadow shrink-0 cursor-pointer"
        >
          <Plus className="w-4 h-4" />
          <span>{i18next.t('merger.list.new')}</span>
        </button>

        {/* Package & Publish stays greyed out until a package is open */}
        <button
          disabled
          className="flex items-center gap-1.5 px-3.5 py-1.5 text-xs font-bold rounded-lg bg-slate-900 border border-slate-800 text-slate-600 cursor-not-allowed opacity-50 shadow shrink-0"
          title={i18next.t('merger.list.publishDisabledTitle')}
        >
          <PackageCheck className="w-4 h-4 text-slate-600" />
          <span>{i18next.t('merger.list.publish')}</span>
        </button>
      </div>
    </div>

    {/* Level 1 Body: Cards or Empty State */}
    {packages.length === 0 ? (
      <div className="flex-1 flex flex-col items-center justify-center p-8 text-center space-y-4">
        <div className="w-16 h-16 rounded-2xl bg-slate-900 border border-slate-800 flex items-center justify-center text-slate-500 shadow-inner">
          <Box className="w-8 h-8 text-emerald-400" />
        </div>
        <div className="space-y-1">
          <h3 className="text-base font-bold text-slate-200">{i18next.t('merger.list.none')}</h3>
          <p className="text-xs text-slate-400 max-w-sm leading-relaxed">
            {i18next.t('merger.list.noneBlurb')}
          </p>
        </div>
        <button
          onClick={onCreate}
          className="px-5 py-2.5 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white text-xs font-bold rounded-xl transition shadow-lg flex items-center gap-2 cursor-pointer"
        >
          <Plus className="w-4 h-4" />
          <span>{i18next.t('merger.list.createFirst')}</span>
        </button>
      </div>
    ) : (
      <div className="flex-1 overflow-y-auto p-6 space-y-4 max-w-5xl w-full mx-auto">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {packages.map((pkg) => {
            const isSelected = pkg.folder_name === selectedFolder;
            return (
              <div
                key={pkg.folder_name}
                onClick={() => onSelect(pkg.folder_name)}
                className={`p-5 rounded-2xl border transition shadow-lg flex flex-col justify-between space-y-4 cursor-pointer ${
                  isSelected
                    ? 'bg-slate-900/95 border-emerald-500/70 ring-1 ring-emerald-500/30'
                    : 'bg-slate-900/50 border-slate-800 hover:border-slate-700 hover:bg-slate-900/80'
                }`}
              >
                <div className="space-y-3">
                  {/* Row 1: Icon, Full Display Name, and Top-Right Action Icons */}
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex items-center gap-3 min-w-0 flex-1">
                      <div className={`w-10 h-10 rounded-xl flex items-center justify-center border shrink-0 ${
                        pkg.is_companion_bridge
                          ? 'bg-cyan-500/10 text-cyan-400 border-cyan-500/30'
                          : pkg.is_packaged
                          ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
                          : 'bg-slate-800 text-slate-400 border-slate-700'
                      }`}>
                        <Package className="w-5 h-5" />
                      </div>
                      <div className="min-w-0 flex-1">
                        <h3 className="text-sm font-bold text-slate-100 leading-snug break-words">
                          {pkg.display_name}
                        </h3>
                        <div className="text-[11px] text-slate-400 font-mono mt-0.5 break-all">
                          {pkg.folder_name}
                        </div>
                        {activeProfileName && !pkg.is_companion_bridge && (
                          <div className="flex flex-wrap gap-1 mt-1.5">
                            {ownersOf(pkg.folder_name).length === 0 ? (
                              <span className="text-[10px] px-2 py-0.5 rounded-full bg-slate-800 text-slate-400 border border-slate-700">
                                {i18next.t('merger.list.noProfile')}
                              </span>
                            ) : (
                              ownersOf(pkg.folder_name).map((owner) => (
                                <span
                                  key={owner}
                                  className={`text-[10px] px-2 py-0.5 rounded-full border ${
                                    owner === activeProfileName && isInActiveProfile(pkg)
                                      ? 'bg-emerald-950/70 text-emerald-300 border-emerald-700/60'
                                      : 'bg-purple-950/50 text-purple-300 border-purple-700/50'
                                  }`}
                                >
                                  {i18next.t('merger.list.profileChip', { name: owner })}
                                </span>
                              ))
                            )}
                            {pkg.built_for_profile && (
                              <span
                                className={`text-[10px] px-2 py-0.5 rounded-full border ${
                                  pkg.built_for_profile === activeProfileName
                                    ? 'bg-slate-900 text-slate-400 border-slate-700'
                                    : 'bg-amber-950/50 text-amber-300 border-amber-700/60'
                                }`}
                                title={i18next.t('merger.list.builtForTitle', { name: pkg.built_for_profile })}
                              >
                                {i18next.t('merger.list.builtFor', { name: pkg.built_for_profile })}
                              </span>
                            )}
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Quick Actions (Rename / Delete or Locked System Badge) */}
                    {!pkg.is_companion_bridge ? (
                      <div className="flex items-center gap-1 shrink-0">
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            onRename(pkg);
                          }}
                          className="p-1.5 hover:text-cyan-300 hover:bg-slate-800 rounded-lg transition text-slate-400 cursor-pointer"
                          title={i18next.t('merger.list.renameTitle')}
                        >
                          <Edit2 className="w-3.5 h-3.5" />
                        </button>

                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            onDelete(pkg.folder_name);
                          }}
                          className="p-1.5 hover:text-red-400 hover:bg-rose-950/40 rounded-lg transition text-slate-400 cursor-pointer"
                          title={i18next.t('merger.list.deleteTitle')}
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    ) : (
                      <div
                        className="flex items-center gap-1 px-2 py-0.5 bg-slate-950 border border-slate-800 rounded-lg text-[10px] text-cyan-400/80 font-mono select-none"
                        title={i18next.t('merger.list.systemTitle')}
                      >
                        <Lock className="w-2.5 h-2.5" />
                        <span>{i18next.t('merger.list.system')}</span>
                      </div>
                    )}
                  </div>

                  {/* What is actually in it. Every package the Studio makes carries the
                      same canned description, so two of them side by side said nothing
                      about which was which. */}
                  {!pkg.is_companion_bridge && (
                    <div className="text-xs text-slate-300 leading-relaxed bg-slate-950/40 p-2 rounded-lg border border-slate-800/60 break-words space-y-1">
                      <div className="font-bold text-slate-200">
                        {pkg.merged_files.length === 0
                          ? i18next.t('merger.list.empty')
                          : i18next.t('merger.list.files', { count: pkg.merged_files.length })}
                      </div>
                      {pkg.merged_files.length > 0 && (
                        <div className="text-[10px] font-mono text-slate-500 leading-snug">
                          {pkg.merged_files
                            .slice(0, 3)
                            .map((f) => f.split('/').pop())
                            .join(', ')}
                          {pkg.merged_files.length > 3 ? i18next.t('merger.list.andMore', { count: pkg.merged_files.length - 3 }) : ''}
                        </div>
                      )}
                    </div>
                  )}
                  {pkg.is_companion_bridge && pkg.description && (
                    <p className="text-xs text-slate-300 leading-relaxed line-clamp-2 bg-slate-950/40 p-2 rounded-lg border border-slate-800/60 break-words">
                      {pkg.description}
                    </p>
                  )}

                  {/* Row 2: Status Badges and ModList Visibility */}
                  <div className="flex items-center gap-2 flex-wrap pt-1 border-t border-slate-800/40">
                    {pkg.is_companion_bridge ? (
                      <span className="text-[10px] font-bold px-2.5 py-0.5 rounded-full bg-cyan-500/20 text-cyan-300 border border-cyan-500/40">
                        {i18next.t('merger.list.bridge')}
                      </span>
                    ) : pkg.is_packaged ? (
                      <span className="text-[10px] font-bold px-2.5 py-0.5 rounded-full bg-emerald-950/90 text-emerald-300 border border-emerald-600 shadow-sm" title={i18next.t('merger.list.packagedTitle')}>
                        {i18next.t('merger.list.packagedClosed')}
                      </span>
                    ) : (
                      <span className="text-[10px] font-bold px-2.5 py-0.5 rounded-full bg-amber-950/90 text-amber-300 border border-amber-600 shadow-sm animate-pulse" title={i18next.t('merger.list.openTitle')}>
                        {i18next.t('merger.list.openDraft')}
                      </span>
                    )}

                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleInModlist(pkg);
                      }}
                      className={`flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-bold border transition shadow cursor-pointer ${
                        pkg.is_active_in_modlist
                          ? 'bg-emerald-950/80 text-emerald-300 border-emerald-500/60 hover:bg-emerald-900'
                          : 'bg-slate-800 text-slate-400 border-slate-700 hover:bg-slate-700 hover:text-slate-200'
                      }`}
                      title={
                        pkg.is_active_in_modlist
                          ? i18next.t('merger.list.visibleTitle')
                          : i18next.t('merger.list.hiddenTitle')
                      }
                    >
                      {pkg.is_active_in_modlist ? <Eye className="w-3 h-3 text-emerald-400" /> : <EyeOff className="w-3 h-3 text-slate-400" />}
                      <span>{pkg.is_active_in_modlist ? i18next.t('merger.list.visible') : i18next.t('merger.list.hidden')}</span>
                    </button>
                  </div>
                </div>

                <div className="flex items-center justify-between pt-3 border-t border-slate-800/80 gap-2">
                  <div className="flex items-center gap-1.5">
                    {!pkg.is_companion_bridge && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onExport(pkg.folder_name);
                        }}
                        className="p-2 bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 rounded-lg text-xs transition cursor-pointer"
                        title={i18next.t('merger.list.exportTitle')}
                      >
                        <Download className="w-3.5 h-3.5 text-emerald-400" />
                      </button>
                    )}

                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onOpenFolder(pkg.folder_name);
                      }}
                      className="flex items-center gap-1.5 px-2.5 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 rounded-lg text-xs transition cursor-pointer font-medium"
                      title={i18next.t('merger.list.folderTitle')}
                    >
                      <FolderOpen className="w-3.5 h-3.5 text-amber-400" />
                      {pkg.is_companion_bridge && <span className="text-[11px]">{i18next.t('merger.list.viewFiles')}</span>}
                    </button>
                  </div>

                  {!pkg.is_companion_bridge ? (
                    <button
                      onClick={() => onOpen(pkg.folder_name)}
                      className="px-4 py-2 bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white rounded-lg text-xs font-bold transition shadow flex items-center gap-1.5 cursor-pointer"
                    >
                      <span>{i18next.t('merger.list.open')}</span>
                    </button>
                  ) : (
                    <div className="text-[11px] text-slate-400 flex items-center gap-1.5 font-mono px-2 py-1 bg-slate-950/60 rounded-lg border border-slate-800">
                      <span className="w-2 h-2 rounded-full bg-cyan-400 animate-pulse"></span>
                      <span>{i18next.t('merger.list.ipcLink')}</span>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    )}
  </div>
);
