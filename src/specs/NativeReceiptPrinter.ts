import type { TurboModule } from 'react-native';
import { TurboModuleRegistry } from 'react-native';

/** Impression native d'un PDF de reçu. Aucune règle commerciale dans ce module. */
export interface Spec extends TurboModule {
  printPdf(filePath: string): Promise<boolean>;
}

export default TurboModuleRegistry.getEnforcing<Spec>('ReceiptPrinter');
