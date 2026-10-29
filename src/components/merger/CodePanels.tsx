import React from 'react';
import { ChangeRegion, VfsConflict } from '../../types';
import { LinedCodeSnippet, modLineFor } from './CodeView';
import { Minus, Plus } from 'lucide-react';
import i18next from 'i18next';

export interface CodePanelsProps {
  currentConflict: VfsConflict;
  totalFileLines: number;
  /** False when the scan put a note in place of a file the base game does not have. */
  hasVanillaBase: boolean;
  editorContent: string;
  isManuallyEdited: boolean;
  setEditorContent: (content: string) => void;
  setIsManuallyEdited: (edited: boolean) => void;
  showToast: (message: string) => void;
  focusLine: number;
  changedBaseRanges: Array<[number, number]>;
  collisionBaseRanges: Array<[number, number]>;
  isCurrentlyPackaged: boolean;
  handleToggleAddMod: (modContent: string, modUid: string) => void;
}

/**
 * The file as each side has it, side by side: the base game's copy with "Load Base", then one
 * panel per mod with "Append" / "Remove", every panel scrolled to the same line and with the
 * changed and colliding ranges marked.
 */
export const CodePanels: React.FC<CodePanelsProps> = ({
  currentConflict,
  totalFileLines,
  hasVanillaBase,
  editorContent,
  isManuallyEdited,
  setEditorContent,
  setIsManuallyEdited,
  showToast,
  focusLine,
  changedBaseRanges,
  collisionBaseRanges,
  isCurrentlyPackaged,
  handleToggleAddMod,
}) => (
  <>
  {/* Horizontal Code Panels Carousel */}
  <div className="flex-1 flex gap-3 overflow-x-auto min-h-0">
    {/* Panel A: Vanilla Base */}
    <div className="w-96 min-w-96 bg-slate-950 border border-slate-800 rounded-lg flex flex-col shadow">
    <div className="px-3 py-1.5 bg-slate-900/90 border-b border-slate-800 flex items-center justify-between gap-2">
      <span className="text-xs font-bold text-slate-300">{i18next.t('merger.panels.vanilla')}</span>
      <div className="flex items-center gap-2">
        <span className="text-[10px] font-mono text-slate-400 px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800">
          {i18next.t('merger.panels.lines', { count: totalFileLines })}
        </span>
        {/* With no vanilla file there is nothing to load: the button used to put
            the "not present" note itself into the editor, one Save away from
            shipping a one-line comment in place of the mods' code. */}
        <button
          disabled={!hasVanillaBase}
          onClick={() => {
            if (!hasVanillaBase) return;
            setEditorContent(currentConflict.base_content);
            setIsManuallyEdited(false);
            showToast(i18next.t('merger.panels.baseRestored'));
          }}
          className={`px-2 py-1 text-[10px] font-semibold rounded border transition shadow-sm ${
            !hasVanillaBase
              ? 'bg-slate-900 text-slate-600 border-slate-800 cursor-not-allowed'
              : editorContent === currentConflict.base_content && !isManuallyEdited
              ? 'bg-slate-800 text-emerald-300 border-emerald-700/80 font-bold cursor-pointer'
              : 'bg-slate-800 text-slate-300 border-slate-700 hover:bg-slate-700 hover:text-white cursor-pointer'
          }`}
          title={
            hasVanillaBase
              ? i18next.t('merger.panels.loadBaseTitle')
              : i18next.t('merger.panels.noBaseTitle')
          }
        >
          {!hasVanillaBase
            ? i18next.t('merger.panels.noBase')
            : editorContent === currentConflict.base_content && !isManuallyEdited
            ? i18next.t('merger.panels.baseActive')
            : i18next.t('merger.panels.loadBase')}
        </button>
      </div>
    </div>
    <LinedCodeSnippet
      content={currentConflict.base_content}
      targetLine={focusLine}
      highlightRanges={changedBaseRanges}
      collisionRanges={collisionBaseRanges}
    />
    </div>

  {/* Competing Mod Panels */}
  {currentConflict.competing_mods.map((mod, idx) => {
    const modUid = mod.mod_id && mod.mod_id !== mod.mod_name
    ? `${mod.mod_name} (${mod.mod_id})`
    : `${mod.mod_name}${currentConflict.competing_mods.filter(m => m.mod_name === mod.mod_name).length > 1 ? ` #${idx + 1}` : ''}`;

    const isModAdded = editorContent.includes(`-- === MOD: [${modUid}] ===`);
    const isModExclusive = editorContent === mod.content;

    return (
    <div
      key={idx}
      className={`w-[450px] min-w-[450px] bg-slate-950 border rounded-lg flex flex-col shadow group transition ${
        isModExclusive
          ? 'border-emerald-500 ring-1 ring-emerald-500/40'
          : isModAdded
          ? 'border-cyan-500/80 ring-1 ring-cyan-500/30'
          : 'border-slate-800 hover:border-slate-700'
      }`}
    >
      <div className="px-3 py-1.5 bg-slate-900/90 border-b border-slate-800 flex items-center justify-between gap-2">
        <span className="text-xs font-bold text-emerald-400 truncate max-w-[150px]" title={modUid}>
          {mod.mod_name}
        </span>
        <div className="flex items-center gap-1.5 shrink-0">
          {/* Toggle Agregar / Quitar Mod */}
          <button
            onClick={() => !isCurrentlyPackaged && handleToggleAddMod(mod.content, modUid)}
            disabled={isCurrentlyPackaged || isModExclusive}
            className={`px-2 py-1 text-[10px] font-semibold rounded border transition shadow-sm flex items-center gap-1 ${
              isCurrentlyPackaged
                ? 'bg-slate-900 border-slate-800 text-slate-600 cursor-not-allowed opacity-50'
                : isModExclusive
                ? 'bg-slate-900 border-slate-800 text-slate-600 cursor-not-allowed opacity-50'
                : isModAdded
                ? 'bg-rose-950/90 text-rose-300 border-rose-700 hover:bg-rose-900 hover:text-white cursor-pointer'
                : 'bg-cyan-950/80 text-cyan-300 border-cyan-800 hover:bg-cyan-700 hover:text-white cursor-pointer'
            }`}
            title={
              isCurrentlyPackaged
                ? i18next.t('merger.panels.packagedTitle')
                : isModExclusive
                ? i18next.t('merger.panels.exclusiveTitle')
                : isModAdded
                ? i18next.t('merger.panels.removeTitle', { mod: mod.mod_name })
                : i18next.t('merger.panels.appendTitle', { mod: mod.mod_name })
            }
          >
            {isModAdded ? (
              <>
                <Minus className="w-3 h-3 text-rose-400" />
                <span>{i18next.t('merger.panels.remove')}</span>
              </>
            ) : (
              <>
                <Plus className="w-3 h-3 text-cyan-400" />
                <span>{i18next.t('merger.panels.append')}</span>
              </>
            )}
          </button>

        </div>
      </div>
      {(() => {
        const summary = currentConflict.mod_changes?.find((m) => m.mod_id === mod.mod_id);
        const own = (summary?.regions ?? []).filter((r) => r.kind !== 'removed');
        const overlapsCollision = (r: ChangeRegion) =>
          collisionBaseRanges.some(([a, b]) => r.base_start <= b && a <= Math.max(r.base_start, r.base_end));
        return (
          <LinedCodeSnippet
            content={mod.content}
            targetLine={summary ? modLineFor(summary, focusLine) : focusLine}
            highlightRanges={own.map((r) => [r.mod_start, r.mod_end] as [number, number])}
            collisionRanges={own.filter(overlapsCollision).map((r) => [r.mod_start, r.mod_end] as [number, number])}
          />
        );
      })()}
    </div>
    );
  })}
</div>
  </>
);
