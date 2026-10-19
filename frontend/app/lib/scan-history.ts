export interface ScanRecord {
  id?: number;
  workspace: string;
  timestamp: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
  total: number;
}

const DB_NAME = "sanctifier-scan-history";
const DB_VERSION = 1;
const STORE_NAME = "scans";
const MAX_RECORDS_PER_WORKSPACE = 20;

function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        const store = db.createObjectStore(STORE_NAME, {
          keyPath: "id",
          autoIncrement: true,
        });
        store.createIndex("workspace", "workspace", { unique: false });
        store.createIndex("timestamp", "timestamp", { unique: false });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

export async function saveScanRecord(record: Omit<ScanRecord, "id">): Promise<void> {
  const db = await openDB();
  const tx = db.transaction(STORE_NAME, "readwrite");
  const store = tx.objectStore(STORE_NAME);

  // Get existing count for this workspace
  const workspaceIndex = store.index("workspace");
  const countRequest = workspaceIndex.count(record.workspace);
  const existingIds: number[] = await new Promise((resolve, reject) => {
    const req = workspaceIndex.getAllKeys(record.workspace);
    req.onsuccess = () => resolve(req.result as number[]);
    req.onerror = () => reject(req.error);
  });

  // Remove oldest records if over the limit
  if (existingIds.length >= MAX_RECORDS_PER_WORKSPACE) {
    const excess = existingIds.length - MAX_RECORDS_PER_WORKSPACE + 1;
    // Sort by id (insertion order approximates chronological order)
    existingIds.sort((a, b) => a - b);
    for (let i = 0; i < excess; i++) {
      store.delete(existingIds[i]);
    }
  }

  store.add(record);
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

export async function getScanHistory(workspace: string): Promise<ScanRecord[]> {
  const db = await openDB();
  const tx = db.transaction(STORE_NAME, "readonly");
  const store = tx.objectStore(STORE_NAME);
  const workspaceIndex = store.index("workspace");

  return new Promise((resolve, reject) => {
    const request = workspaceIndex.getAll(workspace);
    request.onsuccess = () => {
      const records = (request.result as ScanRecord[]).sort(
        (a, b) => a.timestamp - b.timestamp
      );
      resolve(records);
    };
    request.onerror = () => reject(request.error);
  });
}

export async function getAllWorkspaces(): Promise<string[]> {
  const db = await openDB();
  const tx = db.transaction(STORE_NAME, "readonly");
  const store = tx.objectStore(STORE_NAME);
  const workspaceIndex = store.index("workspace");

  return new Promise((resolve, reject) => {
    const request = workspaceIndex.getAllKeys();
    request.onsuccess = () => {
      const keys = request.result as string[];
      resolve([...new Set(keys)]);
    };
    request.onerror = () => reject(request.error);
  });
}

export async function clearScanHistory(workspace?: string): Promise<void> {
  const db = await openDB();
  const tx = db.transaction(STORE_NAME, "readwrite");
  const store = tx.objectStore(STORE_NAME);

  if (workspace) {
    const workspaceIndex = store.index("workspace");
    const request = workspaceIndex.getAllKeys(workspace);
    const keys: number[] = await new Promise((resolve, reject) => {
      request.onsuccess = () => resolve(request.result as number[]);
      request.onerror = () => reject(request.error);
    });
    for (const key of keys) {
      store.delete(key);
    }
  } else {
    store.clear();
  }

  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}
