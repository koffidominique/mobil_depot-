/**
 * Générateur PDF minimal (PDF 1.4, Helvetica) sans dépendance native.
 * Le fichier est 100 % ASCII : les offsets xref sont donc aussi des octets.
 * Aucun montant n'est recalculé ici : le reçu Rust fait autorité.
 */
export interface ReceiptPdfInput {
  depotName: string;
  receiptNumber: string;
  dateLabel: string;
  customer: string;
  lines: Array<{ label: string; quantity: string; amountFcfa: string }>;
  subtotalFcfa: string;
  discountFcfa: string;
  totalFcfa: string;
  paidFcfa: string;
  creditFcfa: string;
  packagingLines: string[];
  notes: string[];
}

function ascii(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^\x20-\x7E]/g, '?');
}

function escapePdf(value: string): string {
  return ascii(value).replace(/\\/g, '\\\\').replace(/\(/g, '\\(').replace(/\)/g, '\\)');
}

function wrap(value: string, max = 92): string[] {
  const words = ascii(value).split(' ');
  const result: string[] = [];
  let line = '';
  for (const word of words) {
    if ((`${line} ${word}`).trim().length > max) {
      if (line) result.push(line);
      line = word;
    } else {
      line = `${line} ${word}`.trim();
    }
  }
  if (line) result.push(line);
  return result;
}

export function buildReceiptPdf(input: ReceiptPdfInput): string {
  const textLines: string[] = [];
  const push = (value: string) => textLines.push(...wrap(value));
  push(`${input.depotName} - ${input.receiptNumber}`);
  push(`${input.dateLabel} - Client: ${input.customer}`);
  push(' ');
  for (const line of input.lines) {
    push(`${line.label} | ${line.quantity} | ${line.amountFcfa} FCFA`);
  }
  push(' ');
  push(`Sous-total: ${input.subtotalFcfa} FCFA`);
  push(`Remise: ${input.discountFcfa} FCFA`);
  push(`Total: ${input.totalFcfa} FCFA`);
  push(`Paye: ${input.paidFcfa} FCFA - Credit: ${input.creditFcfa} FCFA`);
  push(' ');
  push('Emballages:');
  for (const line of input.packagingLines) push(`- ${line}`);
  push(' ');
  for (const note of input.notes) push(note);

  let y = 790;
  const content = ['BT', '/F1 11 Tf', '14 TL', `48 ${y} Td`];
  for (const line of textLines) {
    content.push(`(${escapePdf(line)}) Tj`, 'T*');
    y -= 14;
    if (y < 60) {
      content.push('ET', 'BT', '/F1 11 Tf', '14 TL', '48 790 Td');
      y = 790;
    }
  }
  content.push('ET');
  const stream = `${content.join('\n')}\n`;

  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
    `<< /Length ${stream.length} >>\nstream\n${stream}endstream`,
  ];

  let pdf = '%PDF-1.4\n';
  const offsets: number[] = [];
  objects.forEach((body, index) => {
    offsets.push(pdf.length);
    pdf += `${index + 1} 0 obj\n${body}\nendobj\n`;
  });
  const xrefOffset = pdf.length;
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets) {
    pdf += `${String(offset).padStart(10, '0')} 00000 n \n`;
  }
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;
  return pdf;
}
