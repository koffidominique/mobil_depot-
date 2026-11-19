import { previewFileContent } from '../shims/fs';

/** Partage navigateur : téléchargement du fichier simulé, sinon presse-papiers. */
export async function mockShare(options: { title?: string; message?: string; url?: string }) {
  const path = options.url?.replace(/^file:\/\//, '');
  const content = path ? previewFileContent(path) : null;
  if (path && content != null) {
    const extension = path.endsWith('.pdf') ? 'pdf' : 'csv';
    const blob = new Blob([content], { type: extension === 'pdf' ? 'application/pdf' : 'text/csv;charset=utf-8' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = path.split('/').pop() ?? `depotpro.${extension}`;
    document.body.appendChild(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(link.href), 1000);
    return;
  }
  const text = options.message ?? options.title ?? '';
  if (navigator.clipboard) await navigator.clipboard.writeText(text);
}
