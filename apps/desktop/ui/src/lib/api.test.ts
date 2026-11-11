import { describe as suite, expect, it } from 'vitest';
import { describe } from './api';

suite('describe', () => {
  it('formats app info', () => {
    expect(describe({ name: 'CeeVee', version: '0.1.0', schema_version: 2 })).toBe('CeeVee 0.1.0 (schema 2)');
  });
});
