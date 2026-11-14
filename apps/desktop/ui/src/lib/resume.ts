/** A small form model over JSON-Resume-style data. Fields the form does not know are kept as they were. */

export type Job = { name: string; position: string; startDate: string; endDate: string; highlights: string[] };
export type School = { institution: string; studyType: string; area: string; endDate: string };
export type Resume = {
  name: string;
  label: string;
  email: string;
  phone: string;
  location: string;
  summary: string;
  work: Job[];
  education: School[];
  skills: string[];
};

type Obj = Record<string, unknown>;
const obj = (v: unknown): Obj => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Obj) : {});
const str = (v: unknown): string => (typeof v === 'string' ? v : '');
const arr = (v: unknown): unknown[] => (Array.isArray(v) ? v : []);

export function emptyJob(): Job {
  return { name: '', position: '', startDate: '', endDate: '', highlights: [''] };
}
export function emptySchool(): School {
  return { institution: '', studyType: '', area: '', endDate: '' };
}

/** Read the stored JSON text into form fields. Returns null when it is not JSON at all. */
export function parseResume(text: string): Resume | null {
  let v: unknown;
  try {
    v = JSON.parse(text);
  } catch {
    return null;
  }
  const root = obj(v);
  const b = obj(root.basics);
  return {
    name: str(b.name),
    label: str(b.label),
    email: str(b.email),
    phone: str(b.phone),
    location: str(obj(b.location).city) || str(b.location),
    summary: str(b.summary),
    work: arr(root.work).map((w) => {
      const o = obj(w);
      return {
        name: str(o.name),
        position: str(o.position),
        startDate: str(o.startDate),
        endDate: str(o.endDate),
        highlights: arr(o.highlights).map(str)
      };
    }),
    education: arr(root.education).map((e) => {
      const o = obj(e);
      return { institution: str(o.institution), studyType: str(o.studyType), area: str(o.area), endDate: str(o.endDate) };
    }),
    skills: arr(root.skills).map((s) => str(obj(s).name) || str(s)).filter(Boolean)
  };
}

/** Write the form back over the original text, so fields the form does not know survive. */
export function serializeResume(r: Resume, original: string): string {
  let base: Obj = {};
  try {
    base = obj(JSON.parse(original));
  } catch {
    base = {};
  }
  const basics = obj(base.basics);
  const locationIsObject = typeof basics.location === 'object' && basics.location !== null;
  const out: Obj = {
    ...base,
    basics: {
      ...basics,
      name: r.name,
      label: r.label,
      email: r.email,
      phone: r.phone,
      location: locationIsObject ? { ...obj(basics.location), city: r.location } : r.location,
      summary: r.summary
    },
    work: r.work.map((w, i) => ({
      ...obj(arr(base.work)[i]),
      name: w.name,
      position: w.position,
      startDate: w.startDate,
      endDate: w.endDate,
      highlights: w.highlights.filter((h) => h.trim() !== '')
    })),
    education: r.education.map((e, i) => ({ ...obj(arr(base.education)[i]), ...e })),
    skills: r.skills.filter((s) => s.trim() !== '').map((name) => ({ name }))
  };
  return JSON.stringify(out, null, 2);
}
