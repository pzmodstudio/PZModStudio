import React, { useEffect, useState } from 'react';
import { ModInfo, ModFile } from '../../types';
import { StudioPathsUI } from '../settings/SettingsModule';
import { TauriService, explainError } from '../../services/tauri';
import { Wrench, X } from 'lucide-react';
import i18next from 'i18next';

interface PatchFilePickerProps {
  open: boolean;
  /** A mod already chosen (from the Monitor): its files are listed straight away. */
  initialModId: string | null;
  paths: StudioPathsUI;
  mods: ModInfo[];
  onClose: () => void;
  onPick: (modId: string, relativePath: string) => void;
}

/**
 * Picking a mod's file to patch: a bug in one mod, which no merge can reach. First the mod,
 * then one of the Lua or script files the installed build reads from it.
 */
export const PatchFilePicker: React.FC<PatchFilePickerProps> = ({ open, initialModId, paths, mods, onClose, onPick }) => {
  const [modId, setModId] = useState<string | null>(null);
  const [modSearch, setModSearch] = useState('');
  const [files, setFiles] = useState<ModFile[] | null>(null);
  const [fileSearch, setFileSearch] = useState('');

  /** Lists a mod's files when one is chosen. */
  const chooseMod = async (id: string) => {
    setModId(id);
    setFiles(null);
    setFileSearch('');
    try {
      setFiles(await TauriService.modFiles(paths, id));
    } catch (err) {
      alert(explainError(err));
      setModId(null);
    }
  };

  // Each opening starts over, at the mod the caller named when it named one.
  useEffect(() => {
    if (!open) return;
    setModSearch('');
    if (initialModId) void chooseMod(initialModId);
    else {
      setModId(null);
      setFiles(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, initialModId]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-6" onClick={onClose}>
      <div
        className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-2xl p-5 space-y-3"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-start justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
              <Wrench className="w-4 h-4 text-amber-400" />
              <span>{i18next.t('merger.picker.title')}</span>
            </h3>
            <p className="text-[11px] text-slate-400 mt-0.5 max-w-xl leading-relaxed">
              {i18next.t('merger.picker.blurb')}
            </p>
          </div>
          <button onClick={onClose} className="text-slate-500 hover:text-slate-300 p-1 cursor-pointer">
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        </div>

        {!modId ? (
          <>
            <input
              type="text"
              value={modSearch}
              onChange={(e) => setModSearch(e.target.value)}
              placeholder={i18next.t('merger.picker.whichMod')}
              className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500"
            />
            <div className="max-h-80 overflow-y-auto rounded-xl border border-slate-800 divide-y divide-slate-900">
              {mods
                .filter((m) => !m.mod_id.startsWith('Z_PZModStudio_'))
                .filter((m) => {
                  const q = modSearch.trim().toLowerCase();
                  return !q || m.name.toLowerCase().includes(q) || m.mod_id.toLowerCase().includes(q);
                })
                .slice(0, 200)
                .map((m) => (
                  <button
                    key={m.mod_id}
                    type="button"
                    onClick={() => chooseMod(m.mod_id)}
                    className="w-full text-left px-3 py-2 text-xs text-slate-300 hover:bg-slate-950/70 cursor-pointer flex items-center justify-between gap-2"
                  >
                    <span className="truncate">{m.name}</span>
                    <span className="font-mono text-[10px] text-slate-500 shrink-0">{m.mod_id}</span>
                  </button>
                ))}
            </div>
          </>
        ) : (
          <>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => {
                  setModId(null);
                  setFiles(null);
                }}
                className="text-[11px] px-2 py-1 rounded border border-slate-700 text-slate-300 hover:bg-slate-800 cursor-pointer"
              >
                {i18next.t('merger.picker.anotherMod')}
              </button>
              <input
                type="text"
                value={fileSearch}
                onChange={(e) => setFileSearch(e.target.value)}
                placeholder={i18next.t('merger.picker.whichFile')}
                className="flex-1 bg-slate-950 border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-emerald-500"
              />
            </div>
            <div className="max-h-80 overflow-y-auto rounded-xl border border-slate-800 divide-y divide-slate-900">
              {files === null ? (
                <div className="px-3 py-6 text-center text-xs text-slate-500">{i18next.t('merger.picker.reading')}</div>
              ) : files.length === 0 ? (
                <div className="px-3 py-6 text-center text-xs text-slate-500">
                  {i18next.t('merger.picker.noFiles')}
                </div>
              ) : (
                files
                  .filter((f) => {
                    const q = fileSearch.trim().toLowerCase();
                    return !q || f.relative_path.toLowerCase().includes(q);
                  })
                  .map((f) => (
                    <button
                      key={f.relative_path}
                      type="button"
                      onClick={() => onPick(modId, f.relative_path)}
                      className="w-full text-left px-3 py-2 text-[11px] text-slate-300 hover:bg-slate-950/70 cursor-pointer flex items-center justify-between gap-2"
                    >
                      <span className="font-mono truncate">{f.relative_path}</span>
                      <span className="font-mono text-[10px] text-slate-500 shrink-0">
                        {(f.bytes / 1024).toFixed(1)} KB
                      </span>
                    </button>
                  ))
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
};
