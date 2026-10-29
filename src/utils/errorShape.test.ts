import { describe, expect, it } from 'vitest';
import { errorShape } from './errorShape';

describe('errorShape', () => {
  it('folds the same failure on different frames and lines into one shape', () => {
    const a = 'ERROR: General      f:472> 20)).FHhandleKeybinds> Exception thrown aFancyHandwork.lua:126';
    const b = 'ERROR: General      f:9031> 20)).FHhandleKeybinds> Exception thrown aFancyHandwork.lua:126';
    expect(errorShape(a)).toBe(errorShape(b));
  });

  it('drops the level and the frame prefix, so a WARN and an ERROR of the same text match', () => {
    expect(errorShape('WARN : Lua          f:0 at KahluaThread.call > require("X") failed')).toBe('require("X") failed');
    expect(errorShape('ERROR: General      f:12> Could not find bone index for node name: "Bip01_Prop2_end"')).toBe(
      'Could not find bone index for node name: "BipN_PropN_end"'
    );
  });

  it('keeps different failures apart', () => {
    expect(errorShape('ERROR: General f:1> A failed')).not.toBe(errorShape('ERROR: General f:1> B failed'));
  });
});
