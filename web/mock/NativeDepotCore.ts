/** TurboModule Rust simulé dans le navigateur (aperçu uniquement). */
import { executePreviewCommand } from './engine';

const NativeDepotCore = {
  async executeCommand(request: string): Promise<string> {
    return executePreviewCommand(request);
  },
};

export default NativeDepotCore;
