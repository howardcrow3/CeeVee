import { invoke } from '@tauri-apps/api/core';

export interface AppInfo {
  name: string;
  version: string;
  schema_version: number;
}

/** Typed wrappers over Tauri commands. Replaced by generated bindings (tauri-specta) later. */
export const appInfo = () => invoke<AppInfo>('app_info');

export function describe(info: AppInfo): string {
  return `${info.name} ${info.version} (schema ${info.schema_version})`;
}
