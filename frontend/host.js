import {
  APP_MENU_COMMAND_EVENT,
  SETTINGS_FILE_OPENED_EVENT,
  askDialog,
  invoke,
  openDialog,
  saveDialog,
  tauriEvent,
  tauriWebviewWindow,
  tauriWindow,
} from "./shared.js";

export async function openSettingsPathDialog() {
  if (!openDialog) {
    return undefined;
  }

  return openDialog({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "JSON settings",
        extensions: ["json"],
      },
    ],
  });
}

export async function chooseSavePath(defaultPath) {
  if (!saveDialog) {
    return undefined;
  }

  return saveDialog({
    defaultPath,
    filters: [
      {
        name: "JSON settings",
        extensions: ["json"],
      },
    ],
  });
}

export async function confirmDialog(
  message,
  { title, okLabel, kind = "warning" } = {},
) {
  if (askDialog) {
    return askDialog(message, {
      title,
      kind,
      okLabel,
      cancelLabel: "Cancel",
    });
  }

  return window.confirm(message);
}

export function registerOpenFileListener(onPath) {
  if (!tauriEvent?.listen) {
    return;
  }

  tauriEvent.listen(SETTINGS_FILE_OPENED_EVENT, async (event) => {
    const path = typeof event?.payload === "string" ? event.payload : null;
    if (path) {
      await onPath(path);
    }
  });
}

export function registerAppMenuListener(onCommand) {
  if (!tauriEvent?.listen) {
    return;
  }

  tauriEvent.listen(APP_MENU_COMMAND_EVENT, async (event) => {
    const command = typeof event?.payload === "string" ? event.payload : null;
    if (command) {
      await onCommand(command);
    }
  });
}

export function registerWindowDropListener(onPath) {
  const currentWindow = tauriWebviewWindow?.getCurrentWebviewWindow?.();
  if (!currentWindow?.onDragDropEvent) {
    return;
  }

  currentWindow.onDragDropEvent(async (event) => {
    if (event?.payload?.type !== "drop") {
      return;
    }

    const path = Array.isArray(event.payload.paths)
      ? event.payload.paths[0]
      : null;
    if (path) {
      await onPath(path);
    }
  });
}

export async function closeCurrentWindow() {
  if (invoke) {
    await invoke("quit_app");
    return;
  }

  const currentWindow = tauriWindow?.getCurrentWindow?.();
  if (currentWindow?.close) {
    await currentWindow.close();
    return;
  }

  const currentWebviewWindow = tauriWebviewWindow?.getCurrentWebviewWindow?.();
  if (currentWebviewWindow?.close) {
    await currentWebviewWindow.close();
    return;
  }

  window.close();
}
