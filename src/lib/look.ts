export interface Look { skin?: string; cape?: string; slim: boolean }
const key = (id: string) => `cosmic.look.${id}`;

export function readLook(id: string): Look {
  try { return JSON.parse(localStorage.getItem(key(id)) ?? "") as Look; } catch { return { slim: false }; }
}
export function writeLook(id: string, l: Look) {
  localStorage.setItem(key(id), JSON.stringify(l));
}
