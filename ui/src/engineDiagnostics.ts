import { diagnosticHelp } from "./labels";

export interface EngineDiagnosticPresentation {
  code: string;
  original: string;
  title?: string;
  fix?: string;
}

export interface EngineErrorPresentation {
  original: string;
  diagnostics: EngineDiagnosticPresentation[];
}

export function parseEngineError(original: string): EngineErrorPresentation {
  const diagnostics: EngineDiagnosticPresentation[] = [];
  for (const line of original.split(/\r?\n/)) {
    const match = /\b([EW]\d{3}):\s*(.*)/.exec(line);
    if (!match) continue;
    const code = match[1];
    const help = diagnosticHelp[code];
    diagnostics.push({
      code,
      original: match[2].trim(),
      title: help?.title,
      fix: help?.fix,
    });
  }
  return { original, diagnostics };
}
