import { describe, expect, it } from 'vitest';
import { applicationName, boardColumns, groupByStatus, statusLabel } from './applications';

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

describe('boardColumns', () => {
  it('has a column per open stage plus Closed, even when empty', () => {
    const cols = boardColumns([]);
    expect(cols.map((c) => c.label)).toEqual(['Draft', 'Applied', 'Screening', 'Interview', 'Offer', 'Closed']);
    expect(cols.every((c) => c.total === 0)).toBe(true);
  });
  it('puts ended applications in Closed', () => {
    const cols = boardColumns([a(1, 'rejected'), a(2, 'withdrawn'), a(3, 'offer')]);
    expect(cols.find((c) => c.key === 'closed')?.total).toBe(2);
    expect(cols.find((c) => c.key === 'offer')?.total).toBe(1);
  });
  it('shows the top cards and counts the rest, until expanded', () => {
    const many = [1, 2, 3, 4, 5, 6, 7].map((i) => a(i, 'sent'));
    const col = boardColumns(many, 5).find((c) => c.key === 'sent')!;
    expect(col.cards).toHaveLength(5);
    expect(col.more).toBe(2);
    const open = boardColumns(many, 5, new Set(['sent'])).find((c) => c.key === 'sent')!;
    expect(open.cards).toHaveLength(7);
    expect(open.more).toBe(0);
  });
});
