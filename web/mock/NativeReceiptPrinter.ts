/** Impression simulée : le PDF de démonstration est téléchargé. */
import { previewFileContent } from '../shims/fs';

const Printer = {
  async printPdf(filePath: string) {
    const content = previewFileContent(filePath);
    if (content == null) return false;
    const blob = new Blob([content], { type: 'application/pdf' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = filePath.split('/').pop() ?? 'recu.pdf';
    document.body.appendChild(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(link.href), 1000);
    return true;
  },
};

export default Printer;
