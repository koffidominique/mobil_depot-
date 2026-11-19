import type { TurboModule } from 'react-native';
import { TurboModuleRegistry } from 'react-native';

/**
 * Contrat du TurboModule exposant le moteur Rust.
 * Une seule méthode JSON évite toute duplication des règles métier côté TypeScript.
 */
export interface Spec extends TurboModule {
  executeCommand(request: string): Promise<string>;
}

export default TurboModuleRegistry.getEnforcing<Spec>('DepotCore');
