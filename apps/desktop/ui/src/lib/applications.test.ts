import { describe, expect, it } from 'vitest';
import { applicationName, groupByStatus, statusLabel } from './applications';

const a = (id: number, status: string, company: string | null = 'Acme') => ({
  id,
  title: 'Engineer',
  company,
  url: null,
  status,
  applied_at: null
});

describe('applications', () => {
  it('groups in pipeline order and drops empty groups', () => {
    const g = groupByStatus([a(1, 'interview'), a(2, 'draft'), a(3, 'draft')]);
    expect(g.map((x) => x.status)).toEqual(['draft', 'interview']);
    expect(g[0].items).toHaveLength(2);
  });

  it('names an application', () => {
    expect(applicationName(a(1, 'draft'))).toBe('Engineer at Acme');
    expect(applicationName(a(1, 'draft', null))).toBe('Engineer');
  });

  it('labels statuses', () => {
    expect(statusLabel('screening')).toBe('Screening');
    expect(statusLabel('odd')).toBe('odd');
  });
});
