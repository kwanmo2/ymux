import type { WorkspaceManager } from "../workspace/WorkspaceManager";
import { t } from "../i18n/i18n";

export interface CommandDef {
  id: string;
  label: () => string;
  keybinding?: string;
  action: () => void | Promise<void>;
}

export function builtinCommands(manager: WorkspaceManager): CommandDef[] {
  return [
    {
      id: "pane.splitH",
      label: () => t("shortcut.splitH"),
      keybinding: "Ctrl+Shift+H",
      action: () => void manager.splitFocused("horizontal"),
    },
    {
      id: "pane.splitV",
      label: () => t("shortcut.splitV"),
      keybinding: "Ctrl+Shift+V",
      action: () => void manager.splitFocused("vertical"),
    },
    {
      id: "pane.close",
      label: () => t("shortcut.close"),
      keybinding: "Ctrl+Shift+W",
      action: () => void manager.closeFocused(),
    },
    {
      id: "pane.zoom",
      label: () => t("shortcut.zoom"),
      keybinding: "Ctrl+Shift+Z",
      action: () => manager.toggleZoomFocused(),
    },
    {
      id: "pane.rename",
      label: () => t("shortcut.rename"),
      keybinding: "Ctrl+Shift+R",
      action: () => {
        const current = manager.getFocusedTitle() ?? "";
        const next = window.prompt(t("app.paneTitle"), current);
        if (next !== null) manager.renameFocused(next);
      },
    },
    {
      id: "pane.search",
      label: () => t("shortcut.search"),
      keybinding: "Ctrl+F",
      action: () => manager.toggleSearchOnFocused(),
    },
    {
      id: "pane.focusNext",
      label: () => t("shortcut.focusNext"),
      keybinding: "Ctrl+Tab",
      action: () => manager.cycleFocus(1),
    },
    {
      id: "pane.focusPrev",
      label: () => t("shortcut.focusPrev"),
      keybinding: "Ctrl+Shift+Tab",
      action: () => manager.cycleFocus(-1),
    },
    ...Array.from({ length: 9 }, (_, i) => ({
      id: `workspace.${i + 1}`,
      label: () => `${t("shortcut.workspace")} ${i + 1}`,
      keybinding: `Ctrl+Alt+${i + 1}`,
      action: () => void manager.activate(i + 1),
    })),
  ];
}

export function fuzzyMatch(query: string, text: string): boolean {
  const q = query.toLowerCase();
  const t = text.toLowerCase();
  let qi = 0;
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) qi++;
  }
  return qi === q.length;
}
