import { state } from "./shared.js";

let deps = null;

export function initializeModelReorder(nextDeps) {
  deps = nextDeps;
}

export function moveModelWithinProtocol(uiId, direction) {
  const index = state.models.findIndex((model) => model.uiId === uiId);
  if (index === -1) {
    return;
  }

  const protocol = state.models[index].protocol;
  const candidateIndex =
    direction < 0
      ? findPreviousProtocolIndex(index, protocol)
      : findNextProtocolIndex(index, protocol);

  if (candidateIndex === -1) {
    return;
  }

  const [moved] = state.models.splice(index, 1);
  const insertIndex =
    direction < 0
      ? candidateIndex
      : candidateIndex > index
        ? candidateIndex
        : candidateIndex + 1;
  state.models.splice(insertIndex, 0, moved);
  deps.renderModels();
  deps.renderFastModelControls();
  deps.markStateChanged();
}

export function setDefaultModel(uiId) {
  // The editor records the user's default intent immediately for UX, but the
  // effective default is still normalized authoritatively by Rust preview.
  state.models.forEach((entry) => {
    entry.isDefault = entry.uiId === uiId;
  });
  deps.renderModels();
  deps.renderFastModelControls();
  deps.markStateChanged();
}

export function removeModel(uiId) {
  const index = state.models.findIndex((model) => model.uiId === uiId);
  if (index === -1) {
    return;
  }

  syncModelDragStateWithModels({ removedUiId: uiId });
  state.models.splice(index, 1);
  deps.ensureSelectedModel();
  deps.renderModels();
  deps.renderFastModelControls();
  deps.renderCatalog();
  deps.markStateChanged();
}

export function startModelDrag(sourceUiId) {
  cancelModelDrag();
  state.draggedModelUiId = sourceUiId;
  state.dragTargetModelUiId = null;
  state.dragDropPosition = null;
  document.body.classList.add("dragging-model");
  updateDraggingRowStyles();
  window.addEventListener("pointermove", handleModelDragMove);
  window.addEventListener("pointerup", finishModelDrag, { once: true });
}

export function cancelModelDrag() {
  window.removeEventListener("pointermove", handleModelDragMove);
  window.removeEventListener("pointerup", finishModelDrag);
  state.draggedModelUiId = null;
  state.dragTargetModelUiId = null;
  state.dragDropPosition = null;
  document.body.classList.remove("dragging-model");
  updateDraggingRowStyles();
  clearAllDropIndicators();
}

export function syncModelDragStateWithModels({ removedUiId = null } = {}) {
  if (
    !state.draggedModelUiId &&
    !state.dragTargetModelUiId &&
    !state.dragDropPosition
  ) {
    return;
  }

  if (
    removedUiId &&
    (state.draggedModelUiId === removedUiId ||
      state.dragTargetModelUiId === removedUiId)
  ) {
    cancelModelDrag();
    return;
  }

  const draggedModel = state.models.find(
    (model) => model.uiId === state.draggedModelUiId,
  );
  if (!draggedModel) {
    cancelModelDrag();
    return;
  }

  if (!state.dragTargetModelUiId) {
    return;
  }

  if (!canDropOnModel(state.draggedModelUiId, state.dragTargetModelUiId)) {
    cancelModelDrag();
  }
}

function reorderModelByDrop(sourceUiId, targetUiId, dropPosition) {
  if (!sourceUiId || !targetUiId || sourceUiId === targetUiId) {
    return;
  }

  const sourceIndex = state.models.findIndex(
    (model) => model.uiId === sourceUiId,
  );
  const targetIndex = state.models.findIndex(
    (model) => model.uiId === targetUiId,
  );
  if (sourceIndex === -1 || targetIndex === -1) {
    return;
  }

  const sourceModel = state.models[sourceIndex];
  const targetModel = state.models[targetIndex];
  if (sourceModel.protocol !== targetModel.protocol) {
    return;
  }

  const [moved] = state.models.splice(sourceIndex, 1);
  let insertIndex = state.models.findIndex(
    (model) => model.uiId === targetUiId,
  );
  if (insertIndex === -1) {
    return;
  }

  if (dropPosition === "after") {
    insertIndex += 1;
  }

  state.models.splice(insertIndex, 0, moved);
  deps.renderModels();
  deps.renderFastModelControls();
  deps.markStateChanged();
}

function findPreviousProtocolIndex(index, protocol) {
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    if (state.models[cursor].protocol === protocol) {
      return cursor;
    }
  }
  return -1;
}

function findNextProtocolIndex(index, protocol) {
  for (let cursor = index + 1; cursor < state.models.length; cursor += 1) {
    if (state.models[cursor].protocol === protocol) {
      return cursor;
    }
  }
  return -1;
}

function canDropOnModel(sourceUiId, targetUiId) {
  if (!sourceUiId || sourceUiId === targetUiId) {
    return false;
  }

  const dragged = state.models.find((model) => model.uiId === sourceUiId);
  const target = state.models.find((model) => model.uiId === targetUiId);
  return Boolean(dragged && target && dragged.protocol === target.protocol);
}

function getDropPosition(element, clientY) {
  const rect = element.getBoundingClientRect();
  return clientY < rect.top + rect.height / 2 ? "before" : "after";
}

function setDropIndicator(element, position) {
  element.classList.toggle("drop-before", position === "before");
  element.classList.toggle("drop-after", position === "after");
}

function clearDropIndicator(element) {
  element.classList.remove("drop-before", "drop-after");
}

function clearAllDropIndicators() {
  document
    .querySelectorAll(".model-row.drop-before, .model-row.drop-after")
    .forEach((element) => clearDropIndicator(element));
}

function handleModelDragMove(event) {
  if (!state.draggedModelUiId) {
    return;
  }

  const row = document
    .elementFromPoint(event.clientX, event.clientY)
    ?.closest(".model-row");
  clearAllDropIndicators();

  if (!row) {
    state.dragTargetModelUiId = null;
    state.dragDropPosition = null;
    return;
  }

  const targetUiId = row.dataset.uiId;
  if (!canDropOnModel(state.draggedModelUiId, targetUiId)) {
    state.dragTargetModelUiId = null;
    state.dragDropPosition = null;
    return;
  }

  const dropPosition = getDropPosition(row, event.clientY);
  state.dragTargetModelUiId = targetUiId;
  state.dragDropPosition = dropPosition;
  setDropIndicator(row, dropPosition);
}

function finishModelDrag() {
  const sourceUiId = state.draggedModelUiId;
  const targetUiId = state.dragTargetModelUiId;
  const dropPosition = state.dragDropPosition;

  cancelModelDrag();

  if (sourceUiId && targetUiId && dropPosition) {
    reorderModelByDrop(sourceUiId, targetUiId, dropPosition);
  }
}

function updateDraggingRowStyles() {
  document.querySelectorAll(".model-row").forEach((row) => {
    row.classList.toggle(
      "dragging-model-row",
      row.dataset.uiId === state.draggedModelUiId,
    );
  });
}
