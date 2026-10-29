import { describe, expect, it } from 'vitest';
import { noteChange, onChange } from './changes';

describe('changes', () => {
  it('tells every listener the scope, and a listener taken off hears nothing more', () => {
    const heard: string[] = [];
    const off = onChange((s) => heard.push(`a:${s}`));
    const off2 = onChange((s) => heard.push(`b:${s}`));
    noteChange('mods');
    off();
    noteChange('profiles');
    off2();
    noteChange();
    expect(heard).toEqual(['a:mods', 'b:mods', 'b:profiles']);
  });

  it('a listener that throws does not stop the others', () => {
    const heard: string[] = [];
    const off1 = onChange(() => {
      throw new Error('boom');
    });
    const off2 = onChange((s) => heard.push(s));
    noteChange('other');
    off1();
    off2();
    expect(heard).toEqual(['other']);
  });
});
