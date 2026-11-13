// Shapes mirrored from the Rust structs (serde JSON serialization).
// Detection.adapter / PanelConfig arrive from the Tauri commands in commands.rs.

export type EffortStyle =
  | { Flag: [string, string[]] } // flag name + allowed levels
  | { VariantSuffix: [string[]] } // model#variant levels
  | 'None';

export interface Adapter {
  id: string;
  name: string;
  exe: string;
  version_args: string;
  version_hint: string;
  launch: string;
  model_flag: string | null;
  effort: EffortStyle;
}

export interface Detection {
  adapter: Adapter;
  installed: boolean;
  version: string;
}

export interface AgentEntry {
  id: string;
  name: string;
  commandline: string;
  open_here_input: string;
  model: string | null;
  entry_order: number;
  enabled: boolean;
  manual: boolean;
}

export interface PanelConfig {
  version: number;
  agents: AgentEntry[];
  customize_commandline: string | null;
}

export interface ApplyResult {
  backup_path: string;
  backup_sha256: string;
  new_sha256: string;
}

/** Effort levels for an adapter, plus whether they are variant-suffix style. */
export function effortLevels(e: EffortStyle): { flag: string | null; levels: string[] } {
  if (e === 'None') return { flag: null, levels: [] };
  if ('Flag' in e) return { flag: e.Flag[0], levels: e.Flag[1] };
  return { flag: null, levels: e.VariantSuffix[0] };
}
