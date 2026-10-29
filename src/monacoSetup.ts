// Bundles Monaco with the app instead of letting @monaco-editor/react fetch it from a CDN,
// so the Merger editor also works offline. Only the core editor and Lua highlighting are
// included (the Merger shows Lua and plain-text script files).
import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api.js';
import 'monaco-editor/esm/vs/basic-languages/lua/lua.contribution.js';
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker.js?worker';

self.MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};

loader.config({ monaco });
