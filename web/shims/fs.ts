/** Mini système de fichiers en localStorage pour CSV/PDF de démonstration. */
const PREFIX = 'depotpro-preview-fs:';
export const DocumentDirectoryPath = '/depotpro-preview';

function read(key: string): string | null {
  return window.localStorage.getItem(PREFIX + key);
}

const RNFS = {
  DocumentDirectoryPath,
  async writeFile(_path: string, data: string, _encoding?: string) {
    window.localStorage.setItem(PREFIX + _path, data);
  },
  async readFile(path: string) {
    const data = read(path);
    if (data == null) throw new Error(`Fichier inexistant : ${path}`);
    return data;
  },
  async exists(path: string) {
    return read(path) != null;
  },
  async unlink(path: string) {
    window.localStorage.removeItem(PREFIX + path);
  },
  async mkdir() {},
  async readDir() { return []; },
  async stat(path: string) { return { size: (read(path) ?? '').length }; },
};

export function previewFileContent(path: string) { return read(path); }
export default RNFS;
