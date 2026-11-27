import { describe, expect, it } from 'vitest';
import { buildPrompt, contextChip, contextSummary, friendlyError, suggestions, type ChatContext } from './chat';

const ctx: ChatContext = {
  jobs: [
    { title: 'PM', company: 'Acme', score: 90, state: 'new' },
    { title: 'Designer', company: null, score: 70, state: 'saved' },
    { title: 'Writer', company: 'Globex', score: null, state: 'new' }
  ],
  applications: [
    { title: 'PM', company: 'Acme', status: 'sent' },
    { title: 'Dev', company: 'Initech', status: 'sent' },
    { title: 'Lead', company: 'Hooli', status: 'interview' }
  ],
  jobTypes: [
    { name: 'Product', active: true },
    { name: 'Old', active: false }
  ],
  waiting: 1
};

describe('chat', () => {
  it('summarizes the search from facts only', () => {
    const s = contextSummary(ctx);
    expect(s).toContain('Job types: Product.');
    expect(s).toContain('Jobs found: 3 (2 new, 1 saved).');
    expect(s).toContain('PM at Acme (90%); Designer (70%)');
    expect(s).toContain('Applications: 3 (2 applied, 1 interview).');
    expect(s).toContain('1 item is waiting for approval.');
  });
  it('says so when nothing is set up', () => {
    const s = contextSummary({ jobs: [], applications: [], jobTypes: [], waiting: 0 });
    expect(s).toContain('No job types are set up yet.');
    expect(s).toContain('No applications are tracked yet.');
  });
  it('builds a prompt with recent turns only and the new question last', () => {
    const history = Array.from({ length: 10 }, (_, i) => ({ role: i % 2 ? 'assistant' : 'user', text: `turn ${i}` })) as never;
    const p = buildPrompt('Facts here', history, 'What now?');
    expect(p).toContain('Facts here');
    expect(p).not.toContain('turn 3');
    expect(p).toContain('turn 4');
    expect(p.trimEnd().endsWith('CeeVee:')).toBe(true);
    expect(p).toContain('Person: What now?');
  });
  it('clips very long turns', () => {
    expect(buildPrompt('f', [], 'x'.repeat(2000)).length).toBeLessThan(1200);
  });
  it('labels what the assistant sees', () => {
    expect(contextChip(ctx)).toBe('Sees 3 jobs, 3 applications');
  });
  it('detects a missing model', () => {
    expect(friendlyError('Download the model in Settings first.').needsModel).toBe(true);
    expect(friendlyError('boom')).toEqual({ text: 'boom', needsModel: false });
  });
  it('suggests only what applies', () => {
    expect(suggestions({ jobs: [], applications: [], jobTypes: [], waiting: 0 })).toEqual(['What can you help me with?']);
    expect(suggestions(ctx)).toHaveLength(4);
  });
});

import { focusLine, jobFocus } from './chat';

describe('focus', () => {
  it('adds nothing when nothing is open', () => expect(focusLine('  ')).toBe(''));
  it('names what the person is looking at', () => expect(focusLine('the job PM')).toContain('looking at: the job PM'));
  it('describes a job with company and match', () =>
    expect(jobFocus({ title: 'PM', company: 'Acme', score: 89.6, state: 'new' })).toBe('the job PM at Acme (new, 90% match)'));
  it('copes with a missing company and score', () =>
    expect(jobFocus({ title: 'PM', company: null, score: null, state: 'saved' })).toBe('the job PM (saved)'));
});
