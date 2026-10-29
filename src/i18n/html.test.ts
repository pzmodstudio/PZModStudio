import { describe, expect, it } from 'vitest';
import './index';
import { escapeHtml, tHtml } from './html';

describe('tHtml', () => {
  it('escapes the values put into a translation and keeps its own markup', () => {
    const out = tHtml('browse.pendingNexus', { name: '<img src=x onerror="alert(1)">' });
    expect(out).not.toContain('<img');
    expect(out).toContain('&lt;img src=x onerror=&quot;alert(1)&quot;&gt;');
  });

  it('escapes every HTML-special character', () => {
    expect(escapeHtml(`<a href='x'>&</a>`)).toBe('&lt;a href=&#39;x&#39;&gt;&amp;&lt;/a&gt;');
  });
});
