import RNFS from 'react-native-fs';
import NativeReceiptPrinter from '../specs/NativeReceiptPrinter';
import { buildReceiptPdf, type ReceiptPdfInput } from './receiptPdf';

/** Écrit le reçu PDF dans le répertoire Documents, puis le partage ou l'imprime. */
export async function writeReceiptPdf(input: ReceiptPdfInput): Promise<string> {
  const path = `${RNFS.DocumentDirectoryPath}/recu-${input.receiptNumber.replace(/[^A-Za-z0-9_-]/g, '_')}.pdf`;
  await RNFS.writeFile(path, buildReceiptPdf(input), 'ascii');
  return path;
}

export async function printReceiptPdf(path: string): Promise<boolean> {
  return NativeReceiptPrinter.printPdf(path);
}

export function pdfFileUrl(path: string): string {
  return `file://${path}`;
}
