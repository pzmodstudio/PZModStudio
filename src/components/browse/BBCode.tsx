import React from 'react';
import { TauriService } from '../../services/tauri';

/**
 * Renders Steam / Nexus BBCode descriptions as React elements (never as raw HTML).
 * Unknown tags stay as text; unclosed tags close at the end.
 */

type Node = { kind: 'text'; text: string } | { kind: 'tag'; name: string; arg: string | null; children: Node[] };

const CONTAINER_TAGS = new Set([
  'b', 'i', 'u', 's', 'strike', 'h1', 'h2', 'h3', 'h4', 'url', 'list', 'olist', '*', 'quote', 'spoiler',
  'table', 'tr', 'td', 'th', 'center', 'left', 'right', 'color', 'size', 'font', 'previewyoutube', 'youtube', 'line',
]);
const RAW_TAGS = new Set(['code', 'noparse', 'img']);
const VOID_TAGS = new Set(['hr']);
const BLOCK_TAGS = new Set(['h1', 'h2', 'h3', 'h4', 'list', 'olist', 'quote', 'code', 'spoiler', 'table', 'hr', 'center', 'img']);

const TAG_RE = /\[(\/?)([a-z0-9*]+)(?:=([^\]]*))?\]/gi;

export function parseBBCode(source: string): Node[] {
  const text = source.replace(/\r\n?/g, '\n');
  const root: Node & { kind: 'tag' } = { kind: 'tag', name: 'root', arg: null, children: [] };
  const stack: (Node & { kind: 'tag' })[] = [root];
  const top = () => stack[stack.length - 1];
  const pushText = (t: string) => {
    if (!t) return;
    const last = top().children[top().children.length - 1];
    if (last?.kind === 'text') last.text += t;
    else top().children.push({ kind: 'text', text: t });
  };

  let pos = 0;
  TAG_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = TAG_RE.exec(text))) {
    const [whole, slash, rawName, arg] = m;
    const name = rawName.toLowerCase();
    const known = CONTAINER_TAGS.has(name) || RAW_TAGS.has(name) || VOID_TAGS.has(name);
    if (!known) continue;
    pushText(text.slice(pos, m.index));
    pos = m.index + whole.length;

    if (!slash) {
      if (RAW_TAGS.has(name)) {
        const close = text.toLowerCase().indexOf(`[/${name}]`, pos);
        const end = close < 0 ? text.length : close;
        const inner = text.slice(pos, end);
        top().children.push({ kind: 'tag', name, arg: arg ?? null, children: [{ kind: 'text', text: inner }] });
        pos = close < 0 ? text.length : close + name.length + 3;
        TAG_RE.lastIndex = pos;
        continue;
      }
      if (VOID_TAGS.has(name)) {
        top().children.push({ kind: 'tag', name, arg: null, children: [] });
        continue;
      }
      if (name === '*' && top().name === '*') stack.pop();
      const node: Node & { kind: 'tag' } = { kind: 'tag', name, arg: arg ?? null, children: [] };
      top().children.push(node);
      stack.push(node);
    } else {
      const idx = stack.map((n) => n.name).lastIndexOf(name);
      if (idx > 0) stack.length = idx;
      else if (name !== '*') pushText(whole); // stray closing tag
    }
  }
  pushText(text.slice(pos));
  return root.children;
}

const openLink = (url: string) => {
  if (/^https?:\/\//i.test(url)) TauriService.openExternalUrl(url);
};

const URL_RE = /(https?:\/\/[^\s<>"'\])]+)/g;

const Link: React.FC<{ href: string; children: React.ReactNode }> = ({ href, children }) => (
  <a
    href={href}
    onClick={(e) => {
      e.preventDefault();
      openLink(href);
    }}
    className="text-sky-400 hover:text-sky-300 underline break-all cursor-pointer"
    title={href}
  >
    {children}
  </a>
);

function renderText(text: string, key: string): React.ReactNode[] {
  const out: React.ReactNode[] = [];
  text.split('\n').forEach((line, li) => {
    if (li > 0) out.push(<br key={`${key}-br${li}`} />);
    line.split(URL_RE).forEach((part, pi) => {
      if (!part) return;
      out.push(pi % 2 === 1 ? <Link key={`${key}-${li}-${pi}`} href={part}>{part}</Link> : part);
    });
  });
  return out;
}

const plainText = (nodes: Node[]): string => nodes.map((n) => (n.kind === 'text' ? n.text : plainText(n.children))).join('');

function renderNodes(nodes: Node[], key: string, inList = false): React.ReactNode[] {
  const out: React.ReactNode[] = [];
  nodes.forEach((node, i) => {
    const k = `${key}.${i}`;
    if (node.kind === 'text') {
      let t = node.text;
      const prev = nodes[i - 1];
      const next = nodes[i + 1];
      // Block elements already break the line: drop the newline next to them.
      if (prev?.kind === 'tag' && BLOCK_TAGS.has(prev.name)) t = t.replace(/^\n/, '');
      if (next?.kind === 'tag' && BLOCK_TAGS.has(next.name)) t = t.replace(/\n$/, '');
      if (inList) {
        if (!t.trim()) return;
        t = t.replace(/^\n+|\n+$/g, '');
      }
      t = t.replace(/\n{3,}/g, '\n\n');
      out.push(...renderText(t, k));
      return;
    }
    const kids = () => renderNodes(node.children, k, node.name === 'list' || node.name === 'olist');
    switch (node.name) {
      case 'b':
        out.push(<strong key={k} className="font-bold text-slate-100">{kids()}</strong>);
        break;
      case 'i':
        out.push(<em key={k}>{kids()}</em>);
        break;
      case 'u':
        out.push(<u key={k}>{kids()}</u>);
        break;
      case 's':
      case 'strike':
        out.push(<s key={k}>{kids()}</s>);
        break;
      case 'h1':
        out.push(<h3 key={k} className="text-base font-bold text-slate-100 mt-3 mb-1">{kids()}</h3>);
        break;
      case 'h2':
        out.push(<h4 key={k} className="text-sm font-bold text-slate-100 mt-3 mb-1">{kids()}</h4>);
        break;
      case 'h3':
      case 'h4':
        out.push(<h5 key={k} className="text-xs font-bold text-slate-200 mt-2 mb-1 uppercase tracking-wide">{kids()}</h5>);
        break;
      case 'url': {
        const href = (node.arg ?? plainText(node.children)).trim().replace(/^["']|["']$/g, '');
        out.push(<Link key={k} href={href}>{node.children.length ? kids() : href}</Link>);
        break;
      }
      case 'img': {
        const src = plainText(node.children).trim();
        if (/^https:\/\//i.test(src)) out.push(<img key={k} src={src} alt="" loading="lazy" className="max-w-full h-auto rounded my-2" />);
        break;
      }
      case 'list':
      case 'olist': {
        const ordered = node.name === 'olist' || node.arg === '1';
        const cls = `${ordered ? 'list-decimal' : 'list-disc'} pl-5 my-1 space-y-0.5`;
        out.push(ordered ? <ol key={k} className={cls}>{kids()}</ol> : <ul key={k} className={cls}>{kids()}</ul>);
        break;
      }
      case '*':
        out.push(<li key={k}>{renderNodes(node.children, k, true)}</li>);
        break;
      case 'quote':
        out.push(
          <blockquote key={k} className="border-l-2 border-slate-600 pl-3 my-2 text-slate-400">
            {node.arg && <div className="text-[10px] text-slate-500 mb-0.5">{node.arg} wrote:</div>}
            {kids()}
          </blockquote>
        );
        break;
      case 'code':
      case 'noparse':
        out.push(
          node.name === 'code' ? (
            <pre key={k} className="my-2 p-2 rounded bg-slate-950 border border-slate-800 text-[11px] font-mono whitespace-pre-wrap break-words">{plainText(node.children).replace(/^\n|\n$/g, '')}</pre>
          ) : (
            <span key={k}>{plainText(node.children)}</span>
          )
        );
        break;
      case 'spoiler':
        out.push(
          <details key={k} className="my-2 rounded border border-slate-800 bg-slate-950/60 px-2 py-1">
            <summary className="cursor-pointer text-[11px] text-slate-400">Spoiler</summary>
            {kids()}
          </details>
        );
        break;
      case 'hr':
      case 'line':
        out.push(<hr key={k} className="my-3 border-slate-800" />);
        break;
      case 'table':
        out.push(
          <div key={k} className="overflow-x-auto my-2">
            <table className="text-[11px] border-collapse"><tbody>{renderNodes(node.children, k, true)}</tbody></table>
          </div>
        );
        break;
      case 'tr':
        out.push(<tr key={k}>{renderNodes(node.children, k, true)}</tr>);
        break;
      case 'td':
      case 'th':
        out.push(
          node.name === 'th'
            ? <th key={k} className="border border-slate-800 px-2 py-1 text-left font-bold">{kids()}</th>
            : <td key={k} className="border border-slate-800 px-2 py-1 align-top">{kids()}</td>
        );
        break;
      case 'center':
        out.push(<div key={k} className="text-center">{kids()}</div>);
        break;
      case 'previewyoutube':
      case 'youtube': {
        const id = (node.arg ?? plainText(node.children)).split(';')[0].trim();
        if (/^[\w-]{6,}$/.test(id)) out.push(<Link key={k} href={`https://www.youtube.com/watch?v=${id}`}>YouTube video</Link>);
        break;
      }
      default:
        // left / right / color / size / font: keep the content, ignore the styling.
        out.push(<React.Fragment key={k}>{kids()}</React.Fragment>);
    }
  });
  return out;
}

export const BBCode: React.FC<{ source: string; className?: string }> = ({ source, className }) => {
  const nodes = React.useMemo(() => parseBBCode(source), [source]);
  return <div className={`text-xs text-slate-300 leading-relaxed break-words ${className ?? ''}`}>{renderNodes(nodes, 'r')}</div>;
};
