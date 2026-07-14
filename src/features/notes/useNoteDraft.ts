import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../../services/api";
import { AutosaveQueue } from "../../services/autosave";
import { noteToSaveInput, type Note, type SaveStatus } from "../../types";

function hasContent(note: Note) {
  return Boolean(
    note.title.trim() ||
      note.content.trim() ||
      note.tags.length ||
      note.categoryId ||
      note.isFavorite ||
      note.isTodo ||
      note.dueDate,
  );
}

export function useNoteDraft(note: Note, onSaved?: (note: Note) => void) {
  const [draft, setDraft] = useState(note);
  const [status, setStatus] = useState<SaveStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<Note | null>(null);
  const draftRef = useRef(note);
  const identityRef = useRef({
    id: note.id.startsWith("draft:") ? null : note.id,
    revision: note.id.startsWith("draft:") ? null : note.revision,
  });
  const onSavedRef = useRef(onSaved);
  onSavedRef.current = onSaved;

  const saveSnapshot = useCallback(async (snapshot: Note) => {
    if (!identityRef.current.id && !hasContent(snapshot)) {
      setStatus("idle");
      return;
    }
    setStatus("saving");
    setError(null);
    const input = noteToSaveInput(snapshot);
    input.id = identityRef.current.id;
    input.expectedRevision = identityRef.current.revision;
    let response;
    try {
      response = await api.saveNote(input);
    } catch (saveError) {
      setError(String(saveError));
      setStatus("error");
      throw saveError;
    }
    if (response.status === "conflict") {
      const conflictError = new Error("另一窗口已修改此笔记，请先处理编辑冲突");
      setConflict(response.note);
      setError(conflictError.message);
      setStatus("conflict");
      throw conflictError;
    }
    identityRef.current = { id: response.note.id, revision: response.note.revision };
    setDraft((current) => {
      const next = {
        ...current,
        id: response.note.id,
        revision: response.note.revision,
        createdAt: response.note.createdAt,
        updatedAt: response.note.updatedAt,
        tags: response.note.tags,
        category: response.note.category,
      };
      draftRef.current = next;
      return next;
    });
    setStatus("saved");
    onSavedRef.current?.({ ...response.note, ...draftRef.current, id: response.note.id, revision: response.note.revision });
  }, []);

  const queueRef = useRef<AutosaveQueue<Note> | null>(null);
  if (!queueRef.current) queueRef.current = new AutosaveQueue(saveSnapshot, 500);

  useEffect(() => {
    queueRef.current?.cancel();
    setDraft(note);
    draftRef.current = note;
    identityRef.current = {
      id: note.id.startsWith("draft:") ? null : note.id,
      revision: note.id.startsWith("draft:") ? null : note.revision,
    };
    setStatus("idle");
    setError(null);
    setConflict(null);
  }, [note]);

  const update = useCallback((patch: Partial<Note>) => {
    setDraft((current) => {
      const next = { ...current, ...patch };
      draftRef.current = next;
      setStatus("dirty");
      queueRef.current?.schedule(next);
      return next;
    });
  }, []);

  const flush = useCallback(async () => {
    await queueRef.current?.flush();
    return draftRef.current;
  }, []);

  const setComposing = useCallback((value: boolean) => queueRef.current?.setComposing(value), []);

  const loadLatest = useCallback(() => {
    if (!conflict) return;
    queueRef.current?.cancel();
    setDraft(conflict);
    draftRef.current = conflict;
    identityRef.current = { id: conflict.id, revision: conflict.revision };
    setConflict(null);
    setStatus("idle");
  }, [conflict]);

  const saveAsCopy = useCallback(() => {
    identityRef.current = { id: null, revision: null };
    setConflict(null);
    setStatus("dirty");
    queueRef.current?.schedule({ ...draftRef.current, id: `draft:${crypto.randomUUID()}`, revision: 0 });
  }, []);

  useEffect(() => () => queueRef.current?.cancel(), []);

  return { draft, update, flush, setComposing, status, error, conflict, loadLatest, saveAsCopy };
}
