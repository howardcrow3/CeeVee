import { commands, type AppInfo } from './bindings';

export type { AppInfo };

/** Unwrap a generated command result, throwing on the error branch. */
async function unwrap<T>(r: Promise<{ status: 'ok'; data: T } | { status: 'error'; error: string }>): Promise<T> {
  const res = await r;
  if (res.status === 'error') throw new Error(res.error);
  return res.data;
}

export const appInfo = () => unwrap(commands.appInfo());

export function describe(info: AppInfo): string {
  return `${info.name} ${info.version} (schema ${info.schema_version})`;
}
