import React, { useState } from 'react';
import { MergedPackageInfoUI } from '../../services/tauri';
import { Edit2, Package, Unlock } from 'lucide-react';
import i18next from 'i18next';
import { tHtml } from '../../i18n/html';

/** New package: a name and an optional description; it is created in draft. */
export const CreatePackageModal: React.FC<{
  open: boolean;
  onClose: () => void;
  onCreate: (name: string, description?: string) => Promise<void> | void;
}> = ({ open, onClose, onCreate }) => {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  if (!open) return null;

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    await onCreate(name.trim(), description.trim() || undefined);
    setName('');
    setDescription('');
  };

  return (
    <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <form onSubmit={submit} className="bg-slate-900 border border-slate-800 rounded-2xl p-6 max-w-md w-full space-y-4 shadow-2xl animate-fade-in">
        <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
          <Package className="w-5 h-5 text-emerald-400" />
          <span>{i18next.t('merger.modals.createTitle')}</span>
        </h3>
        <p className="text-xs text-slate-400 leading-relaxed">
          {i18next.t('merger.modals.createBlurb')}
        </p>
        <div>
          <label className="block text-[11px] font-bold text-slate-300 mb-1">{i18next.t('merger.modals.name')}</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={i18next.t('merger.modals.namePlaceholder')}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs text-slate-200 focus:border-emerald-500 focus:outline-none font-mono"
            autoFocus
          />
        </div>
        <div>
          <label className="block text-[11px] font-bold text-slate-300 mb-1">{i18next.t('merger.modals.description')}</label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder={i18next.t('merger.modals.descriptionPlaceholder')}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs text-slate-200 focus:border-emerald-500 focus:outline-none resize-none h-20"
          />
        </div>
        <div className="flex items-center justify-end gap-2 pt-2">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-bold rounded-lg cursor-pointer"
          >
            {i18next.t('merger.modals.cancel')}
          </button>
          <button
            type="submit"
            className="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-bold rounded-lg cursor-pointer"
          >
            {i18next.t('merger.modals.create')}
          </button>
        </div>
      </form>
    </div>
  );
};

/** Rename a package and change its description. `pkg` null = closed. */
export const RenamePackageModal: React.FC<{
  pkg: MergedPackageInfoUI | null;
  onClose: () => void;
  onRename: (folderName: string, name: string, description?: string) => Promise<void> | void;
}> = ({ pkg, onClose, onRename }) => {
  // Keyed on the package so the fields start from its current name each time it opens.
  if (!pkg) return null;
  return <RenamePackageForm key={pkg.folder_name} pkg={pkg} onClose={onClose} onRename={onRename} />;
};

const RenamePackageForm: React.FC<{
  pkg: MergedPackageInfoUI;
  onClose: () => void;
  onRename: (folderName: string, name: string, description?: string) => Promise<void> | void;
}> = ({ pkg, onClose, onRename }) => {
  const [name, setName] = useState(pkg.display_name);
  const [description, setDescription] = useState(pkg.description || '');

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    await onRename(pkg.folder_name, name.trim(), description.trim() || undefined);
  };

  return (
    <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <form onSubmit={submit} className="bg-slate-900 border border-slate-800 rounded-2xl p-6 max-w-md w-full space-y-4 shadow-2xl animate-fade-in">
        <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
          <Edit2 className="w-5 h-5 text-cyan-400" />
          <span>{i18next.t('merger.modals.editTitle')}</span>
        </h3>
        <div>
          <label className="block text-[11px] font-bold text-slate-300 mb-1">{i18next.t('merger.modals.name')}</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs text-slate-200 focus:border-cyan-500 focus:outline-none font-mono"
            autoFocus
          />
        </div>
        <div>
          <label className="block text-[11px] font-bold text-slate-300 mb-1">{i18next.t('merger.modals.description')}</label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder={i18next.t('merger.modals.modDescription')}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs text-slate-200 focus:border-cyan-500 focus:outline-none resize-none h-20"
          />
        </div>
        <div className="flex items-center justify-end gap-2 pt-2">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-bold rounded-lg cursor-pointer"
          >
            {i18next.t('merger.modals.cancel')}
          </button>
          <button
            type="submit"
            className="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-bold rounded-lg cursor-pointer"
          >
            {i18next.t('merger.modals.saveChanges')}
          </button>
        </div>
      </form>
    </div>
  );
};

/** Unpack a published package to edit it: it leaves the game's mod list until published again. */
export const UnpackageModal: React.FC<{
  open: boolean;
  packageName?: string;
  onClose: () => void;
  onConfirm: () => Promise<void> | void;
}> = ({ open, packageName, onClose, onConfirm }) => {
  if (!open) return null;
  return (
    <div className="fixed inset-0 bg-slate-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-amber-500/40 rounded-2xl p-6 max-w-md w-full space-y-4 shadow-2xl animate-fade-in text-slate-200">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/30 flex items-center justify-center shrink-0">
            <Unlock className="w-5 h-5 text-amber-400" />
          </div>
          <div>
            <h3 className="text-base font-bold text-slate-100">{i18next.t('merger.modals.unpackTitle')}</h3>
            <p className="text-xs text-slate-400 mt-0.5">{i18next.t('merger.modals.unpackSub')}</p>
          </div>
        </div>

        <div className="bg-slate-950 border border-slate-800 rounded-xl p-3.5 space-y-2 text-xs text-slate-300">
          <p dangerouslySetInnerHTML={{ __html: tHtml('merger.modals.unpackBody', { name: packageName ?? '' }) }} />
          <div className="p-2.5 bg-amber-950/40 border border-amber-500/30 rounded-lg text-amber-300 text-[11px] font-mono leading-relaxed" dangerouslySetInnerHTML={{ __html: tHtml('merger.modals.unpackNotice') }} />
        </div>

        <div className="flex items-center justify-end gap-3 pt-2">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-semibold rounded-lg cursor-pointer transition"
          >
            {i18next.t('merger.modals.cancel')}
          </button>
          <button
            type="button"
            onClick={async () => {
              onClose();
              await onConfirm();
            }}
            className="px-4 py-2 bg-gradient-to-r from-amber-600 to-orange-600 hover:from-amber-500 hover:to-orange-500 text-white text-xs font-bold rounded-lg cursor-pointer shadow transition"
          >
            {i18next.t('merger.modals.confirmOpen')}
          </button>
        </div>
      </div>
    </div>
  );
};
